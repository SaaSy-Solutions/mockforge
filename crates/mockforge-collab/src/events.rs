//! Event system for real-time updates

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Type of change event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// Mock/route created
    MockCreated,
    /// Mock/route updated
    MockUpdated,
    /// Mock/route deleted
    MockDeleted,
    /// Workspace settings updated
    WorkspaceUpdated,
    /// Member added
    MemberAdded,
    /// Member removed
    MemberRemoved,
    /// Member role changed
    RoleChanged,
    /// Snapshot created
    SnapshotCreated,
    /// User cursor moved (presence)
    CursorMoved,
    /// User joined workspace
    UserJoined,
    /// User left workspace
    UserLeft,
}

/// A change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEvent {
    /// Event ID
    pub id: Uuid,
    /// Workspace ID
    pub workspace_id: Uuid,
    /// Type of change
    pub change_type: ChangeType,
    /// User who triggered the change
    pub user_id: Uuid,
    /// Resource ID (mock ID, member ID, etc.)
    pub resource_id: Option<Uuid>,
    /// Event payload
    pub payload: serde_json::Value,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl ChangeEvent {
    /// Create a new change event
    #[must_use]
    pub fn new(
        workspace_id: Uuid,
        change_type: ChangeType,
        user_id: Uuid,
        resource_id: Option<Uuid>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            change_type,
            user_id,
            resource_id,
            payload,
            timestamp: Utc::now(),
        }
    }
}

/// Event listener trait
#[async_trait::async_trait]
pub trait EventListener: Send + Sync {
    /// Handle an event
    async fn on_event(&self, event: ChangeEvent) -> Result<()>;
}

/// Event bus for broadcasting changes
pub struct EventBus {
    /// Broadcast channel for events
    sender: broadcast::Sender<ChangeEvent>,
}

impl EventBus {
    /// Create a new event bus
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish an event
    pub fn publish(&self, event: ChangeEvent) -> Result<()> {
        // Ignore error if no receivers (it's ok)
        let _ = self.sender.send(event);
        Ok(())
    }

    /// Subscribe to events
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<ChangeEvent> {
        self.sender.subscribe()
    }

    /// Get number of active subscribers
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// Workspace-specific event bus
pub struct WorkspaceEventBus {
    /// Main event bus
    event_bus: Arc<EventBus>,
    /// Workspace ID
    workspace_id: Uuid,
}

impl WorkspaceEventBus {
    /// Create a new workspace event bus
    #[must_use]
    pub const fn new(event_bus: Arc<EventBus>, workspace_id: Uuid) -> Self {
        Self {
            event_bus,
            workspace_id,
        }
    }

    /// Publish an event for this workspace
    pub fn publish(
        &self,
        change_type: ChangeType,
        user_id: Uuid,
        resource_id: Option<Uuid>,
        payload: serde_json::Value,
    ) -> Result<()> {
        let event = ChangeEvent::new(self.workspace_id, change_type, user_id, resource_id, payload);
        self.event_bus.publish(event)
    }

    /// Subscribe to events (need to filter by `workspace_id`)
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<ChangeEvent> {
        self.event_bus.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_event_creation() {
        let workspace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let event = ChangeEvent::new(
            workspace_id,
            ChangeType::MockCreated,
            user_id,
            None,
            serde_json::json!({"mock_id": "123"}),
        );

        assert_eq!(event.workspace_id, workspace_id);
        assert_eq!(event.change_type, ChangeType::MockCreated);
        assert_eq!(event.user_id, user_id);
    }

    #[test]
    fn test_event_bus() {
        let bus = EventBus::new(100);
        assert_eq!(bus.subscriber_count(), 0);

        let _rx1 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        let _rx2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);
    }

    #[tokio::test]
    async fn test_event_publishing() {
        let bus = EventBus::new(100);
        let mut rx = bus.subscribe();

        let workspace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let event = ChangeEvent::new(
            workspace_id,
            ChangeType::MockCreated,
            user_id,
            None,
            serde_json::json!({}),
        );

        bus.publish(event.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.workspace_id, workspace_id);
        assert_eq!(received.change_type, ChangeType::MockCreated);
    }
}
