# MockForge MQTT

MQTT protocol support for MockForge with full broker simulation, topic management, and QoS handling.

This crate provides comprehensive MQTT mocking capabilities for IoT applications, pub/sub systems, and message queue testing. Perfect for testing MQTT clients, brokers, and IoT device communication without requiring external MQTT infrastructure.

## Features

- **Full MQTT Broker**: Complete MQTT 3.1.1 and 5.0 protocol support
- **Topic Management**: Hierarchical topic structure with wildcards
- **QoS Levels**: Support for QoS 0, 1, and 2 message delivery
- **Session Management**: Persistent sessions and clean session handling
- **Retained Messages**: Store and deliver retained messages
- **Will Messages**: Last will and testament message handling
- **Authentication**: Configurable client authentication
- **Metrics & Monitoring**: Comprehensive MQTT metrics collection
- **Fixture System**: YAML-based message templates and auto-publishing

## Quick Start

### Basic MQTT Broker

```rust,no_run
use mockforge_mqtt::{MqttBroker, MqttConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create broker configuration
    let config = MqttConfig {
        host: "127.0.0.1".to_string(),
        port: 1883,
        ..Default::default()
    };

    // Initialize broker
    let spec_registry = Arc::new(MqttSpecRegistry::new());
    let broker = MqttBroker::new(config, spec_registry);

    // Start the broker (this would typically run in a separate task)
    // broker.start().await?;

    Ok(())
}
```

### Testing with MQTT Clients

```rust,no_run
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to MockForge MQTT broker
    let mut mqtt_options = MqttOptions::new("test-client", "localhost", 1883);
    mqtt_options.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

    // Subscribe to a topic
    client.subscribe("sensors/temperature", QoS::AtMostOnce).await?;

    // Publish a message
    client.publish("sensors/temperature", QoS::AtLeastOnce, false, "23.5").await?;

    // Handle events
    loop {
        match eventloop.poll().await {
            Ok(notification) => {
                println!("Received: {:?}", notification);
            }
            Err(e) => {
                println!("Error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}
```

## Core Components

### MqttBroker

The main broker implementation handling all MQTT protocol operations:

```rust,no_run
use mockforge_mqtt::{MqttBroker, MqttConfig, MqttSpecRegistry};

let config = MqttConfig {
    host: "0.0.0.0".to_string(),
    port: 1883,
    max_connections: 1000,
    max_packet_size: 1024 * 1024, // 1MB
    keep_alive_secs: 60,
    version: MqttVersion::V5_0,
};

let spec_registry = Arc::new(MqttSpecRegistry::new());
let broker = MqttBroker::new(config, spec_registry);
```

### Topic Management

Hierarchical topic structure with wildcard support:

```rust,no_run
use mockforge_mqtt::topics::TopicTree;

// Create topic tree
let topic_tree = TopicTree::new();

// Topics support wildcards:
// + (single level) and # (multi-level)
topic_tree.subscribe("client/sensor/+/temperature", qos);
topic_tree.subscribe("home/+/status", qos);
topic_tree.subscribe("iot/devices/#", qos);
```

### QoS Handling

Support for all MQTT Quality of Service levels:

```rust,no_run
use mockforge_mqtt::qos::{QoSHandler, MessageState};

// QoS 0: At most once (fire and forget)
let qos_0 = QoSHandler::publish_at_most_once(&message);

// QoS 1: At least once (acknowledged delivery)
let qos_1 = QoSHandler::publish_at_least_once(&message).await?;

// QoS 2: Exactly once (two-phase commit)
let qos_2 = QoSHandler::publish_exactly_once(&message).await?;
```

### Session Management

Persistent sessions for reliable messaging:

```rust,no_run
use mockforge_mqtt::broker::ClientSession;

// Clean session (default)
let clean_session = ClientSession {
    client_id: "client-1".to_string(),
    subscriptions: HashMap::new(),
    clean_session: true,
    connected_at: now,
    last_seen: now,
};

// Persistent session
let persistent_session = ClientSession {
    client_id: "client-2".to_string(),
    subscriptions: HashMap::new(),
    clean_session: false, // Session persists across connections
    connected_at: now,
    last_seen: now,
};
```

## Fixture System

Define message templates and auto-publishing rules using YAML:

