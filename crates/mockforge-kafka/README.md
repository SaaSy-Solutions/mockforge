# MockForge Kafka

Kafka protocol support for MockForge with full broker simulation, topic management, and consumer group coordination.

This crate provides comprehensive Kafka mocking capabilities, allowing you to simulate Apache Kafka brokers for testing event-driven applications. Perfect for testing Kafka producers, consumers, and stream processing applications without requiring a full Kafka cluster.

## Features

- **Full Kafka Protocol**: Support for 10+ Kafka APIs (Produce, Fetch, Metadata, etc.)
- **Broker Simulation**: Complete Kafka broker implementation without external dependencies
- **Topic Management**: Dynamic topic creation, deletion, and configuration
- **Partition Handling**: Multi-partition topics with proper offset management
- **Consumer Groups**: Simulate consumer group coordination and rebalancing
- **Message Fixtures**: YAML-based message templates and auto-production
- **Metrics & Monitoring**: Comprehensive metrics with Prometheus integration
- **Protocol Compliance**: Full Kafka protocol v2.8+ compatibility

## Quick Start

### Basic Kafka Broker

```rust,no_run
use mockforge_kafka::KafkaMockBroker;
use mockforge_core::config::KafkaConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create broker configuration
    let config = KafkaConfig {
        host: "127.0.0.1".to_string(),
        port: 9092,
        ..Default::default()
    };

    // Initialize and start broker
    let broker = KafkaMockBroker::new(config).await?;
    broker.start().await?;

    Ok(())
}
```

### Testing with Kafka Clients

```rust,no_run
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to MockForge Kafka broker
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .set("message.timeout.ms", "5000")
        .create()?;

    // Produce a message
    let delivery_status = producer
        .send(
            FutureRecord::to("test-topic")
                .payload("Hello from MockForge!")
                .key("test-key"),
            Duration::from_secs(0),
        )
        .await;

    match delivery_status {
        Ok((partition, offset)) => {
            println!("Message delivered to partition {} at offset {}", partition, offset);
        }
        Err((e, _)) => println!("Failed to deliver message: {}", e),
    }

    Ok(())
}
```

## Core Components

### KafkaMockBroker

The main broker implementation that handles all Kafka protocol operations:

```rust,no_run
use mockforge_kafka::KafkaMockBroker;
use mockforge_core::config::KafkaConfig;

let config = KafkaConfig {
    host: "0.0.0.0".to_string(),
    port: 9092,
    auto_create_topics: true,
    default_partitions: 3,
    ..Default::default()
};

let broker = KafkaMockBroker::new(config).await?;
broker.start().await?;
```

### Topic Management

Create and manage Kafka topics dynamically:

```rust,no_run
use mockforge_kafka::topics::{Topic, TopicConfig};

// Create a topic with specific configuration
let topic_config = TopicConfig {
    name: "user-events".to_string(),
    partitions: 3,
    replication_factor: 1,
    retention_ms: Some(604800000), // 7 days
};

let topic = Topic::new(topic_config);

// Topics are automatically created when first accessed
// or can be pre-created through the broker API
```

### Message Production

Handle produce requests with full protocol compliance:

```rust,no_run
use mockforge_kafka::partitions::KafkaMessage;

// Create messages for production
let messages = vec![
    KafkaMessage {
        key: Some(b"user-123".to_vec()),
        value: b"{\"action\": \"login\", \"user_id\": 123}".to_vec(),
        timestamp: Some(chrono::Utc::now().timestamp_millis()),
        headers: None,
    },
    KafkaMessage {
        key: Some(b"user-456".to_vec()),
        value: b"{\"action\": \"logout\", \"user_id\": 456}".to_vec(),
        timestamp: Some(chrono::Utc::now().timestamp_millis()),
        headers: None,
    },
];

// Messages are automatically routed to appropriate partitions
// based on key hashing (if key provided) or round-robin
```

### Consumer Groups

Simulate consumer group behavior and coordination:

```rust,no_run
use mockforge_kafka::consumer_groups::{ConsumerGroup, ConsumerGroupManager};

// Create consumer group manager
let group_manager = ConsumerGroupManager::new();

// Consumer groups are automatically managed when consumers join
// Partition assignment follows Kafka's standard algorithms
let group = ConsumerGroup::new(
    "my-consumer-group".to_string(),
    vec!["consumer-1".to_string(), "consumer-2".to_string()],
);

// Group handles partition rebalancing when members join/leave
```

