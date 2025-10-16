//! Streaming protocol abstractions for pub/sub and messaging patterns
//!
//! This module provides traits and types for protocols that support streaming,
//! pub/sub, and asynchronous messaging patterns like MQTT, Kafka, and RabbitMQ.

use crate::Result;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;

/// A message in a streaming protocol
#[derive(Debug, Clone)]
pub struct ProtocolMessage {
    /// Message ID or sequence number
    pub id: Option<String>,
    /// Topic or channel name
    pub topic: String,
    /// Message payload
    pub payload: Vec<u8>,
    /// Message metadata (headers, properties, etc.)
    pub metadata: std::collections::HashMap<String, String>,
    /// Quality of Service level
    pub qos: Option<u8>,
    /// Timestamp when message was received
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// A stream of protocol messages
pub type MessageStream = Pin<Box<dyn Stream<Item = Result<ProtocolMessage>> + Send>>;

/// Metadata about a streaming connection
#[derive(Debug, Clone)]
pub struct StreamingMetadata {
    /// Protocol being used
    pub protocol: super::Protocol,
    /// Connection identifier
    pub connection_id: String,
    /// Server information
    pub server_info: Option<String>,
    /// Active subscriptions/topics
    pub subscriptions: Vec<String>,
    /// Connection status
    pub connected: bool,
}

/// Trait for protocols that support streaming and pub/sub patterns
#[async_trait]
pub trait StreamingProtocol: Send + Sync {
    /// Subscribe to a topic and return a stream of messages
    async fn subscribe(&self, topic: &str, consumer_id: &str) -> Result<MessageStream>;

    /// Publish a message to a topic
    async fn publish(&self, topic: &str, message: ProtocolMessage) -> Result<()>;

    /// Unsubscribe from a topic
    async fn unsubscribe(&self, _topic: &str, _consumer_id: &str) -> Result<()> {
        // Default implementation does nothing
        Ok(())
    }

    /// Get metadata about the streaming connection
    fn get_metadata(&self) -> StreamingMetadata;

    /// Check if the connection is healthy
    fn is_connected(&self) -> bool {
        self.get_metadata().connected
    }
}

/// A registry for managing multiple streaming protocol handlers
pub struct StreamingProtocolRegistry {
    handlers: std::collections::HashMap<super::Protocol, Arc<dyn StreamingProtocol>>,
}

impl StreamingProtocolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            handlers: std::collections::HashMap::new(),
        }
    }

    /// Register a streaming protocol handler
    pub fn register_handler(
        &mut self,
        protocol: super::Protocol,
        handler: Arc<dyn StreamingProtocol>,
    ) {
        self.handlers.insert(protocol, handler);
    }

    /// Get a handler for a specific protocol
    pub fn get_handler(&self, protocol: &super::Protocol) -> Option<&Arc<dyn StreamingProtocol>> {
        self.handlers.get(protocol)
    }

    /// Get all registered protocols
    pub fn registered_protocols(&self) -> Vec<super::Protocol> {
        self.handlers.keys().cloned().collect()
    }

    /// Check if a protocol is supported
    pub fn supports_protocol(&self, protocol: &super::Protocol) -> bool {
        self.handlers.contains_key(protocol)
    }
}

impl Default for StreamingProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper for creating protocol messages
pub struct MessageBuilder {
    message: ProtocolMessage,
}

impl MessageBuilder {
    /// Create a new message builder
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            message: ProtocolMessage {
                id: None,
                topic: topic.into(),
                payload: Vec::new(),
                metadata: std::collections::HashMap::new(),
                qos: None,
                timestamp: Some(chrono::Utc::now()),
            },
        }
    }

    /// Set the message ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.message.id = Some(id.into());
        self
    }

    /// Set the message payload
    pub fn payload(mut self, payload: impl Into<Vec<u8>>) -> Self {
        self.message.payload = payload.into();
        self
    }

    /// Set the message payload from a string
    pub fn text(mut self, text: impl AsRef<str>) -> Self {
        self.message.payload = text.as_ref().as_bytes().to_vec();
        self
    }

    /// Set the message payload from JSON
    pub fn json<T: serde::Serialize>(mut self, value: &T) -> Result<Self> {
        self.message.payload = serde_json::to_vec(value)?;
        self.message
            .metadata
            .insert("content-type".to_string(), "application/json".to_string());
        Ok(self)
    }

    /// Add metadata
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.message.metadata.insert(key.into(), value.into());
        self
    }

    /// Set QoS level
    pub fn qos(mut self, qos: u8) -> Self {
        self.message.qos = Some(qos);
        self
    }

    /// Build the message
    pub fn build(self) -> ProtocolMessage {
        self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_builder() {
        let message = MessageBuilder::new("test-topic")
            .id("msg-123")
            .text("Hello, World!")
            .metadata("priority", "high")
            .qos(1)
            .build();

        assert_eq!(message.topic, "test-topic");
        assert_eq!(message.id, Some("msg-123".to_string()));
        assert_eq!(message.payload, b"Hello, World!");
        assert_eq!(message.metadata.get("priority"), Some(&"high".to_string()));
        assert_eq!(message.qos, Some(1));
        assert!(message.timestamp.is_some());
    }

    #[test]
    fn test_message_builder_json() {
        #[derive(serde::Serialize)]
        struct TestData {
            name: String,
            value: i32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let message = MessageBuilder::new("json-topic").json(&data).unwrap().build();

        assert_eq!(message.topic, "json-topic");
        assert_eq!(message.metadata.get("content-type"), Some(&"application/json".to_string()));
        assert!(!message.payload.is_empty());
    }

    #[test]
    fn test_streaming_registry() {
        let registry = StreamingProtocolRegistry::new();

        // Registry should start empty
        assert!(!registry.supports_protocol(&crate::protocol_abstraction::Protocol::Mqtt));
        assert_eq!(registry.registered_protocols().len(), 0);

        // Note: We can't easily test with actual handlers without implementing mock streaming protocols
        // This would require creating mock implementations of StreamingProtocol
    }
}
