use cosmwasm_std::{
    entry_point, from_binary, to_binary, Env, Deps, DepsMut,
    MessageInfo, Response, StdError, StdResult, Addr, CanonicalAddr,
    Binary, CosmosMsg, Uint128
};
use crate::error::ContractError;
use crate::msg::{ HandleReceiveMsg, ExecuteMsg, PackBuildMsg, PackTransferMsg, InstantiateMsg, QueryMsg, HistoryToken, PackMain, PackMember, BuildInfoResponse };
use crate::state::{ State, CONFIG_ITEM, LEVEL_ITEM, PAID_ADDRESSES_ITEM, RANK_STORE, PACK_MAIN_STORE, PACK_MEMBER_STORE, ADMIN_ITEM, MY_ADDRESS_ITEM, PREFIX_REVOKED_PERMITS, HISTORY_STORE};
use crate::rand::{sha_256};
use secret_toolkit::{
    snip20::{ transfer_msg },
    snip721::{
        batch_transfer_nft_msg, batch_burn_nft_msg, register_receive_nft_msg, set_viewing_key_msg, nft_dossier_query, transfer_nft_msg, set_metadata_msg, Transfer, Trait, ViewerInfo, MediaFile, Metadata, NftDossier, Burn
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
        valid_payments: msg.valid_payments.clone(), 
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
    PAID_ADDRESSES_ITEM.save(deps.storage, &Vec::new())?;

    for rank in msg.ranks.iter() {
        RANK_STORE.insert(deps.storage, &rank.token_id, &rank.rank)?;
    }
 
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

    if let Some(valid_payments) = &msg.valid_payments{
        for valid_payment in valid_payments.iter() {  
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
        } => receive(deps, _env, &info.sender, &sender, &from, amount, msg),
        ExecuteMsg::SendNftBack { token_id, owner } => {
            try_send_nft_back(deps, _env, &info.sender, token_id, owner)
        }
    }
} 

fn receive(
    deps: DepsMut,
    _env: Env,
    info_sender: &Addr,//snip contract
    sender: &Addr,//for snip 20 sender and from are the same. Wth??
    from: &Addr,//user
    amount: Uint128,
    msg: Option<Binary>
) -> Result<Response, ContractError> { 
    deps.api.debug(&format!("Receive received"));
    let state = CONFIG_ITEM.load(deps.storage)?;
    let payment_contract = state.valid_payments.as_ref().unwrap().iter().find(|&x| &x.address == info_sender);
    if !payment_contract.is_some(){
        return Err(ContractError::CustomError {val: info_sender.to_string() + &" Address is not correct snip contract".to_string()});  
    }  

    if payment_contract.unwrap().payment_needed != amount {
        return Err(ContractError::CustomError {val: "You've sent the wrong amount".to_string()});  
    }

    let sender_raw = deps.api.addr_canonicalize(&sender.to_string())?; 
    let mut paid_addresses = PAID_ADDRESSES_ITEM.load(deps.storage)?;

    if paid_addresses.iter().any(|x| x == &sender_raw){
        return Err(ContractError::CustomError {val: sender.to_string() + &" Address is already building".to_string()});  
    } 
    
    paid_addresses.push(sender_raw);

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
    //TODO: 
    // Function to move wolf to another alpha 
    // function to add/remove payment method and if payment is needed
    // Queries
    //   - Get History, Get distinct traits, Get Pack info, Get Leaderboard

    if let Some(bin_msg) = msg {
        match from_binary(&bin_msg)? {
            HandleReceiveMsg::ReceivePackBuild{ pack_build } => join_pack(
                _env,
                deps,
                sender,
                from, 
                token_ids,
                pack_build
            ),
            HandleReceiveMsg::ReceiveTransferBuild{ transfer_build } => transfer_pack(
                _env,
                deps,
                sender,
                from, 
                token_ids,
                transfer_build
            )
        }
    } else {
        return Err(ContractError::CustomError {val: "data should be given".to_string()});
    }
}

pub fn join_pack(
    _env: Env,
    deps: DepsMut,
    sender: &Addr,
    from: &Addr,
    token_ids: Vec<String>, 
    pmsg: PackBuildMsg
) -> Result<Response, ContractError> {
    let mut token_ids_mut: Vec<String> = token_ids;
    let mut response_msgs: Vec<CosmosMsg> = Vec::new();
    let mut response_attrs = vec![];
    let mut state = CONFIG_ITEM.load(deps.storage)?;   
    let levels = LEVEL_ITEM.load(deps.storage)?;   
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

    let mut pack_members = PACK_MEMBER_STORE.get(deps.storage, &pmsg.main_token_id).unwrap_or_else(Vec::new);

    //Check to make sure main_token_id exists in list and remove from the list
    let pos = token_ids_mut.iter().position(|x| x == &pmsg.main_token_id);
    if pos.is_none(){
        return Err(ContractError::CustomError {val: "Main Token is not in the list".to_string()});  
    }
    else{
        token_ids_mut.remove(pos.unwrap());
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
        let mut xp_total: u32 = 0;
        let mut pack_rank_total: u32 = 0;

        for token_id in token_ids_mut.iter() { 
            let rank: u16 = RANK_STORE.get(deps.storage, &token_id)
            .ok_or_else(|| StdError::generic_err("Rank pool doesn't have token"))?;

            let wolf_meta: NftDossier =  nft_dossier_query(
                deps.querier,
                token_id.to_string(),
                viewer.clone(),
                None,
                BLOCK_SIZE,
                state.nft_contract.code_hash.clone(),
                state.nft_contract.address.to_string(),
            )?;
            let alpha_trait = wolf_meta.public_metadata.as_ref().unwrap().extension.as_ref().unwrap().attributes.as_ref().unwrap().iter().find(|&x| x.trait_type == Some("Alpha".to_string()));
            if alpha_trait.is_some(){
                return Err(ContractError::CustomError {val: "You can't combine two Alphas".to_string()});  
            } 
            let pub_attributes = wolf_meta.public_metadata.as_ref().unwrap().extension.as_ref().unwrap().attributes.as_ref().unwrap().clone();
            let current_xp_trait = wolf_meta.public_metadata.as_ref().unwrap().extension.as_ref().unwrap().attributes.as_ref().unwrap().iter().find(|&x| x.trait_type == Some("XP".to_string())).unwrap();
            xp_total = xp_total + current_xp_trait.value.parse::<u32>().unwrap();
            pack_rank_total = pack_rank_total + (state.collection_size - rank) as u32;
            //check that lvl/xp is high enough to be added to the pack
            if current_xp_trait.value.parse::<u32>().unwrap() < 464{
                return Err(ContractError::CustomError {val: "Wolf's level is not high enough".to_string()});  
            }
            public_media_to_add.push(wolf_meta.public_metadata.unwrap().extension.unwrap().media.unwrap().first().unwrap().clone());
            private_media_to_add.push(wolf_meta.private_metadata.unwrap().extension.unwrap().media.unwrap().first().unwrap().clone());
            
            pack_members.push(PackMember{
                token_id: token_id.to_string(),
                rank:  rank,
                attributes: pub_attributes
            });
            PACK_MEMBER_STORE.insert(deps.storage, &pmsg.main_token_id, &pack_members)?;
        }
        //Burn nfts that are not the main token
        let mut burns: Vec<Burn> = Vec::new(); 
        burns.push(
            Burn{ 
                token_ids: token_ids_mut.clone(),
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

        // ------------------------------------------
        //        Master NFT IMAGE AND XP UPDATE
        // ------------------------------------------
        let group_master_meta: NftDossier =  nft_dossier_query(
            deps.querier,
            pmsg.main_token_id.to_string(),
            viewer.clone(),
            None,
            BLOCK_SIZE,
            state.nft_contract.code_hash.clone(),
            state.nft_contract.address.to_string(),
        )?;

        state.total_burned = state.total_burned + token_ids_mut.len()as u16;

        PAID_ADDRESSES_ITEM.save(deps.storage, &paid_addresses)?; 

        //update public metadata first
        let new_public_ext = 
                if let Some(Metadata { extension, .. }) = group_master_meta.public_metadata {
                    if let Some(mut ext) = extension {  
                        //update name field
                        ext.name = Some(pmsg.name.to_string());
                        //add new images 
                        for media in public_media_to_add.iter() { 
                            ext.media.as_mut().unwrap().push(media.clone());
                        } 
                        let alpha_trait = ext.attributes.as_ref().unwrap().iter().find(|&x| x.trait_type == Some("Alpha".to_string()));
                        if !alpha_trait.is_some(){
                            return Err(ContractError::CustomError {val: "The main token id is not an Alpha".to_string()});  
                        }  

                        let current_xp_trait = ext.attributes.as_ref().unwrap().iter().find(|&x| x.trait_type == Some("XP".to_string())).unwrap();
                        let current_xp = current_xp_trait.value.parse::<u32>().unwrap() + xp_total;
                        let current_lvl_trait = ext.attributes.as_ref().unwrap().iter().find(|&x| x.trait_type == Some("LVL".to_string())).unwrap();
                        let current_lvl = current_lvl_trait.value.parse::<u16>().unwrap();

                        //add pack rank attribute to the public metadata if it doesnt exist
                        if !ext.attributes.as_mut().unwrap().iter().any(|x| x.trait_type == Some("Pack Rank".to_string())){
                            ext.attributes.as_mut().unwrap().push(Trait{
                                trait_type: Some("Pack Rank".to_string()),
                                value: "0".to_string(),
                                display_type: None,
                                max_value: None
                            });
                        } 

                        let mut new_pack_size:u16 = 0;
                        let mut new_pack_rank:u32 = 0;

                        for attr in ext.attributes.as_mut().unwrap().iter_mut() {

                            if attr.trait_type == Some("XP".to_string()) {
                                attr.value = current_xp.to_string();
                            }  
                            if attr.trait_type == Some("Pack".to_string()) {
                                new_pack_size = token_ids_mut.len()as u16 + attr.value.parse::<u16>().unwrap();
                                attr.value = new_pack_size.to_string(); 
                            }

                            if attr.trait_type == Some("Pack Rank".to_string()) {
                                new_pack_rank = pack_rank_total + attr.value.parse::<u32>().unwrap();
                                attr.value = new_pack_rank.to_string();  
                            }

                            if attr.trait_type == Some("LVL".to_string()) {
                                let shouldbe_lvl = if attr.value.parse::<u16>().unwrap() < state.level_cap {
                                        levels.iter().find(|&x| x.xp_needed > current_xp).unwrap().level - 1
                                    } 
                                    else { 
                                        attr.value.parse::<u16>().unwrap() 
                                    }; 
                                attr.value = shouldbe_lvl.to_string(); 

                                if shouldbe_lvl > current_lvl {
                                    response_attrs.push(("lvl_increase".to_string(), shouldbe_lvl.to_string()));
                                }
                            }  
                        }
                        
                        //update store for the leaderboard
                        PACK_MAIN_STORE.insert(deps.storage, &pmsg.main_token_id, &PackMain{
                            token_id: pmsg.main_token_id.to_string(),
                            pack_rank:  new_pack_rank,
                            pack_count: new_pack_size,
                            name: pmsg.name.to_string()
                        })?;

                        ext 
                   }
                    else {
                        return Err(ContractError::CustomError {val: "unable to set metadata with uri".to_string()});
                    }
                } 
                else {
                    return Err(ContractError::CustomError {val: "unable to get metadata from nft contract".to_string()});
                };
             
   
        let new_privat_ext = 
                if let Some(Metadata { extension, .. }) = group_master_meta.private_metadata {
                    if let Some(mut ext) = extension {  
                        for media in private_media_to_add.iter() { 
                            ext.media.as_mut().unwrap().push(media.clone());
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
        //add metadata update to responses
        let cosmos_msg = set_metadata_msg(
            pmsg.main_token_id.to_string(),
            Some(Metadata {
                token_uri: None,
                extension: Some(new_public_ext),
            }),
            Some(Metadata {
                token_uri: None,
                extension: Some(new_privat_ext),
            }), 
            None,
            BLOCK_SIZE,
            state.nft_contract.code_hash.clone(),
            state.nft_contract.address.to_string()
        )?;
        response_msgs.push(cosmos_msg); 

        // add transfer update to responses
        let cosmos_transfer_msg = transfer_nft_msg(
            from.to_string(),
            pmsg.main_token_id.to_string(),
            None,
            None,
            BLOCK_SIZE,
            state.nft_contract.code_hash.to_string(),
            state.nft_contract.address.to_string()
        )?;
        response_msgs.push(cosmos_transfer_msg);
        //enter history record
        let history_token: HistoryToken = { HistoryToken {
            wolf_main_token_id: pmsg.main_token_id.to_string(),
            pack_member_token_ids: token_ids_mut.clone(),
            pack_build_date: Some(_env.block.time.seconds())
        }};
        
        history_store.push(deps.storage, &history_token)?;

        
        CONFIG_ITEM.save(deps.storage, &state)?; 
     }
     else{
        return Err(ContractError::CustomError {val: "Not a valid contract address".to_string()});
     }  
 
   Ok(Response::new().add_messages(response_msgs).add_attributes(response_attrs))
}

pub fn transfer_pack(
    _env: Env,
    deps: DepsMut,
    sender: &Addr,
    from: &Addr,
    token_ids: Vec<String>, 
    pmsg: PackTransferMsg
) -> Result<Response, ContractError> {
    let mut response_msgs: Vec<CosmosMsg> = Vec::new();  
    let state = CONFIG_ITEM.load(deps.storage)?;    
    let mut pack_member = PackMember{ token_id: "".to_string(), rank: 0, attributes: Vec::new()};
    // pub main_token_id: String,
    // pub transfer_to_token_id: String,
    // pub token_id: String
    // Check to make sure main_token_id exists in list and remove from the list
    if !token_ids.iter().any(|x| x == &pmsg.main_token_id){
        return Err(ContractError::CustomError {val: "Main Token is not in the list".to_string()}); 
    } 

    if !token_ids.iter().any(|x| x == &pmsg.transfer_to_token_id){
        return Err(ContractError::CustomError {val: "Transfer To Token is not in the list".to_string()}); 
    }
 

    let mut main_pack_members = PACK_MEMBER_STORE.get(deps.storage, &pmsg.main_token_id).ok_or_else(|| StdError::generic_err("This tokenid doesn't exist"))?;
    let mut transfer_to_pack_members = PACK_MEMBER_STORE.get(deps.storage, &pmsg.transfer_to_token_id).unwrap_or_else(Vec::new);
    let pos = main_pack_members.iter().position(|x| x.token_id == pmsg.token_id);
    if pos.is_none(){
        return Err(ContractError::CustomError {val: "Token is not a pack member".to_string()});  
    }
    else{
        pack_member = main_pack_members.remove(pos.unwrap());
        transfer_to_pack_members.push(pack_member.clone());
    }
    
        
    PACK_MEMBER_STORE.insert(deps.storage, &pmsg.main_token_id, &main_pack_members)?;
    PACK_MEMBER_STORE.insert(deps.storage, &pmsg.transfer_to_token_id, &transfer_to_pack_members)?;
     
 
    
    if sender == &state.nft_contract.address{ 

        let history_store = HISTORY_STORE.add_suffix(from.to_string().as_bytes());
       
        // Get viewing key for NFTs
        let viewer = Some(ViewerInfo {
            address: _env.contract.address.to_string(),
            viewing_key: state.viewing_key.as_ref().unwrap().to_string(),
        });

        let main_meta: NftDossier =  nft_dossier_query(
            deps.querier,
            pmsg.main_token_id.to_string(),
            viewer.clone(),
            None,
            BLOCK_SIZE,
            state.nft_contract.code_hash.clone(),
            state.nft_contract.address.to_string(),
        )?;

        let transfer_to_meta: NftDossier =  nft_dossier_query(
            deps.querier,
            pmsg.transfer_to_token_id.to_string(),
            viewer.clone(),
            None,
            BLOCK_SIZE,
            state.nft_contract.code_hash.clone(),
            state.nft_contract.address.to_string(),
        )?;
        let main_alpha_trait = main_meta.public_metadata.as_ref().unwrap().extension.as_ref().unwrap().attributes.as_ref().unwrap().iter().find(|&x| x.trait_type == Some("Alpha".to_string()));
        if main_alpha_trait.is_none(){
            return Err(ContractError::CustomError {val: "You can't do this action with a non-alpha".to_string()});  
        } 
        let transfer_to_alpha_trait = transfer_to_meta.public_metadata.as_ref().unwrap().extension.as_ref().unwrap().attributes.as_ref().unwrap().iter().find(|&x| x.trait_type == Some("Alpha".to_string()));
        if transfer_to_alpha_trait.is_none(){
            return Err(ContractError::CustomError {val: "You can't do this action with a non-alpha".to_string()});  
        }  
        let mut pub_media_file = MediaFile::default();
        let mut priv_media_file = MediaFile::default();

        //update public metadata first
        let new_public_ext = 
                if let Some(Metadata { extension, .. }) = main_meta.public_metadata {
                    if let Some(mut ext) = extension {  
                        //remove image of token being transfered
                        pub_media_file = ext.media.as_mut().unwrap().remove((pmsg.member_index+1)as usize);
                        //update rank
                        let mut new_pack_rank: u32 = 0;
                        let mut new_pack_size: u16 = 0;
                        
                        for attr in ext.attributes.as_mut().unwrap().iter_mut() {
 
                            if attr.trait_type == Some("Pack".to_string()) {
                                new_pack_size = attr.value.parse::<u16>().unwrap() - 1;
                                attr.value = new_pack_size.to_string(); 
                            }

                            if attr.trait_type == Some("Pack Rank".to_string()) { 
                                new_pack_rank = attr.value.parse::<u32>().unwrap() - (state.collection_size - pack_member.rank)as u32;
                                attr.value = new_pack_rank.to_string();  
                            } 
                        }
                        let mut pack_main = PACK_MAIN_STORE.get(deps.storage, &pmsg.main_token_id).unwrap();
                        pack_main.pack_rank = new_pack_rank;
                        pack_main.pack_count = new_pack_size;
             
                        //update store for the leaderboard
                        PACK_MAIN_STORE.insert(deps.storage, &pmsg.main_token_id, &pack_main)?;

                        ext 
                   }
                    else {
                        return Err(ContractError::CustomError {val: "unable to set metadata with uri".to_string()});
                    }
                } 
                else {
                    return Err(ContractError::CustomError {val: "unable to get metadata from nft contract".to_string()});
                };
             
   
        let new_private_ext = 
                if let Some(Metadata { extension, .. }) = main_meta.private_metadata {
                    if let Some(mut ext) = extension {  
                        priv_media_file = ext.media.as_mut().unwrap().remove(pmsg.member_index as usize);
                        ext 
                   }
                    else {
                        return Err(ContractError::CustomError {val: "unable to set metadata with uri".to_string()});
                    }
                } 
                else {
                    return Err(ContractError::CustomError {val: "unable to get metadata from nft contract".to_string()});
                };
 
        let mut pack_member_token_ids: Vec<String> = Vec::new();
        pack_member_token_ids.push(pmsg.token_id.clone());
        //enter history record
        let history_token: HistoryToken = { HistoryToken {
            wolf_main_token_id: pmsg.transfer_to_token_id.to_string(),
            pack_member_token_ids: pack_member_token_ids,
            pack_build_date: Some(_env.block.time.seconds())
        }};
        
        history_store.push(deps.storage, &history_token)?;

        //move member to new alpha
        let new_transfer_public_ext = 
        if let Some(Metadata { extension, .. }) = transfer_to_meta.public_metadata {
            if let Some(mut ext) = extension {   
                ext.media.as_mut().unwrap().push(pub_media_file.clone());
                //add pack rank attribute to the public metadata if it doesnt exist
                if !ext.attributes.as_mut().unwrap().iter().any(|x| x.trait_type == Some("Pack Rank".to_string())){
                    ext.attributes.as_mut().unwrap().push(Trait{
                        trait_type: Some("Pack Rank".to_string()),
                        value: "0".to_string(),
                        display_type: None,
                        max_value: None
                    });
                } 
                //update rank
                let mut new_pack_rank: u32 = 0;
                let mut new_pack_size: u16 = 0;
                
                for attr in ext.attributes.as_mut().unwrap().iter_mut() {

                    if attr.trait_type == Some("Pack".to_string()) {
                        new_pack_size = attr.value.parse::<u16>().unwrap() + 1;
                        attr.value = new_pack_size.to_string(); 
                    }

                    if attr.trait_type == Some("Pack Rank".to_string()) { 
                        new_pack_rank = attr.value.parse::<u32>().unwrap() + (state.collection_size - pack_member.rank) as u32;
                        attr.value = new_pack_rank.to_string();  
                    } 
                }
                let mut pack_transfer_to = PACK_MAIN_STORE.get(deps.storage, &pmsg.transfer_to_token_id)
                .unwrap_or(PackMain{
                    token_id: pmsg.transfer_to_token_id.to_string(),
                    pack_rank:  0,
                    pack_count: 0,
                    name: "".to_string()
                });
                pack_transfer_to.pack_rank = new_pack_rank;
                pack_transfer_to.pack_count = new_pack_size;
     
                //update store for the leaderboard 
                PACK_MAIN_STORE.insert(deps.storage, &pmsg.transfer_to_token_id, &pack_transfer_to)?;
                
                ext 
           }
            else {
                return Err(ContractError::CustomError {val: "unable to set metadata with uri".to_string()});
            }
        } 
        else {
            return Err(ContractError::CustomError {val: "unable to get metadata from nft contract".to_string()});
        };

        let new_transfer_privat_ext = 
        if let Some(Metadata { extension, .. }) = transfer_to_meta.private_metadata {
            if let Some(mut ext) = extension {  
                ext.media.as_mut().unwrap().push(priv_media_file.clone());
                ext 
           }
            else {
                return Err(ContractError::CustomError {val: "unable to set metadata with uri".to_string()});
            }
        } 
        else {
            return Err(ContractError::CustomError {val: "unable to get metadata from nft contract".to_string()});
        };

                //add metadata update to responses 
                response_msgs.push(
                    set_metadata_msg(
                        pmsg.main_token_id.to_string(),
                        Some(Metadata {
                            token_uri: None,
                            extension: Some(new_public_ext),
                        }),
                        Some(Metadata {
                            token_uri: None,
                            extension: Some(new_private_ext),
                        }), 
                        None,
                        BLOCK_SIZE,
                        state.nft_contract.code_hash.clone(),
                        state.nft_contract.address.to_string()
                    )?
                );  
                response_msgs.push(
                    set_metadata_msg(
                        pmsg.transfer_to_token_id.to_string(),
                        Some(Metadata {
                            token_uri: None,
                            extension: Some(new_transfer_public_ext),
                        }),
                        Some(Metadata {
                            token_uri: None,
                            extension: Some(new_transfer_privat_ext),
                        }), 
                        None,
                        BLOCK_SIZE,
                        state.nft_contract.code_hash.clone(),
                        state.nft_contract.address.to_string()
                    )?
                ); 

                let mut transfers: Vec<Transfer> = Vec::new();
                transfers.push(
                    Transfer{
                        recipient: from.to_string(),
                        token_ids: token_ids,
                        memo: None
                    }
                );
             
                response_msgs.push(batch_transfer_nft_msg(
                    transfers,
                    None,
                    BLOCK_SIZE,
                    state.nft_contract.code_hash.clone(),
                    state.nft_contract.address.to_string(),
                )?); 
     }
     else{
        return Err(ContractError::CustomError {val: "Not a valid contract address".to_string()});
     }  
 
   Ok(Response::new().add_messages(response_msgs))
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
        QueryMsg::GetPackBuildInfo {} => to_binary(&query_pack_build_info(deps)?),  
        QueryMsg::GetNumUserHistory { permit } => to_binary(&query_num_user_history(deps, permit)?),
        QueryMsg::GetUserHistory {permit, start_page, page_size} => to_binary(&query_user_history(deps, permit, start_page, page_size)?),
        QueryMsg::GetNumPacks { } => to_binary(&query_num_packs(deps)?),
        QueryMsg::GetPacks { start_page, page_size } => to_binary(&query_packs(deps, start_page, page_size)?),
        QueryMsg::GetPackMembers { main_token_id } => to_binary(&query_pack_members(deps, main_token_id )?),
        QueryMsg::GetPackMembersTraits { main_token_id } => to_binary(&query_pack_member_traits(deps, main_token_id )?)
    }
}

 // Get pack members, get distinct traits 
fn query_pack_build_info(
    deps: Deps,
) -> StdResult<BuildInfoResponse> { 
    let state = CONFIG_ITEM.load(deps.storage)?;

    Ok(BuildInfoResponse { pack_max: state.pack_max, total_burned: state.total_burned, valid_payments: state.valid_payments })
} 
 
fn query_num_user_history(
    deps: Deps, 
    permit: Permit
) -> StdResult<u32> { 
    let (user_raw, my_addr) = get_querier(deps, permit)?;
    let history_store = HISTORY_STORE.add_suffix(&user_raw);
    let num = history_store.get_len(deps.storage)?;
    Ok(num)
}  

fn query_user_history(
    deps: Deps, 
    permit: Permit,
    start_page: u32, 
    page_size: u32
) -> StdResult<Vec<HistoryToken>> {
    let (user_raw, my_addr) = get_querier(deps, permit)?;
    
    let history_store = HISTORY_STORE.add_suffix(&user_raw); 
    let history = history_store.paging(deps.storage, start_page, page_size)?;
    Ok(history)
} 

fn query_num_packs(
    deps: Deps
) -> StdResult<u32> {
    let num_staked_keys = PACK_MAIN_STORE.get_len(deps.storage).unwrap();
    Ok(num_staked_keys)
}

fn query_packs(
    deps: Deps, 
    start_page: u32, 
    page_size: u32
) -> StdResult<Vec<PackMain>> {
    let packs = PACK_MAIN_STORE.paging(deps.storage, start_page, page_size)?; 

    let mut packs_mut: Vec<PackMain> = Vec::new();
 
    for (index, (key_value, value)) in packs.iter().enumerate() {
        packs_mut.push(value.clone());
    } 
  
    Ok(packs_mut)
}
 
fn query_pack_members(
    deps: Deps, 
    main_token_id: String
) -> StdResult<Vec<PackMember>> {
    let pack_members = PACK_MEMBER_STORE.get(deps.storage, &main_token_id).unwrap_or_else(Vec::new);
    Ok(pack_members)
}

fn query_pack_member_traits(
    deps: Deps, 
    main_token_id: String
) -> StdResult<Vec<Trait>> {
    
    let pack_members = PACK_MEMBER_STORE.get(deps.storage, &main_token_id).unwrap_or_else(Vec::new);
    let mut distinct_traits: Vec<Trait> = Vec::new();
    for (index, value) in pack_members.iter().enumerate() {
        for member_trait in &value.attributes {
            if !distinct_traits.contains(&member_trait) {
                distinct_traits.push(member_trait.clone());
            }
        }
    }
    Ok(distinct_traits)
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

