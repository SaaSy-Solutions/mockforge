//! MockForge AMQP (RabbitMQ) Protocol Support
//!
//! This crate provides AMQP 0.9.1 protocol support for MockForge,
//! enabling testing of message queue patterns, pub/sub, and enterprise messaging scenarios.

pub mod bindings;
pub mod broker;
pub mod consumers;
pub mod exchanges;
pub mod fixtures;
pub mod messages;
pub mod protocol;
pub mod queues;
pub mod spec_registry;

pub use broker::AmqpBroker;
pub use spec_registry::AmqpSpecRegistry;
