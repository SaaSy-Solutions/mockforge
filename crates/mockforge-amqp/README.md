# MockForge AMQP

AMQP 0.9.1 protocol support for MockForge, enabling testing of message queue patterns, pub/sub, and enterprise messaging scenarios.

This crate provides a RabbitMQ-compatible AMQP broker implementation that allows you to mock message queue interactions, test pub/sub patterns, and simulate enterprise messaging scenarios in your applications.

## Features

- **AMQP 0.9.1 Protocol**: Full RabbitMQ-compatible protocol support
- **Mock Broker**: Simulate message broker behavior for testing
- **Exchange Management**: Direct, topic, headers, and fanout exchanges
- **Queue Management**: Dynamic queue creation and management
- **Message Routing**: Flexible message routing and binding rules
- **Consumer Simulation**: Mock consumer behavior and acknowledgments
- **Fixture Support**: Pre-configured broker states and message flows

## Quick Start

### Basic AMQP Broker

```rust,no_run
use mockforge_amqp::AmqpBroker;
use mockforge_core::config::AmqpConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create AMQP configuration
    let config = AmqpConfig {
        host: "127.0.0.1".to_string(),
        port: 5672,
        ..Default::default()
    };

    // Create broker instance
    let spec_registry = Arc::new(AmqpSpecRegistry::new());
    let broker = AmqpBroker::new(config, spec_registry);

    // Start the broker
    broker.start().await?;

    Ok(())
}
```

## Core Components

### AmqpBroker

The main broker implementation that handles AMQP connections and protocol operations:

```rust,no_run
use mockforge_amqp::AmqpBroker;
use mockforge_core::config::AmqpConfig;

let config = AmqpConfig::default();
let spec_registry = Arc::new(AmqpSpecRegistry::new());
let broker = AmqpBroker::new(config, spec_registry);

// The broker handles:
// - Connection establishment and authentication
// - Channel management
// - Exchange and queue operations
// - Message publishing and consumption
```

### Exchange Management

Support for different AMQP exchange types:

```rust,no_run
use mockforge_amqp::exchanges::{ExchangeManager, ExchangeType};

// Create exchange manager
let mut exchange_manager = ExchangeManager::new();

// Declare exchanges
exchange_manager.declare_exchange(
    "direct_exchange",
    ExchangeType::Direct,
    true, // durable
    false, // auto_delete
    HashMap::new(), // arguments
)?;

// Bind queues to exchanges
exchange_manager.bind_queue(
    "my_queue",
    "direct_exchange",
    "routing_key",
)?;
```

### Queue Management

Dynamic queue creation and management:

```rust,no_run
use mockforge_amqp::queues::QueueManager;

// Create queue manager
let mut queue_manager = QueueManager::new();

// Declare a queue
queue_manager.declare_queue(
    "my_queue",
    true, // durable
    false, // exclusive
    false, // auto_delete
    HashMap::new(), // arguments
)?;

// Publish messages to queue
queue_manager.publish_message(
    "my_queue",
    "Hello, AMQP!".as_bytes(),
    HashMap::new(), // properties
)?;
```

## Message Handling

### Publishing Messages

```rust,no_run
use mockforge_amqp::messages::AmqpMessage;

// Create a message
let message = AmqpMessage {
    body: "Hello, World!".as_bytes().to_vec(),
    properties: HashMap::new(),
    exchange: "my_exchange".to_string(),
    routing_key: "my.routing.key".to_string(),
    mandatory: false,
    immediate: false,
};

// Publish through broker
broker.publish_message(message).await?;
```

### Consumer Simulation

```rust,no_run
use mockforge_amqp::consumers::ConsumerManager;

// Create consumer manager
let consumer_manager = ConsumerManager::new();

// Register a consumer
consumer_manager.register_consumer(
    "consumer_tag",
    "queue_name",
    Box::new(|message| {
        println!("Received: {:?}", message);
        // Process message
        Ok(())
    }),
)?;
```

## Fixture System

Define broker configurations and message flows using fixtures:

```yaml
# amqp-fixture.yaml
exchanges:
  - name: "orders_exchange"
    type: "topic"
    durable: true
    bindings:
      - queue: "order_processing"
        routing_key: "orders.#"

queues:
  - name: "order_processing"
    durable: true

messages:
  - exchange: "orders_exchange"
    routing_key: "orders.created"
    body: '{"order_id": "12345", "amount": 99.99}'
    properties:
      content_type: "application/json"
```

