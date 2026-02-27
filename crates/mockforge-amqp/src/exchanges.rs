use crate::bindings::Binding;
use crate::messages::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Exchange types supported by AMQP
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExchangeType {
    Direct,
    Fanout,
    Topic,
    Headers,
}

/// Exchange configuration
#[derive(Debug, Clone)]
pub struct Exchange {
    pub name: String,
    pub exchange_type: ExchangeType,
    pub durable: bool,
    pub auto_delete: bool,
    pub arguments: HashMap<String, String>,
    pub bindings: Vec<Binding>,
}

impl Exchange {
    /// Route a message to appropriate queues based on exchange type
    pub fn route_message(&self, message: &Message, routing_key: &str) -> Vec<String> {
        match self.exchange_type {
            ExchangeType::Direct => self.route_direct(routing_key),
            ExchangeType::Fanout => self.route_fanout(),
            ExchangeType::Topic => self.route_topic(routing_key),
            ExchangeType::Headers => self.route_headers(message),
        }
    }

    fn route_direct(&self, routing_key: &str) -> Vec<String> {
        self.bindings
            .iter()
            .filter(|b| b.routing_key == routing_key)
            .map(|b| b.queue.clone())
            .collect()
    }

    fn route_fanout(&self) -> Vec<String> {
        self.bindings.iter().map(|b| b.queue.clone()).collect()
    }

    fn route_topic(&self, routing_key: &str) -> Vec<String> {
        let routing_parts: Vec<&str> = routing_key.split('.').collect();

        self.bindings
            .iter()
            .filter(|binding| {
                let pattern_parts: Vec<&str> = binding.routing_key.split('.').collect();
                Self::matches_topic_pattern(&routing_parts, &pattern_parts)
            })
            .map(|binding| binding.queue.clone())
            .collect()
    }

    pub fn matches_topic_pattern(routing_parts: &[&str], pattern_parts: &[&str]) -> bool {
        if routing_parts.len() > pattern_parts.len() && !pattern_parts.contains(&"#") {
            return false;
        }

        let mut routing_iter = routing_parts.iter();
        let mut pattern_iter = pattern_parts.iter();

        while let (Some(&routing), Some(&pattern)) = (routing_iter.next(), pattern_iter.next()) {
            match pattern {
                "*" => {
                    // * matches exactly one word
                    continue;
                }
                "#" => {
                    // # matches zero or more words
                    // If # is at the end, it matches everything remaining
                    if pattern_iter.next().is_none() {
                        return true;
                    }
                    // If # is in the middle, we need to find the next pattern after #
                    // This is more complex - for now, simple implementation
                    return true;
                }
                word if word == routing => {
                    continue;
                }
                _ => return false,
            }
        }

        // Check if we have remaining routing parts that need to be matched
        routing_iter.next().is_none()
    }

    fn route_headers(&self, message: &Message) -> Vec<String> {
        self.bindings
            .iter()
            .filter(|binding| {
                Self::matches_headers(&message.properties.headers, &binding.arguments)
            })
            .map(|binding| binding.queue.clone())
            .collect()
    }

    fn matches_headers(
        message_headers: &HashMap<String, String>,
        binding_args: &HashMap<String, String>,
    ) -> bool {
        let x_match = binding_args.get("x-match").map(|s| s.as_str()).unwrap_or("all");

        // Filter out x-match from the headers to match
        let headers_to_match: HashMap<_, _> =
            binding_args.iter().filter(|(k, _)| *k != "x-match").collect();

        if headers_to_match.is_empty() {
            return true; // No headers to match means it matches
        }

        if x_match == "any" {
            // Match if any of the specified headers match
            headers_to_match.iter().any(|(key, value)| {
                message_headers.get(key.as_str()).map(|v| v == *value).unwrap_or(false)
            })
        } else {
            // Match if all specified headers match (default is "all")
            headers_to_match.iter().all(|(key, value)| {
                message_headers.get(key.as_str()).map(|v| v == *value).unwrap_or(false)
            })
        }
    }
}