## Fixture System

Define message templates and auto-production rules using YAML:

```yaml
# kafka-fixture.yaml
topics:
  - name: "user-events"
    partitions: 3
    config:
      retention.ms: "604800000"  # 7 days

  - name: "order-events"
    partitions: 2

fixtures:
  - topic: "user-events"
    key_template: "{{uuid}}"
    value_template: |
      {
        "user_id": "{{uuid}}",
        "action": "{{random_element 'login' 'logout' 'signup' 'update_profile'}}",
        "timestamp": "{{now}}",
        "metadata": {
          "source": "web",
          "version": "1.0"
        }
      }
    headers:
      content-type: "application/json"

auto_produce:
  - topic: "user-events"
    rate_per_second: 5
    duration_seconds: 300  # 5 minutes
    key_template: "{{uuid}}"
    value_template: |
      {
        "event_type": "heartbeat",
        "service": "user-service",
        "timestamp": "{{now}}"
      }

  - topic: "order-events"
    rate_per_second: 2
    duration_seconds: 600  # 10 minutes
    key_template: "order-{{sequence}}"
    value_template: |
      {
        "order_id": "{{sequence}}",
        "user_id": "{{uuid}}",
        "amount": {{float_range 10.0 1000.0}},
        "items": {{int_range 1 10}},
        "status": "created",
        "created_at": "{{now}}"
      }
```

### Loading Fixtures

```rust,no_run
use mockforge_kafka::{KafkaMockBroker, KafkaSpecRegistry};

// Create broker with fixture support
let spec_registry = KafkaSpecRegistry::new();
let broker = KafkaMockBroker::with_registry(config, spec_registry).await?;

// Load fixtures from file
broker.load_fixtures_from_file("kafka-fixture.yaml").await?;

// Or create fixtures programmatically
use mockforge_kafka::fixtures::{KafkaFixture, AutoProduceConfig};

let fixture = KafkaFixture {
    topics: vec![/* ... */],
    fixtures: vec![/* ... */],
    auto_produce: vec![/* ... */],
};

broker.add_fixture(fixture).await?;
```

## Supported Kafka APIs

MockForge Kafka implements the following Kafka protocol APIs:

- **Produce (API 0)**: Message production with acknowledgments
- **Fetch (API 1)**: Message consumption with offset management
- **Metadata (API 3)**: Topic and broker metadata discovery
- **ListGroups (API 9)**: Consumer group listing
- **DescribeGroups (API 15)**: Consumer group details and member information
- **ApiVersions (API 18)**: Protocol version negotiation
- **CreateTopics (API 19)**: Dynamic topic creation
- **DeleteTopics (API 20)**: Topic deletion
- **DescribeConfigs (API 32)**: Configuration retrieval

## Metrics & Monitoring

### Prometheus Metrics

Comprehensive metrics exported in Prometheus format:

```rust,no_run
use mockforge_kafka::metrics::MetricsExporter;

// Create metrics exporter
let exporter = MetricsExporter::new();

// Export current metrics
let metrics = exporter.export_prometheus().await?;
println!("{}", metrics);

// Sample metrics:
// kafka_requests_total{api="produce"} 150
// kafka_messages_produced_total{topic="user-events"} 1000
// kafka_consumer_groups_total 5
// kafka_connections_active 12
```

### Metrics Categories

- **Request Metrics**: Total requests, errors, latency by API
- **Message Metrics**: Messages produced/consumed by topic
- **Connection Metrics**: Active connections, connection rate
- **Consumer Group Metrics**: Group count, partition assignments
- **Topic Metrics**: Topic count, partition count, message count

## Configuration

### KafkaConfig

```rust,no_run
use mockforge_core::config::KafkaConfig;

let config = KafkaConfig {
    host: "0.0.0.0".to_string(),
    port: 9092,
    auto_create_topics: true,
    default_partitions: 3,
    default_replication_factor: 1,
    log_retention_hours: 168, // 7 days
    max_message_size: 1048576, // 1MB
    num_threads: 4,
    ..Default::default()
};
```

