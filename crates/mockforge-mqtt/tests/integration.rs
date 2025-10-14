//! Integration tests for MQTT broker

use mockforge_mqtt::{MqttBroker, MqttSpecRegistry, MqttFixture, MqttResponse};
use mockforge_mqtt::broker::MqttConfig;
use mockforge_core::protocol_abstraction::{SpecRegistry, Protocol, MessagePattern};
use std::sync::Arc;



#[tokio::test]
async fn test_broker_creation() {
    let config = MqttConfig::default();
    let spec_registry = Arc::new(MqttSpecRegistry::new());
    let broker = MqttBroker::new(config, spec_registry);

    // Basic test - broker should be created without error
    assert_eq!(broker.config().port, 1883);
    assert_eq!(broker.config().host, "0.0.0.0");
}

#[tokio::test]
async fn test_topic_matching() {
    use mockforge_mqtt::TopicTree;

    let mut tree = TopicTree::new();

    // Subscribe to topics
    tree.subscribe("sensors/temperature/+", 1, "client1");
    tree.subscribe("sensors/#", 0, "client2");
    tree.subscribe("devices/+/status", 2, "client3");

    // Test matching
    let matches = tree.match_topic("sensors/temperature/room1");
    assert_eq!(matches.len(), 2);

    let matches = tree.match_topic("sensors/humidity/room1");
    assert_eq!(matches.len(), 1); // Only client2's wildcard match

    let matches = tree.match_topic("devices/light1/status");
    assert_eq!(matches.len(), 1); // Only client3 match

    let matches = tree.match_topic("other/topic");
    assert_eq!(matches.len(), 0); // No matches
}

#[tokio::test]
async fn test_topic_wildcards() {
    use mockforge_mqtt::TopicTree;

    let mut tree = TopicTree::new();

    // Test single level wildcard (+)
    tree.subscribe("sensors/+/temperature", 0, "client1");
    tree.subscribe("sensors/+/humidity", 0, "client2");

    // Test multi-level wildcard (#)
    tree.subscribe("devices/#", 0, "client3");

    // Test matches
    let matches = tree.match_topic("sensors/room1/temperature");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].client_id, "client1");

    let matches = tree.match_topic("sensors/room1/humidity");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].client_id, "client2");

    let matches = tree.match_topic("devices/light1/status");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].client_id, "client3");

    let matches = tree.match_topic("devices/room1/light1/brightness");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].client_id, "client3");
}

#[tokio::test]
async fn test_retained_messages() {
    use mockforge_mqtt::TopicTree;

    let mut tree = TopicTree::new();

    // Store retained messages
    tree.retain_message("sensors/temp/room1", b"25.5".to_vec(), 1);
    tree.retain_message("sensors/temp/room2", b"22.0".to_vec(), 0);
    tree.retain_message("status/online", b"true".to_vec(), 1);

    // Test retrieval
    let retained = tree.get_retained("sensors/temp/room1");
    assert!(retained.is_some());
    assert_eq!(retained.unwrap().payload, b"25.5");
    assert_eq!(retained.unwrap().qos, 1);

    let retained = tree.get_retained("sensors/temp/room2");
    assert!(retained.is_some());
    assert_eq!(retained.unwrap().payload, b"22.0");
    assert_eq!(retained.unwrap().qos, 0);

    // Test retained message removal (empty payload)
    tree.retain_message("sensors/temp/room1", vec![], 0);
    let retained = tree.get_retained("sensors/temp/room1");
    assert!(retained.is_none());

    // Test retained messages for filter
    tree.subscribe("sensors/temp/+", 0, "client1");
    let retained_for_filter = tree.get_retained_for_filter("sensors/temp/+");
    assert_eq!(retained_for_filter.len(), 1);
    assert_eq!(retained_for_filter[0].1.payload, b"22.0");
}

#[tokio::test]
async fn test_spec_registry() {
    let mut registry = MqttSpecRegistry::new();

    // Add fixtures
    let fixture1 = MqttFixture {
        identifier: "temp-sensor".to_string(),
        name: "Temperature Sensor".to_string(),
        topic_pattern: r"^sensors/temp/[^/]+$".to_string(),
        qos: 1,
        retained: false,
        response: MqttResponse {
            payload: serde_json::json!({"temp": 25.0}),
        },
        auto_publish: None,
    };

    let fixture2 = MqttFixture {
        identifier: "humidity-sensor".to_string(),
        name: "Humidity Sensor".to_string(),
        topic_pattern: r"^sensors/humidity/[^/]+$".to_string(),
        qos: 0,
        retained: true,
        response: MqttResponse {
            payload: serde_json::json!({"humidity": 60.0}),
        },
        auto_publish: None,
    };

    registry.add_fixture(fixture1);
    registry.add_fixture(fixture2);

    // Test operations
    let operations = registry.operations();
    assert_eq!(operations.len(), 2);

    // Test finding operations
    let op = registry.find_operation("", "sensors/temp/room1");
    assert!(op.is_some());
    assert_eq!(op.unwrap().name, "temp-sensor");

    let op = registry.find_operation("", "sensors/humidity/room1");
    assert!(op.is_some());
    assert_eq!(op.unwrap().name, "humidity-sensor");

    let op = registry.find_operation("", "unknown/topic");
    assert!(op.is_none());
}

