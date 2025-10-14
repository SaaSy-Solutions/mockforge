//! SMTP server mocking for MockForge
//!
//! This crate provides SMTP server functionality for MockForge, allowing you to mock
//! email servers for testing purposes.
//!
//! # Example
//!
//! ```no_run
//! use mockforge_smtp::{SmtpServer, SmtpConfig, SmtpSpecRegistry};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = SmtpConfig::default();
//!     let registry = Arc::new(SmtpSpecRegistry::new());
//!
//!     let server = SmtpServer::new(config, registry);
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

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// SMTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    /// Server port (default: 1025)
    pub port: u16,
    /// Host address (default: 0.0.0.0)
    pub host: String,
    /// Server hostname for SMTP greeting
    pub hostname: String,
    /// Directory containing fixture files
    pub fixtures_dir: Option<PathBuf>,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
    /// Maximum connections
    pub max_connections: usize,
    /// Enable mailbox storage
    pub enable_mailbox: bool,
    /// Maximum mailbox size
    pub max_mailbox_messages: usize,
    /// Enable STARTTLS support
    pub enable_starttls: bool,
    /// Path to TLS certificate file
    pub tls_cert_path: Option<PathBuf>,
    /// Path to TLS private key file
    pub tls_key_path: Option<PathBuf>,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            port: 1025,
            host: "0.0.0.0".to_string(),
            hostname: "mockforge-smtp".to_string(),
            fixtures_dir: Some(PathBuf::from("./fixtures/smtp")),
            timeout_secs: 300,
            max_connections: 10,
            enable_mailbox: true,
            max_mailbox_messages: 1000,
            enable_starttls: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::protocol_abstraction::Protocol;

    #[test]
    fn test_default_config() {
        let config = SmtpConfig::default();
        assert_eq!(config.port, 1025);
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.hostname, "mockforge-smtp");
    }

    #[test]
    fn test_protocol_display() {
        assert_eq!(Protocol::Smtp.to_string(), "SMTP");
    }
}