### Environment Variables

```bash
# Server configuration
export KAFKA_HOST=0.0.0.0
export KAFKA_PORT=9092

# Topic defaults
export KAFKA_AUTO_CREATE_TOPICS=true
export KAFKA_DEFAULT_PARTITIONS=3

# Performance
export KAFKA_MAX_MESSAGE_SIZE=1048576
export KAFKA_NUM_THREADS=4
```

## Testing Examples

### Producer Testing

```rust,no_run
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;

#[tokio::test]
async fn test_kafka_producer() {
    // Start MockForge Kafka broker in background
    let broker = KafkaMockBroker::new(KafkaConfig::default()).await.unwrap();
    tokio::spawn(async move { broker.start().await.unwrap() });

    // Give broker time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test producer
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .set("message.timeout.ms", "5000")
        .create()
        .unwrap();

    // Send test message
    let result = producer
        .send(
            FutureRecord::to("test-topic")
                .payload("test message")
                .key("test-key"),
            Duration::from_secs(5),
        )
        .await;

    assert!(result.is_ok());
    let (partition, offset) = result.unwrap();
    assert!(partition >= 0);
    assert!(offset >= 0);
}
```

### Consumer Testing

```rust,no_run
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::Message;
use futures::StreamExt;

#[tokio::test]
async fn test_kafka_consumer() {
    // Start broker and produce test messages
    // ... setup code ...

    // Create consumer
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .set("group.id", "test-group")
        .set("auto.offset.reset", "earliest")
        .create()
        .unwrap();

    consumer.subscribe(&["test-topic"]).unwrap();

    // Consume messages
    let mut message_stream = consumer.stream();
    let message = message_stream.next().await.unwrap().unwrap();

    let payload = message.payload().unwrap();
    assert_eq!(std::str::from_utf8(payload).unwrap(), "test message");
}
```

### Consumer Group Testing

```rust,no_run
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};

#[tokio::test]
async fn test_consumer_groups() {
    // Start broker
    // ... setup code ...

    // Create multiple consumers in same group
    let mut consumers = vec![];

    for i in 0..3 {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", "localhost:9092")
            .set("group.id", "test-group")
            .set("client.id", &format!("consumer-{}", i))
            .create()
            .unwrap();

        consumer.subscribe(&["test-topic"]).unwrap();
        consumers.push(consumer);
    }

    // Verify partition assignment
    // Consumers should automatically balance partitions
    for consumer in consumers {
        let assignment = consumer.assignment().unwrap();
        assert!(!assignment.is_empty());
    }
}
```

## Performance

MockForge Kafka is optimized for testing scenarios:

- **In-Memory Storage**: Fast message operations without disk persistence
- **Concurrent Connections**: Handle multiple simultaneous Kafka clients
- **Low Latency**: Minimal overhead for message operations
- **Scalable**: Support for high-throughput testing scenarios
- **Resource Efficient**: Configurable memory limits and cleanup

## Integration with MockForge

MockForge Kafka integrates seamlessly with the MockForge ecosystem:

- **MockForge Core**: Shared configuration and logging
- **MockForge CLI**: Command-line Kafka broker management
- **MockForge Data**: Enhanced message generation with templates
- **MockForge Observability**: Metrics and tracing integration

## Troubleshooting

### Common Issues

**Connection refused:**
- Ensure broker is started and listening on correct port
- Check firewall settings and port availability
- Verify client configuration (bootstrap servers)

**Messages not consumed:**
- Check consumer group configuration
- Verify topic exists (auto-create may be disabled)
- Check offset reset policy (earliest/latest)

**High latency:**
- Adjust broker thread count for better concurrency
- Check system resources (CPU, memory)
- Review message size and batch settings

**Protocol errors:**
- Ensure client and broker use compatible Kafka versions
- Check message format and serialization
- Verify topic and partition configurations

## Examples

See the [examples directory](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples) for complete working examples including:

- Basic Kafka broker setup
- Producer/consumer testing patterns
- Consumer group coordination
- Fixture-driven message generation
- Load testing scenarios

## Related Crates

- [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
- [`rdkafka`](https://docs.rs/rdkafka): Kafka client library for testing

## License

Licensed under MIT OR Apache-2.0
