# Protocol Crate Testing Guide

This guide provides patterns and examples for testing protocol crates (kafka, mqtt, amqp, ftp, tcp, smtp) in MockForge.

## Overview

Protocol crates require comprehensive testing to ensure:
- Connection establishment and teardown
- Message serialization/deserialization
- Error handling and recovery
- Integration with mockforge-core routing
- Protocol-specific edge cases

## Test Structure

### Standard Test Organization

```
crates/mockforge-{protocol}/
├── src/
│   └── lib.rs
└── tests/
    ├── integration.rs          # Main integration tests
    ├── connection_tests.rs     # Connection-related tests
    ├── message_tests.rs        # Message handling tests
    └── error_tests.rs          # Error handling tests
```

## Common Test Patterns

### 1. Connection Tests

Test connection establishment, maintenance, and teardown:

```rust
// tests/connection_tests.rs
use mockforge_kafka::KafkaServer;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_kafka_connection_establishment() {
    // Arrange: Start test server
    let server = KafkaServer::new("localhost:9092").await.unwrap();
    
    // Act: Connect client
    let client = server.connect().await.unwrap();
    
    // Assert: Connection is established
    assert!(client.is_connected());
}

#[tokio::test]
async fn test_kafka_connection_timeout() {
    // Arrange: Use invalid address
    let server = KafkaServer::new("invalid:9092");
    
    // Act: Attempt connection with timeout
    let result = timeout(Duration::from_secs(2), server.connect()).await;
    
    // Assert: Connection times out
    assert!(result.is_err() || result.unwrap().is_err());
}

#[tokio::test]
async fn test_kafka_connection_recovery() {
    // Arrange: Establish connection
    let server = KafkaServer::new("localhost:9092").await.unwrap();
    let mut client = server.connect().await.unwrap();
    
    // Act: Simulate connection loss
    server.disconnect().await;
    
    // Wait a bit
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Reconnect
    let result = client.reconnect().await;
    
    // Assert: Reconnection succeeds
    assert!(result.is_ok());
}
```

### 2. Message Tests

Test message send/receive, serialization, and routing:

```rust
// tests/message_tests.rs
use mockforge_kafka::{KafkaServer, KafkaMessage};
use mockforge_core::RouteRegistry;

#[tokio::test]
async fn test_kafka_message_send_receive() {
    // Arrange: Set up server and route
    let server = KafkaServer::new("localhost:9092").await.unwrap();
    let mut registry = RouteRegistry::new();
    
    registry.add_route(create_test_route("test-topic")).unwrap();
    
    // Act: Send message
    let message = KafkaMessage::new("test-topic", b"test payload");
    server.send(message).await.unwrap();
    
    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Assert: Message was received and routed
    let received = server.receive("test-topic").await.unwrap();
    assert_eq!(received.payload(), b"test payload");
}

#[tokio::test]
async fn test_kafka_message_serialization() {
    // Arrange: Create message with JSON payload
    let payload = serde_json::json!({"key": "value"});
    let message = KafkaMessage::new("test-topic", &payload.to_string());
    
    // Act: Serialize and deserialize
    let serialized = message.serialize().unwrap();
    let deserialized = KafkaMessage::deserialize(&serialized).unwrap();
    
    // Assert: Data is preserved
    assert_eq!(message.topic(), deserialized.topic());
    assert_eq!(message.payload(), deserialized.payload());
}

#[tokio::test]
async fn test_kafka_large_message() {
    // Arrange: Create large message (1MB)
    let large_payload = vec![0u8; 1024 * 1024];
    let message = KafkaMessage::new("test-topic", &large_payload);
    
    // Act: Send large message
    let server = KafkaServer::new("localhost:9092").await.unwrap();
    let result = server.send(message).await;
    
    // Assert: Large message is handled
    assert!(result.is_ok());
}
```

### 3. Error Handling Tests

Test error conditions and recovery:

```rust
// tests/error_tests.rs
use mockforge_kafka::{KafkaServer, KafkaError};

#[tokio::test]
async fn test_kafka_invalid_topic() {
    // Arrange: Server without topic
    let server = KafkaServer::new("localhost:9092").await.unwrap();
    
    // Act: Send to non-existent topic
    let message = KafkaMessage::new("non-existent", b"payload");
    let result = server.send(message).await;
    
    // Assert: Error is returned
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), KafkaError::TopicNotFound));
}

#[tokio::test]
async fn test_kafka_network_failure() {
    // Arrange: Start server then kill it
    let server = KafkaServer::new("localhost:9092").await.unwrap();
    let client = server.connect().await.unwrap();
    
    // Act: Kill server
    server.shutdown().await;
    
    // Try to send message
    let message = KafkaMessage::new("test-topic", b"payload");
    let result = client.send(message).await;
    
    // Assert: Network error is detected
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), KafkaError::NetworkError));
}

#[tokio::test]
async fn test_kafka_protocol_error() {
    // Arrange: Server with invalid protocol version
    let server = KafkaServer::new("localhost:9092")
        .with_protocol_version(999)  // Invalid version
        .await
        .unwrap();
    
    // Act: Attempt connection
    let result = server.connect().await;
    
    // Assert: Protocol error is returned
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), KafkaError::ProtocolError));
}
```

### 4. Integration Tests

Test integration with mockforge-core:

