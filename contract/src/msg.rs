use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Timestamp, Uint128};
use crate::state::{Config, TokenRoyalty};

#[cw_serde]
pub struct CollectionParams {
    pub code_id: u64,
    pub name: String,
    pub symbol: String,
    pub info: CollectionInfo,
}

#[cw_serde]
pub struct CollectionInfo {
    pub creator: String,
    pub description: String,
    pub image: String,
    pub external_link: Option<String>,
    pub start_trading_time: Option<Timestamp>,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub collection_params: CollectionParams,
    pub creator_royalty_bps: u64,      // Global royalty % (e.g., 500 = 5%)
    pub initial_allowlist: Vec<String>, // Initial addresses allowed to mint
}

#[cw_serde]
pub enum ExecuteMsg {
    // Mint NFT (sender must be on allowlist, mints to sender)
    Mint {
        token_uri: String,
    },
    // Admin: Add addresses to allowlist
    AddToAllowlist {
        addresses: Vec<String>,
    },
    // Admin: Remove addresses from allowlist
    RemoveFromAllowlist {
        addresses: Vec<String>,
    },
    // Admin: Pause/unpause minting
    SetPaused {
        paused: bool,
    },
    // Admin: Update global creator royalty %
    UpdateCreatorRoyalty {
        creator_royalty_bps: u64,
    },
    // Admin: Update collection trading start time
    UpdateStartTradingTime {
        start_time: Option<Timestamp>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},

    #[returns(RoyaltyInfoResponse)]
    RoyaltyInfo {
        token_id: String,
        sale_price: Uint128,
    },

    #[returns(TokenInfoResponse)]
    TokenInfo {
        token_id: String,
    },

    #[returns(TokensByCreatorResponse)]
    TokensByCreator {
        creator: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(IsAllowedResponse)]
    IsAllowed {
        address: String,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct RoyaltyInfoResponse {
    pub address: String,
    pub royalty_amount: Uint128,
}

#[cw_serde]
pub struct TokenInfoResponse {
    pub token_royalty: TokenRoyalty,
}

#[cw_serde]
pub struct TokensByCreatorResponse {
    pub tokens: Vec<String>,
}

#[cw_serde]
pub struct IsAllowedResponse {
    pub allowed: bool,
}
