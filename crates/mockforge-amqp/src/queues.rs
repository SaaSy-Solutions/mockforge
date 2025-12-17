use crate::messages::QueuedMessage;
use std::collections::VecDeque;
use std::time::Duration;

/// Queue properties for TTL, length limits, etc.
#[derive(Debug, Clone)]
pub struct QueueProperties {
    pub max_length: Option<usize>,
    pub max_length_bytes: Option<usize>,
    pub message_ttl: Option<Duration>,
    pub dead_letter_exchange: Option<String>,
    pub dead_letter_routing_key: Option<String>,
}

/// A message queue
#[derive(Debug)]
pub struct Queue {
    pub name: String,
    pub durable: bool,
    pub exclusive: bool,
    pub auto_delete: bool,
    pub messages: VecDeque<QueuedMessage>,
    pub consumers: Vec<String>, // Consumer tags
    pub properties: QueueProperties,
}

impl Queue {
    pub fn new(name: String, durable: bool, exclusive: bool, auto_delete: bool) -> Self {
        Self {
            name,
            durable,
            exclusive,
            auto_delete,
            messages: VecDeque::new(),
            consumers: Vec::new(),
            properties: QueueProperties {
                max_length: None,
                max_length_bytes: None,
                message_ttl: None,
                dead_letter_exchange: None,
                dead_letter_routing_key: None,
            },
        }
    }

