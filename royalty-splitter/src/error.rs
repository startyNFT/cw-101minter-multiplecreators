use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Invalid collection: expected {expected}, got {got}")]
    InvalidCollection { expected: String, got: String },

    #[error("Token not found in minter: {token_id}")]
    TokenNotFound { token_id: u32 },

    #[error("Insufficient funds: need {needed}, have {available}")]
    InsufficientFunds { needed: String, available: String },

    #[error("Invalid royalty BPS: {bps} (must be <= 10000)")]
    InvalidRoyaltyBps { bps: u64 },

    #[error("No pending balance to distribute")]
    NoPendingBalance {},

    #[error("Failed to query minter for token {token_id}: {reason}")]
    MinterQueryFailed { token_id: u32, reason: String },
}