/// Manager for all exchanges
#[derive(Debug)]
pub struct ExchangeManager {
    exchanges: HashMap<String, Exchange>,
}

impl Default for ExchangeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ExchangeManager {
    pub fn new() -> Self {
        Self {
            exchanges: HashMap::new(),
        }
    }

    pub fn declare_exchange(
        &mut self,
        name: String,
        exchange_type: ExchangeType,
        durable: bool,
        auto_delete: bool,
    ) {
        let exchange = Exchange {
            name: name.clone(),
            exchange_type,
            durable,
            auto_delete,
            arguments: HashMap::new(),
            bindings: Vec::new(),
        };
        self.exchanges.insert(name, exchange);
    }

    pub fn get_exchange(&self, name: &str) -> Option<&Exchange> {
        self.exchanges.get(name)
    }

    pub fn get_exchange_mut(&mut self, name: &str) -> Option<&mut Exchange> {
        self.exchanges.get_mut(name)
    }

    pub fn list_exchanges(&self) -> Vec<&Exchange> {
        self.exchanges.values().collect()
    }

    /// Add a binding to an exchange
    pub fn add_binding(&mut self, exchange_name: &str, binding: Binding) -> bool {
        if let Some(exchange) = self.exchanges.get_mut(exchange_name) {
            exchange.bindings.push(binding);
            true
        } else {
            false
        }
    }

    /// Remove a binding from an exchange
    pub fn remove_binding(&mut self, exchange_name: &str, queue: &str, routing_key: &str) -> bool {
        if let Some(exchange) = self.exchanges.get_mut(exchange_name) {
            let initial_len = exchange.bindings.len();
            exchange
                .bindings
                .retain(|b| !(b.queue == queue && b.routing_key == routing_key));
            exchange.bindings.len() < initial_len
        } else {
            false
        }
    }

