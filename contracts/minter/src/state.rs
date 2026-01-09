use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    /// Admin address (can update settings)
    pub admin: Addr,
    /// Collection contract address
    pub collection: Option<Addr>,
    /// Collection code ID (for instantiation)
    pub collection_code_id: u64,
    /// Global royalty percentage in basis points (max 1000 = 10%)
    pub royalty_bps: u64,
    /// Whether minting is paused
    pub is_paused: bool,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const ALLOWLIST: Map<&Addr, bool> = Map::new("allowlist");
pub const TOKEN_INDEX: Item<u64> = Item::new("token_index");

pub fn increment_token_index(store: &mut dyn Storage) -> StdResult<u64> {
    let val = TOKEN_INDEX.may_load(store)?.unwrap_or_default() + 1;
    TOKEN_INDEX.save(store, &val)?;
    Ok(val)
}
