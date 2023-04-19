use schemars::JsonSchema;
use serde::{ Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Addr}; 
use secret_toolkit::{ 
    storage:: { Item, Keymap, AppendStore }
};
use crate::msg::{HistoryToken, PaymentContractInfo, ContractInfo, Level};

pub static CONFIG_KEY: &[u8] = b"config"; 
pub const ADMIN_KEY: &[u8] = b"admin";
pub const MY_ADDRESS_KEY: &[u8] = b"my_address"; 
pub const INHOLDING_NFT_KEY: &[u8] = b"inholding_nft";
pub const PREFIX_REVOKED_PERMITS: &str = "revoke";
pub const PAID_KEY: &[u8] = b"paid";
pub const HISTORY_KEY: &[u8] = b"level";
pub const LEVEL_KEY: &[u8] = b"level";

pub static CONFIG_ITEM: Item<State> = Item::new(CONFIG_KEY); 
pub static PAID_ADDRESSES_ITEM: Item<Vec<CanonicalAddr>> = Item::new(PAID_KEY);
pub static ADMIN_ITEM: Item<CanonicalAddr> = Item::new(ADMIN_KEY); 
pub static MY_ADDRESS_ITEM: Item<CanonicalAddr> = Item::new(MY_ADDRESS_KEY);   
pub static HISTORY_STORE: AppendStore<HistoryToken> = AppendStore::new(HISTORY_KEY);
pub static LEVEL_ITEM: Item<Vec<Level>> = Item::new(LEVEL_KEY);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {  
    pub owner: Addr,   
    pub nft_contract: ContractInfo,
    pub is_payment_needed: bool,
    pub valid_payments: Option<Vec<PaymentContractInfo>>,
    pub viewing_key: Option<String>,
    pub receiving_address: Addr,
    pub total_burned: i32,
    pub pack_max: i32,
    pub collection_size: i32,
    pub level_cap: i32
} 