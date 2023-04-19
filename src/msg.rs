use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{
   Addr, Binary, Uint128
};
use secret_toolkit::{ 
    permit:: { Permit }
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {  
    pub entropy: String,
    pub nft_contract: ContractInfo, 
    pub is_payment_needed: bool,
    pub valid_payments: Option<Vec<PaymentContractInfo>>,
    pub receiving_address: Addr,
    pub pack_max: i32,
    pub collection_size: i32,
    pub level_cap: i32,
    pub levels: Vec<Level>
} 

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Level {
    pub level: i32,
    pub xp_needed: i32
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
    pub payment_needed: Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct HistoryToken {
    pub wolf_token_id: String,
    pub powerup_token_ids: Vec<String>, 
    pub pack_build_date: Option<u64>, 
    pub power_up_amount: i32
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PackBuildMsg {
    pub main_token_id: String
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
    SendNftBack{ 
        token_id: String,
        owner: Addr
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {   
    GetPackBuildInfo {},
    GetUserHistory { 
        permit: Permit,
        start_page: u32, 
        page_size: u32 
    },
    GetNumUserHistory{
        permit: Permit
    }
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PaymentContractsResponse {
    pub contract_infos: Vec<PaymentContractInfo>,
}