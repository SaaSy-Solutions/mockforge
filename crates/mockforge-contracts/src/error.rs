//! Error types for mockforge-contracts

/// Error type for contract operations
#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP request error
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    /// Webhook error
    #[error("Webhook error: {0}")]
    Webhook(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

/// Result type alias for contract operations
pub type Result<T> = std::result::Result<T, ContractError>;
