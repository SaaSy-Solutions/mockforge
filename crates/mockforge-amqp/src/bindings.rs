use std::collections::HashMap;

/// A binding between an exchange and a queue
#[derive(Debug, Clone)]
pub struct Binding {
    pub exchange: String,
    pub queue: String,
    pub routing_key: String,
    pub arguments: HashMap<String, String>,
}

impl Binding {
    pub fn new(exchange: String, queue: String, routing_key: String) -> Self {
        Self {
            exchange,
            queue,
            routing_key,
            arguments: HashMap::new(),
        }
    }

    /// Check if this binding matches the given routing key and headers
    pub fn matches(&self, routing_key: &str, _headers: &HashMap<String, String>) -> bool {
        // For now, simple routing key match
        self.routing_key == routing_key
    }
}