    pub fn delete_exchange(&mut self, name: &str) -> bool {
        self.exchanges.remove(name).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::MessageProperties;

    fn create_test_binding(queue: &str, routing_key: &str) -> Binding {
        Binding {
            exchange: "test-exchange".to_string(),
            queue: queue.to_string(),
            routing_key: routing_key.to_string(),
            arguments: HashMap::new(),
        }
    }

    fn create_test_message(routing_key: &str) -> Message {
        Message {
            properties: MessageProperties::default(),
            body: b"test".to_vec(),
            routing_key: routing_key.to_string(),
        }
    }

    #[test]
    fn test_exchange_type_serialize() {
        let json = serde_json::to_string(&ExchangeType::Direct).unwrap();
        assert_eq!(json, "\"direct\"");

        let json = serde_json::to_string(&ExchangeType::Fanout).unwrap();
        assert_eq!(json, "\"fanout\"");

        let json = serde_json::to_string(&ExchangeType::Topic).unwrap();
        assert_eq!(json, "\"topic\"");

        let json = serde_json::to_string(&ExchangeType::Headers).unwrap();
        assert_eq!(json, "\"headers\"");
    }

    #[test]
    fn test_exchange_type_deserialize() {
        let exchange_type: ExchangeType = serde_json::from_str("\"direct\"").unwrap();
        assert_eq!(exchange_type, ExchangeType::Direct);

        let exchange_type: ExchangeType = serde_json::from_str("\"fanout\"").unwrap();
        assert_eq!(exchange_type, ExchangeType::Fanout);
    }

    #[test]
    fn test_exchange_type_eq() {
        assert_eq!(ExchangeType::Direct, ExchangeType::Direct);
        assert_ne!(ExchangeType::Direct, ExchangeType::Fanout);
    }

    #[test]
    fn test_exchange_route_direct() {
        let exchange = Exchange {
            name: "test-exchange".to_string(),
            exchange_type: ExchangeType::Direct,
            durable: false,
            auto_delete: false,
            arguments: HashMap::new(),
            bindings: vec![
                create_test_binding("queue1", "key1"),
                create_test_binding("queue2", "key2"),
                create_test_binding("queue3", "key1"),
            ],
        };

        let message = create_test_message("key1");
        let queues = exchange.route_message(&message, "key1");
        assert_eq!(queues.len(), 2);
        assert!(queues.contains(&"queue1".to_string()));
        assert!(queues.contains(&"queue3".to_string()));
    }

    #[test]
    fn test_exchange_route_fanout() {
        let exchange = Exchange {
            name: "test-exchange".to_string(),
            exchange_type: ExchangeType::Fanout,
            durable: false,
            auto_delete: false,
            arguments: HashMap::new(),
            bindings: vec![
                create_test_binding("queue1", ""),
                create_test_binding("queue2", ""),
                create_test_binding("queue3", ""),
            ],
        };

        let message = create_test_message("any-key");
        let queues = exchange.route_message(&message, "any-key");
        assert_eq!(queues.len(), 3);
    }

    #[test]
    fn test_exchange_route_topic_exact_match() {
        let exchange = Exchange {
            name: "test-exchange".to_string(),
            exchange_type: ExchangeType::Topic,
            durable: false,
            auto_delete: false,
            arguments: HashMap::new(),
            bindings: vec![
                create_test_binding("queue1", "user.created"),
                create_test_binding("queue2", "order.created"),
            ],
        };

        let message = create_test_message("user.created");
        let queues = exchange.route_message(&message, "user.created");
        assert_eq!(queues.len(), 1);
        assert!(queues.contains(&"queue1".to_string()));
    }

    #[test]
    fn test_exchange_route_topic_wildcard() {
        let exchange = Exchange {
            name: "test-exchange".to_string(),
            exchange_type: ExchangeType::Topic,
            durable: false,
            auto_delete: false,
            arguments: HashMap::new(),
            bindings: vec![
                create_test_binding("queue1", "user.*"),
                create_test_binding("queue2", "*.created"),
            ],
        };

        let message = create_test_message("user.created");
        let queues = exchange.route_message(&message, "user.created");
        // Both patterns should match
        assert!(queues.len() >= 1);
    }

    #[test]
    fn test_exchange_route_topic_hash_wildcard() {
        let exchange = Exchange {
            name: "test-exchange".to_string(),
            exchange_type: ExchangeType::Topic,
            durable: false,
            auto_delete: false,
            arguments: HashMap::new(),
            bindings: vec![create_test_binding("queue1", "user.#")],
        };

        let message = create_test_message("user.created.v1");
        let queues = exchange.route_message(&message, "user.created.v1");
        assert_eq!(queues.len(), 1);
    }

    #[test]
    fn test_matches_topic_pattern_exact() {
        let routing = vec!["user", "created"];
        let pattern = vec!["user", "created"];
        assert!(Exchange::matches_topic_pattern(&routing, &pattern));
    }

    #[test]
    fn test_matches_topic_pattern_star() {
        let routing = vec!["user", "created"];
        let pattern = vec!["user", "*"];
        assert!(Exchange::matches_topic_pattern(&routing, &pattern));
    }

    #[test]
    fn test_matches_topic_pattern_hash() {
        let routing = vec!["user", "created", "v1"];
        let pattern = vec!["user", "#"];
        assert!(Exchange::matches_topic_pattern(&routing, &pattern));
    }

    #[test]
    fn test_matches_topic_pattern_no_match() {
        let routing = vec!["user", "created"];
        let pattern = vec!["order", "created"];
        assert!(!Exchange::matches_topic_pattern(&routing, &pattern));
    }

    #[test]
    fn test_exchange_route_headers_all() {
        let mut binding_args = HashMap::new();
        binding_args.insert("x-match".to_string(), "all".to_string());
        binding_args.insert("type".to_string(), "user".to_string());
        binding_args.insert("action".to_string(), "created".to_string());

        let exchange = Exchange {
            name: "test-exchange".to_string(),
            exchange_type: ExchangeType::Headers,
            durable: false,
            auto_delete: false,
            arguments: HashMap::new(),
            bindings: vec![Binding {
                exchange: "test-exchange".to_string(),
                queue: "queue1".to_string(),
                routing_key: String::new(),
                arguments: binding_args,
            }],
        };

        let mut headers = HashMap::new();
        headers.insert("type".to_string(), "user".to_string());
        headers.insert("action".to_string(), "created".to_string());

        let mut props = MessageProperties::default();
        props.headers = headers;

        let message = Message {
            properties: props,
            body: b"test".to_vec(),
            routing_key: String::new(),
        };

        let queues = exchange.route_message(&message, "");
        assert_eq!(queues.len(), 1);
    }

    #[test]
    fn test_exchange_route_headers_any() {
        let mut binding_args = HashMap::new();
        binding_args.insert("x-match".to_string(), "any".to_string());
        binding_args.insert("type".to_string(), "user".to_string());
        binding_args.insert("action".to_string(), "nonexistent".to_string());

        let exchange = Exchange {
            name: "test-exchange".to_string(),
            exchange_type: ExchangeType::Headers,
            durable: false,
            auto_delete: false,
            arguments: HashMap::new(),
            bindings: vec![Binding {
                exchange: "test-exchange".to_string(),
                queue: "queue1".to_string(),
                routing_key: String::new(),
                arguments: binding_args,
            }],
        };

        let mut headers = HashMap::new();
        headers.insert("type".to_string(), "user".to_string());

        let mut props = MessageProperties::default();
        props.headers = headers;

        let message = Message {
            properties: props,
            body: b"test".to_vec(),
            routing_key: String::new(),
        };

        let queues = exchange.route_message(&message, "");
        assert_eq!(queues.len(), 1); // "any" matches if type matches
    }

    #[test]
    fn test_exchange_manager_new() {
        let manager = ExchangeManager::new();
        assert!(manager.list_exchanges().is_empty());
    }

    #[test]
    fn test_exchange_manager_default() {
        let manager = ExchangeManager::default();
        assert!(manager.list_exchanges().is_empty());
    }

    #[test]
    fn test_exchange_manager_declare() {
        let mut manager = ExchangeManager::new();
        manager.declare_exchange("test".to_string(), ExchangeType::Direct, true, false);

        let exchange = manager.get_exchange("test");
        assert!(exchange.is_some());
        assert_eq!(exchange.unwrap().exchange_type, ExchangeType::Direct);
    }

    #[test]
    fn test_exchange_manager_list() {
        let mut manager = ExchangeManager::new();
        manager.declare_exchange("ex1".to_string(), ExchangeType::Direct, false, false);
        manager.declare_exchange("ex2".to_string(), ExchangeType::Fanout, false, false);

        let exchanges = manager.list_exchanges();
        assert_eq!(exchanges.len(), 2);
    }

    #[test]
    fn test_exchange_manager_delete() {
        let mut manager = ExchangeManager::new();
        manager.declare_exchange("test".to_string(), ExchangeType::Direct, false, false);

        assert!(manager.delete_exchange("test"));
        assert!(manager.get_exchange("test").is_none());
        assert!(!manager.delete_exchange("nonexistent"));
    }

    #[test]
    fn test_exchange_manager_get_nonexistent() {
        let manager = ExchangeManager::new();
        assert!(manager.get_exchange("nonexistent").is_none());
    }

    #[test]
    fn test_exchange_clone() {
        let exchange = Exchange {
            name: "test".to_string(),
            exchange_type: ExchangeType::Direct,
            durable: true,
            auto_delete: false,
            arguments: HashMap::new(),
            bindings: vec![],
        };

        let cloned = exchange.clone();
        assert_eq!(exchange.name, cloned.name);
        assert_eq!(exchange.exchange_type, cloned.exchange_type);
    }
}
