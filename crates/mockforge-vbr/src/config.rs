//! VBR engine configuration
//!
//! This module defines the configuration structure for the Virtual Backend Reality engine,
//! including storage backend selection, entity definitions, session configuration, and
//! time-based data evolution settings.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Storage backend type for the virtual database
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "backend", rename_all = "lowercase")]
pub enum StorageBackend {
    /// SQLite database backend (persistent, production-like)
    Sqlite {
        /// Path to the SQLite database file
        path: PathBuf,
    },
    /// JSON file backend (human-readable, easy to inspect)
    Json {
        /// Path to the JSON file
        path: PathBuf,
    },
    /// In-memory backend (fast, no persistence)
    Memory,
}

impl Default for StorageBackend {
    fn default() -> Self {
        StorageBackend::Sqlite {
            path: PathBuf::from("./data/vbr.db"),
        }
    }
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Whether sessions are enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Session timeout in seconds
    #[serde(default = "default_session_timeout")]
    pub timeout: u64,

    /// Whether to use session-scoped data (per-session virtual DB)
    #[serde(default)]
    pub scoped_data: bool,
}

fn default_true() -> bool {
    true
}

fn default_session_timeout() -> u64 {
    3600 // 1 hour
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout: default_session_timeout(),
            scoped_data: false,
        }
    }
}

/// Data aging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgingConfig {
    /// Whether data aging is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Cleanup interval in seconds
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval: u64,

    /// Whether to automatically update timestamp fields
    #[serde(default = "default_true")]
    pub auto_update_timestamps: bool,
}

fn default_cleanup_interval() -> u64 {
    3600 // 1 hour
}

impl Default for AgingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cleanup_interval: default_cleanup_interval(),
            auto_update_timestamps: true,
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Whether authentication is enabled
    #[serde(default)]
    pub enabled: bool,

    /// JWT secret for token generation
    #[serde(default)]
    pub jwt_secret: Option<String>,

    /// Token expiration in seconds
    #[serde(default = "default_token_expiration")]
    pub token_expiration: u64,
}

fn default_token_expiration() -> u64 {
    86400 // 24 hours
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            jwt_secret: None,
            token_expiration: default_token_expiration(),
        }
    }
}

/// VBR engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VbrConfig {
    /// Storage backend configuration
    #[serde(default)]
    pub storage: StorageBackend,

    /// Session configuration
    #[serde(default)]
    pub sessions: SessionConfig,

    /// Data aging configuration
    #[serde(default)]
    pub aging: AgingConfig,

    /// Authentication configuration
    #[serde(default)]
    pub auth: AuthConfig,

    /// API base path prefix
    #[serde(default = "default_api_prefix")]
    pub api_prefix: String,
}

fn default_api_prefix() -> String {
    "/api".to_string()
}

impl Default for VbrConfig {
    fn default() -> Self {
        Self {
            storage: StorageBackend::default(),
            sessions: SessionConfig::default(),
            aging: AgingConfig::default(),
            auth: AuthConfig::default(),
            api_prefix: default_api_prefix(),
        }
    }
}

impl VbrConfig {
    /// Create a new VBR configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the storage backend
    pub fn with_storage_backend(mut self, backend: StorageBackend) -> Self {
        self.storage = backend;
        self
    }

    /// Set session configuration
    pub fn with_sessions(mut self, config: SessionConfig) -> Self {
        self.sessions = config;
        self
    }

    /// Set aging configuration
    pub fn with_aging(mut self, config: AgingConfig) -> Self {
        self.aging = config;
        self
    }

    /// Set authentication configuration
    pub fn with_auth(mut self, config: AuthConfig) -> Self {
        self.auth = config;
        self
    }

