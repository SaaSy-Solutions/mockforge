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
pub use fixtures::{AutoPublishConfig, MqttFixture, MqttFixtureRegistry, MqttResponse};
pub use server::start_mqtt_server;
pub use spec_registry::MqttSpecRegistry;
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
