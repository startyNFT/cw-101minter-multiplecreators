use cosmwasm_schema::{cw_serde, QueryResponses};
use crate::state::Config;

#[cw_serde]
pub struct CollectionParams {
    pub code_id: u64,
    pub name: String,
    pub symbol: String,
    pub description: Option<String>,
    pub image: Option<String>,
    pub external_link: Option<String>,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Collection parameters for instantiation
    pub collection_params: CollectionParams,
    /// Global royalty percentage in basis points (e.g., 500 = 5%, max 1000 = 10%)
    pub royalty_bps: u64,
    /// Initial addresses allowed to mint
    pub initial_allowlist: Vec<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Mint a new NFT (sender must be on allowlist)
    /// If `use_per_token_royalty` is true (default), the token will have per-token royalty
    /// with the minter as creator and the current royalty_bps from config.
    /// If false, the token will use the collection's general royalty.
    Mint {
        token_uri: String,
        /// Whether to set per-token royalty for this token (default: true)
        use_per_token_royalty: Option<bool>,
    },
    /// Add addresses to allowlist (admin only)
    AddToAllowlist { addresses: Vec<String> },
    /// Remove addresses from allowlist (admin only)
    RemoveFromAllowlist { addresses: Vec<String> },
    /// Pause or unpause minting (admin only)
    SetPaused { paused: bool },
    /// Update royalty percentage (admin only)
    UpdateRoyalty { royalty_bps: u64 },
    /// Transfer admin to new address (admin only)
    UpdateAdmin { new_admin: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get minter configuration
    #[returns(ConfigResponse)]
    Config {},
    /// Check if address is on allowlist
    #[returns(IsAllowedResponse)]
    IsAllowed { address: String },
    /// Get current token count
    #[returns(TokenCountResponse)]
    TokenCount {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct IsAllowedResponse {
    pub allowed: bool,
}

#[cw_serde]
pub struct TokenCountResponse {
    pub count: u64,
}