#[tokio::test]
async fn test_fixture_matching() {
    let mut registry = MqttSpecRegistry::new();

    let fixture = MqttFixture {
        identifier: "sensor".to_string(),
        name: "Sensor".to_string(),
        topic_pattern: r"^sensors/([^/]+)/([^/]+)$".to_string(),
        qos: 1,
        retained: false,
        response: MqttResponse {
            payload: serde_json::json!({
                "sensor_type": "{{topic_param 1}}",
                "sensor_id": "{{topic_param 2}}",
                "value": 42
            }),
        },
        auto_publish: None,
    };

    registry.add_fixture(fixture);

    // Test matching
    let matched = registry.find_fixture_by_topic("sensors/temp/room1");
    assert!(matched.is_some());

    let matched = registry.find_fixture_by_topic("sensors/humidity/bathroom");
    assert!(matched.is_some());

    let matched = registry.find_fixture_by_topic("other/topic");
    assert!(matched.is_none());
}

#[tokio::test]
async fn test_validation() {
    let registry = MqttSpecRegistry::new();

    // Test validation without fixtures (should fail)
    let request = mockforge_core::protocol_abstraction::ProtocolRequest {
        protocol: Protocol::Mqtt,
        pattern: MessagePattern::OneWay,
        operation: "publish".to_string(),
        path: "".to_string(),
        topic: None,
        routing_key: None,
        partition: None,
        qos: Some(0),
        metadata: std::collections::HashMap::new(),
        body: None,
        client_ip: None,
    };

    let result = registry.validate_request(&request);
    assert!(result.is_ok());
    assert!(!result.unwrap().valid);

    // Test with topic
    let request_with_topic = mockforge_core::protocol_abstraction::ProtocolRequest {
        topic: Some("test/topic".to_string()),
        ..request
    };

    let result = registry.validate_request(&request_with_topic);
    assert!(result.is_ok());
    let validation = result.unwrap();
    assert!(!validation.valid); // No fixtures match
    assert_eq!(validation.errors.len(), 1);
}

#[tokio::test]
async fn test_qo_s_handling() {
    use mockforge_mqtt::qos::{QoSHandler, QoS, MessageState};

    let handler = QoSHandler::new();

    // Test QoS 0
    let message = MessageState {
        packet_id: 1,
        topic: "test/topic".to_string(),
        payload: b"test".to_vec(),
        qos: QoS::AtMostOnce,
        retained: false,
        timestamp: 1234567890,
    };

    let result = handler.handle_qo_s0(message).await;
    assert!(result.is_ok());

    // Test QoS 1
    let message = MessageState {
        packet_id: 2,
        topic: "test/topic".to_string(),
        payload: b"test".to_vec(),
        qos: QoS::AtLeastOnce,
        retained: false,
        timestamp: 1234567890,
    };

    let result = handler.handle_qo_s1(message, "client1").await;
    assert!(result.is_ok());

    // Test QoS 2
    let message = MessageState {
        packet_id: 3,
        topic: "test/topic".to_string(),
        payload: b"test".to_vec(),
        qos: QoS::ExactlyOnce,
        retained: false,
        timestamp: 1234567890,
    };

    let result = handler.handle_qo_s2(message, "client1").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_session_management() {
    let config = MqttConfig::default();
    let spec_registry = Arc::new(MqttSpecRegistry::new());
    let broker = MqttBroker::new(config, spec_registry);

    // Test clean session connection
    let result = broker.client_connect("client1", true).await;
    assert!(result.is_ok());

    // Test persistent session connection
    let result = broker.client_connect("client2", false).await;
    assert!(result.is_ok());

    // Test subscription with clean session
    let subscriptions = vec![("sensors/temp/+".to_string(), 1)];
    let result = broker.client_subscribe("client1", subscriptions).await;
    assert!(result.is_ok());

    // Test subscription with persistent session
    let subscriptions = vec![("devices/#".to_string(), 0)];
    let result = broker.client_subscribe("client2", subscriptions).await;
    assert!(result.is_ok());

    // Test disconnection
    let result = broker.client_disconnect("client1").await;
    assert!(result.is_ok());

    let result = broker.client_disconnect("client2").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_template_integration() {
    let mut registry = MqttSpecRegistry::new();

    let fixture = MqttFixture {
        identifier: "templated-sensor".to_string(),
        name: "Templated Sensor".to_string(),
        topic_pattern: r"^sensors/.*$".to_string(),
        qos: 0,
        retained: false,
        response: MqttResponse {
            payload: serde_json::json!({
                "timestamp": "{{now}}",
                "random_value": "{{faker.float 0.0 100.0}}",
                "topic": "{{topic}}"
            }),
        },
        auto_publish: None,
    };

    registry.add_fixture(fixture);

    // Test mock response generation
    let request = mockforge_core::protocol_abstraction::ProtocolRequest {
        protocol: Protocol::Mqtt,
        pattern: MessagePattern::OneWay,
        operation: "publish".to_string(),
        path: "".to_string(),
        topic: Some("sensors/temp/room1".to_string()),
        routing_key: None,
        partition: None,
        qos: Some(0),
        metadata: std::collections::HashMap::new(),
        body: Some(vec![]),
        client_ip: None,
    };

    let result = registry.generate_mock_response(&request);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.status, mockforge_core::protocol_abstraction::ResponseStatus::MqttStatus(true));
    assert!(!response.body.is_empty());

    // Parse the JSON response
    let json: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert!(json.get("timestamp").is_some());
    assert!(json.get("random_value").is_some());
    assert_eq!(json.get("topic").unwrap(), "sensors/temp/room1");
}