### Loading Fixtures

```rust,no_run
use mockforge_amqp::AmqpSpecRegistry;

// Create spec registry
let registry = AmqpSpecRegistry::new();

// Load fixture from file
registry.load_fixture_from_file("amqp-fixture.yaml").await?;

// Or create fixture programmatically
use mockforge_amqp::fixtures::AmqpFixture;

let fixture = AmqpFixture {
    exchanges: vec![/* ... */],
    queues: vec![/* ... */],
    messages: vec![/* ... */],
};

registry.add_fixture(fixture)?;
```

## Protocol Support

MockForge AMQP supports the full AMQP 0.9.1 protocol including:

- **Connection Operations**: Open, close, authentication
- **Channel Management**: Multiple channels per connection
- **Exchange Operations**: Declare, delete, bind, unbind
- **Queue Operations**: Declare, delete, bind, unbind, purge
- **Message Operations**: Publish, consume, acknowledge, reject
- **Transaction Support**: Basic transaction semantics
- **QoS Settings**: Prefetch and flow control

## Testing AMQP Clients

Use MockForge AMQP to test AMQP client applications:

```rust,no_run
use lapin::{Connection, ConnectionProperties};
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start AMQP broker in background
    let config = AmqpConfig {
        host: "127.0.0.1".to_string(),
        port: 5672,
        ..Default::default()
    };

    let spec_registry = Arc::new(AmqpSpecRegistry::new());
    let broker = AmqpBroker::new(config.clone(), spec_registry);
    task::spawn(async move {
        broker.start().await.unwrap();
    });

    // Give broker time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test AMQP client
    let addr = format!("amqp://{}:{}", config.host, config.port);
    let conn = Connection::connect(&addr, ConnectionProperties::default()).await?;

    let channel = conn.create_channel().await?;

    // Declare exchange and queue
    channel.exchange_declare(
        "test_exchange",
        lapin::ExchangeKind::Direct,
        lapin::options::ExchangeDeclareOptions::default(),
        lapin::types::FieldTable::default(),
    ).await?;

    channel.queue_declare(
        "test_queue",
        lapin::options::QueueDeclareOptions::default(),
        lapin::types::FieldTable::default(),
    ).await?;

    // Bind queue to exchange
    channel.queue_bind(
        "test_queue",
        "test_exchange",
        "test_key",
        lapin::options::QueueBindOptions::default(),
        lapin::types::FieldTable::default(),
    ).await?;

    // Publish a message
    channel.basic_publish(
        "test_exchange",
        "test_key",
        lapin::options::BasicPublishOptions::default(),
        b"Hello from MockForge!",
        lapin::types::BasicProperties::default(),
    ).await?;

    conn.close(0, "").await?;

    Ok(())
}
```

## Configuration

### AmqpConfig

```rust,no_run
use mockforge_core::config::AmqpConfig;

let config = AmqpConfig {
    host: "0.0.0.0".to_string(),        // Bind address
    port: 5672,                         // AMQP port
    max_connections: 1000,              // Connection limit
    heartbeat_interval: 60,             // Heartbeat interval in seconds
    ..Default::default()
};
```

## Performance

MockForge AMQP is optimized for testing scenarios:

- **In-Memory Operations**: Fast message routing without persistence
- **Concurrent Connections**: Handle multiple simultaneous AMQP clients
- **Low Latency**: Minimal overhead for message operations
- **Scalable**: Support for high-throughput testing scenarios

## Integration with MockForge

MockForge AMQP integrates seamlessly with the MockForge ecosystem:

- **MockForge Core**: Shared configuration and logging
- **MockForge CLI**: Command-line interface for AMQP broker management
- **MockForge Plugins**: Extend AMQP functionality with custom plugins

## Examples

See the [examples directory](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples) for complete working examples including:

- Basic AMQP broker setup
- Message routing scenarios
- Consumer testing patterns
- Integration testing with real AMQP clients

## Related Crates

- [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
- [`lapin`](https://docs.rs/lapin): Underlying AMQP client library

## License

Licensed under MIT OR Apache-2.0
