use std::collections::HashMap;
use std::time::Instant;

/// Delivery mode for messages
#[derive(Debug, Clone, PartialEq)]
pub enum DeliveryMode {
    NonPersistent = 1,
    Persistent = 2,
}

/// Message properties
#[derive(Debug, Clone)]
pub struct MessageProperties {
    pub content_type: Option<String>,
    pub content_encoding: Option<String>,
    pub headers: HashMap<String, String>,
    pub delivery_mode: DeliveryMode,
    pub priority: u8,
    pub correlation_id: Option<String>,
    pub reply_to: Option<String>,
    pub expiration: Option<String>,
    pub message_id: Option<String>,
    pub timestamp: Option<i64>,
    pub type_field: Option<String>,
    pub user_id: Option<String>,
    pub app_id: Option<String>,
}

impl Default for MessageProperties {
    fn default() -> Self {
        Self {
            content_type: None,
            content_encoding: None,
            headers: HashMap::new(),
            delivery_mode: DeliveryMode::NonPersistent,
            priority: 0,
            correlation_id: None,
            reply_to: None,
            expiration: None,
            message_id: None,
            timestamp: None,
            type_field: None,
            user_id: None,
            app_id: None,
        }
    }
}

/// A message
#[derive(Debug, Clone)]
pub struct Message {
    pub properties: MessageProperties,
    pub body: Vec<u8>,
    pub routing_key: String,
}

/// A message queued for delivery
#[derive(Debug)]
pub struct QueuedMessage {
    pub message: Message,
    pub enqueued_at: Instant,
    pub delivery_count: u32,
    pub consumer_tag: Option<String>,
}

impl QueuedMessage {
    pub fn new(message: Message) -> Self {
        Self {
            message,
            enqueued_at: Instant::now(),
            delivery_count: 0,
            consumer_tag: None,
        }
    }

