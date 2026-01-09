use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

/// Main contract configuration
#[cw_serde]
pub struct Config {
    /// Admin address (can update config)
    pub admin: Addr,
    /// The minter contract to query for token creators
    pub minter: Addr,
    /// The SG721 collection address this splitter handles
    pub collection: Addr,
    /// Royalty percentage in basis points (e.g., 500 = 5%)
    pub royalty_bps: u64,
}

/// Tracks total royalties distributed to each creator
#[cw_serde]
pub struct CreatorStats {
    pub total_received: Uint128,
    pub total_sales: u64,
}

impl Default for CreatorStats {
    fn default() -> Self {
        Self {
            total_received: Uint128::zero(),
            total_sales: 0,
        }
    }
}

/// Contract configuration
pub const CONFIG: Item<Config> = Item::new("config");

/// Tracks royalties distributed per creator (creator_addr -> stats)
pub const CREATOR_STATS: Map<&Addr, CreatorStats> = Map::new("creator_stats");

/// Tracks royalties distributed per token (token_id -> total_royalties)
pub const TOKEN_ROYALTIES: Map<u32, Uint128> = Map::new("token_royalties");

/// Pending balance waiting to be distributed (for manual distribution fallback)
pub const PENDING_BALANCE: Item<Uint128> = Item::new("pending_balance");
