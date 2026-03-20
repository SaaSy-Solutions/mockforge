//! Error types for the Kafka crate

use thiserror::Error;

/// Kafka-specific errors
#[derive(Debug, Error)]
pub enum KafkaError {
    /// I/O error during network operations
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Protocol parsing error
    #[error("protocol error: {0}")]
    Protocol(String),

    /// Consumer group error
    #[error("consumer group error: {0}")]
    ConsumerGroup(String),

    /// Topic or partition not found
    #[error("not found: {0}")]
    NotFound(String),

    /// Serialization/deserialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Configuration error
    #[error("configuration error: {0}")]
    Config(String),

    /// Generic internal error
    #[error("{0}")]
    Internal(String),
}

/// Convenience Result type for Kafka operations
pub type Result<T> = std::result::Result<T, KafkaError>;
