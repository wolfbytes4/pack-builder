use schemars::JsonSchema;
use serde::{ Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Addr}; 
use secret_toolkit::{ 
    storage:: { Item, Keymap, AppendStore }
};
use crate::msg::{HistoryToken, PaymentContractInfo, ContractInfo, Level, PackMain, PackMember};

pub static CONFIG_KEY: &[u8] = b"config"; 
pub const ADMIN_KEY: &[u8] = b"admin";
pub const MY_ADDRESS_KEY: &[u8] = b"my_address"; 
pub const INHOLDING_NFT_KEY: &[u8] = b"inholding_nft";
pub const PREFIX_REVOKED_PERMITS: &str = "revoke";
pub const PAID_KEY: &[u8] = b"paid";
pub const HISTORY_KEY: &[u8] = b"history";
pub const LEVEL_KEY: &[u8] = b"level";
pub const RANK_KEY: &[u8] = b"rank";
pub const PACK_KEY: &[u8] = b"pack";
pub const PACK_MEMBER_KEY: &[u8] = b"pack_member";

pub static CONFIG_ITEM: Item<State> = Item::new(CONFIG_KEY); 
pub static PAID_ADDRESSES_ITEM: Item<Vec<CanonicalAddr>> = Item::new(PAID_KEY);
pub static ADMIN_ITEM: Item<CanonicalAddr> = Item::new(ADMIN_KEY); 
pub static MY_ADDRESS_ITEM: Item<CanonicalAddr> = Item::new(MY_ADDRESS_KEY);   
pub static HISTORY_STORE: AppendStore<HistoryToken> = AppendStore::new(HISTORY_KEY);
pub static LEVEL_ITEM: Item<Vec<Level>> = Item::new(LEVEL_KEY); 
pub static RANK_STORE: Keymap<String, u16> = Keymap::new(RANK_KEY);
pub static PACK_MAIN_STORE: Keymap<String, PackMain> = Keymap::new(PACK_KEY);
pub static PACK_MEMBER_STORE: Keymap<String, Vec<PackMember>> = Keymap::new(PACK_MEMBER_KEY);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {  
    pub owner: Addr,   
    pub nft_contract: ContractInfo,
    pub is_payment_needed: bool,
    pub valid_payments: Option<Vec<PaymentContractInfo>>,
    pub viewing_key: Option<String>,
    pub receiving_address: Addr,
    pub total_burned: u16,
    pub pack_max: u16,
    pub collection_size: u16,
    pub level_cap: u16
} 