//! Error types for MockForge test utilities

use std::io;
use thiserror::Error;

/// Result type for MockForge test operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when using MockForge test utilities
#[derive(Error, Debug)]
pub enum Error {
    /// Server failed to start
    #[error("Failed to start MockForge server: {0}")]
    ServerStartFailed(String),

    /// Server health check failed
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    /// Server health check timed out
    #[error("Health check timed out after {0}s")]
    HealthCheckTimeout(u64),

    /// HTTP request error
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    /// Process error
    #[error("Process error: {0}")]
    ProcessError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Scenario switching error
    #[error("Failed to switch scenario: {0}")]
    ScenarioError(String),

    /// Port allocation error
    #[error("Port {0} is already in use")]
    PortInUse(u16),

    /// MockForge binary not found
    #[error("MockForge binary not found. Please ensure it's installed or in PATH")]
    BinaryNotFound,

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    /// Workspace error
    #[error("Workspace error: {0}")]
    WorkspaceError(String),

    /// Invalid response from server
    #[error("Invalid response from server: {0}")]
    InvalidResponse(String),
}
