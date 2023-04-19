use cosmwasm_std::{
    entry_point, to_binary, Env, Deps, DepsMut,
    MessageInfo, Response, StdError, StdResult, Addr, CanonicalAddr,
    Binary, CosmosMsg, Uint128
};
use crate::error::ContractError;
use crate::msg::{ PaymentContractInfo, ExecuteMsg, PackBuildMsg, InstantiateMsg, QueryMsg, HistoryToken };
use crate::state::{ State, CONFIG_ITEM, LEVEL_ITEM, PAID_ADDRESSES_ITEM, ADMIN_ITEM, MY_ADDRESS_ITEM, PREFIX_REVOKED_PERMITS, HISTORY_STORE};
use crate::rand::{sha_256};
use secret_toolkit::{
    snip20::{ transfer_msg },
    snip721::{
        batch_burn_nft_msg, register_receive_nft_msg, set_viewing_key_msg, nft_dossier_query, transfer_nft_msg, set_metadata_msg, ViewerInfo, MediaFile, Metadata, NftDossier, Burn
    },
    permit::{validate, Permit, RevokedPermits}
};  
pub const BLOCK_SIZE: usize = 256;


#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg
) -> Result<Response, StdError> {
    let prng_seed: Vec<u8> = sha_256(base64::encode(msg.entropy).as_bytes()).to_vec();
    let viewing_key = base64::encode(&prng_seed);

    // create initial state
    let state = State { 
        viewing_key: Some(viewing_key),
        owner: info.sender.clone(),  
        nft_contract: msg.nft_contract, 
        valid_payments: msg.valid_payments, 
        receiving_address: msg.receiving_address,
        total_burned: 0,
        pack_max: msg.pack_max,
        collection_size: msg.collection_size,
        level_cap: msg.level_cap,
        is_payment_needed: msg.is_payment_needed
    }; 

    //Save Contract state
    CONFIG_ITEM.save(deps.storage, &state)?;
    LEVEL_ITEM.save(deps.storage, &msg.levels)?;
    ADMIN_ITEM.save(deps.storage, &deps.api.addr_canonicalize(&info.sender.to_string())?)?;
    MY_ADDRESS_ITEM.save(deps.storage,  &deps.api.addr_canonicalize(&_env.contract.address.to_string())?)?;
 
 
    let mut response_msgs: Vec<CosmosMsg> = Vec::new();
   
    deps.api.debug(&format!("Contract was initialized by {}", info.sender));
     
    let vk = state.viewing_key.unwrap();

    response_msgs.push(register_receive_nft_msg(
        _env.contract.code_hash.clone(),
        Some(true),
        None,
        BLOCK_SIZE,
        state.nft_contract.code_hash.clone(),
        state.nft_contract.address.clone().to_string(),
    )?);
    response_msgs.push(set_viewing_key_msg(
        vk.to_string(),
        None,
        BLOCK_SIZE,
        state.nft_contract.code_hash,
        state.nft_contract.address.to_string(),
    )?);

    if msg.valid_payments.is_some(){
        for valid_payment in msg.valid_payments.unwrap().iter() {  
            response_msgs.push(
                set_viewing_key_msg(
                    vk.to_string(),
                    None,
                    BLOCK_SIZE,
                    valid_payment.code_hash.to_string(),
                    valid_payment.address.to_string(),
                )?
            );
        }
    } 
     
    Ok(Response::new().add_messages(response_msgs)) 
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg
) -> Result<Response, ContractError> {
    match msg { 
        ExecuteMsg::RevokePermit { permit_name } => {
            try_revoke_permit(deps, &info.sender, &permit_name)
        },
        ExecuteMsg::BatchReceiveNft { from, token_ids, msg } => {
            try_batch_receive(deps, _env, &info.sender, &from, token_ids, msg)
        },
        ExecuteMsg::Receive {
            sender,
            from,
            amount,
            msg
        } => receive(deps, _env, &info.sender, &sender, &from, amount, msg)
    }
} 

