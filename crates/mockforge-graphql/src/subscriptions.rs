//! GraphQL subscription support with WebSocket
//!
//! Provides real-time GraphQL subscriptions over WebSocket connections.

use async_graphql::Value;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info};

/// Subscription ID
pub type SubscriptionId = String;

/// Topic for subscription routing
pub type Topic = String;

/// Subscription event
#[derive(Clone, Debug)]
pub struct SubscriptionEvent {
    /// Topic this event belongs to
    pub topic: Topic,
    /// Event data
    pub data: Value,
    /// Optional operation name
    pub operation_name: Option<String>,
}

impl SubscriptionEvent {
    /// Create a new subscription event
    pub fn new(topic: Topic, data: Value) -> Self {
        Self {
            topic,
            data,
            operation_name: None,
        }
    }

    /// Set the operation name
    pub fn with_operation(mut self, operation_name: String) -> Self {
        self.operation_name = Some(operation_name);
        self
    }
}

/// Subscription manager for GraphQL subscriptions
pub struct SubscriptionManager {
    /// Active subscriptions by topic
    subscriptions: Arc<RwLock<HashMap<Topic, broadcast::Sender<SubscriptionEvent>>>>,
    /// Subscription metadata
    metadata: Arc<RwLock<HashMap<SubscriptionId, SubscriptionMetadata>>>,
}

/// Metadata for a subscription
#[derive(Clone, Debug)]
pub struct SubscriptionMetadata {
    /// Subscription ID
    pub id: SubscriptionId,
    /// Topic being subscribed to
    pub topic: Topic,
    /// Operation name
    pub operation_name: Option<String>,
    /// When this subscription was created
    pub created_at: std::time::Instant,
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe to a topic
    pub fn subscribe(
        &self,
        id: SubscriptionId,
        topic: Topic,
        operation_name: Option<String>,
    ) -> broadcast::Receiver<SubscriptionEvent> {
        let mut subs = self.subscriptions.write();

        // Get or create the sender for this topic
        let sender = subs.entry(topic.clone()).or_insert_with(|| broadcast::channel(100).0);

        let receiver = sender.subscribe();

        // Store metadata
        let mut metadata = self.metadata.write();
        let topic_clone = topic.clone();
        metadata.insert(
            id.clone(),
            SubscriptionMetadata {
                id,
                topic,
                operation_name,
                created_at: std::time::Instant::now(),
            },
        );

        info!("New subscription to topic: {}", topic_clone);
        receiver
    }

    /// Unsubscribe from a topic
    pub fn unsubscribe(&self, id: &SubscriptionId) {
        let mut metadata = self.metadata.write();
        if let Some(meta) = metadata.remove(id) {
            debug!("Unsubscribed from topic: {}", meta.topic);
        }
    }

    /// Publish an event to a topic
    pub fn publish(&self, event: SubscriptionEvent) -> usize {
        let subs = self.subscriptions.read();

        if let Some(sender) = subs.get(&event.topic) {
            match sender.send(event.clone()) {
                Ok(count) => {
                    debug!("Published to {} subscribers on topic: {}", count, event.topic);
                    count
                }
                Err(_) => {
                    debug!("No active subscribers for topic: {}", event.topic);
                    0
                }
            }
        } else {
            debug!("Topic not found: {}", event.topic);
            0
        }
    }

    /// Get all active topics
    pub fn topics(&self) -> Vec<Topic> {
        self.subscriptions.read().keys().cloned().collect()
    }

    /// Get number of subscribers for a topic
    pub fn subscriber_count(&self, topic: &Topic) -> usize {
        self.subscriptions
            .read()
            .get(topic)
            .map(|sender| sender.receiver_count())
            .unwrap_or(0)
    }

    /// Get all active subscriptions
    pub fn active_subscriptions(&self) -> Vec<SubscriptionMetadata> {
        self.metadata.read().values().cloned().collect()
    }

