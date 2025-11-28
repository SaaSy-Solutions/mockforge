//! Pipeline event system
//!
//! Events trigger pipeline execution. Events are emitted by various `MockForge`
//! components and consumed by the pipeline executor.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error};
use uuid::Uuid;

/// Pipeline event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PipelineEventType {
    /// Schema changed (OpenAPI/Protobuf)
    SchemaChanged,
    /// Scenario published
    ScenarioPublished,
    /// Drift threshold exceeded
    DriftThresholdExceeded,
    /// Promotion completed
    PromotionCompleted,
    /// Workspace created
    WorkspaceCreated,
    /// Persona published
    PersonaPublished,
    /// Configuration changed
    ConfigChanged,
}

impl PipelineEventType {
    /// Get event type as string
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SchemaChanged => "schema.changed",
            Self::ScenarioPublished => "scenario.published",
            Self::DriftThresholdExceeded => "drift.threshold_exceeded",
            Self::PromotionCompleted => "promotion.completed",
            Self::WorkspaceCreated => "workspace.created",
            Self::PersonaPublished => "persona.published",
            Self::ConfigChanged => "config.changed",
        }
    }

    /// Parse event type from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "schema.changed" => Some(Self::SchemaChanged),
            "scenario.published" => Some(Self::ScenarioPublished),
            "drift.threshold_exceeded" => Some(Self::DriftThresholdExceeded),
            "promotion.completed" => Some(Self::PromotionCompleted),
            "workspace.created" => Some(Self::WorkspaceCreated),
            "persona.published" => Some(Self::PersonaPublished),
            "config.changed" => Some(Self::ConfigChanged),
            _ => None,
        }
    }
}

/// Pipeline event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEvent {
    /// Event ID
    pub id: Uuid,
    /// Event type
    pub event_type: PipelineEventType,
    /// Workspace ID (if applicable)
    pub workspace_id: Option<Uuid>,
    /// Organization ID (if applicable)
    pub org_id: Option<Uuid>,
    /// Event payload (flexible JSON data)
    pub payload: HashMap<String, serde_json::Value>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Source component that emitted the event
    pub source: String,
}

impl PipelineEvent {
    /// Create a new pipeline event
    #[must_use]
    pub fn new(
        event_type: PipelineEventType,
        workspace_id: Option<Uuid>,
        org_id: Option<Uuid>,
        payload: HashMap<String, serde_json::Value>,
        source: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            workspace_id,
            org_id,
            payload,
            timestamp: Utc::now(),
            source,
        }
    }

    /// Create a schema changed event
    #[must_use]
    pub fn schema_changed(
        workspace_id: Uuid,
        schema_type: String,
        changes: HashMap<String, serde_json::Value>,
    ) -> Self {
        let mut payload = HashMap::new();
        payload.insert("schema_type".to_string(), serde_json::Value::String(schema_type));
        payload.insert("changes".to_string(), serde_json::to_value(changes).unwrap_or_default());

        Self::new(
            PipelineEventType::SchemaChanged,
            Some(workspace_id),
            None,
            payload,
            "mockforge-recorder".to_string(),
        )
    }

    /// Create a scenario published event
    #[must_use]
    pub fn scenario_published(
        workspace_id: Uuid,
        scenario_id: Uuid,
        scenario_name: String,
        version: Option<String>,
    ) -> Self {
        let mut payload = HashMap::new();
        payload.insert(
            "scenario_id".to_string(),
            serde_json::to_value(scenario_id.to_string()).unwrap(),
        );
        payload.insert("scenario_name".to_string(), serde_json::Value::String(scenario_name));
        if let Some(v) = version {
            payload.insert("version".to_string(), serde_json::Value::String(v));
        }

        Self::new(
            PipelineEventType::ScenarioPublished,
            Some(workspace_id),
            None,
            payload,
            "mockforge-registry".to_string(),
        )
    }

    /// Create a drift threshold exceeded event
    #[must_use]
    pub fn drift_threshold_exceeded(
        workspace_id: Uuid,
        endpoint: String,
        drift_count: i32,
        threshold: i32,
    ) -> Self {
        let mut payload = HashMap::new();
        payload.insert("endpoint".to_string(), serde_json::Value::String(endpoint));
        payload.insert("drift_count".to_string(), serde_json::to_value(drift_count).unwrap());
        payload.insert("threshold".to_string(), serde_json::to_value(threshold).unwrap());

        Self::new(
            PipelineEventType::DriftThresholdExceeded,
            Some(workspace_id),
            None,
            payload,
            "mockforge-core".to_string(),
        )
    }

    /// Create a promotion completed event
    #[must_use]
    pub fn promotion_completed(
        workspace_id: Uuid,
        promotion_id: Uuid,
        entity_type: String,
        from_env: String,
        to_env: String,
    ) -> Self {
        let mut payload = HashMap::new();
        payload.insert(
            "promotion_id".to_string(),
            serde_json::to_value(promotion_id.to_string()).unwrap(),
        );
        payload.insert("entity_type".to_string(), serde_json::Value::String(entity_type));
        payload.insert("from_environment".to_string(), serde_json::Value::String(from_env));
        payload.insert("to_environment".to_string(), serde_json::Value::String(to_env));

        Self::new(
            PipelineEventType::PromotionCompleted,
            Some(workspace_id),
            None,
            payload,
            "mockforge-collab".to_string(),
        )
    }
}

