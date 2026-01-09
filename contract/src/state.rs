use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdResult, Storage, Timestamp};
use cw_storage_plus::{Item, Map};

/// Main contract configuration
#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub collection_address: Option<Addr>,
    pub collection_code_id: u64,
    pub creator_royalty_bps: u64,  // Global royalty % for all creators (e.g., 500 = 5%)
    pub is_paused: bool,
}

/// Per-token metadata (creator info)
#[cw_serde]
pub struct TokenRoyalty {
    pub creator: Addr,        // Creator who minted this token (receives royalties)
    pub token_uri: String,
    pub minted_at: Timestamp,
}

/// Initial configuration of the minter
pub const CONFIG: Item<Config> = Item::new("config");

/// Allowlist of addresses that can mint (address -> true if allowed)
pub const ALLOWLIST: Map<Addr, bool> = Map::new("allowlist");

/// Stores per-token metadata (token_id -> TokenRoyalty)
pub const TOKEN_ROYALTIES: Map<String, TokenRoyalty> = Map::new("token_royalties");

/// This keeps track of the token index for the token_ids
pub const TOKEN_INDEX: Item<u64> = Item::new("token_index");

pub fn increment_token_index(store: &mut dyn Storage) -> StdResult<u64> {
    let val = TOKEN_INDEX.may_load(store)?.unwrap_or_default() + 1;
    TOKEN_INDEX.save(store, &val)?;
    Ok(val)
}
