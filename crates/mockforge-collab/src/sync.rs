//! Real-time synchronization engine

use crate::error::Result;
use crate::events::{ChangeEvent, EventBus};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Sync message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMessage {
    /// Client subscribes to workspace
    Subscribe { workspace_id: Uuid },
    /// Client unsubscribes from workspace
    Unsubscribe { workspace_id: Uuid },
    /// Change event notification
    Change { event: ChangeEvent },
    /// Sync state request
    StateRequest { workspace_id: Uuid, version: i64 },
    /// Sync state response
    StateResponse {
        workspace_id: Uuid,
        version: i64,
        state: serde_json::Value,
    },
    /// Heartbeat/ping
    Ping,
    /// Heartbeat/pong response
    Pong,
    /// Error message
    Error { message: String },
}

/// Sync state for a workspace
#[derive(Debug, Clone)]
pub struct SyncState {
    /// Current version
    pub version: i64,
    /// Full state
    pub state: serde_json::Value,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl SyncState {
    /// Create a new sync state
    pub fn new(version: i64, state: serde_json::Value) -> Self {
        Self {
            version,
            state,
            last_updated: chrono::Utc::now(),
        }
    }

    /// Update the state
    pub fn update(&mut self, new_state: serde_json::Value) {
        self.version += 1;
        self.state = new_state;
        self.last_updated = chrono::Utc::now();
    }
}

/// Sync engine for managing real-time synchronization
pub struct SyncEngine {
    /// Event bus for broadcasting changes
    event_bus: Arc<EventBus>,
    /// Workspace states cache
    states: DashMap<Uuid, SyncState>,
    /// Active connections per workspace
    connections: DashMap<Uuid, Vec<Uuid>>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            event_bus,
            states: DashMap::new(),
            connections: DashMap::new(),
        }
    }

    /// Subscribe a client to a workspace
    pub fn subscribe(
        &self,
        workspace_id: Uuid,
        client_id: Uuid,
    ) -> Result<broadcast::Receiver<ChangeEvent>> {
        // Add to connections list
        self.connections.entry(workspace_id).or_insert_with(Vec::new).push(client_id);

        // Return event receiver
        Ok(self.event_bus.subscribe())
    }

    /// Unsubscribe a client from a workspace
    pub fn unsubscribe(&self, workspace_id: Uuid, client_id: Uuid) -> Result<()> {
        if let Some(mut connections) = self.connections.get_mut(&workspace_id) {
            connections.retain(|id| *id != client_id);
        }
        Ok(())
    }

    /// Publish a change event
    pub fn publish_change(&self, event: ChangeEvent) -> Result<()> {
        self.event_bus.publish(event)
    }

    /// Get current state for a workspace
    pub fn get_state(&self, workspace_id: Uuid) -> Option<SyncState> {
        self.states.get(&workspace_id).map(|s| s.clone())
    }

    /// Update state for a workspace
    pub fn update_state(&self, workspace_id: Uuid, new_state: serde_json::Value) -> Result<()> {
        if let Some(mut state) = self.states.get_mut(&workspace_id) {
            state.update(new_state);
        } else {
            self.states.insert(workspace_id, SyncState::new(1, new_state));
        }
        Ok(())
    }

    /// Get connected clients for a workspace
    pub fn get_connections(&self, workspace_id: Uuid) -> Vec<Uuid> {
        self.connections.get(&workspace_id).map(|c| c.clone()).unwrap_or_default()
    }

    /// Get total number of connections
    pub fn connection_count(&self) -> usize {
        self.connections.iter().map(|c| c.value().len()).sum()
    }

    /// Check if a workspace has any active connections
    pub fn has_connections(&self, workspace_id: Uuid) -> bool {
        self.connections.get(&workspace_id).map(|c| !c.is_empty()).unwrap_or(false)
    }

    /// Clean up inactive workspaces (no connections)
    pub fn cleanup_inactive(&self) {
        let inactive: Vec<Uuid> = self
            .connections
            .iter()
            .filter(|entry| entry.value().is_empty())
            .map(|entry| *entry.key())
            .collect();

        for workspace_id in inactive {
            self.connections.remove(&workspace_id);
        }
    }
}