```rust
// tests/integration.rs
use mockforge_kafka::KafkaServer;
use mockforge_core::{RouteRegistry, Route, Protocol};

#[tokio::test]
async fn test_kafka_routing_integration() {
    // Arrange: Set up route registry and server
    let mut registry = RouteRegistry::new();
    let route = Route::new(
        Protocol::Kafka,
        "test-topic".to_string(),
        create_test_response(),
    );
    registry.add_route(route).unwrap();
    
    let server = KafkaServer::new("localhost:9092")
        .with_registry(registry)
        .await
        .unwrap();
    
    // Act: Send message
    let message = KafkaMessage::new("test-topic", b"test");
    let response = server.send_and_wait(message).await.unwrap();
    
    // Assert: Message was routed correctly
    assert_eq!(response.status(), 200);
    assert_eq!(response.body(), b"test response");
}

#[tokio::test]
async fn test_kafka_cross_protocol_bridge() {
    // Arrange: Set up HTTP to Kafka bridge
    let mut registry = RouteRegistry::new();
    let route = Route::new(
        Protocol::Http,
        "/kafka/test-topic".to_string(),
        create_kafka_bridge_response(),
    );
    registry.add_route(route).unwrap();
    
    // Act: Send HTTP request that bridges to Kafka
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3000/kafka/test-topic")
        .json(&serde_json::json!({"message": "test"}))
        .send()
        .await
        .unwrap();
    
    // Assert: HTTP request was bridged to Kafka
    assert_eq!(response.status(), 200);
}
```

## Protocol-Specific Patterns

### Kafka Testing

```rust
// Kafka-specific test patterns
#[tokio::test]
async fn test_kafka_consumer_groups() {
    // Test consumer group coordination
}

#[tokio::test]
async fn test_kafka_partitioning() {
    // Test message partitioning
}

#[tokio::test]
async fn test_kafka_offset_management() {
    // Test offset tracking and recovery
}
```

### MQTT Testing

```rust
// MQTT-specific test patterns
#[tokio::test]
async fn test_mqtt_qos_levels() {
    // Test QoS 0, 1, 2
}

#[tokio::test]
async fn test_mqtt_will_message() {
    // Test last will and testament
}

#[tokio::test]
async fn test_mqtt_retain_flag() {
    // Test retained messages
}
```

### AMQP Testing

```rust
// AMQP-specific test patterns
#[tokio::test]
async fn test_amqp_exchanges() {
    // Test exchange types (direct, topic, fanout)
}

#[tokio::test]
async fn test_amqp_queues() {
    // Test queue declaration and binding
}

#[tokio::test]
async fn test_amqp_acknowledgments() {
    // Test message acknowledgments
}
```

### FTP Testing

```rust
// FTP-specific test patterns
#[tokio::test]
async fn test_ftp_passive_mode() {
    // Test passive mode connections
}

#[tokio::test]
async fn test_ftp_file_operations() {
    // Test file upload/download
}

#[tokio::test]
async fn test_ftp_directory_operations() {
    // Test directory listing and navigation
}
```

### TCP Testing

```rust
// TCP-specific test patterns
#[tokio::test]
async fn test_tcp_connection_pooling() {
    // Test connection pool management
}

#[tokio::test]
async fn test_tcp_keepalive() {
    // Test keepalive mechanisms
}

#[tokio::test]
async fn test_tcp_binary_protocols() {
    // Test binary protocol handling
}
```

### SMTP Testing

```rust
// SMTP-specific test patterns
#[tokio::test]
async fn test_smtp_authentication() {
    // Test SMTP auth (PLAIN, LOGIN, CRAM-MD5)
}

#[tokio::test]
async fn test_smtp_multipart_messages() {
    // Test multipart email handling
}

#[tokio::test]
async fn test_smtp_attachments() {
    // Test email attachments
}
```

## Test Utilities

### Mock Server Helper

```rust
// tests/common/mod.rs
pub struct MockKafkaServer {
    addr: String,
    handle: Option<JoinHandle<()>>,
}

impl MockKafkaServer {
    pub async fn new() -> Self {
        // Start embedded Kafka server
        // Return configured instance
    }
    
    pub fn addr(&self) -> &str {
        &self.addr
    }
}

impl Drop for MockKafkaServer {
    fn drop(&mut self) {
        // Clean up server
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}
```

### Test Data Generators

```rust
// tests/common/mod.rs
pub fn create_test_message(topic: &str) -> KafkaMessage {
    KafkaMessage::new(topic, b"test payload")
}

pub fn create_test_route(topic: &str) -> Route {
    Route::new(
        Protocol::Kafka,
        topic.to_string(),
        create_test_response(),
    )
}

pub fn create_test_response() -> Response {
    Response::new(200, b"test response".to_vec())
}
```

## Coverage Goals

Each protocol crate should aim for:

- **Minimum**: 75% line coverage
- **Target**: 80% line coverage
- **Ideal**: 85%+ line coverage

### Coverage Priorities

1. **Connection handling**: 90%+ coverage
2. **Message processing**: 85%+ coverage
3. **Error handling**: 90%+ coverage
4. **Integration**: 80%+ coverage
5. **Edge cases**: 70%+ coverage

## Running Protocol Tests

```bash
# Run all protocol crate tests
cargo test --package mockforge-kafka
cargo test --package mockforge-mqtt
cargo test --package mockforge-amqp
cargo test --package mockforge-ftp
cargo test --package mockforge-tcp
cargo test --package mockforge-smtp

# Run with coverage
cargo llvm-cov --package mockforge-kafka --all-features

# Run specific test
cargo test --package mockforge-kafka test_kafka_connection_establishment
```

## Best Practices

1. **Use Embedded Servers**: Prefer embedded/test servers over external dependencies
2. **Test Isolation**: Each test should be independent
3. **Cleanup**: Always clean up resources in test teardown
4. **Timeouts**: Use timeouts for network operations
5. **Error Cases**: Test all error conditions
6. **Edge Cases**: Test boundary conditions (empty messages, large messages, etc.)

## References

- [Testing Standards](TESTING_STANDARDS.md) - General testing guidelines
- [Coverage Maintenance](COVERAGE_MAINTENANCE.md) - Coverage improvement process

