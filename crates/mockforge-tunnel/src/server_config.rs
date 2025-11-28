//! Server configuration for tunnel server

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to certificate file
    pub cert_path: PathBuf,
    /// Path to private key file
    pub key_path: PathBuf,
    /// Enable TLS
    pub enabled: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: PathBuf::from("cert.pem"),
            key_path: PathBuf::from("key.pem"),
            enabled: false,
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server port
    pub port: u16,
    /// Bind address
    pub bind_address: String,
    /// TLS configuration
    pub tls: Option<TlsConfig>,
    /// Database path (for persistent storage)
    pub database_path: Option<PathBuf>,
    /// Use in-memory storage (for testing)
    pub use_in_memory_storage: bool,
    /// Rate limiting configuration
    pub rate_limit: crate::rate_limit::RateLimitConfig,
    /// Enable audit logging
    pub audit_logging_enabled: bool,
    /// Audit log file path (optional)
    pub audit_log_path: Option<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 4040,
            bind_address: "0.0.0.0".to_string(),
            tls: None,
            database_path: Some(PathBuf::from("tunnels.db")),
            use_in_memory_storage: false,
            rate_limit: crate::rate_limit::RateLimitConfig::default(),
            audit_logging_enabled: true,
            audit_log_path: None,
        }
    }
}

impl ServerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(port) = std::env::var("TUNNEL_SERVER_PORT") {
            if let Ok(p) = port.parse() {
                config.port = p;
            }
        }

        if let Ok(addr) = std::env::var("TUNNEL_SERVER_BIND") {
            config.bind_address = addr;
        }

        if let Ok(db_path) = std::env::var("TUNNEL_DATABASE_PATH") {
            config.database_path = Some(PathBuf::from(db_path));
        }

        if std::env::var("TUNNEL_USE_IN_MEMORY_STORAGE").is_ok() {
            config.use_in_memory_storage = true;
            config.database_path = None;
        }

        if let Ok(cert_path) = std::env::var("TUNNEL_TLS_CERT") {
            if let Ok(key_path) = std::env::var("TUNNEL_TLS_KEY") {
                config.tls = Some(TlsConfig {
                    cert_path: PathBuf::from(cert_path),
                    key_path: PathBuf::from(key_path),
                    enabled: true,
                });
            }
        }

        if let Ok(rate_limit) = std::env::var("TUNNEL_RATE_LIMIT_ENABLED") {
            config.rate_limit.enabled = rate_limit.parse().unwrap_or(true);
        }

        if let Ok(rate_limit_rpm) = std::env::var("TUNNEL_RATE_LIMIT_RPM") {
            if let Ok(rpm) = rate_limit_rpm.parse() {
                config.rate_limit.global_requests_per_minute = rpm;
            }
        }

        if let Ok(audit_log) = std::env::var("TUNNEL_AUDIT_LOG_ENABLED") {
            config.audit_logging_enabled = audit_log.parse().unwrap_or(true);
        }

        if let Ok(audit_log_path) = std::env::var("TUNNEL_AUDIT_LOG_PATH") {
            config.audit_log_path = Some(PathBuf::from(audit_log_path));
        }

        config
    }
}