fn receive(
    deps: DepsMut,
    _env: Env,
    info_sender: &Addr,
    sender: &Addr,//for snip 20 sender and from are the same. Wth??
    from: &Addr,
    amount: Uint128,
    msg: Option<Binary>
) -> Result<Response, ContractError> { 
    deps.api.debug(&format!("Receive received"));
    let state = CONFIG_ITEM.load(deps.storage)?;

    let payment_contract = state.valid_payments.unwrap().iter().find(|&x| &x.address == sender);
    if payment_contract.is_some(){
        return Err(ContractError::CustomError {val: from.to_string() + &" Address is not correct snip contract".to_string()});  
    } 
    let info_sender_raw = deps.api.addr_canonicalize(&info_sender.to_string())?;
    let mut paid_addresses = PAID_ADDRESSES_ITEM.load(deps.storage)?;
    if paid_addresses.iter().any(|&x| x == info_sender_raw){
        return Err(ContractError::CustomError {val: info_sender.to_string() + &" Address is already building".to_string()});  
    } 
    
    paid_addresses.push(info_sender_raw);

    PAID_ADDRESSES_ITEM.save(deps.storage, &paid_addresses)?;
    
    Ok(Response::new()
    .add_message(transfer_msg(
        state.receiving_address.to_string(),
        amount,
        None,
        None,
        BLOCK_SIZE,
        payment_contract.unwrap().code_hash.to_string(),
        payment_contract.unwrap().address.to_string(),
        )?)
    )
}