    /// Set the API prefix
    pub fn with_api_prefix(mut self, prefix: String) -> Self {
        self.api_prefix = prefix;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // StorageBackend tests
    #[test]
    fn test_storage_backend_default() {
        let backend = StorageBackend::default();
        assert!(matches!(backend, StorageBackend::Sqlite { .. }));
    }

    #[test]
    fn test_storage_backend_sqlite() {
        let backend = StorageBackend::Sqlite {
            path: PathBuf::from("/tmp/test.db"),
        };
        if let StorageBackend::Sqlite { path } = backend {
            assert_eq!(path, PathBuf::from("/tmp/test.db"));
        } else {
            panic!("Expected Sqlite variant");
        }
    }

    #[test]
    fn test_storage_backend_json() {
        let backend = StorageBackend::Json {
            path: PathBuf::from("/tmp/data.json"),
        };
        if let StorageBackend::Json { path } = backend {
            assert_eq!(path, PathBuf::from("/tmp/data.json"));
        } else {
            panic!("Expected Json variant");
        }
    }

    #[test]
    fn test_storage_backend_memory() {
        let backend = StorageBackend::Memory;
        assert!(matches!(backend, StorageBackend::Memory));
    }

    #[test]
    fn test_storage_backend_serialize_sqlite() {
        let backend = StorageBackend::Sqlite {
            path: PathBuf::from("./data/vbr.db"),
        };
        let json = serde_json::to_string(&backend).unwrap();
        assert!(json.contains("\"backend\":\"sqlite\""));
        assert!(json.contains("vbr.db"));
    }

    #[test]
    fn test_storage_backend_serialize_memory() {
        let backend = StorageBackend::Memory;
        let json = serde_json::to_string(&backend).unwrap();
        assert!(json.contains("\"backend\":\"memory\""));
    }

    #[test]
    fn test_storage_backend_clone() {
        let backend = StorageBackend::Memory;
        let cloned = backend.clone();
        assert!(matches!(cloned, StorageBackend::Memory));
    }

    #[test]
    fn test_storage_backend_debug() {
        let backend = StorageBackend::Memory;
        let debug = format!("{:?}", backend);
        assert!(debug.contains("Memory"));
    }

    #[test]
    fn test_storage_backend_eq() {
        let b1 = StorageBackend::Memory;
        let b2 = StorageBackend::Memory;
        assert_eq!(b1, b2);

        let b3 = StorageBackend::Sqlite {
            path: PathBuf::from("a.db"),
        };
        let b4 = StorageBackend::Sqlite {
            path: PathBuf::from("b.db"),
        };
        assert_ne!(b3, b4);
    }

    // SessionConfig tests
    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert!(config.enabled);
        assert_eq!(config.timeout, 3600);
        assert!(!config.scoped_data);
    }