/// Conflict-free replicated data type (CRDT) helpers
pub mod crdt {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    /// Last-write-wins register
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LwwRegister<T> {
        /// Current value
        pub value: T,
        /// Timestamp (logical clock)
        pub timestamp: u64,
        /// Client ID that made the last write
        pub client_id: Uuid,
    }

    impl<T> LwwRegister<T> {
        /// Create a new LWW register
        pub fn new(value: T, timestamp: u64, client_id: Uuid) -> Self {
            Self {
                value,
                timestamp,
                client_id,
            }
        }

        /// Merge with another register (keep the latest)
        pub fn merge(&mut self, other: Self)
        where
            T: Clone,
        {
            if other.timestamp > self.timestamp
                || (other.timestamp == self.timestamp && other.client_id > self.client_id)
            {
                self.value = other.value;
                self.timestamp = other.timestamp;
                self.client_id = other.client_id;
            }
        }
    }

    /// Operation-based CRDT for text editing
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TextOperation {
        /// Position in the text
        pub position: usize,
        /// Operation type
        pub op: TextOp,
        /// Timestamp
        pub timestamp: u64,
        /// Client ID
        pub client_id: Uuid,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "lowercase")]
    pub enum TextOp {
        /// Insert text
        Insert { text: String },
        /// Delete text
        Delete { length: usize },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_state() {
        let mut state = SyncState::new(1, serde_json::json!({"key": "value"}));
        assert_eq!(state.version, 1);

        state.update(serde_json::json!({"key": "new_value"}));
        assert_eq!(state.version, 2);
    }

    #[test]
    fn test_sync_engine() {
        let event_bus = Arc::new(EventBus::new(100));
        let engine = SyncEngine::new(event_bus);

        let workspace_id = Uuid::new_v4();
        let client_id = Uuid::new_v4();

        assert_eq!(engine.connection_count(), 0);

        let _rx = engine.subscribe(workspace_id, client_id).unwrap();
        assert_eq!(engine.connection_count(), 1);
        assert!(engine.has_connections(workspace_id));

        engine.unsubscribe(workspace_id, client_id).unwrap();
        assert_eq!(engine.connection_count(), 0);
    }

    #[test]
    fn test_state_management() {
        let event_bus = Arc::new(EventBus::new(100));
        let engine = SyncEngine::new(event_bus);

        let workspace_id = Uuid::new_v4();
        let state = serde_json::json!({"mocks": []});

        engine.update_state(workspace_id, state.clone()).unwrap();

        let retrieved = engine.get_state(workspace_id).unwrap();
        assert_eq!(retrieved.version, 1);
        assert_eq!(retrieved.state, state);
    }

    #[test]
    fn test_crdt_lww_register() {
        use super::crdt::LwwRegister;

        let client1 = Uuid::new_v4();
        let client2 = Uuid::new_v4();

        let mut reg1 = LwwRegister::new("value1", 1, client1);
        let reg2 = LwwRegister::new("value2", 2, client2);

        reg1.merge(reg2);
        assert_eq!(reg1.value, "value2");
        assert_eq!(reg1.timestamp, 2);
    }

    #[test]
    fn test_cleanup_inactive() {
        let event_bus = Arc::new(EventBus::new(100));
        let engine = SyncEngine::new(event_bus);

        let workspace_id = Uuid::new_v4();
        let client_id = Uuid::new_v4();

        let _rx = engine.subscribe(workspace_id, client_id).unwrap();
        assert_eq!(engine.connection_count(), 1);

        engine.unsubscribe(workspace_id, client_id).unwrap();
        assert_eq!(engine.connection_count(), 0);

        engine.cleanup_inactive();
        assert!(!engine.has_connections(workspace_id));
    }
}
