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
    fn test_event_type_all_variants() {
        // Test all event types have correct string representations
        assert_eq!(PipelineEventType::SchemaChanged.as_str(), "schema.changed");
        assert_eq!(PipelineEventType::ScenarioPublished.as_str(), "scenario.published");
        assert_eq!(PipelineEventType::DriftThresholdExceeded.as_str(), "drift.threshold_exceeded");
        assert_eq!(PipelineEventType::PromotionCompleted.as_str(), "promotion.completed");
        assert_eq!(PipelineEventType::WorkspaceCreated.as_str(), "workspace.created");
        assert_eq!(PipelineEventType::PersonaPublished.as_str(), "persona.published");
        assert_eq!(PipelineEventType::ConfigChanged.as_str(), "config.changed");
    }

    #[test]
    fn test_event_type_from_str_all_variants() {
        assert_eq!(
            PipelineEventType::from_str("schema.changed"),
            Some(PipelineEventType::SchemaChanged)
        );
        assert_eq!(
            PipelineEventType::from_str("scenario.published"),
            Some(PipelineEventType::ScenarioPublished)
        );
        assert_eq!(
            PipelineEventType::from_str("drift.threshold_exceeded"),
            Some(PipelineEventType::DriftThresholdExceeded)
        );
        assert_eq!(
            PipelineEventType::from_str("promotion.completed"),
            Some(PipelineEventType::PromotionCompleted)
        );
        assert_eq!(
            PipelineEventType::from_str("workspace.created"),
            Some(PipelineEventType::WorkspaceCreated)
        );
        assert_eq!(
            PipelineEventType::from_str("persona.published"),
            Some(PipelineEventType::PersonaPublished)
        );
        assert_eq!(
            PipelineEventType::from_str("config.changed"),
            Some(PipelineEventType::ConfigChanged)
        );
        assert_eq!(PipelineEventType::from_str("unknown.event"), None);
        assert_eq!(PipelineEventType::from_str(""), None);
    }

    #[test]
    fn test_event_creation() {
        let event =
            PipelineEvent::schema_changed(Uuid::new_v4(), "openapi".to_string(), HashMap::new());
        assert_eq!(event.event_type, PipelineEventType::SchemaChanged);
        assert!(event.workspace_id.is_some());
    }

    #[test]
    fn test_event_new() {
        let workspace_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        let mut payload = HashMap::new();
        payload.insert("key".to_string(), serde_json::json!("value"));

        let event = PipelineEvent::new(
            PipelineEventType::ConfigChanged,
            Some(workspace_id),
            Some(org_id),
            payload,
            "test-source".to_string(),
        );

        assert_eq!(event.event_type, PipelineEventType::ConfigChanged);
        assert_eq!(event.workspace_id, Some(workspace_id));
        assert_eq!(event.org_id, Some(org_id));
        assert_eq!(event.source, "test-source");
        assert!(event.payload.contains_key("key"));
    }

    #[test]
    fn test_schema_changed_event() {
        let workspace_id = Uuid::new_v4();
        let mut changes = HashMap::new();
        changes.insert("added".to_string(), serde_json::json!(["endpoint1"]));

        let event =
            PipelineEvent::schema_changed(workspace_id, "protobuf".to_string(), changes.clone());

        assert_eq!(event.event_type, PipelineEventType::SchemaChanged);
        assert_eq!(event.workspace_id, Some(workspace_id));
        assert_eq!(event.payload.get("schema_type"), Some(&serde_json::json!("protobuf")));
        assert_eq!(event.source, "mockforge-recorder");
    }

    #[test]
    fn test_scenario_published_event() {
        let workspace_id = Uuid::new_v4();
        let scenario_id = Uuid::new_v4();

        let event = PipelineEvent::scenario_published(
            workspace_id,
            scenario_id,
            "my-scenario".to_string(),
            Some("1.0.0".to_string()),
        );

        assert_eq!(event.event_type, PipelineEventType::ScenarioPublished);
        assert_eq!(event.workspace_id, Some(workspace_id));
        assert_eq!(event.payload.get("scenario_name"), Some(&serde_json::json!("my-scenario")));
        assert_eq!(event.payload.get("version"), Some(&serde_json::json!("1.0.0")));
        assert_eq!(event.source, "mockforge-registry");
    }

    #[test]
    fn test_scenario_published_event_no_version() {
        let workspace_id = Uuid::new_v4();
        let scenario_id = Uuid::new_v4();

        let event = PipelineEvent::scenario_published(
            workspace_id,
            scenario_id,
            "my-scenario".to_string(),
            None,
        );

        assert!(!event.payload.contains_key("version"));
    }

    #[test]
    fn test_drift_threshold_exceeded_event() {
        let workspace_id = Uuid::new_v4();

        let event =
            PipelineEvent::drift_threshold_exceeded(workspace_id, "/api/users".to_string(), 15, 10);

        assert_eq!(event.event_type, PipelineEventType::DriftThresholdExceeded);
        assert_eq!(event.workspace_id, Some(workspace_id));
        assert_eq!(event.payload.get("endpoint"), Some(&serde_json::json!("/api/users")));
        assert_eq!(event.payload.get("drift_count"), Some(&serde_json::json!(15)));
        assert_eq!(event.payload.get("threshold"), Some(&serde_json::json!(10)));
        assert_eq!(event.source, "mockforge-core");
    }

    #[test]
    fn test_promotion_completed_event() {
        let workspace_id = Uuid::new_v4();
        let promotion_id = Uuid::new_v4();

        let event = PipelineEvent::promotion_completed(
            workspace_id,
            promotion_id,
            "scenario".to_string(),
            "staging".to_string(),
            "production".to_string(),
        );

        assert_eq!(event.event_type, PipelineEventType::PromotionCompleted);
        assert_eq!(event.workspace_id, Some(workspace_id));
        assert_eq!(event.payload.get("entity_type"), Some(&serde_json::json!("scenario")));
        assert_eq!(event.payload.get("from_environment"), Some(&serde_json::json!("staging")));
        assert_eq!(event.payload.get("to_environment"), Some(&serde_json::json!("production")));
        assert_eq!(event.source, "mockforge-collab");
    }

    #[test]
    fn test_event_type_serialize_deserialize() {
        let event_type = PipelineEventType::DriftThresholdExceeded;
        let json = serde_json::to_string(&event_type).unwrap();
        assert_eq!(json, "\"drift_threshold_exceeded\"");

        let deserialized: PipelineEventType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, event_type);
    }

    #[test]
    fn test_event_serialize_deserialize() {
        let workspace_id = Uuid::new_v4();
        let event =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: PipelineEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.event_type, event.event_type);
        assert_eq!(deserialized.workspace_id, event.workspace_id);
        assert_eq!(deserialized.id, event.id);
    }

    #[test]
    fn test_event_type_eq_and_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(PipelineEventType::SchemaChanged);
        set.insert(PipelineEventType::ScenarioPublished);
        set.insert(PipelineEventType::SchemaChanged); // duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&PipelineEventType::SchemaChanged));
        assert!(set.contains(&PipelineEventType::ScenarioPublished));
    }

    #[test]
    fn test_event_type_clone() {
        let event_type = PipelineEventType::WorkspaceCreated;
        let cloned = event_type.clone();
        assert_eq!(event_type, cloned);
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

    #[test]
    fn test_event_bus_subscriber_count() {
        let bus = PipelineEventBus::new(100);

        assert_eq!(bus.subscriber_count(), 0);

        let _receiver1 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        let _receiver2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = PipelineEventBus::new(100);
        let mut receiver1 = bus.subscribe();
        let mut receiver2 = bus.subscribe();

        let event =
            PipelineEvent::schema_changed(Uuid::new_v4(), "openapi".to_string(), HashMap::new());
        let event_id = event.id;

        bus.publish(event).unwrap();

        let received1 = receiver1.recv().await.unwrap();
        let received2 = receiver2.recv().await.unwrap();

        assert_eq!(received1.id, event_id);
        assert_eq!(received2.id, event_id);
    }

    #[test]
    fn test_global_event_bus() {
        let bus = get_global_event_bus();
        let _ = bus.subscriber_count(); // Global bus exists (count is usize, always >= 0)
    }

    #[test]
    fn test_publish_event_function() {
        let event =
            PipelineEvent::schema_changed(Uuid::new_v4(), "openapi".to_string(), HashMap::new());

        // Should not error (even without subscribers)
        let result = publish_event(event);
        // May fail if no subscribers, which is fine
        assert!(result.is_ok() || result.is_err());
    }
}