    pub fn enqueue(&mut self, message: QueuedMessage) -> Result<(), String> {
        // Check length limits
        if let Some(max_len) = self.properties.max_length {
            if self.messages.len() >= max_len {
                return Err("Queue length limit exceeded".to_string());
            }
        }
        self.messages.push_back(message);
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<QueuedMessage> {
        while let Some(message) = self.messages.front() {
            // Check message expiration
            if message.is_expired() {
                self.messages.pop_front();
                continue;
            }

            // Check queue TTL
            if let Some(ttl) = self.properties.message_ttl {
                if message.enqueued_at.elapsed() >= ttl {
                    self.messages.pop_front();
                    continue;
                }
            }

            // Message is valid, return it
            return self.messages.pop_front();
        }
        None
    }
}

/// Manager for all queues
pub struct QueueManager {
    queues: std::collections::HashMap<String, Queue>,
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            queues: std::collections::HashMap::new(),
        }
    }

    pub fn declare_queue(
        &mut self,
        name: String,
        durable: bool,
        exclusive: bool,
        auto_delete: bool,
    ) {
        let queue = Queue::new(name.clone(), durable, exclusive, auto_delete);
        self.queues.insert(name, queue);
    }

    pub fn get_queue(&self, name: &str) -> Option<&Queue> {
        self.queues.get(name)
    }

    pub fn get_queue_mut(&mut self, name: &str) -> Option<&mut Queue> {
        self.queues.get_mut(name)
    }

    pub fn delete_queue(&mut self, name: &str) -> bool {
        self.queues.remove(name).is_some()
    }

    pub fn list_queues(&self) -> Vec<&Queue> {
        self.queues.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{Message, MessageProperties};
    use std::thread;

    #[test]
    fn test_queue_properties_default() {
        let props = QueueProperties {
            max_length: None,
            max_length_bytes: None,
            message_ttl: None,
            dead_letter_exchange: None,
            dead_letter_routing_key: None,
        };

        assert!(props.max_length.is_none());
        assert!(props.max_length_bytes.is_none());
        assert!(props.message_ttl.is_none());
        assert!(props.dead_letter_exchange.is_none());
        assert!(props.dead_letter_routing_key.is_none());
    }

    #[test]
    fn test_queue_properties_with_values() {
        let props = QueueProperties {
            max_length: Some(1000),
            max_length_bytes: Some(10_000_000),
            message_ttl: Some(Duration::from_secs(60)),
            dead_letter_exchange: Some("dlx".to_string()),
            dead_letter_routing_key: Some("dlx.key".to_string()),
        };

        assert_eq!(props.max_length, Some(1000));
        assert_eq!(props.max_length_bytes, Some(10_000_000));
        assert_eq!(props.message_ttl, Some(Duration::from_secs(60)));
        assert_eq!(props.dead_letter_exchange, Some("dlx".to_string()));
        assert_eq!(props.dead_letter_routing_key, Some("dlx.key".to_string()));
    }

    #[test]
    fn test_queue_new() {
        let queue = Queue::new("test-queue".to_string(), true, false, false);

        assert_eq!(queue.name, "test-queue");
        assert!(queue.durable);
        assert!(!queue.exclusive);
        assert!(!queue.auto_delete);
        assert!(queue.messages.is_empty());
        assert!(queue.consumers.is_empty());
        assert!(queue.properties.max_length.is_none());
    }

    #[test]
    fn test_queue_enqueue() {
        let mut queue = Queue::new("test-queue".to_string(), true, false, false);
        let message = Message {
            properties: MessageProperties::default(),
            body: b"test message".to_vec(),
            routing_key: "test.key".to_string(),
        };
        let queued_message = QueuedMessage::new(message);

        let result = queue.enqueue(queued_message);
        assert!(result.is_ok());
        assert_eq!(queue.messages.len(), 1);
    }

    #[test]
    fn test_queue_enqueue_max_length() {
        let mut queue = Queue::new("test-queue".to_string(), true, false, false);
        queue.properties.max_length = Some(2);

        // Enqueue two messages (should succeed)
        for i in 0..2 {
            let message = Message {
                properties: MessageProperties::default(),
                body: format!("message {}", i).into_bytes(),
                routing_key: "test.key".to_string(),
            };
            let queued_message = QueuedMessage::new(message);
            assert!(queue.enqueue(queued_message).is_ok());
        }

        // Try to enqueue third message (should fail)
        let message = Message {
            properties: MessageProperties::default(),
            body: b"message 3".to_vec(),
            routing_key: "test.key".to_string(),
        };
        let queued_message = QueuedMessage::new(message);
        let result = queue.enqueue(queued_message);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Queue length limit exceeded");
    }

    #[test]
    fn test_queue_dequeue_empty() {
        let mut queue = Queue::new("test-queue".to_string(), true, false, false);
        let message = queue.dequeue();
        assert!(message.is_none());
    }

    #[test]
    fn test_queue_dequeue() {
        let mut queue = Queue::new("test-queue".to_string(), true, false, false);
        let message = Message {
            properties: MessageProperties::default(),
            body: b"test message".to_vec(),
            routing_key: "test.key".to_string(),
        };
        let queued_message = QueuedMessage::new(message);
        queue.enqueue(queued_message).unwrap();

        let dequeued = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().message.body, b"test message".to_vec());
        assert!(queue.messages.is_empty());
    }

    #[test]
    fn test_queue_dequeue_expired_message() {
        let mut queue = Queue::new("test-queue".to_string(), true, false, false);

        // Create a message that expires immediately
        let mut props = MessageProperties::default();
        props.expiration = Some("0".to_string());

        let message = Message {
            properties: props,
            body: b"expired message".to_vec(),
            routing_key: "test.key".to_string(),
        };
        let queued_message = QueuedMessage::new(message);
        queue.enqueue(queued_message).unwrap();

        // Wait a tiny bit to ensure expiration
        thread::sleep(Duration::from_millis(1));

        let dequeued = queue.dequeue();
        // The expired message should be skipped
        assert!(dequeued.is_none());
        assert!(queue.messages.is_empty());
    }

    #[test]
    fn test_queue_dequeue_with_ttl() {
        let mut queue = Queue::new("test-queue".to_string(), true, false, false);
        queue.properties.message_ttl = Some(Duration::from_millis(1));

        let message = Message {
            properties: MessageProperties::default(),
            body: b"ttl message".to_vec(),
            routing_key: "test.key".to_string(),
        };
        let queued_message = QueuedMessage::new(message);
        queue.enqueue(queued_message).unwrap();

        // Wait for TTL to expire
        thread::sleep(Duration::from_millis(5));

        let dequeued = queue.dequeue();
        // Message should be expired due to queue TTL
        assert!(dequeued.is_none());
    }

    #[test]
    fn test_queue_dequeue_multiple_messages() {
        let mut queue = Queue::new("test-queue".to_string(), true, false, false);

        // Enqueue multiple messages
        for i in 0..5 {
            let message = Message {
                properties: MessageProperties::default(),
                body: format!("message {}", i).into_bytes(),
                routing_key: "test.key".to_string(),
            };
            let queued_message = QueuedMessage::new(message);
            queue.enqueue(queued_message).unwrap();
        }

        // Dequeue all messages in FIFO order
        for i in 0..5 {
            let dequeued = queue.dequeue();
            assert!(dequeued.is_some());
            let expected = format!("message {}", i).into_bytes();
            assert_eq!(dequeued.unwrap().message.body, expected);
        }

        assert!(queue.messages.is_empty());
    }

    #[test]
    fn test_queue_manager_new() {
        let manager = QueueManager::new();
        assert!(manager.list_queues().is_empty());
    }

    #[test]
    fn test_queue_manager_default() {
        let manager = QueueManager::default();
        assert!(manager.list_queues().is_empty());
    }

    #[test]
    fn test_queue_manager_declare_queue() {
        let mut manager = QueueManager::new();
        manager.declare_queue("test-queue".to_string(), true, false, false);

        let queue = manager.get_queue("test-queue");
        assert!(queue.is_some());
        assert_eq!(queue.unwrap().name, "test-queue");
        assert!(queue.unwrap().durable);
    }

    #[test]
    fn test_queue_manager_get_queue_nonexistent() {
        let manager = QueueManager::new();
        let queue = manager.get_queue("nonexistent");
        assert!(queue.is_none());
    }

    #[test]
    fn test_queue_manager_get_queue_mut() {
        let mut manager = QueueManager::new();
        manager.declare_queue("test-queue".to_string(), true, false, false);

        let queue = manager.get_queue_mut("test-queue");
        assert!(queue.is_some());

        // Modify the queue
        let queue = queue.unwrap();
        let message = Message {
            properties: MessageProperties::default(),
            body: b"test".to_vec(),
            routing_key: "key".to_string(),
        };
        queue.enqueue(QueuedMessage::new(message)).unwrap();

        // Verify modification
        let queue = manager.get_queue("test-queue").unwrap();
        assert_eq!(queue.messages.len(), 1);
    }

    #[test]
    fn test_queue_manager_delete_queue() {
        let mut manager = QueueManager::new();
        manager.declare_queue("test-queue".to_string(), true, false, false);

        assert!(manager.delete_queue("test-queue"));
        assert!(manager.get_queue("test-queue").is_none());
        assert!(!manager.delete_queue("nonexistent"));
    }

    #[test]
    fn test_queue_manager_list_queues() {
        let mut manager = QueueManager::new();
        manager.declare_queue("queue1".to_string(), true, false, false);
        manager.declare_queue("queue2".to_string(), false, true, false);
        manager.declare_queue("queue3".to_string(), false, false, true);

        let queues = manager.list_queues();
        assert_eq!(queues.len(), 3);
    }

    #[test]
    fn test_queue_properties_clone() {
        let props = QueueProperties {
            max_length: Some(100),
            max_length_bytes: Some(1000),
            message_ttl: Some(Duration::from_secs(30)),
            dead_letter_exchange: Some("dlx".to_string()),
            dead_letter_routing_key: Some("dlx.key".to_string()),
        };

        let cloned = props.clone();
        assert_eq!(props.max_length, cloned.max_length);
        assert_eq!(props.message_ttl, cloned.message_ttl);
        assert_eq!(props.dead_letter_exchange, cloned.dead_letter_exchange);
    }

    #[test]
    fn test_queue_debug() {
        let queue = Queue::new("test-queue".to_string(), true, false, false);
        let debug = format!("{:?}", queue);
        assert!(debug.contains("Queue"));
        assert!(debug.contains("test-queue"));
    }

    #[test]
    fn test_queue_consumers() {
        let mut queue = Queue::new("test-queue".to_string(), true, false, false);
        assert!(queue.consumers.is_empty());

        queue.consumers.push("consumer1".to_string());
        queue.consumers.push("consumer2".to_string());

        assert_eq!(queue.consumers.len(), 2);
        assert!(queue.consumers.contains(&"consumer1".to_string()));
        assert!(queue.consumers.contains(&"consumer2".to_string()));
    }

    #[test]
    fn test_queue_exclusive_flag() {
        let queue = Queue::new("test-queue".to_string(), false, true, false);
        assert!(!queue.durable);
        assert!(queue.exclusive);
        assert!(!queue.auto_delete);
    }

    #[test]
    fn test_queue_auto_delete_flag() {
        let queue = Queue::new("test-queue".to_string(), false, false, true);
        assert!(!queue.durable);
        assert!(!queue.exclusive);
        assert!(queue.auto_delete);
    }

    #[test]
    fn test_queue_dequeue_skips_expired_and_returns_valid() {
        let mut queue = Queue::new("test-queue".to_string(), true, false, false);

        // Enqueue an expired message
        let mut props1 = MessageProperties::default();
        props1.expiration = Some("0".to_string());
        let message1 = Message {
            properties: props1,
            body: b"expired".to_vec(),
            routing_key: "key".to_string(),
        };
        queue.enqueue(QueuedMessage::new(message1)).unwrap();

        // Wait for expiration
        thread::sleep(Duration::from_millis(1));

        // Enqueue a valid message
        let message2 = Message {
            properties: MessageProperties::default(),
            body: b"valid".to_vec(),
            routing_key: "key".to_string(),
        };
        queue.enqueue(QueuedMessage::new(message2)).unwrap();

        // Dequeue should skip expired and return valid
        let dequeued = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().message.body, b"valid".to_vec());
    }

    #[test]
    fn test_queue_properties_debug() {
        let props = QueueProperties {
            max_length: Some(100),
            max_length_bytes: None,
            message_ttl: None,
            dead_letter_exchange: None,
            dead_letter_routing_key: None,
        };

        let debug = format!("{:?}", props);
        assert!(debug.contains("QueueProperties"));
    }
}
