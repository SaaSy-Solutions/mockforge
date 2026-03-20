//! Error types for the world state engine

use thiserror::Error;

/// Errors that can occur in the world state engine
#[derive(Debug, Error)]
pub enum WorldStateError {
    /// An aggregator failed to collect state
    #[error("aggregation failed: {0}")]
    Aggregation(String),

    /// Serialization/deserialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Internal error
    #[error("{0}")]
    Internal(String),
}

/// Result type alias for world state operations
pub type Result<T> = std::result::Result<T, WorldStateError>;
