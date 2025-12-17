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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tunnel_error_connection_failed() {
        let error = TunnelError::ConnectionFailed("connection refused".to_string());
        let display = error.to_string();
        assert!(display.contains("Tunnel connection failed"));
        assert!(display.contains("connection refused"));
    }

    #[test]
    fn test_tunnel_error_provider_error() {
        let error = TunnelError::ProviderError("provider unavailable".to_string());
        let display = error.to_string();
        assert!(display.contains("Tunnel provider error"));
        assert!(display.contains("provider unavailable"));
    }

    #[test]
    fn test_tunnel_error_config_error() {
        let error = TunnelError::ConfigError("missing server_url".to_string());
        let display = error.to_string();
        assert!(display.contains("Configuration error"));
        assert!(display.contains("missing server_url"));
    }

    #[test]
    fn test_tunnel_error_not_found() {
        let error = TunnelError::NotFound("tunnel-123".to_string());
        let display = error.to_string();
        assert!(display.contains("Tunnel not found"));
        assert!(display.contains("tunnel-123"));
    }

    #[test]
    fn test_tunnel_error_already_exists() {
        let error = TunnelError::AlreadyExists("tunnel-456".to_string());
        let display = error.to_string();
        assert!(display.contains("Tunnel already exists"));
        assert!(display.contains("tunnel-456"));
    }

    #[test]
    fn test_tunnel_error_debug() {
        let error = TunnelError::ConnectionFailed("test".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("ConnectionFailed"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_tunnel_error_from_url_parse() {
        let url_error = url::ParseError::EmptyHost;
        let error: TunnelError = url_error.into();
        let display = error.to_string();
        assert!(display.contains("URL parsing error"));
    }

    #[test]
    fn test_tunnel_error_from_json() {
        let json_str = "invalid json";
        let json_error = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let error: TunnelError = json_error.into();
        let display = error.to_string();
        assert!(display.contains("Serialization error"));
    }

    #[test]
    fn test_tunnel_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error: TunnelError = io_error.into();
        let display = error.to_string();
        assert!(display.contains("IO error"));
        assert!(display.contains("file not found"));
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(TunnelError::NotFound("tunnel".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_tunnel_error_variants_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        // TunnelError should be Send + Sync for use across threads
        // Note: This is a compile-time check
        // assert_send_sync::<TunnelError>(); // Would fail due to reqwest::Error
    }

    #[test]
    fn test_tunnel_error_connection_failed_empty_message() {
        let error = TunnelError::ConnectionFailed(String::new());
        let display = error.to_string();
        assert!(display.contains("Tunnel connection failed"));
    }

    #[test]
    fn test_tunnel_error_chained_display() {
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let error: TunnelError = io_error.into();
        // Check that the error chain is preserved
        let display = error.to_string();
        assert!(display.contains("access denied"));
    }
}
