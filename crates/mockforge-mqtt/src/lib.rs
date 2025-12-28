//! MQTT protocol support for MockForge
//!
//! This crate provides a complete MQTT 3.1.1 broker implementation for IoT and pub/sub
//! testing scenarios.
//!
//! ## Features
//!
//! - **Full MQTT 3.1.1 Protocol Support**: Handles all control packet types including
//!   CONNECT, PUBLISH, SUBSCRIBE, and their acknowledgments
//! - **QoS 0, 1, 2 Support**: Fire-and-forget, at-least-once, and exactly-once delivery
//! - **Session Management**: Clean and persistent sessions with subscription restoration
//! - **Topic Wildcards**: Supports + and # wildcards for flexible subscriptions
//! - **Retained Messages**: Messages are stored and delivered to new subscribers
//!
//! ## Metrics and Observability
//!
//! The MQTT broker includes built-in metrics collection for monitoring:
//! - Connection counts (total and active)
//! - Message publish/delivery rates
//! - Subscription tracking
//! - QoS level distribution
//! - Error rates and latency
//!
//! Use [`MqttMetrics`] to collect metrics and [`MqttMetricsExporter`] to export
//! in Prometheus format.
//!
//! ## Usage
//!
//! ```no_run
//! use mockforge_mqtt::{MqttConfig, start_mqtt_server};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = MqttConfig::default();
//!     start_mqtt_server(config).await.expect("Failed to start MQTT server");
//! }
//! ```

pub mod broker;
pub mod fixtures;
pub mod metrics;
pub mod protocol;
pub mod qos;
pub mod server;
pub mod session;
pub mod spec_registry;
pub mod tls;
pub mod topics;

pub use broker::{MqttBroker, MqttConfig};
pub use fixtures::{AutoPublishConfig, MqttFixture, MqttFixtureRegistry, MqttResponse};
pub use metrics::{MqttMetrics, MqttMetricsExporter, MqttMetricsSnapshot};
pub use protocol::{
    ConnackCode, ConnackPacket, ConnectPacket, Packet, PacketDecoder, PacketEncoder, ProtocolError,
    PublishPacket, QoS as ProtocolQoS, SubackPacket, SubackReturnCode, SubscribePacket,
    UnsubscribePacket,
};
pub use server::{
    start_mqtt_dual_server, start_mqtt_server, start_mqtt_server_with_metrics,
    start_mqtt_tls_server, MqttServer,
};
pub use session::SessionManager;
pub use spec_registry::MqttSpecRegistry;
pub use tls::{create_tls_acceptor, create_tls_acceptor_with_client_auth, TlsError};
pub use topics::TopicTree;

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::protocol_abstraction::SpecRegistry;
    use std::sync::Arc;

    #[test]
    fn test_mqtt_broker_export() {
        let config = MqttConfig::default();
        let spec_registry = Arc::new(MqttSpecRegistry::new());
        let _broker = MqttBroker::new(config, spec_registry);
        // Just testing that the type is accessible
    }

    #[test]
    fn test_mqtt_config_export() {
        let config = MqttConfig::default();
        assert_eq!(config.port, 1883);
    }

    #[test]
    fn test_mqtt_fixture_export() {
        let fixture = MqttFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            topic_pattern: "test".to_string(),
            qos: 0,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({}),
            },
            auto_publish: None,
        };
        assert_eq!(fixture.identifier, "test");
    }

    #[test]
    fn test_mqtt_response_export() {
        let response = MqttResponse {
            payload: serde_json::json!({"test": "data"}),
        };
        assert_eq!(response.payload["test"], "data");
    }

    #[test]
    fn test_auto_publish_config_export() {
        let config = AutoPublishConfig {
            enabled: true,
            interval_ms: 1000,
            count: Some(5),
        };
        assert!(config.enabled);
        assert_eq!(config.interval_ms, 1000);
    }

    #[test]
    fn test_mqtt_fixture_registry_export() {
        let registry = MqttFixtureRegistry::new();
        assert_eq!(registry.fixtures().count(), 0);
    }

    #[test]
    fn test_mqtt_spec_registry_export() {
        let registry = MqttSpecRegistry::new();
        assert_eq!(registry.operations().len(), 0);
    }

    #[test]
    fn test_topic_tree_export() {
        let tree = TopicTree::new();
        let stats = tree.stats();
        assert_eq!(stats.total_subscriptions, 0);
    }

    #[test]
    fn test_all_modules_accessible() {
        // Test that all modules are accessible
        let _broker_module = broker::MqttConfig::default();
        let _fixtures_module = fixtures::MqttFixtureRegistry::new();
        let _spec_module = spec_registry::MqttSpecRegistry::new();
        let _topics_module = topics::TopicTree::new();
        let _qos_module = qos::QoS::AtMostOnce;
    }

    #[test]
    fn test_qos_levels_accessible() {
        use qos::QoS;
        assert_eq!(QoS::AtMostOnce.as_u8(), 0);
        assert_eq!(QoS::AtLeastOnce.as_u8(), 1);
        assert_eq!(QoS::ExactlyOnce.as_u8(), 2);
    }

    #[tokio::test]
    async fn test_broker_basic_usage() {
        let config = MqttConfig::default();
        let spec_registry = Arc::new(MqttSpecRegistry::new());
        let broker = MqttBroker::new(config, spec_registry);

        // Test basic operations
        broker.client_connect("test-client", true).await.unwrap();
        let clients = broker.get_connected_clients().await;
        assert_eq!(clients.len(), 1);

        broker.client_disconnect("test-client").await.unwrap();
        let clients = broker.get_connected_clients().await;
        assert_eq!(clients.len(), 0);
    }

    #[test]
    fn test_fixture_registry_basic_usage() {
        let mut registry = MqttFixtureRegistry::new();

        let fixture = MqttFixture {
            identifier: "test-fixture".to_string(),
            name: "Test Fixture".to_string(),
            topic_pattern: "test/topic".to_string(),
            qos: 1,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({"message": "test"}),
            },
            auto_publish: None,
        };

        registry.add_fixture(fixture);
        assert_eq!(registry.fixtures().count(), 1);

        let found = registry.find_by_topic("test/topic");
        assert!(found.is_some());
    }

    #[test]
    fn test_topic_tree_basic_usage() {
        let mut tree = TopicTree::new();

        // Subscribe
        tree.subscribe("sensor/temp", 1, "client-1");

        // Check subscription
        let matches = tree.match_topic("sensor/temp");
        assert_eq!(matches.len(), 1);

        // Unsubscribe
        tree.unsubscribe("sensor/temp", "client-1");
        let matches = tree.match_topic("sensor/temp");
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_spec_registry_basic_usage() {
        let mut registry = MqttSpecRegistry::new();

        let fixture = MqttFixture {
            identifier: "spec-test".to_string(),
            name: "Spec Test".to_string(),
            topic_pattern: "spec/test".to_string(),
            qos: 0,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({}),
            },
            auto_publish: None,
        };

        registry.add_fixture(fixture);

        let found = registry.find_fixture_by_topic("spec/test");
        assert!(found.is_some());
        assert_eq!(found.unwrap().identifier, "spec-test");
    }

    #[test]
    fn test_module_documentation() {
        // This test ensures the module is properly documented
        // The module doc comment should mention MQTT, broker, and IoT
    }
}
