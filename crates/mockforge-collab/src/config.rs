//! Configuration for collaboration server

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Collaboration server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabConfig {
    /// JWT secret for authentication
    pub jwt_secret: String,
    /// Database URL (SQLite or PostgreSQL)
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
        }
    }
}

impl CollabConfig {
    /// Load configuration from environment variables
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