/// Pipeline event bus for broadcasting events
pub struct PipelineEventBus {
    /// Broadcast channel for events
    sender: broadcast::Sender<PipelineEvent>,
}

impl PipelineEventBus {
    /// Create a new pipeline event bus
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish an event
    pub fn publish(&self, event: PipelineEvent) -> Result<(), String> {
        let event_type = event.event_type.clone();
        match self.sender.send(event) {
            Ok(_) => {
                debug!("Published pipeline event: {:?}", event_type);
                Ok(())
            }
            Err(e) => {
                error!("Failed to publish pipeline event: {}", e);
                Err(format!("Failed to publish event: {e}"))
            }
        }
    }

    /// Subscribe to events
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<PipelineEvent> {
        self.sender.subscribe()
    }

    /// Get number of active subscribers
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// Global pipeline event bus (singleton pattern)
static GLOBAL_PIPELINE_EVENT_BUS: std::sync::LazyLock<Arc<PipelineEventBus>> =
    std::sync::LazyLock::new(|| Arc::new(PipelineEventBus::new(1000)));

/// Get the global pipeline event bus
pub fn get_global_event_bus() -> Arc<PipelineEventBus> {
    GLOBAL_PIPELINE_EVENT_BUS.clone()
}

/// Publish an event to the global event bus
pub fn publish_event(event: PipelineEvent) -> Result<(), String> {
    get_global_event_bus().publish(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_serialization() {
        let event_type = PipelineEventType::SchemaChanged;
        assert_eq!(event_type.as_str(), "schema.changed");
        assert_eq!(
            PipelineEventType::from_str("schema.changed"),
            Some(PipelineEventType::SchemaChanged)
        );
    }

    #[test]
    fn test_event_creation() {
        let event =
            PipelineEvent::schema_changed(Uuid::new_v4(), "openapi".to_string(), HashMap::new());
        assert_eq!(event.event_type, PipelineEventType::SchemaChanged);
        assert!(event.workspace_id.is_some());
    }

    #[tokio::test]
    async fn test_event_bus() {
        let bus = PipelineEventBus::new(100);
        let mut receiver = bus.subscribe();

        let event =
            PipelineEvent::schema_changed(Uuid::new_v4(), "openapi".to_string(), HashMap::new());

        bus.publish(event.clone()).unwrap();

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.event_type, event.event_type);
        assert_eq!(received.workspace_id, event.workspace_id);
    }
}
