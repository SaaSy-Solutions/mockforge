//! Error types for the analytics module

use thiserror::Error;

/// Result type for analytics operations
pub type Result<T> = std::result::Result<T, AnalyticsError>;

/// Error types for analytics operations
#[derive(Debug, Error)]
pub enum AnalyticsError {
    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP error (when querying Prometheus)
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Query error
    #[error("Query error: {0}")]
    Query(String),

    /// Export error
    #[error("Export error: {0}")]
    Export(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl From<String> for AnalyticsError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

impl From<&str> for AnalyticsError {
    fn from(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}
