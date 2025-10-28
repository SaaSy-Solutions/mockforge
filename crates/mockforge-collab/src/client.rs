//! Collaboration client for connecting to servers

use crate::error::{CollabError, Result};
use crate::sync::SyncMessage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Server WebSocket URL
    pub server_url: String,
    /// Authentication token
    pub auth_token: String,
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected and ready
    Connected,
    /// Reconnecting after error
    Reconnecting,
}

/// Collaboration client
pub struct CollabClient {
    /// Configuration
    config: ClientConfig,
    /// Client ID
    client_id: Uuid,
    /// Connection state
    state: Arc<RwLock<ConnectionState>>,
}

impl CollabClient {
    /// Connect to a collaboration server
    pub async fn connect(config: ClientConfig) -> Result<Self> {
        let client = Self {
            config,
            client_id: Uuid::new_v4(),
            state: Arc::new(RwLock::new(ConnectionState::Connecting)),
        };

        // TODO: Implement WebSocket connection

        *client.state.write().await = ConnectionState::Connected;

        Ok(client)
    }

    /// Subscribe to a workspace
    pub async fn subscribe_to_workspace(&self, workspace_id: &str) -> Result<()> {
        let workspace_id = Uuid::parse_str(workspace_id)
            .map_err(|e| CollabError::InvalidInput(format!("Invalid workspace ID: {}", e)))?;

        let _message = SyncMessage::Subscribe { workspace_id };

        // TODO: Send message over WebSocket

        Ok(())
    }

    /// Get connection state
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Disconnect from server
    pub async fn disconnect(&self) -> Result<()> {
        *self.state.write().await = ConnectionState::Disconnected;
        Ok(())
    }
}
