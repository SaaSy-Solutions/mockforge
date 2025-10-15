use std::collections::VecDeque;
use std::time::Duration;
use crate::messages::QueuedMessage;

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
        self.messages.pop_front()
    }
}

/// Manager for all queues
pub struct QueueManager {
    queues: std::collections::HashMap<String, Queue>,
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            queues: std::collections::HashMap::new(),
        }
    }

    pub fn declare_queue(&mut self, name: String, durable: bool, exclusive: bool, auto_delete: bool) {
        let queue = Queue::new(name.clone(), durable, exclusive, auto_delete);
        self.queues.insert(name, queue);
    }

    pub fn get_queue(&self, name: &str) -> Option<&Queue> {
        self.queues.get(name)
    }

    pub fn get_queue_mut(&mut self, name: &str) -> Option<&mut Queue> {
        self.queues.get_mut(name)
    }
}
