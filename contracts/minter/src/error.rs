use cosmwasm_std::StdError;
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

    #[error("Invalid royalty: cannot exceed 1000 basis points (10%)")]
    InvalidRoyalty {},

    #[error("Invalid token URI: {uri}")]
    InvalidTokenUri { uri: String },

    #[error("Invalid URI scheme: {scheme}. Only https and ipfs are allowed")]
    InvalidUriScheme { scheme: String },

    #[error("Empty token URI not allowed")]
    EmptyTokenUri {},

    #[error("Collection not initialized")]
    CollectionNotInitialized {},

    #[error("Invalid reply ID")]
    InvalidReplyId {},

    #[error("Failed to instantiate collection")]
    InstantiateCollectionError {},
}
