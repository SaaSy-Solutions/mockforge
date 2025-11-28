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

    #[test]
    fn test_storage_backend_default() {
        let backend = StorageBackend::default();
        assert!(matches!(backend, StorageBackend::Sqlite { .. }));
    }

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
    fn test_vbr_config_builder() {
        let config = VbrConfig::new()
            .with_api_prefix("/v1/api".to_string())
            .with_storage_backend(StorageBackend::Memory);

        assert_eq!(config.api_prefix, "/v1/api");
        assert!(matches!(config.storage, StorageBackend::Memory));
    }
}
