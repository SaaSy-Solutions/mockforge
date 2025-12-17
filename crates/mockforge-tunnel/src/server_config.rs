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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::default();
        assert_eq!(config.cert_path, PathBuf::from("cert.pem"));
        assert_eq!(config.key_path, PathBuf::from("key.pem"));
        assert!(!config.enabled);
    }

    #[test]
    fn test_tls_config_clone() {
        let config = TlsConfig {
            cert_path: PathBuf::from("/path/to/cert.pem"),
            key_path: PathBuf::from("/path/to/key.pem"),
            enabled: true,
        };

        let cloned = config.clone();
        assert_eq!(config.cert_path, cloned.cert_path);
        assert_eq!(config.key_path, cloned.key_path);
        assert_eq!(config.enabled, cloned.enabled);
    }

    #[test]
    fn test_tls_config_debug() {
        let config = TlsConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("TlsConfig"));
        assert!(debug.contains("cert.pem"));
        assert!(debug.contains("key.pem"));
    }

    #[test]
    fn test_tls_config_serialize() {
        let config = TlsConfig {
            cert_path: PathBuf::from("test.crt"),
            key_path: PathBuf::from("test.key"),
            enabled: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test.crt"));
        assert!(json.contains("test.key"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_tls_config_deserialize() {
        let json = r#"{
            "cert_path": "custom.pem",
            "key_path": "custom.key",
            "enabled": true
        }"#;

        let config: TlsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.cert_path, PathBuf::from("custom.pem"));
        assert_eq!(config.key_path, PathBuf::from("custom.key"));
        assert!(config.enabled);
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.port, 4040);
        assert_eq!(config.bind_address, "0.0.0.0");
        assert!(config.tls.is_none());
        assert_eq!(config.database_path, Some(PathBuf::from("tunnels.db")));
        assert!(!config.use_in_memory_storage);
        assert!(config.audit_logging_enabled);
        assert!(config.audit_log_path.is_none());
    }

    #[test]
    fn test_server_config_clone() {
        let config = ServerConfig::default();
        let cloned = config.clone();

        assert_eq!(config.port, cloned.port);
        assert_eq!(config.bind_address, cloned.bind_address);
        assert_eq!(config.use_in_memory_storage, cloned.use_in_memory_storage);
    }

    #[test]
    fn test_server_config_debug() {
        let config = ServerConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("ServerConfig"));
        assert!(debug.contains("4040"));
        assert!(debug.contains("0.0.0.0"));
    }

    #[test]
    fn test_server_config_serialize() {
        let config = ServerConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"port\":4040"));
        assert!(json.contains("\"bind_address\":\"0.0.0.0\""));
    }

    #[test]
    fn test_server_config_deserialize() {
        let json = r#"{
            "port": 8080,
            "bind_address": "127.0.0.1",
            "tls": null,
            "database_path": null,
            "use_in_memory_storage": true,
            "rate_limit": {
                "global_requests_per_minute": 1000,
                "per_ip_requests_per_minute": 100,
                "burst": 200,
                "per_ip": true,
                "enabled": true
            },
            "audit_logging_enabled": false,
            "audit_log_path": null
        }"#;

        let config: ServerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.port, 8080);
        assert_eq!(config.bind_address, "127.0.0.1");
        assert!(config.use_in_memory_storage);
        assert!(!config.audit_logging_enabled);
    }

    #[test]
    fn test_server_config_from_env_default() {
        // Clear relevant env vars
        env::remove_var("TUNNEL_SERVER_PORT");
        env::remove_var("TUNNEL_SERVER_BIND");
        env::remove_var("TUNNEL_DATABASE_PATH");
        env::remove_var("TUNNEL_USE_IN_MEMORY_STORAGE");
        env::remove_var("TUNNEL_TLS_CERT");
        env::remove_var("TUNNEL_TLS_KEY");
        env::remove_var("TUNNEL_RATE_LIMIT_ENABLED");
        env::remove_var("TUNNEL_RATE_LIMIT_RPM");
        env::remove_var("TUNNEL_AUDIT_LOG_ENABLED");
        env::remove_var("TUNNEL_AUDIT_LOG_PATH");

        let config = ServerConfig::from_env();
        assert_eq!(config.port, 4040);
        assert_eq!(config.bind_address, "0.0.0.0");
    }

    #[test]
    fn test_server_config_from_env_custom_port() {
        env::set_var("TUNNEL_SERVER_PORT", "9000");

        let config = ServerConfig::from_env();
        assert_eq!(config.port, 9000);

        env::remove_var("TUNNEL_SERVER_PORT");
    }

    #[test]
    fn test_server_config_from_env_custom_bind() {
        env::set_var("TUNNEL_SERVER_BIND", "127.0.0.1");

        let config = ServerConfig::from_env();
        assert_eq!(config.bind_address, "127.0.0.1");

        env::remove_var("TUNNEL_SERVER_BIND");
    }

    #[test]
    fn test_server_config_from_env_custom_database_path() {
        env::set_var("TUNNEL_DATABASE_PATH", "/custom/path/tunnels.db");

        let config = ServerConfig::from_env();
        assert_eq!(config.database_path, Some(PathBuf::from("/custom/path/tunnels.db")));

        env::remove_var("TUNNEL_DATABASE_PATH");
    }

    #[test]
    fn test_server_config_from_env_in_memory_storage() {
        env::set_var("TUNNEL_USE_IN_MEMORY_STORAGE", "1");

        let config = ServerConfig::from_env();
        assert!(config.use_in_memory_storage);
        assert!(config.database_path.is_none());

        env::remove_var("TUNNEL_USE_IN_MEMORY_STORAGE");
    }

    #[test]
    fn test_server_config_from_env_tls_enabled() {
        env::set_var("TUNNEL_TLS_CERT", "/path/to/cert.pem");
        env::set_var("TUNNEL_TLS_KEY", "/path/to/key.pem");

        let config = ServerConfig::from_env();
        assert!(config.tls.is_some());

        let tls = config.tls.unwrap();
        assert_eq!(tls.cert_path, PathBuf::from("/path/to/cert.pem"));
        assert_eq!(tls.key_path, PathBuf::from("/path/to/key.pem"));
        assert!(tls.enabled);

        env::remove_var("TUNNEL_TLS_CERT");
        env::remove_var("TUNNEL_TLS_KEY");
    }

    #[test]
    fn test_server_config_from_env_tls_incomplete() {
        // Only cert, no key - should not enable TLS
        env::set_var("TUNNEL_TLS_CERT", "/path/to/cert.pem");
        env::remove_var("TUNNEL_TLS_KEY");

        let config = ServerConfig::from_env();
        assert!(config.tls.is_none());

        env::remove_var("TUNNEL_TLS_CERT");
    }

    #[test]
    fn test_server_config_from_env_rate_limit_disabled() {
        env::set_var("TUNNEL_RATE_LIMIT_ENABLED", "false");

        let config = ServerConfig::from_env();
        assert!(!config.rate_limit.enabled);

        env::remove_var("TUNNEL_RATE_LIMIT_ENABLED");
    }

    #[test]
    fn test_server_config_from_env_rate_limit_rpm() {
        env::set_var("TUNNEL_RATE_LIMIT_RPM", "5000");

        let config = ServerConfig::from_env();
        assert_eq!(config.rate_limit.global_requests_per_minute, 5000);

        env::remove_var("TUNNEL_RATE_LIMIT_RPM");
    }

    #[test]
    fn test_server_config_from_env_audit_logging_disabled() {
        env::set_var("TUNNEL_AUDIT_LOG_ENABLED", "false");

        let config = ServerConfig::from_env();
        assert!(!config.audit_logging_enabled);

        env::remove_var("TUNNEL_AUDIT_LOG_ENABLED");
    }

    #[test]
    fn test_server_config_from_env_audit_log_path() {
        env::set_var("TUNNEL_AUDIT_LOG_PATH", "/var/log/tunnel-audit.log");

        let config = ServerConfig::from_env();
        assert_eq!(config.audit_log_path, Some(PathBuf::from("/var/log/tunnel-audit.log")));

        env::remove_var("TUNNEL_AUDIT_LOG_PATH");
    }

    #[test]
    fn test_server_config_from_env_invalid_port() {
        env::set_var("TUNNEL_SERVER_PORT", "invalid");

        let config = ServerConfig::from_env();
        // Should fall back to default
        assert_eq!(config.port, 4040);

        env::remove_var("TUNNEL_SERVER_PORT");
    }

    #[test]
    fn test_server_config_from_env_invalid_rate_limit() {
        env::set_var("TUNNEL_RATE_LIMIT_RPM", "not_a_number");

        let config = ServerConfig::from_env();
        // Should fall back to default
        assert_eq!(config.rate_limit.global_requests_per_minute, 1000);

        env::remove_var("TUNNEL_RATE_LIMIT_RPM");
    }

    #[test]
    fn test_server_config_with_tls_some() {
        let tls_config = TlsConfig {
            cert_path: PathBuf::from("server.crt"),
            key_path: PathBuf::from("server.key"),
            enabled: true,
        };

        let config = ServerConfig {
            tls: Some(tls_config.clone()),
            ..Default::default()
        };

        assert!(config.tls.is_some());
        let tls = config.tls.unwrap();
        assert_eq!(tls.cert_path, PathBuf::from("server.crt"));
        assert!(tls.enabled);
    }

    #[test]
    fn test_server_config_custom_values() {
        let config = ServerConfig {
            port: 8443,
            bind_address: "192.168.1.100".to_string(),
            tls: None,
            database_path: Some(PathBuf::from("/data/tunnels.db")),
            use_in_memory_storage: false,
            rate_limit: crate::rate_limit::RateLimitConfig {
                global_requests_per_minute: 2000,
                per_ip_requests_per_minute: 200,
                burst: 400,
                per_ip: false,
                enabled: false,
            },
            audit_logging_enabled: false,
            audit_log_path: Some(PathBuf::from("/logs/audit.log")),
        };

        assert_eq!(config.port, 8443);
        assert_eq!(config.bind_address, "192.168.1.100");
        assert_eq!(config.database_path, Some(PathBuf::from("/data/tunnels.db")));
        assert!(!config.rate_limit.enabled);
        assert!(!config.audit_logging_enabled);
        assert_eq!(config.audit_log_path, Some(PathBuf::from("/logs/audit.log")));
    }

    #[test]
    fn test_server_config_roundtrip_serialization() {
        let config = ServerConfig {
            port: 5050,
            bind_address: "0.0.0.0".to_string(),
            tls: Some(TlsConfig {
                cert_path: PathBuf::from("test.crt"),
                key_path: PathBuf::from("test.key"),
                enabled: true,
            }),
            database_path: Some(PathBuf::from("test.db")),
            use_in_memory_storage: false,
            rate_limit: crate::rate_limit::RateLimitConfig::default(),
            audit_logging_enabled: true,
            audit_log_path: Some(PathBuf::from("audit.log")),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ServerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.port, deserialized.port);
        assert_eq!(config.bind_address, deserialized.bind_address);
        assert_eq!(config.use_in_memory_storage, deserialized.use_in_memory_storage);
        assert_eq!(config.audit_logging_enabled, deserialized.audit_logging_enabled);
    }

    #[test]
    fn test_server_config_env_vars_combined() {
        // Set multiple env vars
        env::set_var("TUNNEL_SERVER_PORT", "7777");
        env::set_var("TUNNEL_SERVER_BIND", "10.0.0.1");
        env::set_var("TUNNEL_DATABASE_PATH", "/tmp/test.db");
        env::set_var("TUNNEL_RATE_LIMIT_RPM", "3000");
        env::set_var("TUNNEL_AUDIT_LOG_ENABLED", "true");

        let config = ServerConfig::from_env();

        assert_eq!(config.port, 7777);
        assert_eq!(config.bind_address, "10.0.0.1");
        assert_eq!(config.database_path, Some(PathBuf::from("/tmp/test.db")));
        assert_eq!(config.rate_limit.global_requests_per_minute, 3000);
        assert!(config.audit_logging_enabled);

        // Clean up
        env::remove_var("TUNNEL_SERVER_PORT");
        env::remove_var("TUNNEL_SERVER_BIND");
        env::remove_var("TUNNEL_DATABASE_PATH");
        env::remove_var("TUNNEL_RATE_LIMIT_RPM");
        env::remove_var("TUNNEL_AUDIT_LOG_ENABLED");
    }
}
