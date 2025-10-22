//! Error types for the MockForge SDK

use thiserror::Error;

/// SDK Result type
pub type Result<T> = std::result::Result<T, Error>;

/// SDK Error types
#[derive(Error, Debug)]
pub enum Error {
    /// Server already started
    #[error("Mock server is already running on port {0}")]
    ServerAlreadyStarted(u16),

    /// Server not started
    #[error("Mock server has not been started yet")]
    ServerNotStarted,

    /// Port already in use
    #[error("Port {0} is already in use")]
    PortInUse(u16),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Invalid stub
    #[error("Invalid stub: {0}")]
    InvalidStub(String),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(#[from] axum::http::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// MockForge core error
    #[error("Core error: {0}")]
    Core(#[from] mockforge_core::Error),

    /// General error
    #[error("{0}")]
    General(String),
}
