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

    pub fn list_exchanges(&self) -> Vec<&Exchange> {
        self.exchanges.values().collect()
    }

    pub fn delete_exchange(&mut self, name: &str) -> bool {
        self.exchanges.remove(name).is_some()
    }
}