fn try_batch_receive(
    deps: DepsMut,
    _env: Env,
    sender: &Addr,
    from: &Addr,
    token_ids: Vec<String>,
    msg: Option<Binary>,
) -> Result<Response, ContractError> { 
    deps.api.debug(&format!("Batch received"));
    //TODO: update and calculate DAO score
    // save trait info for the main nft
    //
    let mut response_msgs: Vec<CosmosMsg> = Vec::new();
    let mut response_attrs = vec![];
    let mut state = CONFIG_ITEM.load(deps.storage)?;   
    let mut levels = LEVEL_ITEM.load(deps.storage)?;   
    let mut paid_addresses = PAID_ADDRESSES_ITEM.load(deps.storage)?;

    let raw_address = &deps.api.addr_canonicalize(&from.to_string())?;
    let position = paid_addresses.iter().position(|x| x == raw_address);
    

    // Check is payment is needed and if it is check if payment was received
    if state.is_payment_needed && position.is_none(){
        return Err(ContractError::CustomError {val: "Payment not received".to_string()});  
    }
    else{ 
        paid_addresses.remove(position.unwrap());
    }

    if let Some(bin) = msg { 
     let bytes = base64::decode(bin.to_base64()).unwrap();
     let pmsg: PackBuildMsg = serde_json::from_slice(&bytes).unwrap();

    //Check to make sure main_token_id exists in list and remove from the list
    let pos = token_ids.iter().position(|&x| x == pmsg.main_token_id);
    if pos.is_none(){
        return Err(ContractError::CustomError {val: "Main Token is not in the list".to_string()});  
    }
    else{
        token_ids.remove(pos.unwrap());
    }
    
     
     if sender == &state.nft_contract.address{ 

        let history_store = HISTORY_STORE.add_suffix(from.to_string().as_bytes());
       
        // Get viewing key for NFTs
        let viewer = Some(ViewerInfo {
            address: _env.contract.address.to_string(),
            viewing_key: state.viewing_key.as_ref().unwrap().to_string(),
        });
 
        let mut public_media_to_add: Vec<MediaFile> = Vec::new();
        let mut private_media_to_add: Vec<MediaFile> = Vec::new();
        let mut xp_total: i32 = 0;
        let mut pack_rank_total: i32 = 0;

        for token_id in token_ids.iter() { 
            let wolf_meta: NftDossier =  nft_dossier_query(
                deps.querier,
                token_id.to_string(),
                viewer.clone(),
                None,
                BLOCK_SIZE,
                state.nft_contract.code_hash.clone(),
                state.nft_contract.address.to_string(),
            )?;
            let current_xp_trait = wolf_meta.public_metadata.unwrap().extension.unwrap().attributes.as_ref().unwrap().iter().find(|&x| x.trait_type == Some("XP".to_string())).unwrap();
            xp_total = xp_total + current_xp_trait.value.parse::<i32>().unwrap();
            //TODO: calculate pack rank
            //pack_rank_total = pack_rank_total + (state.collection_size - )
            public_media_to_add.push(wolf_meta.public_metadata.unwrap().extension.unwrap().media.unwrap()[0]);
            private_media_to_add.push(wolf_meta.private_metadata.unwrap().extension.unwrap().media.unwrap()[0]);
        }
        //Burn nfts that are not the main token
        let mut burns: Vec<Burn> = Vec::new(); 
        burns.push(
            Burn{ 
                token_ids: token_ids.clone(),
                memo: None
            }
        );

        let cosmos_batch_msg = batch_burn_nft_msg(
            burns,
            None,
            BLOCK_SIZE,
            state.nft_contract.code_hash.clone(),
            state.nft_contract.address.to_string(),
        )?;
        response_msgs.push(cosmos_batch_msg);

        // Add images to the master nft and update xp
        let group_master_meta: NftDossier =  nft_dossier_query(
            deps.querier,
            pmsg.main_token_id.to_string(),
            viewer.clone(),
            None,
            BLOCK_SIZE,
            state.nft_contract.code_hash.clone(),
            state.nft_contract.address.to_string(),
        )?;

        state.total_burned = state.total_burned + token_ids.len()as i32;

        PAID_ADDRESSES_ITEM.save(deps.storage, &paid_addresses)?;
        CONFIG_ITEM.save(deps.storage, &state)?;

        //update public metadata first
        let new_public_ext = 
                if let Some(Metadata { extension, .. }) = group_master_meta.public_metadata {
                    if let Some(mut ext) = extension {  
                        for media in public_media_to_add.iter() { 
                            ext.media.unwrap().push(*media);
                        } 
                        let current_xp_trait = ext.attributes.as_ref().unwrap().iter().find(|&x| x.trait_type == Some("XP".to_string())).unwrap();
                        let current_xp = current_xp_trait.value.parse::<i32>().unwrap() + xp_total;
                        let current_lvl_trait = ext.attributes.as_ref().unwrap().iter().find(|&x| x.trait_type == Some("LVL".to_string())).unwrap();
                        let current_lvl = current_lvl_trait.value.parse::<i32>().unwrap();
                        for attr in ext.attributes.as_mut().unwrap().iter_mut() {

                            if attr.trait_type == Some("XP".to_string()) {
                                attr.value = current_xp.to_string();
                            }  
                            if attr.trait_type == Some("Pack".to_string()) {
                                let new_pack_size = token_ids.len()as i32 + attr.value.parse::<i32>().unwrap();
                                attr.value = new_pack_size.to_string(); 
                            }

                            if attr.trait_type == Some("Pack Rank".to_string()) {
                                let new_pack_rank = pack_rank_total + attr.value.parse::<i32>().unwrap();
                                attr.value = new_pack_rank.to_string(); 
                            }

                            if attr.trait_type == Some("LVL".to_string()) {
                                let shouldbe_lvl = if attr.value.parse::<i32>().unwrap() < state.level_cap {
                                        levels.iter().find(|&x| x.xp_needed > current_xp).unwrap().level - 1
                                    } 
                                    else { 
                                        attr.value.parse::<i32>().unwrap() 
                                    }; 
                                attr.value = shouldbe_lvl.to_string(); 

                                if shouldbe_lvl > current_lvl {
                                    response_attrs.push(("lvl_increase".to_string(), shouldbe_lvl.to_string()));
                                }
                            }  
                        }
                        ext 
                   }
                    else {
                        return Err(ContractError::CustomError {val: "unable to set metadata with uri".to_string()});
                    }
                } 
                else {
                    return Err(ContractError::CustomError {val: "unable to get metadata from nft contract".to_string()});
                };
             
                //update public metadata first
        let new_privat_ext = 
                if let Some(Metadata { extension, .. }) = group_master_meta.public_metadata {
                    if let Some(mut ext) = extension {  
                        for media in private_media_to_add.iter() { 
                            ext.media.unwrap().push(media.clone());
                        }
                        ext 
                   }
                    else {
                        return Err(ContractError::CustomError {val: "unable to set metadata with uri".to_string()});
                    }
                } 
                else {
                    return Err(ContractError::CustomError {val: "unable to get metadata from nft contract".to_string()});
                };
        // //add metadata update to responses
        // let cosmos_msg = set_metadata_msg(
        //     inholding_nft.token_id.to_string(),
        //     Some(Metadata {
        //         token_uri: None,
        //         extension: Some(new_ext),
        //     }),
        //     None,
        //     None,
        //     BLOCK_SIZE,
        //     state.wolf_pack_contract.code_hash.clone(),
        //     state.wolf_pack_contract.address.to_string()
        // )?;
        // response_msgs.push(cosmos_msg); 

        // // ensure that the NFT exists and is owned by the contract
        // if wolf_meta.owner.unwrap() != _env.contract.address.to_string() {
        //     return Err(ContractError::CustomError {val: "Wolf not owned by contract".to_string()}); 
        // }

        // response_attrs.push(("xp_boost_amount".to_string(), xp_boost.to_string()));

        // // add transfer update to responses
        // let cosmos_transfer_msg = transfer_nft_msg(
        //     inholding_nft.owner.to_string(),
        //     inholding_nft.token_id.to_string(),
        //     None,
        //     None,
        //     BLOCK_SIZE,
        //     state.wolf_pack_contract.code_hash.to_string(),
        //     state.wolf_pack_contract.address.to_string()
        // )?;
        // response_msgs.push(cosmos_transfer_msg);
        // //enter history record
        // let history_token: HistoryToken = { HistoryToken {
        //     wolf_token_id: inholding_nft.token_id.to_string(),
        //     powerup_token_ids: token_ids.clone(),
        //     power_up_date: Some(_env.block.time.seconds()), 
        //     power_up_amount: xp_boost
        // }};
        
        // history_store.push(deps.storage, &history_token)?;

        // //update state
        // state.total_power_ups = state.total_power_ups + 1;
        // state.total_xp_boost = state.total_xp_boost + xp_boost;
        // state.total_bones_used = state.total_bones_used + token_ids.len()as i32;
        
        // CONFIG_ITEM.save(deps.storage, &state)?;
        // //remove nft from inholding store
        // INHOLDING_NFT_STORE.remove(deps.storage, &deps.api.addr_canonicalize(&from.to_string())?)?;
     }
     else{
        return Err(ContractError::CustomError {val: "Not a valid contract address".to_string()});
     }

       
     }
   
   else{
    return Err(ContractError::CustomError {val: "Invalid message received".to_string()});
   }   

 
   Ok(Response::new().add_messages(response_msgs).add_attributes(response_attrs))
    
}