```yaml
# mqtt-fixture.yaml
topics:
  - name: "sensors/temperature"
    retained: false
  - name: "devices/status"
    retained: true

fixtures:
  - topic: "sensors/temperature"
    payload: '{"sensor_id": "temp-001", "value": 23.5, "unit": "celsius"}'
    qos: 1
    retain: false

  - topic: "devices/status"
    payload: '{"device_id": "dev-001", "status": "online", "battery": 85}'
    qos: 0
    retain: true

auto_publish:
  - topic: "sensors/temperature"
    payload_template: '{"sensor_id": "temp-{{sensor_id}}", "value": {{temperature}}, "timestamp": "{{now}}"}'
    qos: 1
    interval_seconds: 30
    duration_seconds: 300
    variables:
      sensor_id: "001"
      temperature: "22.5"

  - topic: "iot/heartbeat"
    payload_template: '{"service": "{{service_name}}", "status": "alive", "uptime": {{uptime}}}'
    qos: 0
    interval_seconds: 60
    variables:
      service_name: "mockforge-mqtt"
      uptime: 3600
```

### Loading Fixtures

```rust,no_run
use mockforge_mqtt::{MqttBroker, MqttSpecRegistry};

// Create broker with fixture support
let spec_registry = Arc::new(MqttSpecRegistry::new());
let broker = MqttBroker::new(config, spec_registry);

// Load fixtures from file
broker.load_fixtures_from_file("mqtt-fixture.yaml").await?;

// Or create fixtures programmatically
use mockforge_mqtt::fixtures::{MqttFixture, AutoPublishConfig};

let fixture = MqttFixture {
    topics: vec![/* ... */],
    fixtures: vec![/* ... */],
    auto_publish: vec![/* ... */],
};

broker.add_fixture(fixture).await?;
```

## Supported MQTT Features

### Protocol Versions
- **MQTT 3.1.1**: Legacy protocol support
- **MQTT 5.0**: Latest protocol with enhanced features

### Message Types
- **CONNECT**: Client connection establishment
- **CONNACK**: Connection acknowledgment
- **PUBLISH**: Message publication
- **PUBACK/PUBREC/PUBREL/PUBCOMP**: QoS flow control
- **SUBSCRIBE**: Topic subscription
- **SUBACK**: Subscription acknowledgment
- **UNSUBSCRIBE**: Topic unsubscription
- **UNSUBACK**: Unsubscription acknowledgment
- **PINGREQ/PINGRESP**: Keep-alive handling
- **DISCONNECT**: Clean disconnection

### Advanced Features
- **Will Messages**: Last will and testament
- **Retained Messages**: Persistent topic messages
- **Topic Aliases**: Bandwidth optimization (MQTT 5.0)
- **Subscription Identifiers**: Subscription tracking (MQTT 5.0)
- **User Properties**: Custom metadata (MQTT 5.0)

## Configuration

### MqttConfig

```rust,no_run
use mockforge_mqtt::{MqttConfig, MqttVersion};

let config = MqttConfig {
    host: "0.0.0.0".to_string(),
    port: 1883,
    max_connections: 1000,
    max_packet_size: 1024 * 1024, // 1MB
    keep_alive_secs: 60,
    version: MqttVersion::V5_0,
};
```

### Environment Variables

```bash
# Server configuration
export MQTT_HOST=0.0.0.0
export MQTT_PORT=1883

# Connection limits
export MQTT_MAX_CONNECTIONS=1000
export MQTT_MAX_PACKET_SIZE=1048576

# Protocol settings
export MQTT_KEEP_ALIVE_SECS=60
export MQTT_VERSION=v5
```

## Testing Examples

### Publisher Testing

```rust,no_run
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::time::Duration;

#[tokio::test]
async fn test_mqtt_publisher() {
    // Start MockForge MQTT broker in background
    let broker = MqttBroker::new(MqttConfig::default(), Arc::new(MqttSpecRegistry::new()));
    tokio::spawn(async move { broker.start().await.unwrap() });

    // Give broker time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test publisher
    let mut mqtt_options = MqttOptions::new("test-publisher", "localhost", 1883);
    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

    // Publish test message
    client
        .publish("test/topic", QoS::AtLeastOnce, false, "Hello MQTT!")
        .await
        .unwrap();

    // Verify message was published (check broker state)
    // ... verification logic ...
}
```

### Subscriber Testing

