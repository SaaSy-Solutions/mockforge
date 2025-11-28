//! TCP server mocking for MockForge
//!
//! This crate provides TCP server functionality for MockForge, allowing you to mock
//! raw TCP connections for testing purposes.
//!
//! # Example
//!
//! ```no_run
//! use mockforge_tcp::{TcpServer, TcpConfig, TcpSpecRegistry};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = TcpConfig::default();
//!     let registry = Arc::new(TcpSpecRegistry::new());
//!
//!     let server = TcpServer::new(config, registry)?;
//!     server.start().await?;
//!
//!     Ok(())
//! }
//! ```

mod fixtures;
mod server;
mod spec_registry;

pub use fixtures::*;
pub use server::*;
pub use spec_registry::*;

/// TCP server configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TcpConfig {
    /// Server port (default: 9999)
    pub port: u16,
    /// Host address (default: 0.0.0.0)
    pub host: String,
    /// Directory containing fixture files
    pub fixtures_dir: Option<std::path::PathBuf>,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
    /// Maximum connections
    pub max_connections: usize,
    /// Buffer size for reading data (bytes)
    pub read_buffer_size: usize,
    /// Buffer size for writing data (bytes)
    pub write_buffer_size: usize,
    /// Enable TLS/SSL support
    pub enable_tls: bool,
    /// Path to TLS certificate file
    pub tls_cert_path: Option<std::path::PathBuf>,
    /// Path to TLS private key file
    pub tls_key_path: Option<std::path::PathBuf>,
    /// Echo mode: echo back received data (if no fixture matches)
    pub echo_mode: bool,
    /// Delimiter for message boundaries (None = stream mode, Some = frame by delimiter)
    pub delimiter: Option<Vec<u8>>,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            port: 9999,
            host: "0.0.0.0".to_string(),
            fixtures_dir: Some(std::path::PathBuf::from("./fixtures/tcp")),
            timeout_secs: 300,
            max_connections: 100,
            read_buffer_size: 8192,
            write_buffer_size: 8192,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            echo_mode: true,
            delimiter: None, // Stream mode by default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::protocol_abstraction::Protocol;

    #[test]
    fn test_default_config() {
        let config = TcpConfig::default();
        assert_eq!(config.port, 9999);
        assert_eq!(config.host, "0.0.0.0");
        assert!(config.echo_mode);
    }

    #[test]
    fn test_protocol_display() {
        assert_eq!(Protocol::Tcp.to_string(), "TCP");
    }
}
