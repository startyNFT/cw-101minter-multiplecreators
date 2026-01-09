use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw721::{
    AllNftInfoResponse, ApprovalResponse, ApprovalsResponse, ContractInfoResponse,
    NftInfoResponse, NumTokensResponse, OperatorsResponse, OwnerOfResponse, TokensResponse,
};
use cw721_base::MinterResponse;
use sg721_base::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};

/// Token extension that stores per-token royalty info
#[cw_serde]
#[derive(Default)]
pub struct TokenExtension {
    /// Optional royalty info for this specific token
    pub royalty: Option<TokenRoyaltyInfo>,
}

/// Per-token royalty information
#[cw_serde]
pub struct TokenRoyaltyInfo {
    /// Address that receives royalties (the creator)
    pub payment_address: String,
    /// Royalty percentage in basis points (e.g., 500 = 5%)
    pub share: u64,
}

/// CW2981 RoyaltyInfo query response
#[cw_serde]
pub struct RoyaltyInfoResponse {
    pub address: String,
    pub royalty_amount: Uint128,
}

/// CW2981 CheckRoyalties query response
#[cw_serde]
pub struct CheckRoyaltiesResponse {
    pub royalty_payments: bool,
}

/// Combined query messages: SG721 base + CW2981 royalty queries
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // ============ CW2981 Royalty Queries ============
    /// Returns royalty info for a token sale
    #[returns(RoyaltyInfoResponse)]
    RoyaltyInfo { token_id: String, sale_price: Uint128 },

    /// Check if contract supports royalty queries
    #[returns(CheckRoyaltiesResponse)]
    CheckRoyalties {},

    // ============ Standard SG721 Queries (forwarded to base) ============
    #[returns(OwnerOfResponse)]
    OwnerOf {
        token_id: String,
        include_expired: Option<bool>,
    },

    #[returns(ApprovalResponse)]
    Approval {
        token_id: String,
        spender: String,
        include_expired: Option<bool>,
    },

    #[returns(ApprovalsResponse)]
    Approvals {
        token_id: String,
        include_expired: Option<bool>,
    },

    #[returns(OperatorsResponse)]
    AllOperators {
        owner: String,
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(NumTokensResponse)]
    NumTokens {},

    #[returns(ContractInfoResponse)]
    ContractInfo {},

    #[returns(NftInfoResponse<TokenExtension>)]
    NftInfo { token_id: String },

    #[returns(AllNftInfoResponse<TokenExtension>)]
    AllNftInfo {
        token_id: String,
        include_expired: Option<bool>,
    },

    #[returns(TokensResponse)]
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(TokensResponse)]
    AllTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(MinterResponse)]
    Minter {},

    #[returns(CollectionInfoResponse)]
    CollectionInfo {},
}

impl From<QueryMsg> for Sg721QueryMsg {
    fn from(msg: QueryMsg) -> Sg721QueryMsg {
        match msg {
            QueryMsg::OwnerOf {
                token_id,
                include_expired,
            } => Sg721QueryMsg::OwnerOf {
                token_id,
                include_expired,
            },
            QueryMsg::Approval {
                token_id,
                spender,
                include_expired,
            } => Sg721QueryMsg::Approval {
                token_id,
                spender,
                include_expired,
            },
            QueryMsg::Approvals {
                token_id,
                include_expired,
            } => Sg721QueryMsg::Approvals {
                token_id,
                include_expired,
            },
            QueryMsg::AllOperators {
                owner,
                include_expired,
                start_after,
                limit,
            } => Sg721QueryMsg::AllOperators {
                owner,
                include_expired,
                start_after,
                limit,
            },
            QueryMsg::NumTokens {} => Sg721QueryMsg::NumTokens {},
            QueryMsg::ContractInfo {} => Sg721QueryMsg::ContractInfo {},
            QueryMsg::NftInfo { token_id } => Sg721QueryMsg::NftInfo { token_id },
            QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            } => Sg721QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            },
            QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            } => Sg721QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            },
            QueryMsg::AllTokens { start_after, limit } => {
                Sg721QueryMsg::AllTokens { start_after, limit }
            }
            QueryMsg::Minter {} => Sg721QueryMsg::Minter {},
            QueryMsg::CollectionInfo {} => Sg721QueryMsg::CollectionInfo {},
            // CW2981 queries don't map to base
            _ => unreachable!(),
        }
    }
}