    #[test]
    fn test_session_config_clone() {
        let config = SessionConfig {
            enabled: false,
            timeout: 7200,
            scoped_data: true,
        };
        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.timeout, cloned.timeout);
        assert_eq!(config.scoped_data, cloned.scoped_data);
    }

    #[test]
    fn test_session_config_serialize() {
        let config = SessionConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"timeout\":3600"));
    }

    #[test]
    fn test_session_config_deserialize() {
        let json = r#"{"enabled": false, "timeout": 7200, "scoped_data": true}"#;
        let config: SessionConfig = serde_json::from_str(json).unwrap();
        assert!(!config.enabled);
        assert_eq!(config.timeout, 7200);
        assert!(config.scoped_data);
    }

    #[test]
    fn test_session_config_debug() {
        let config = SessionConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("SessionConfig"));
    }

    // AgingConfig tests
    #[test]
    fn test_aging_config_default() {
        let config = AgingConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.cleanup_interval, 3600);
        assert!(config.auto_update_timestamps);
    }

    #[test]
    fn test_aging_config_clone() {
        let config = AgingConfig {
            enabled: true,
            cleanup_interval: 1800,
            auto_update_timestamps: false,
        };
        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.cleanup_interval, cloned.cleanup_interval);
    }

    #[test]
    fn test_aging_config_serialize() {
        let config = AgingConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"enabled\":false"));
        assert!(json.contains("\"cleanup_interval\":3600"));
    }

    #[test]
    fn test_aging_config_debug() {
        let config = AgingConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("AgingConfig"));
    }

    // AuthConfig tests
    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.enabled);
        assert!(config.jwt_secret.is_none());
        assert_eq!(config.token_expiration, 86400);
    }

    #[test]
    fn test_auth_config_with_secret() {
        let config = AuthConfig {
            enabled: true,
            jwt_secret: Some("my-secret-key".to_string()),
            token_expiration: 3600,
        };
        assert!(config.enabled);
        assert_eq!(config.jwt_secret, Some("my-secret-key".to_string()));
        assert_eq!(config.token_expiration, 3600);
    }

    #[test]
    fn test_auth_config_clone() {
        let config = AuthConfig {
            enabled: true,
            jwt_secret: Some("secret".to_string()),
            token_expiration: 7200,
        };
        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.jwt_secret, cloned.jwt_secret);
    }

    #[test]
    fn test_auth_config_serialize() {
        let config = AuthConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"enabled\":false"));
        assert!(json.contains("\"token_expiration\":86400"));
    }

    #[test]
    fn test_auth_config_debug() {
        let config = AuthConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("AuthConfig"));
    }

    // VbrConfig tests
    #[test]
    fn test_vbr_config_default() {
        let config = VbrConfig::default();
        assert!(config.sessions.enabled);
        assert_eq!(config.sessions.timeout, 3600);
        assert!(!config.aging.enabled);
        assert!(!config.auth.enabled);
        assert_eq!(config.api_prefix, "/api");
    }

    #[test]
    fn test_vbr_config_new() {
        let config = VbrConfig::new();
        assert_eq!(config.api_prefix, "/api");
    }

    #[test]
    fn test_vbr_config_builder() {
        let config = VbrConfig::new()
            .with_api_prefix("/v1/api".to_string())
            .with_storage_backend(StorageBackend::Memory);

        assert_eq!(config.api_prefix, "/v1/api");
        assert!(matches!(config.storage, StorageBackend::Memory));
    }

    #[test]
    fn test_vbr_config_with_sessions() {
        let session_config = SessionConfig {
            enabled: false,
            timeout: 1800,
            scoped_data: true,
        };

        let config = VbrConfig::new().with_sessions(session_config);
        assert!(!config.sessions.enabled);
        assert_eq!(config.sessions.timeout, 1800);
        assert!(config.sessions.scoped_data);
    }

    #[test]
    fn test_vbr_config_with_aging() {
        let aging_config = AgingConfig {
            enabled: true,
            cleanup_interval: 600,
            auto_update_timestamps: false,
        };

        let config = VbrConfig::new().with_aging(aging_config);
        assert!(config.aging.enabled);
        assert_eq!(config.aging.cleanup_interval, 600);
        assert!(!config.aging.auto_update_timestamps);
    }

    #[test]
    fn test_vbr_config_with_auth() {
        let auth_config = AuthConfig {
            enabled: true,
            jwt_secret: Some("test-secret".to_string()),
            token_expiration: 3600,
        };

        let config = VbrConfig::new().with_auth(auth_config);
        assert!(config.auth.enabled);
        assert_eq!(config.auth.jwt_secret, Some("test-secret".to_string()));
    }

    #[test]
    fn test_vbr_config_full_builder_chain() {
        let config = VbrConfig::new()
            .with_storage_backend(StorageBackend::Json {
                path: PathBuf::from("/tmp/vbr.json"),
            })
            .with_sessions(SessionConfig {
                enabled: true,
                timeout: 7200,
                scoped_data: true,
            })
            .with_aging(AgingConfig {
                enabled: true,
                cleanup_interval: 300,
                auto_update_timestamps: true,
            })
            .with_auth(AuthConfig {
                enabled: true,
                jwt_secret: Some("secret".to_string()),
                token_expiration: 86400,
            })
            .with_api_prefix("/v2/api".to_string());

        assert!(matches!(config.storage, StorageBackend::Json { .. }));
        assert!(config.sessions.scoped_data);
        assert!(config.aging.enabled);
        assert!(config.auth.enabled);
        assert_eq!(config.api_prefix, "/v2/api");
    }

    #[test]
    fn test_vbr_config_clone() {
        let config = VbrConfig::new()
            .with_api_prefix("/test".to_string())
            .with_storage_backend(StorageBackend::Memory);

        let cloned = config.clone();
        assert_eq!(config.api_prefix, cloned.api_prefix);
    }

    #[test]
    fn test_vbr_config_serialize() {
        let config = VbrConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"api_prefix\":\"/api\""));
    }

    #[test]
    fn test_vbr_config_deserialize() {
        let json = r#"{
            "storage": {"backend": "memory"},
            "sessions": {"enabled": true, "timeout": 3600, "scoped_data": false},
            "aging": {"enabled": false, "cleanup_interval": 3600, "auto_update_timestamps": true},
            "auth": {"enabled": false, "jwt_secret": null, "token_expiration": 86400},
            "api_prefix": "/custom"
        }"#;

        let config: VbrConfig = serde_json::from_str(json).unwrap();
        assert!(matches!(config.storage, StorageBackend::Memory));
        assert_eq!(config.api_prefix, "/custom");
    }

    #[test]
    fn test_vbr_config_debug() {
        let config = VbrConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("VbrConfig"));
    }
}
