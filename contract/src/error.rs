use cosmwasm_std::{StdError, Timestamp};
use thiserror::Error;
use url::ParseError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    ParseError(#[from] ParseError),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Address not on allowlist")]
    NotOnAllowlist {},

    #[error("Minting is currently paused")]
    MintingPaused {},

    #[error("Invalid creator royalty: cannot exceed 1000 basis points (10%)")]
    InvalidCreatorRoyalty {},

    #[error("Invalid token URI: {uri}")]
    InvalidTokenUri { uri: String },

    #[error("Invalid URI scheme: {scheme}. Only https and ipfs are allowed")]
    InvalidUriScheme { scheme: String },

    #[error("Empty token URI not allowed")]
    EmptyTokenUri {},

    #[error("Token not found: {token_id}")]
    TokenNotFound { token_id: String },

    #[error("Invalid reply ID")]
    InvalidReplyID {},

    #[error("Instantiate sg721 error")]
    InstantiateSg721Error {},

    #[error("Invalid start trading time: {0} < {1}")]
    InvalidStartTradingTime(Timestamp, Timestamp),

    #[error("Trading time cannot be in the past")]
    TradingTimeInPast {},

    #[error("Calculation overflow")]
    CalculationOverflow {},
}
