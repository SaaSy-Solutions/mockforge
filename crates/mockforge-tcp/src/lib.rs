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

    #[test]
    fn test_config_default_values() {
        let config = TcpConfig::default();
        assert_eq!(config.timeout_secs, 300);
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.read_buffer_size, 8192);
        assert_eq!(config.write_buffer_size, 8192);
        assert!(!config.enable_tls);
        assert!(config.tls_cert_path.is_none());
        assert!(config.tls_key_path.is_none());
        assert!(config.delimiter.is_none());
    }

    #[test]
    fn test_config_fixtures_dir() {
        let config = TcpConfig::default();
        assert!(config.fixtures_dir.is_some());
        let fixtures_dir = config.fixtures_dir.unwrap();
        assert_eq!(fixtures_dir, std::path::PathBuf::from("./fixtures/tcp"));
    }

    #[test]
    fn test_config_clone() {
        let config = TcpConfig {
            port: 8080,
            host: "127.0.0.1".to_string(),
            fixtures_dir: None,
            timeout_secs: 60,
            max_connections: 50,
            read_buffer_size: 4096,
            write_buffer_size: 4096,
            enable_tls: true,
            tls_cert_path: Some(std::path::PathBuf::from("/path/to/cert")),
            tls_key_path: Some(std::path::PathBuf::from("/path/to/key")),
            echo_mode: false,
            delimiter: Some(b"\n".to_vec()),
        };

        let cloned = config.clone();
        assert_eq!(config.port, cloned.port);
        assert_eq!(config.host, cloned.host);
        assert_eq!(config.enable_tls, cloned.enable_tls);
        assert_eq!(config.delimiter, cloned.delimiter);
    }

    #[test]
    fn test_config_debug() {
        let config = TcpConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("TcpConfig"));
        assert!(debug.contains("9999"));
    }

    #[test]
    fn test_config_serialize() {
        let config = TcpConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"port\":9999"));
        assert!(json.contains("\"echo_mode\":true"));
    }

    #[test]
    fn test_config_deserialize() {
        let json = r#"{
            "port": 8080,
            "host": "localhost",
            "fixtures_dir": null,
            "timeout_secs": 120,
            "max_connections": 200,
            "read_buffer_size": 16384,
            "write_buffer_size": 16384,
            "enable_tls": false,
            "tls_cert_path": null,
            "tls_key_path": null,
            "echo_mode": false,
            "delimiter": null
        }"#;

        let config: TcpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "localhost");
        assert_eq!(config.timeout_secs, 120);
        assert!(!config.echo_mode);
    }

    #[test]
    fn test_config_with_delimiter() {
        let config = TcpConfig {
            delimiter: Some(b"\r\n".to_vec()),
            ..Default::default()
        };

        assert_eq!(config.delimiter, Some(b"\r\n".to_vec()));
    }
}