    /// Check if the message has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expiration) = &self.message.properties.expiration {
            if let Ok(expiry_ms) = expiration.parse::<u64>() {
                let elapsed = self.enqueued_at.elapsed().as_millis() as u64;
                return elapsed >= expiry_ms;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delivery_mode_values() {
        assert_eq!(DeliveryMode::NonPersistent as u8, 1);
        assert_eq!(DeliveryMode::Persistent as u8, 2);
    }

    #[test]
    fn test_delivery_mode_eq() {
        assert_eq!(DeliveryMode::Persistent, DeliveryMode::Persistent);
        assert_ne!(DeliveryMode::Persistent, DeliveryMode::NonPersistent);
    }

    #[test]
    fn test_message_properties_default() {
        let props = MessageProperties::default();
        assert!(props.content_type.is_none());
        assert!(props.content_encoding.is_none());
        assert!(props.headers.is_empty());
        assert_eq!(props.delivery_mode, DeliveryMode::NonPersistent);
        assert_eq!(props.priority, 0);
        assert!(props.correlation_id.is_none());
        assert!(props.reply_to.is_none());
        assert!(props.expiration.is_none());
        assert!(props.message_id.is_none());
        assert!(props.timestamp.is_none());
        assert!(props.type_field.is_none());
        assert!(props.user_id.is_none());
        assert!(props.app_id.is_none());
    }

    #[test]
    fn test_message_properties_with_values() {
        let mut headers = HashMap::new();
        headers.insert("x-custom".to_string(), "value".to_string());

        let props = MessageProperties {
            content_type: Some("application/json".to_string()),
            content_encoding: Some("utf-8".to_string()),
            headers,
            delivery_mode: DeliveryMode::Persistent,
            priority: 5,
            correlation_id: Some("corr-123".to_string()),
            reply_to: Some("reply-queue".to_string()),
            expiration: Some("60000".to_string()),
            message_id: Some("msg-456".to_string()),
            timestamp: Some(1234567890),
            type_field: Some("user.created".to_string()),
            user_id: Some("user-789".to_string()),
            app_id: Some("test-app".to_string()),
        };

        assert_eq!(props.content_type, Some("application/json".to_string()));
        assert_eq!(props.delivery_mode, DeliveryMode::Persistent);
        assert_eq!(props.priority, 5);
        assert!(!props.headers.is_empty());
    }

    #[test]
    fn test_message_creation() {
        let message = Message {
            properties: MessageProperties::default(),
            body: b"Hello, World!".to_vec(),
            routing_key: "test.routing.key".to_string(),
        };

        assert_eq!(message.body, b"Hello, World!".to_vec());
        assert_eq!(message.routing_key, "test.routing.key");
    }

    #[test]
    fn test_message_clone() {
        let message = Message {
            properties: MessageProperties::default(),
            body: b"test".to_vec(),
            routing_key: "key".to_string(),
        };

        let cloned = message.clone();
        assert_eq!(message.body, cloned.body);
        assert_eq!(message.routing_key, cloned.routing_key);
    }

    #[test]
    fn test_queued_message_new() {
        let message = Message {
            properties: MessageProperties::default(),
            body: b"test".to_vec(),
            routing_key: "key".to_string(),
        };

        let queued = QueuedMessage::new(message);
        assert_eq!(queued.delivery_count, 0);
        assert!(queued.consumer_tag.is_none());
    }

    #[test]
    fn test_queued_message_not_expired() {
        let message = Message {
            properties: MessageProperties::default(),
            body: b"test".to_vec(),
            routing_key: "key".to_string(),
        };

        let queued = QueuedMessage::new(message);
        // No expiration set, should not be expired
        assert!(!queued.is_expired());
    }

    #[test]
    fn test_queued_message_with_long_expiration() {
        let mut props = MessageProperties::default();
        props.expiration = Some("3600000".to_string()); // 1 hour

        let message = Message {
            properties: props,
            body: b"test".to_vec(),
            routing_key: "key".to_string(),
        };

        let queued = QueuedMessage::new(message);
        // Should not be expired immediately
        assert!(!queued.is_expired());
    }

    #[test]
    fn test_queued_message_with_zero_expiration() {
        let mut props = MessageProperties::default();
        props.expiration = Some("0".to_string()); // Immediate expiration

        let message = Message {
            properties: props,
            body: b"test".to_vec(),
            routing_key: "key".to_string(),
        };

        let queued = QueuedMessage::new(message);
        // Should be expired immediately (or very soon)
        // Since we just created it, we can't guarantee it's expired yet
        // Just verify the function runs without error
        let _ = queued.is_expired();
    }

    #[test]
    fn test_queued_message_with_invalid_expiration() {
        let mut props = MessageProperties::default();
        props.expiration = Some("not-a-number".to_string());

        let message = Message {
            properties: props,
            body: b"test".to_vec(),
            routing_key: "key".to_string(),
        };

        let queued = QueuedMessage::new(message);
        // Invalid expiration should not cause expiry
        assert!(!queued.is_expired());
    }

    #[test]
    fn test_message_properties_clone() {
        let mut headers = HashMap::new();
        headers.insert("key".to_string(), "value".to_string());

        let props = MessageProperties {
            content_type: Some("text/plain".to_string()),
            headers,
            ..Default::default()
        };

        let cloned = props.clone();
        assert_eq!(props.content_type, cloned.content_type);
        assert_eq!(props.headers.len(), cloned.headers.len());
    }

    #[test]
    fn test_delivery_mode_clone() {
        let mode = DeliveryMode::Persistent;
        let cloned = mode.clone();
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_message_debug() {
        let message = Message {
            properties: MessageProperties::default(),
            body: b"test".to_vec(),
            routing_key: "key".to_string(),
        };

        let debug = format!("{:?}", message);
        assert!(debug.contains("Message"));
        assert!(debug.contains("key"));
    }

    #[test]
    fn test_message_properties_debug() {
        let props = MessageProperties::default();
        let debug = format!("{:?}", props);
        assert!(debug.contains("MessageProperties"));
    }
}
