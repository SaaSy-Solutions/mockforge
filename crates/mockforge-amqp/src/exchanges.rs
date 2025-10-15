use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::bindings::Binding;
use crate::messages::Message;

/// Exchange types supported by AMQP
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    fn matches_topic_pattern(routing_parts: &[&str], pattern_parts: &[&str]) -> bool {
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

    fn route_headers(&self, _message: &Message) -> Vec<String> {
        // TODO: Implement header matching
        vec![]
    }
}

/// Manager for all exchanges
pub struct ExchangeManager {
    exchanges: HashMap<String, Exchange>,
}

impl ExchangeManager {
    pub fn new() -> Self {
        Self {
            exchanges: HashMap::new(),
        }
    }

    pub fn declare_exchange(&mut self, name: String, exchange_type: ExchangeType, durable: bool, auto_delete: bool) {
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
}
