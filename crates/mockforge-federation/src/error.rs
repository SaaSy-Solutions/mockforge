//! Error types for the federation crate

use thiserror::Error;

/// Errors that can occur during federation operations
#[derive(Debug, Error)]
pub enum FederationError {
    /// Database error
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// JSON serialization error
    #[error("serialization error: {0}")]
    SerializationJson(#[from] serde_json::Error),

    /// UUID parsing error
    #[error("invalid UUID: {0}")]
    UuidParse(#[from] uuid::Error),

    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Invalid configuration
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    /// Internal error
    #[error("{0}")]
    Internal(String),
}

/// Result type alias for federation operations
pub type Result<T> = std::result::Result<T, FederationError>;
