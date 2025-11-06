//! Error types for the scenarios marketplace

use thiserror::Error;

/// Result type for scenario operations
pub type Result<T> = std::result::Result<T, ScenarioError>;

/// Errors that can occur in scenario operations
#[derive(Error, Debug)]
pub enum ScenarioError {
    /// Scenario not found
    #[error("Scenario not found: {0}")]
    NotFound(String),

    /// Invalid scenario manifest
    #[error("Invalid scenario manifest: {0}")]
    InvalidManifest(String),

    /// Invalid version specification
    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    /// Scenario already exists
    #[error("Scenario already exists: {0}")]
    AlreadyExists(String),

    /// Authentication required
    #[error("Authentication required")]
    AuthRequired,

    /// Permission denied
    #[error("Permission denied")]
    PermissionDenied,

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// File system error
    #[error("File system error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Invalid source specification
    #[error("Invalid source: {0}")]
    InvalidSource(String),

    /// Checksum verification failed
    #[error("Checksum verification failed: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// Dependency resolution failed
    #[error("Dependency resolution failed: {0}")]
    DependencyResolution(String),

    /// Generic error
    #[error("{0}")]
    Generic(String),
}

impl From<anyhow::Error> for ScenarioError {
    fn from(err: anyhow::Error) -> Self {
        ScenarioError::Generic(err.to_string())
    }
}
