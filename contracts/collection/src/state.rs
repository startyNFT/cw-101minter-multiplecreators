use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use crate::msg::{CollectionExtension, GeneralRoyaltyInfo, TokenExtension};

/// Collection configuration
#[cosmwasm_schema::cw_serde]
pub struct CollectionConfig {
    pub name: String,
    pub symbol: String,
    pub minter: Option<Addr>,
    pub creator: Option<Addr>,
}

/// Token info stored per token
#[cosmwasm_schema::cw_serde]
pub struct TokenInfo {
    pub owner: Addr,
    pub token_uri: Option<String>,
    pub extension: TokenExtension,
    pub approvals: Vec<cw721::Approval>,
}

pub const CONFIG: Item<CollectionConfig> = Item::new("config");
pub const COLLECTION_INFO: Item<Option<CollectionExtension>> = Item::new("collection_info");
pub const GENERAL_ROYALTY: Item<Option<GeneralRoyaltyInfo>> = Item::new("general_royalty");
pub const TOKENS: Map<&str, TokenInfo> = Map::new("tokens");
pub const OPERATORS: Map<(&Addr, &Addr), cw721::Expiration> = Map::new("operators");
pub const TOKEN_COUNT: Item<u64> = Item::new("token_count");
