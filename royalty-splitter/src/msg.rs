use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};
use crate::state::{Config, CreatorStats};

#[cw_serde]
pub struct InstantiateMsg {
    /// The minter contract address to query for token creators
    pub minter: String,
    /// The SG721 collection address this splitter handles
    pub collection: String,
    /// Royalty percentage in basis points (e.g., 500 = 5%)
    pub royalty_bps: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Admin: Update the minter contract address
    UpdateMinter { minter: String },

    /// Admin: Update the collection address
    UpdateCollection { collection: String },

    /// Admin: Update royalty percentage
    UpdateRoyaltyBps { royalty_bps: u64 },

    /// Admin: Manual distribution fallback (if sale hook fails)
    /// Distributes pending funds to a specific creator for a token
    ManualDistribute {
        token_id: u32,
        amount: Uint128,
    },

    /// Admin: Withdraw any stuck funds (emergency only)
    EmergencyWithdraw {
        recipient: String,
        amount: Uint128,
    },
}

/// SudoMsg - Called by privileged contracts (marketplace)
#[cw_serde]
pub enum SudoMsg {
    /// Called by marketplace when an NFT sale occurs
    /// This is the main entry point for automatic royalty distribution
    SaleHook {
        collection: String,
        token_id: u32,
        price: Coin,
        seller: String,
        buyer: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get contract configuration
    #[returns(ConfigResponse)]
    Config {},

    /// Get stats for a specific creator
    #[returns(CreatorStatsResponse)]
    CreatorStats { creator: String },

    /// Get total royalties distributed for a token
    #[returns(TokenRoyaltiesResponse)]
    TokenRoyalties { token_id: u32 },

    /// Get pending balance (undistributed funds)
    #[returns(PendingBalanceResponse)]
    PendingBalance {},

    /// Get all creator stats (paginated)
    #[returns(AllCreatorStatsResponse)]
    AllCreatorStats {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

// Query responses

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct CreatorStatsResponse {
    pub creator: String,
    pub stats: CreatorStats,
}

#[cw_serde]
pub struct TokenRoyaltiesResponse {
    pub token_id: u32,
    pub total_royalties: Uint128,
}

#[cw_serde]
pub struct PendingBalanceResponse {
    pub pending: Uint128,
}

#[cw_serde]
pub struct AllCreatorStatsResponse {
    pub creators: Vec<CreatorStatsResponse>,
}

// Message to query the minter contract for token info

#[cw_serde]
pub enum MinterQueryMsg {
    /// Query token info from the minter to get creator address
    TokenInfo { token_id: String },
}

#[cw_serde]
pub struct MinterTokenInfoResponse {
    pub token_royalty: TokenRoyalty,
}

#[cw_serde]
pub struct TokenRoyalty {
    pub creator: String,
    pub token_uri: String,
    pub minted_at: String,
}
