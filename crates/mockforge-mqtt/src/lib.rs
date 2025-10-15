//! MQTT protocol support for MockForge
//!
//! This crate provides MQTT broker functionality for IoT and pub/sub testing scenarios.

pub mod broker;
pub mod fixtures;
pub mod qos;
pub mod server;
pub mod spec_registry;
pub mod topics;

pub use broker::{MqttBroker, MqttConfig};
pub use fixtures::{AutoPublishConfig, MqttFixture, MqttResponse};
pub use server::start_mqtt_server;
pub use spec_registry::MqttSpecRegistry;
pub use topics::TopicTree;
