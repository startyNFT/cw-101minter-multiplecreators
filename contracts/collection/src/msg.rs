use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Timestamp, Uint128};

/// General/default royalty info for the collection
#[cw_serde]
#[derive(Default)]
pub struct GeneralRoyaltyInfo {
    /// Default royalty recipient address
    pub address: String,
    /// Default royalty percentage in basis points (e.g., 500 = 5%)
    pub royalty_bps: u64,
}

/// Per-token extension storing optional creator royalty override
#[cw_serde]
#[derive(Default)]
pub struct TokenExtension {
    /// Creator address who receives royalties for this token (overrides general)
    pub creator: Option<String>,
    /// Royalty percentage in basis points (overrides general)
    pub royalty_bps: Option<u64>,
    /// When the token was minted
    pub minted_at: Option<Timestamp>,
}

/// Collection-level extension (optional metadata)
#[cw_serde]
#[derive(Default)]
pub struct CollectionExtension {
    pub description: Option<String>,
    pub image: Option<String>,
    pub external_link: Option<String>,
    pub explicit_content: Option<bool>,
    pub start_trading_time: Option<Timestamp>,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Name of the NFT collection
    pub name: String,
    /// Symbol of the NFT collection
    pub symbol: String,
    /// Minter address (who can mint tokens)
    pub minter: String,
    /// Creator/admin address
    pub creator: Option<String>,
    /// Collection metadata
    pub collection_info: Option<CollectionExtension>,
    /// General/default royalty info for the collection
    pub general_royalty: Option<GeneralRoyaltyInfo>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Mint a new token with creator royalty info
    Mint {
        token_id: String,
        owner: String,
        token_uri: Option<String>,
        extension: TokenExtension,
    },
    /// Transfer NFT to another address
    TransferNft {
        recipient: String,
        token_id: String,
    },
    /// Send NFT to a contract
    SendNft {
        contract: String,
        token_id: String,
        msg: cosmwasm_std::Binary,
    },
    /// Approve spender for a token
    Approve {
        spender: String,
        token_id: String,
        expires: Option<cw721::Expiration>,
    },
    /// Revoke approval
    Revoke {
        spender: String,
        token_id: String,
    },
    /// Approve all tokens for an operator
    ApproveAll {
        operator: String,
        expires: Option<cw721::Expiration>,
    },
    /// Revoke all approvals for an operator
    RevokeAll {
        operator: String,
    },
    /// Burn a token (only owner)
    Burn {
        token_id: String,
    },
    /// Update minter address (only current minter)
    UpdateMinter {
        new_minter: Option<String>,
    },
    /// Update general royalty info (only creator/admin)
    UpdateGeneralRoyalty {
        general_royalty: GeneralRoyaltyInfo,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// CW2981: Get royalty info for a token sale
    /// If token_id is provided, returns token-specific royalty if set, otherwise general royalty
    /// If token_id is None, returns the general/default royalty info
    #[returns(RoyaltyInfoResponse)]
    RoyaltyInfo {
        token_id: Option<String>,
        sale_price: Uint128,
    },
    /// CW2981: Check if royalties are supported
    #[returns(CheckRoyaltiesResponse)]
    CheckRoyalties {},
    /// Get general/default royalty info for the collection
    #[returns(GeneralRoyaltyInfoResponse)]
    GeneralRoyalty {},
    /// Get token info with extension
    #[returns(NftInfoResponse)]
    NftInfo {
        token_id: String,
    },
    /// Get all token info including ownership
    #[returns(AllNftInfoResponse)]
    AllNftInfo {
        token_id: String,
        include_expired: Option<bool>,
    },
    /// Get owner of a token
    #[returns(cw721::msg::OwnerOfResponse)]
    OwnerOf {
        token_id: String,
        include_expired: Option<bool>,
    },
    /// Get all operators for an owner
    #[returns(cw721::msg::OperatorsResponse)]
    AllOperators {
        owner: String,
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Get number of tokens
    #[returns(cw721::msg::NumTokensResponse)]
    NumTokens {},
    /// Get contract info
    #[returns(ContractInfoResponse)]
    ContractInfo {},
    /// Get tokens owned by an address
    #[returns(cw721::msg::TokensResponse)]
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Get all tokens
    #[returns(cw721::msg::TokensResponse)]
    AllTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Get minter address
    #[returns(MinterResponse)]
    Minter {},
    /// Get collection info
    #[returns(CollectionInfoResponse)]
    CollectionInfo {},
    /// Get approval for a token
    #[returns(cw721::msg::ApprovalResponse)]
    Approval {
        token_id: String,
        spender: String,
        include_expired: Option<bool>,
    },
    /// Get all approvals for a token
    #[returns(cw721::msg::ApprovalsResponse)]
    Approvals {
        token_id: String,
        include_expired: Option<bool>,
    },
}

// Response types

#[cw_serde]
pub struct RoyaltyInfoResponse {
    pub address: String,
    pub royalty_amount: Uint128,
}

#[cw_serde]
pub struct CheckRoyaltiesResponse {
    pub royalty_payments: bool,
}

#[cw_serde]
pub struct GeneralRoyaltyInfoResponse {
    pub general_royalty: Option<GeneralRoyaltyInfo>,
}

#[cw_serde]
pub struct NftInfoResponse {
    pub token_uri: Option<String>,
    pub extension: TokenExtension,
}

#[cw_serde]
pub struct AllNftInfoResponse {
    pub access: cw721::msg::OwnerOfResponse,
    pub info: NftInfoResponse,
}

#[cw_serde]
pub struct MinterResponse {
    pub minter: Option<String>,
}

#[cw_serde]
pub struct CollectionInfoResponse {
    pub name: String,
    pub symbol: String,
    pub extension: Option<CollectionExtension>,
}

#[cw_serde]
pub struct ContractInfoResponse {
    pub name: String,
    pub symbol: String,
}
