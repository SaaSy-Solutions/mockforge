//! MockForge Tunneling Service
//!
//! Provides functionality to expose local MockForge servers via public URLs
//! without requiring cloud deployment. Supports multiple tunneling backends
//! and provides a unified API for tunnel management.

#[cfg(feature = "server")]
pub mod audit;
pub mod client;
pub mod config;
pub mod manager;
pub mod provider;
#[cfg(feature = "server")]
pub mod rate_limit;
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "server")]
pub mod server_config;
#[cfg(feature = "server")]
pub mod storage;

pub use client::TunnelClient;
pub use config::{TunnelConfig, TunnelProvider};
pub use manager::TunnelManager;
pub use provider::{TunnelProvider as ProviderTrait, TunnelStatus};

/// Result type for tunnel operations
pub type Result<T> = std::result::Result<T, TunnelError>;

/// Error type for tunnel operations
#[derive(Debug, thiserror::Error)]
pub enum TunnelError {
    #[error("Tunnel connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Tunnel provider error: {0}")]
    ProviderError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Tunnel not found: {0}")]
    NotFound(String),

    #[error("Tunnel already exists: {0}")]
    AlreadyExists(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
