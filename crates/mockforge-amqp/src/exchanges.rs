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

    fn route_topic(&self, _routing_key: &str) -> Vec<String> {
        // TODO: Implement topic pattern matching
        vec![]
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