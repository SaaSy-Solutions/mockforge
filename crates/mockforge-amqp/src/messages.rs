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
}