pub fn try_send_nft_back(
    deps: DepsMut,
    _env: Env,
    sender: &Addr,
    token_id: String,
    owner: Addr
) -> Result<Response, ContractError> { 
    let state = CONFIG_ITEM.load(deps.storage)?;

    if sender.clone() != state.owner {
        return Err(ContractError::CustomError {val: "You don't have the permissions to execute this command".to_string()});
    }  
 
    Ok(Response::new()
        .add_message(transfer_nft_msg(
            owner.to_string(),
            token_id.to_string(),
            None,
            None,
            BLOCK_SIZE,
            state.nft_contract.code_hash.to_string(),
            state.nft_contract.address.to_string()
        )?)
    )
}

fn try_revoke_permit(
    deps: DepsMut,
    sender: &Addr,
    permit_name: &str,
) -> Result<Response, ContractError> {
    RevokedPermits::revoke_permit(deps.storage, PREFIX_REVOKED_PERMITS, &sender.to_string(), permit_name);
    
    Ok(Response::default())
}

#[entry_point]
pub fn query(
    deps: Deps,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {   
    }
}

 

fn get_querier(
    deps: Deps,
    permit: Permit,
) -> StdResult<(CanonicalAddr, Option<CanonicalAddr>)> {
    if let pmt = permit {
        let me_raw: CanonicalAddr = MY_ADDRESS_ITEM.load(deps.storage)?;
        let my_address = deps.api.addr_humanize(&me_raw)?;
        let querier = deps.api.addr_canonicalize(&validate(
            deps,
            PREFIX_REVOKED_PERMITS,
            &pmt,
            my_address.to_string(),
            None
        )?)?;
        if !pmt.check_permission(&secret_toolkit::permit::TokenPermissions::Owner) {
            return Err(StdError::generic_err(format!(
                "Owner permission is required for history queries, got permissions {:?}",
                pmt.params.permissions
            )));
        }
        return Ok((querier, Some(me_raw)));
    }
    return Err(StdError::generic_err(
        "Unauthorized",
    ));  
}