    /// Clear all subscriptions
    pub fn clear(&self) {
        self.subscriptions.write().clear();
        self.metadata.write().clear();
        info!("All subscriptions cleared");
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Subscription handler trait
#[async_trait::async_trait]
pub trait SubscriptionHandler: Send + Sync {
    /// Handle a new subscription
    async fn on_subscribe(
        &self,
        topic: &str,
        variables: &HashMap<String, Value>,
    ) -> Result<(), String>;

    /// Generate initial data for a subscription
    async fn initial_data(&self, topic: &str, variables: &HashMap<String, Value>) -> Option<Value>;

    /// Check if this handler handles the given subscription
    fn handles_subscription(&self, operation_name: &str) -> bool;
}

/// Mock subscription handler for testing
pub struct MockSubscriptionHandler {
    operation_name: String,
}

impl MockSubscriptionHandler {
    /// Create a new mock subscription handler
    pub fn new(operation_name: String) -> Self {
        Self { operation_name }
    }
}

#[async_trait::async_trait]
impl SubscriptionHandler for MockSubscriptionHandler {
    async fn on_subscribe(
        &self,
        _topic: &str,
        _variables: &HashMap<String, Value>,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn initial_data(
        &self,
        _topic: &str,
        _variables: &HashMap<String, Value>,
    ) -> Option<Value> {
        Some(Value::Null)
    }

    fn handles_subscription(&self, operation_name: &str) -> bool {
        operation_name == self.operation_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_event_creation() {
        let event = SubscriptionEvent::new("orderStatusChanged".to_string(), Value::Null);

        assert_eq!(event.topic, "orderStatusChanged");
        assert!(event.operation_name.is_none());
    }

    #[test]
    fn test_subscription_event_with_operation() {
        let event = SubscriptionEvent::new("orderStatusChanged".to_string(), Value::Null)
            .with_operation("OrderStatusSubscription".to_string());

        assert_eq!(event.operation_name, Some("OrderStatusSubscription".to_string()));
    }

    #[test]
    fn test_subscription_manager_creation() {
        let manager = SubscriptionManager::new();
        assert_eq!(manager.topics().len(), 0);
    }

    #[test]
    fn test_subscribe() {
        let manager = SubscriptionManager::new();
        let _receiver = manager.subscribe(
            "sub-1".to_string(),
            "orderStatusChanged".to_string(),
            Some("OrderStatusSubscription".to_string()),
        );

        assert_eq!(manager.topics().len(), 1);
        assert_eq!(manager.subscriber_count(&"orderStatusChanged".to_string()), 1);
    }

    #[test]
    fn test_publish() {
        let manager = SubscriptionManager::new();
        let mut _receiver =
            manager.subscribe("sub-1".to_string(), "orderStatusChanged".to_string(), None);

        let event = SubscriptionEvent::new(
            "orderStatusChanged".to_string(),
            Value::String("SHIPPED".to_string()),
        );

        let count = manager.publish(event);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_unsubscribe() {
        let manager = SubscriptionManager::new();
        let _receiver =
            manager.subscribe("sub-1".to_string(), "orderStatusChanged".to_string(), None);

        assert_eq!(manager.active_subscriptions().len(), 1);

        manager.unsubscribe(&"sub-1".to_string());
        assert_eq!(manager.active_subscriptions().len(), 0);
    }

    #[test]
    fn test_multiple_subscribers() {
        let manager = SubscriptionManager::new();

        let _recv1 = manager.subscribe("sub-1".to_string(), "topic".to_string(), None);
        let _recv2 = manager.subscribe("sub-2".to_string(), "topic".to_string(), None);

        assert_eq!(manager.subscriber_count(&"topic".to_string()), 2);
    }

    #[test]
    fn test_clear() {
        let manager = SubscriptionManager::new();
        manager.subscribe("sub-1".to_string(), "topic1".to_string(), None);
        manager.subscribe("sub-2".to_string(), "topic2".to_string(), None);

        assert_eq!(manager.topics().len(), 2);

        manager.clear();
        assert_eq!(manager.topics().len(), 0);
        assert_eq!(manager.active_subscriptions().len(), 0);
    }

    #[tokio::test]
    async fn test_mock_subscription_handler() {
        let handler = MockSubscriptionHandler::new("OrderStatusSubscription".to_string());

        assert!(handler.handles_subscription("OrderStatusSubscription"));
        assert!(!handler.handles_subscription("ProductSubscription"));

        let result = handler.on_subscribe("topic", &HashMap::new()).await;
        assert!(result.is_ok());

        let data = handler.initial_data("topic", &HashMap::new()).await;
        assert!(data.is_some());
    }
}
