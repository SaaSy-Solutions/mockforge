//! MQTT protocol support for MockForge
//!
//! This crate provides MQTT broker functionality for IoT and pub/sub testing scenarios.

pub mod broker;
pub mod topics;
pub mod fixtures;
pub mod spec_registry;
pub mod qos;
pub mod server;

pub use broker::{MqttBroker, MqttConfig};
pub use topics::TopicTree;
pub use fixtures::{MqttFixture, MqttResponse, AutoPublishConfig};
pub use spec_registry::MqttSpecRegistry;
pub use server::start_mqtt_server;
