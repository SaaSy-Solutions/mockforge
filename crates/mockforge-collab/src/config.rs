//! Configuration for collaboration server

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Collaboration server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabConfig {
    /// JWT secret for authentication
    pub jwt_secret: String,
    /// Database URL (`SQLite` or `PostgreSQL`)
    pub database_url: String,
    /// Server bind address
    pub bind_address: String,
    /// Maximum connections per workspace
    pub max_connections_per_workspace: usize,
    /// Event bus capacity
    pub event_bus_capacity: usize,
    /// Enable auto-commit for changes
    pub auto_commit: bool,
    /// Session timeout duration
    pub session_timeout: Duration,
    /// WebSocket ping interval
    pub websocket_ping_interval: Duration,
    /// Maximum message size (bytes)
    pub max_message_size: usize,
    /// Directory for workspace storage (for `CoreBridge`)
    pub workspace_dir: Option<String>,
    /// Directory for backup storage
    pub backup_dir: Option<String>,
}

impl Default for CollabConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "change-me-in-production".to_string(),
            database_url: "sqlite://mockforge-collab.db".to_string(),
            bind_address: "127.0.0.1:8080".to_string(),
            max_connections_per_workspace: 100,
            event_bus_capacity: 1000,
            auto_commit: true,
            session_timeout: Duration::from_secs(24 * 3600), // 24 hours
            websocket_ping_interval: Duration::from_secs(30),
            max_message_size: 1024 * 1024, // 1 MB
            workspace_dir: Some("./workspaces".to_string()),
            backup_dir: Some("./backups".to_string()),
        }
    }
}

impl CollabConfig {
    /// Load configuration from environment variables
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            jwt_secret: std::env::var("MOCKFORGE_JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            database_url: std::env::var("MOCKFORGE_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://mockforge-collab.db".to_string()),
            bind_address: std::env::var("MOCKFORGE_BIND_ADDRESS")
                .unwrap_or_else(|_| "127.0.0.1:8080".to_string()),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CollabConfig::default();

        assert_eq!(config.jwt_secret, "change-me-in-production");
        assert_eq!(config.database_url, "sqlite://mockforge-collab.db");
        assert_eq!(config.bind_address, "127.0.0.1:8080");
        assert_eq!(config.max_connections_per_workspace, 100);
        assert_eq!(config.event_bus_capacity, 1000);
        assert!(config.auto_commit);
        assert_eq!(config.session_timeout, Duration::from_secs(24 * 3600));
        assert_eq!(config.websocket_ping_interval, Duration::from_secs(30));
        assert_eq!(config.max_message_size, 1024 * 1024);
        assert_eq!(config.workspace_dir, Some("./workspaces".to_string()));
        assert_eq!(config.backup_dir, Some("./backups".to_string()));
    }

    #[test]
    fn test_from_env_defaults() {
        // Clear environment variables
        std::env::remove_var("MOCKFORGE_JWT_SECRET");
        std::env::remove_var("MOCKFORGE_DATABASE_URL");
        std::env::remove_var("MOCKFORGE_BIND_ADDRESS");

        let config = CollabConfig::from_env();

        assert_eq!(config.jwt_secret, "change-me-in-production");
        assert_eq!(config.database_url, "sqlite://mockforge-collab.db");
        assert_eq!(config.bind_address, "127.0.0.1:8080");
    }

    #[test]
    fn test_from_env_with_values() {
        // Set environment variables
        std::env::set_var("MOCKFORGE_JWT_SECRET", "test-secret");
        std::env::set_var("MOCKFORGE_DATABASE_URL", "postgres://localhost/test");
        std::env::set_var("MOCKFORGE_BIND_ADDRESS", "0.0.0.0:9090");

        let config = CollabConfig::from_env();

        assert_eq!(config.jwt_secret, "test-secret");
        assert_eq!(config.database_url, "postgres://localhost/test");
        assert_eq!(config.bind_address, "0.0.0.0:9090");

        // Clean up
        std::env::remove_var("MOCKFORGE_JWT_SECRET");
        std::env::remove_var("MOCKFORGE_DATABASE_URL");
        std::env::remove_var("MOCKFORGE_BIND_ADDRESS");
    }

    #[test]
    fn test_config_serialization() {
        let config = CollabConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: CollabConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(config.jwt_secret, deserialized.jwt_secret);
        assert_eq!(config.database_url, deserialized.database_url);
        assert_eq!(config.bind_address, deserialized.bind_address);
    }

    #[test]
    fn test_config_clone() {
        let config = CollabConfig::default();
        let cloned = config.clone();

        assert_eq!(config.jwt_secret, cloned.jwt_secret);
        assert_eq!(config.max_connections_per_workspace, cloned.max_connections_per_workspace);
    }

    #[test]
    fn test_config_debug() {
        let config = CollabConfig::default();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("CollabConfig"));
        assert!(debug_str.contains("jwt_secret"));
    }
}
