use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{
   Addr, Binary, Uint128
};
use secret_toolkit::{ 
    permit:: { Permit },
    snip721::{
        Trait
    }
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {  
    pub entropy: String,
    pub nft_contract: ContractInfo, 
    pub is_payment_needed: bool,
    pub valid_payments: Option<Vec<PaymentContractInfo>>,
    pub receiving_address: Addr,
    pub pack_max: u16,
    pub collection_size: u16,
    pub level_cap: u16,
    pub levels: Vec<Level>,
    pub ranks: Vec<Rank>
} 

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Level {
    pub level: u16,
    pub xp_needed: u32
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Rank {
    pub token_id: String,
    pub rank: u16
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ContractInfo {
    /// contract's code hash string
    pub code_hash: String,
    /// contract's address
    pub address: Addr
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PaymentContractInfo {
    /// contract's code hash string
    pub code_hash: String,
    /// contract's address
    pub address: Addr,
    pub payment_needed: Uint128,
    pub name: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct HistoryToken {
    pub wolf_main_token_id: String,
    pub pack_member_token_ids: Vec<String>, 
    pub pack_build_date: Option<u64>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ReceiveMsg {
    pub quantity: u16
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PackBuildMsg {
    pub main_token_id: String,
    pub name: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PackTransferMsg {
    pub main_token_id: String,
    pub transfer_to_token_id: String,
    pub token_id: String,
    pub member_index: u8 
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PackMember {
    pub token_id: String,
    pub rank: u16,
    pub attributes: Vec<Trait>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PackMain {
    pub token_id: String,
    pub pack_rank: u32,
    pub pack_count: u16,
    pub name: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct BuildInfoResponse {
    pub pack_max: u16,
    pub total_burned: u16,
    pub valid_payments: Option<Vec<PaymentContractInfo>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleReceiveMsg {
    ReceivePackBuild {
        pack_build: PackBuildMsg
    },
    // ReceiveTransferBuild {
    //     transfer_build: PackTransferMsg
    // },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg { 
    RevokePermit{
        permit_name: String
    },   
    Receive{ 
        sender: Addr,
        from: Addr,
        amount: Uint128,
        msg: Option<Binary>
    }, 
    BatchReceiveNft{
        from: Addr, 
        token_ids: Vec<String>,
        msg: Option<Binary>
    },
    // ClaimBack{  
    // },
    SendNftBack{ 
        token_id: String,
        owner: Addr
    },
    AddPayment{ 
        payment: PaymentContractInfo
    },
    RemovePayment{
        payment_name: String
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {   
    GetPackBuildInfo {},
    GetNumUserHistory{
        permit: Permit
    },
    GetUserHistory { 
        permit: Permit,
        start_page: u32, 
        page_size: u32 
    },
    GetNumPacks {},
    GetPacks {
        start_page: u32,
        page_size: u32 
    },
    GetPackMembers{
        main_token_id: String
    },
    GetPackMembersTraits{
        main_token_id: String
    },
    GetHolding{
        addr: Addr
    }
} 

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PaymentContractsResponse {
    pub contract_infos: Vec<PaymentContractInfo>,
}