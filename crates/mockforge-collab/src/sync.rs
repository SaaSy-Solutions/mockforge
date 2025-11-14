//! Real-time synchronization engine

use crate::core_bridge::CoreBridge;
use crate::error::{CollabError, Result};
use crate::events::{ChangeEvent, EventBus};
use crate::workspace::WorkspaceService;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite};
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
    pub last_updated: chrono::DateTime<Utc>,
}

impl SyncState {
    /// Create a new sync state
    pub fn new(version: i64, state: serde_json::Value) -> Self {
        Self {
            version,
            state,
            last_updated: Utc::now(),
        }
    }

    /// Update the state
    pub fn update(&mut self, new_state: serde_json::Value) {
        self.version += 1;
        self.state = new_state;
        self.last_updated = Utc::now();
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
    /// Database pool for state snapshots
    db: Option<Pool<Sqlite>>,
    /// Core bridge for workspace state conversion
    core_bridge: Option<Arc<CoreBridge>>,
    /// Workspace service for getting workspace data
    workspace_service: Option<Arc<WorkspaceService>>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            event_bus,
            states: DashMap::new(),
            connections: DashMap::new(),
            db: None,
            core_bridge: None,
            workspace_service: None,
        }
    }

    /// Create a new sync engine with database support for state snapshots
    pub fn with_db(event_bus: Arc<EventBus>, db: Pool<Sqlite>) -> Self {
        Self {
            event_bus,
            states: DashMap::new(),
            connections: DashMap::new(),
            db: Some(db),
            core_bridge: None,
            workspace_service: None,
        }
    }

    /// Create a new sync engine with full integration
    pub fn with_integration(
        event_bus: Arc<EventBus>,
        db: Pool<Sqlite>,
        core_bridge: Arc<CoreBridge>,
        workspace_service: Arc<WorkspaceService>,
    ) -> Self {
        Self {
            event_bus,
            states: DashMap::new(),
            connections: DashMap::new(),
            db: Some(db),
            core_bridge: Some(core_bridge),
            workspace_service: Some(workspace_service),
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
        let version = if let Some(state) = self.states.get(&workspace_id) {
            state.version + 1
        } else {
            1
        };

        if let Some(mut state) = self.states.get_mut(&workspace_id) {
            state.update(new_state.clone());
        } else {
            self.states.insert(workspace_id, SyncState::new(version, new_state.clone()));
        }

        // Update the TeamWorkspace in the database if we have the services
        if let (Some(core_bridge), Some(workspace_service)) =
            (self.core_bridge.as_ref(), self.workspace_service.as_ref())
        {
            let core_bridge = core_bridge.clone();
            let workspace_service = workspace_service.clone();
            let workspace_id = workspace_id;
            let state_data = new_state.clone();
            tokio::spawn(async move {
                if let Ok(mut team_workspace) = workspace_service.get_workspace(workspace_id).await
                {
                    if let Err(e) = core_bridge
                        .update_workspace_state_from_json(&mut team_workspace, &state_data)
                    {
                        tracing::error!("Failed to update workspace state: {}", e);
                    } else {
                        // Update the workspace in the database
                        // Note: This would need a method to update just the config field
                        // For now, we'll save to disk
                        if let Err(e) = core_bridge.save_workspace_to_disk(&team_workspace).await {
                            tracing::error!("Failed to save workspace to disk: {}", e);
                        }
                    }
                }
            });
        }

        // Save state snapshot to database if available
        if let Some(db) = &self.db {
            // Spawn async task to save snapshot
            let db = db.clone();
            let workspace_id = workspace_id;
            let state_data = new_state.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    Self::save_state_snapshot(&db, workspace_id, version, &state_data).await
                {
                    tracing::error!("Failed to save state snapshot: {}", e);
                }
            });
        }

        Ok(())
    }

    /// Get full workspace state for a workspace
    ///
    /// Uses CoreBridge to get the complete workspace state including all mocks.
    pub async fn get_full_workspace_state(
        &self,
        workspace_id: Uuid,
    ) -> Result<Option<serde_json::Value>> {
        if let (Some(core_bridge), Some(workspace_service)) =
            (self.core_bridge.as_ref(), self.workspace_service.as_ref())
        {
            // Get TeamWorkspace
            let team_workspace = workspace_service.get_workspace(workspace_id).await?;

            // Get full state using CoreBridge
            let state_json = core_bridge.get_workspace_state_json(&team_workspace)?;
            Ok(Some(state_json))
        } else {
            // Fallback to in-memory state
            Ok(self.get_state(workspace_id).map(|s| s.state))
        }
    }

    /// Save state snapshot to database
    async fn save_state_snapshot(
        db: &Pool<Sqlite>,
        workspace_id: Uuid,
        version: i64,
        state: &serde_json::Value,
    ) -> Result<()> {
        // Calculate hash for deduplication
        let state_json = serde_json::to_string(state)?;
        let mut hasher = Sha256::new();
        hasher.update(state_json.as_bytes());
        let state_hash = format!("{:x}", hasher.finalize());

        // Check if snapshot with this hash already exists
        let existing = sqlx::query!(
            r#"
            SELECT id FROM workspace_state_snapshots
            WHERE workspace_id = ? AND state_hash = ?
            "#,
            workspace_id,
            state_hash
        )
        .fetch_optional(db)
        .await?;

        if existing.is_some() {
            // Snapshot already exists, skip
            return Ok(());
        }

        // Insert new snapshot
        let snapshot_id = Uuid::new_v4();
        let snapshot_id_str = snapshot_id.to_string();
        let workspace_id_str = workspace_id.to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query!(
            r#"
            INSERT INTO workspace_state_snapshots (id, workspace_id, state_hash, state_data, version, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            snapshot_id_str,
            workspace_id_str,
            state_hash,
            state_json,
            version,
            now
        )
        .execute(db)
        .await?;

        Ok(())
    }

    /// Load state snapshot from database
    pub async fn load_state_snapshot(
        &self,
        workspace_id: Uuid,
        version: Option<i64>,
    ) -> Result<Option<SyncState>> {
        let db = self.db.as_ref().ok_or_else(|| {
            CollabError::Internal("Database not available for state snapshots".to_string())
        })?;

        let workspace_id_str = workspace_id.to_string();
        // Use runtime queries with query_as to avoid type mismatch between different query structures
        let snapshot: Option<(String, i64, String)> = if let Some(version) = version {
            sqlx::query_as(
                r#"
                SELECT state_data, version, created_at
                FROM workspace_state_snapshots
                WHERE workspace_id = ? AND version = ?
                ORDER BY created_at DESC
                LIMIT 1
                "#,
            )
            .bind(&workspace_id_str)
            .bind(version)
            .fetch_optional(db)
            .await?
        } else {
            sqlx::query_as(
                r#"
                SELECT state_data, version, created_at
                FROM workspace_state_snapshots
                WHERE workspace_id = ?
                ORDER BY version DESC, created_at DESC
                LIMIT 1
                "#,
            )
            .bind(&workspace_id_str)
            .fetch_optional(db)
            .await?
        };

        if let Some((state_data, snap_version, created_at_str)) = snapshot {
            let state: serde_json::Value = serde_json::from_str(&state_data)
                .map_err(|e| CollabError::Internal(format!("Failed to parse state: {}", e)))?;
            // Parse timestamp (stored as TEXT in SQLite, format: ISO8601)
            // SQLite stores timestamps as TEXT, try parsing as RFC3339 first, then fallback
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .or_else(|_| {
                    // Try parsing as ISO8601 without timezone (SQLite default format)
                    chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S%.f")
                        .or_else(|_| {
                            chrono::NaiveDateTime::parse_from_str(
                                &created_at_str,
                                "%Y-%m-%d %H:%M:%S",
                            )
                        })
                        .map(|dt| dt.and_utc())
                })
                .map_err(|e| {
                    CollabError::Internal(format!(
                        "Failed to parse timestamp '{}': {}",
                        created_at_str, e
                    ))
                })?;

            Ok(Some(SyncState {
                version: snap_version,
                state,
                last_updated: created_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Record a state change for incremental sync
    pub async fn record_state_change(
        &self,
        workspace_id: Uuid,
        change_type: &str,
        change_data: serde_json::Value,
        version: i64,
        user_id: Uuid,
    ) -> Result<()> {
        let db = self.db.as_ref().ok_or_else(|| {
            CollabError::Internal("Database not available for state changes".to_string())
        })?;

        let change_id = Uuid::new_v4();
        let change_id_str = change_id.to_string();
        let change_data_str = serde_json::to_string(&change_data)?;
        let workspace_id_str = workspace_id.to_string();
        let user_id_str = user_id.to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query!(
            r#"
            INSERT INTO workspace_state_changes (id, workspace_id, change_type, change_data, version, created_at, created_by)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            change_id_str,
            workspace_id_str,
            change_type,
            change_data_str,
            version,
            now,
            user_id_str
        )
        .execute(db)
        .await?;

        Ok(())
    }

    /// Get state changes since a specific version
    pub async fn get_state_changes_since(
        &self,
        workspace_id: Uuid,
        since_version: i64,
    ) -> Result<Vec<serde_json::Value>> {
        let db = self.db.as_ref().ok_or_else(|| {
            CollabError::Internal("Database not available for state changes".to_string())
        })?;

        let workspace_id_str = workspace_id.to_string();
        let changes = sqlx::query!(
            r#"
            SELECT change_data
            FROM workspace_state_changes
            WHERE workspace_id = ? AND version > ?
            ORDER BY version ASC
            "#,
            workspace_id_str,
            since_version
        )
        .fetch_all(db)
        .await?;

        let mut result = Vec::new();
        for change in changes {
            let data: serde_json::Value =
                serde_json::from_str(&change.change_data).map_err(|e| {
                    CollabError::Internal(format!("Failed to parse change data: {}", e))
                })?;
            result.push(data);
        }

        Ok(result)
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
