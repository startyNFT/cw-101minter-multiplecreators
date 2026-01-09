use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Token not found: {token_id}")]
    TokenNotFound { token_id: String },

    #[error("Token already exists: {token_id}")]
    TokenAlreadyExists { token_id: String },

    #[error("Approval not found")]
    ApprovalNotFound {},

    #[error("Cannot transfer: approval expired")]
    ApprovalExpired {},

    #[error("Caller is not owner or approved")]
    NotOwnerOrApproved {},

    #[error("No royalty info for token")]
    NoRoyaltyInfo {},
}