```rust,no_run
use rumqttc::{AsyncClient, MqttOptions, QoS, Event};
use futures::StreamExt;

#[tokio::test]
async fn test_mqtt_subscriber() {
    // Start broker and publish test message
    // ... setup code ...

    // Create subscriber
    let mut mqtt_options = MqttOptions::new("test-subscriber", "localhost", 1883);
    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

    // Subscribe to topic
    client.subscribe("test/topic", QoS::AtMostOnce).await.unwrap();

    // Publish a message
    client.publish("test/topic", QoS::AtLeastOnce, false, "test message").await.unwrap();

    // Receive message
    let event = eventloop.next().await.unwrap().unwrap();
    match event {
        Event::Incoming(incoming) => {
            if let rumqttc::Packet::Publish(publish) = incoming {
                let payload = std::str::from_utf8(&publish.payload).unwrap();
                assert_eq!(payload, "test message");
            }
        }
        _ => panic!("Expected publish event"),
    }
}
```

### QoS Testing

```rust,no_run
use rumqttc::{AsyncClient, MqttOptions, QoS};

#[tokio::test]
async fn test_mqtt_qos_levels() {
    // Test QoS 0 (At most once)
    let (client, mut eventloop) = AsyncClient::new(MqttOptions::new("qos-test", "localhost", 1883), 10);
    client.subscribe("qos/test", QoS::AtMostOnce).await.unwrap();
    client.publish("qos/test", QoS::AtMostOnce, false, "QoS 0 message").await.unwrap();

    // Test QoS 1 (At least once)
    client.publish("qos/test", QoS::AtLeastOnce, false, "QoS 1 message").await.unwrap();

    // Test QoS 2 (Exactly once)
    client.publish("qos/test", QoS::ExactlyOnce, false, "QoS 2 message").await.unwrap();

    // Verify messages are received (broker should handle QoS flows)
}
```

### Retained Messages

```rust,no_run
use rumqttc::{AsyncClient, MqttOptions, QoS};

#[tokio::test]
async fn test_retained_messages() {
    // Publish retained message
    let (publisher, _) = AsyncClient::new(MqttOptions::new("publisher", "localhost", 1883), 10);
    publisher
        .publish("retained/topic", QoS::AtLeastOnce, true, "retained message")
        .await
        .unwrap();

    // New subscriber should receive retained message immediately
    let (subscriber, mut eventloop) = AsyncClient::new(MqttOptions::new("subscriber", "localhost", 1883), 10);
    subscriber.subscribe("retained/topic", QoS::AtMostOnce).await.unwrap();

    // Should receive retained message
    let event = eventloop.next().await.unwrap().unwrap();
    match event {
        Event::Incoming(incoming) => {
            if let rumqttc::Packet::Publish(publish) = incoming {
                assert!(publish.retain);
                let payload = std::str::from_utf8(&publish.payload).unwrap();
                assert_eq!(payload, "retained message");
            }
        }
        _ => panic!("Expected retained publish event"),
    }
}
```

## Performance

MockForge MQTT is optimized for testing scenarios:

- **In-Memory Operations**: Fast message routing without persistence
- **Concurrent Connections**: Handle multiple simultaneous MQTT clients
- **Low Latency**: Minimal overhead for message operations
- **Scalable**: Support for high-throughput IoT testing scenarios
- **Resource Efficient**: Configurable connection limits and cleanup

## Integration with MockForge

MockForge MQTT integrates seamlessly with the MockForge ecosystem:

- **MockForge Core**: Shared configuration and logging
- **MockForge CLI**: Command-line MQTT broker management
- **MockForge Data**: Enhanced message generation with templates
- **MockForge Observability**: Metrics and tracing integration

## Troubleshooting

### Common Issues

**Connection refused:**
- Ensure broker is started and listening on correct port
- Check firewall settings and port availability
- Verify client connection parameters

**Messages not received:**
- Check topic subscription patterns and wildcards
- Verify QoS levels match between publisher and subscriber
- Check retained message settings

**QoS issues:**
- Ensure broker supports requested QoS level
- Check network reliability for higher QoS levels
- Verify client acknowledgment handling

**Session persistence:**
- Check clean session flag settings
- Verify client ID consistency across connections
- Check session expiry settings

## Examples

See the [examples directory](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples) for complete working examples including:

- Basic MQTT broker setup
- Publisher/subscriber testing patterns
- QoS level verification
- Retained message scenarios
- IoT device simulation
- Load testing with multiple clients

## Related Crates

- [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
- [`rumqttc`](https://docs.rs/rumqttc): MQTT client library for testing
- [`rumqttd`](https://docs.rs/rumqttd): Underlying MQTT broker implementation

## License

Licensed under MIT OR Apache-2.0
