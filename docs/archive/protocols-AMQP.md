# AMQP Protocol Guide

This guide covers MockForge's AMQP 0-9-1 broker implementation for testing message queue applications.

## Overview

MockForge includes a lightweight AMQP broker compatible with RabbitMQ clients:
- AMQP 0-9-1 protocol support
- Exchange types: direct, fanout, topic, headers
- Durable and transient queues
- Message acknowledgments
- Publisher confirms
- Dead letter exchanges
- TTL and message expiration
- TLS encryption

## Quick Start

### Basic Configuration

```yaml
# mockforge.yaml
amqp:
  enabled: true
  port: 5672
  host: "0.0.0.0"
  default_vhost: "/"
  default_user: "guest"
  default_password: "guest"
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_AMQP_ENABLED` | `false` | Enable AMQP broker |
| `MOCKFORGE_AMQP_PORT` | `5672` | AMQP broker port |
| `MOCKFORGE_AMQP_HOST` | `0.0.0.0` | Bind address |
| `MOCKFORGE_AMQP_DEFAULT_VHOST` | `/` | Default virtual host |
| `MOCKFORGE_AMQP_DEFAULT_USER` | `guest` | Default username |
| `MOCKFORGE_AMQP_DEFAULT_PASSWORD` | `guest` | Default password |

### Starting the Broker

```bash
# Via CLI
mockforge serve --amqp

# With custom port
mockforge serve --amqp --amqp-port 5673

# With authentication
mockforge serve --amqp --amqp-user admin --amqp-password secret
```

## Exchange Types

### Direct Exchange

Routes messages to queues based on exact routing key match:

```yaml
amqp:
  exchanges:
    - name: "orders"
      type: direct
      durable: true
      bindings:
        - queue: "order-processing"
          routing_key: "new"
        - queue: "order-notifications"
          routing_key: "completed"
```

### Fanout Exchange

Broadcasts messages to all bound queues:

```yaml
amqp:
  exchanges:
    - name: "events"
      type: fanout
      bindings:
        - queue: "audit-log"
        - queue: "analytics"
        - queue: "notifications"
```

### Topic Exchange

Routes based on wildcard patterns:

```yaml
amqp:
  exchanges:
    - name: "logs"
      type: topic
      bindings:
        - queue: "all-errors"
          routing_key: "*.error"
        - queue: "critical-alerts"
          routing_key: "*.critical.*"
        - queue: "service-a-logs"
          routing_key: "service-a.#"
```

Pattern syntax:
- `*` matches exactly one word
- `#` matches zero or more words

### Headers Exchange

Routes based on message headers:

```yaml
amqp:
  exchanges:
    - name: "reports"
      type: headers
      bindings:
        - queue: "pdf-reports"
          match: all  # or 'any'
          headers:
            format: "pdf"
            priority: "high"
```

## Queue Configuration

### Durable Queues

Survive broker restarts:

```yaml
amqp:
  queues:
    - name: "important-jobs"
      durable: true
      auto_delete: false
```

### TTL and Expiration

```yaml
amqp:
  queues:
    - name: "temp-events"
      arguments:
        x-message-ttl: 60000      # Messages expire after 60s
        x-expires: 3600000        # Queue expires after 1 hour of no use
        x-max-length: 10000       # Maximum 10k messages
        x-max-length-bytes: 1048576  # Maximum 1MB total
```

### Dead Letter Exchange

```yaml
amqp:
  queues:
    - name: "orders"
      arguments:
        x-dead-letter-exchange: "dlx"
        x-dead-letter-routing-key: "orders.failed"

  exchanges:
    - name: "dlx"
      type: direct
      bindings:
        - queue: "failed-orders"
          routing_key: "orders.failed"
```

## Mocking Messages

### Pre-populated Messages

Seed queues with test messages:

```yaml
amqp:
  seed:
    - queue: "orders"
      messages:
        - payload: '{"order_id": "123", "status": "pending"}'
          properties:
            content_type: "application/json"
            delivery_mode: 2  # Persistent
        - payload: '{"order_id": "124", "status": "pending"}'
```

### Auto-Response

Automatically respond to messages:

```yaml
amqp:
  auto_response:
    - queue: "rpc-requests"
      response:
        exchange: ""
        routing_key: "{{properties.reply_to}}"
        payload: '{"success": true, "correlation_id": "{{properties.correlation_id}}"}'
        delay_ms: 50
```

### Message Templates

```yaml
amqp:
  templates:
    - exchange: "events"
      routing_key: "user.created"
      response:
        payload: |
          {
            "event_id": "{{uuid}}",
            "timestamp": "{{now}}",
            "user_id": "{{random_int 1000 9999}}",
            "email": "user{{random_int 1 100}}@example.com"
          }
```

## Testing Patterns

### Basic Publish/Consume

```rust
use lapin::{Connection, ConnectionProperties, options::*, types::FieldTable, BasicProperties};

#[tokio::test]
async fn test_publish_consume() {
    let conn = Connection::connect("amqp://guest:guest@localhost:5672", ConnectionProperties::default())
        .await
        .unwrap();

    let channel = conn.create_channel().await.unwrap();

    // Declare queue
    channel.queue_declare(
        "test-queue",
        QueueDeclareOptions::default(),
        FieldTable::default(),
    ).await.unwrap();

    // Publish message
    channel.basic_publish(
        "",
        "test-queue",
        BasicPublishOptions::default(),
        b"Hello, AMQP!",
        BasicProperties::default(),
    ).await.unwrap();

    // Consume message
    let mut consumer = channel.basic_consume(
        "test-queue",
        "test-consumer",
        BasicConsumeOptions::default(),
        FieldTable::default(),
    ).await.unwrap();

    let delivery = consumer.next().await.unwrap().unwrap();
    assert_eq!(delivery.data, b"Hello, AMQP!");
    delivery.ack(BasicAckOptions::default()).await.unwrap();
}
```

### RPC Pattern

```rust
#[tokio::test]
async fn test_rpc_pattern() {
    let conn = Connection::connect("amqp://localhost:5672", ConnectionProperties::default())
        .await.unwrap();

    let channel = conn.create_channel().await.unwrap();

    // Create reply queue
    let reply_queue = channel.queue_declare(
        "",  // Auto-generated name
        QueueDeclareOptions { exclusive: true, ..Default::default() },
        FieldTable::default(),
    ).await.unwrap();

    let correlation_id = uuid::Uuid::new_v4().to_string();

    // Send RPC request
    channel.basic_publish(
        "",
        "rpc-queue",
        BasicPublishOptions::default(),
        b"calculate",
        BasicProperties::default()
            .with_reply_to(reply_queue.name().clone())
            .with_correlation_id(correlation_id.clone().into()),
    ).await.unwrap();

    // Wait for reply
    let mut consumer = channel.basic_consume(
        reply_queue.name().as_str(),
        "",
        BasicConsumeOptions::default(),
        FieldTable::default(),
    ).await.unwrap();

    let delivery = consumer.next().await.unwrap().unwrap();
    assert_eq!(
        delivery.properties.correlation_id().as_ref().unwrap().as_str(),
        &correlation_id
    );
}
```

### Publisher Confirms

```rust
#[tokio::test]
async fn test_publisher_confirms() {
    let conn = Connection::connect("amqp://localhost:5672", ConnectionProperties::default())
        .await.unwrap();

    let channel = conn.create_channel().await.unwrap();

    // Enable publisher confirms
    channel.confirm_select(ConfirmSelectOptions::default()).await.unwrap();

    channel.queue_declare("confirm-queue", QueueDeclareOptions::default(), FieldTable::default())
        .await.unwrap();

    // Publish with confirmation
    let confirm = channel.basic_publish(
        "",
        "confirm-queue",
        BasicPublishOptions::default(),
        b"confirmed message",
        BasicProperties::default().with_delivery_mode(2),
    ).await.unwrap();

    // Wait for confirmation
    confirm.await.unwrap();
}
```

### Acknowledgment Modes

```rust
#[tokio::test]
async fn test_manual_ack() {
    // ... setup ...

    // Manual acknowledgment
    let delivery = consumer.next().await.unwrap().unwrap();

    // Process message...
    match process_message(&delivery.data) {
        Ok(_) => delivery.ack(BasicAckOptions::default()).await.unwrap(),
        Err(_) => {
            // Requeue for retry
            delivery.nack(BasicNackOptions { requeue: true, ..Default::default() }).await.unwrap();
        }
    }
}

#[tokio::test]
async fn test_auto_ack() {
    // Auto-acknowledge on receive
    let consumer = channel.basic_consume(
        "queue",
        "consumer",
        BasicConsumeOptions { no_ack: true, ..Default::default() },
        FieldTable::default(),
    ).await.unwrap();
}
```

### Dead Letter Queue Testing

```rust
#[tokio::test]
async fn test_dead_letter_queue() {
    let channel = conn.create_channel().await.unwrap();

    // Setup DLX
    channel.exchange_declare("dlx", ExchangeKind::Direct, ExchangeDeclareOptions::default(), FieldTable::default())
        .await.unwrap();

    channel.queue_declare("dlq", QueueDeclareOptions::default(), FieldTable::default())
        .await.unwrap();

    channel.queue_bind("dlq", "dlx", "rejected", QueueBindOptions::default(), FieldTable::default())
        .await.unwrap();

    // Main queue with DLX
    let mut args = FieldTable::default();
    args.insert("x-dead-letter-exchange".into(), AMQPValue::LongString("dlx".into()));
    args.insert("x-dead-letter-routing-key".into(), AMQPValue::LongString("rejected".into()));

    channel.queue_declare("main-queue", QueueDeclareOptions::default(), args)
        .await.unwrap();

    // Publish and reject
    channel.basic_publish("", "main-queue", BasicPublishOptions::default(), b"test", BasicProperties::default())
        .await.unwrap();

    let mut consumer = channel.basic_consume("main-queue", "", BasicConsumeOptions::default(), FieldTable::default())
        .await.unwrap();

    let delivery = consumer.next().await.unwrap().unwrap();
    delivery.reject(BasicRejectOptions { requeue: false }).await.unwrap();

    // Verify message in DLQ
    let mut dlq_consumer = channel.basic_consume("dlq", "", BasicConsumeOptions::default(), FieldTable::default())
        .await.unwrap();

    let dead_letter = dlq_consumer.next().await.unwrap().unwrap();
    assert_eq!(dead_letter.data, b"test");
}
```

## Chaos Testing

### Connection Failures

```yaml
amqp:
  chaos:
    enabled: true
    connection_drop_probability: 0.02  # 2% random disconnect
    channel_close_probability: 0.01     # 1% channel close
```

### Message Delays

```yaml
amqp:
  chaos:
    delivery_delay:
      min_ms: 10
      max_ms: 500
    publish_delay:
      min_ms: 5
      max_ms: 100
```

### Simulated Failures

```yaml
amqp:
  chaos:
    reject_probability: 0.05     # 5% message rejection
    nack_probability: 0.03       # 3% negative acknowledgment
```

## Metrics and Monitoring

### Available Metrics

```
# Connections
amqp_connections_total{status="active"} 15
amqp_channels_total{status="open"} 45

# Messages
amqp_messages_published_total{exchange="orders"} 5678
amqp_messages_delivered_total{queue="order-processing"} 5432
amqp_messages_acknowledged_total 5200
amqp_messages_rejected_total 32

# Queues
amqp_queue_messages{queue="orders"} 246
amqp_queue_consumers{queue="orders"} 3

# Exchanges
amqp_exchange_messages_in{exchange="events"} 10000
amqp_exchange_messages_routed{exchange="events"} 30000
```

### REST API

```bash
# List exchanges
curl http://localhost:3000/__mockforge/amqp/exchanges

# List queues
curl http://localhost:3000/__mockforge/amqp/queues

# Get queue stats
curl http://localhost:3000/__mockforge/amqp/queues/orders

# Publish message via HTTP
curl -X POST http://localhost:3000/__mockforge/amqp/publish \
  -H "Content-Type: application/json" \
  -d '{
    "exchange": "orders",
    "routing_key": "new",
    "payload": {"order_id": "test-123"},
    "properties": {
      "content_type": "application/json",
      "delivery_mode": 2
    }
  }'

# Purge queue
curl -X DELETE http://localhost:3000/__mockforge/amqp/queues/orders/messages
```

## Virtual Hosts

```yaml
amqp:
  vhosts:
    - name: "/production"
      users:
        - username: "prod-user"
          password: "prod-pass"
          permissions:
            configure: ".*"
            write: ".*"
            read: ".*"

    - name: "/staging"
      users:
        - username: "staging-user"
          password: "staging-pass"
```

## TLS Configuration

```yaml
amqp:
  tls:
    enabled: true
    port: 5671
    cert_path: "./certs/server.crt"
    key_path: "./certs/server.key"
    ca_path: "./certs/ca.crt"
    verify_peer: true
```

## Best Practices

1. **Use durable queues and persistent messages** for data that must survive restarts
2. **Implement dead letter queues** to handle failed messages
3. **Use publisher confirms** for critical messages
4. **Set appropriate prefetch counts** to balance throughput and fairness
5. **Use separate vhosts** for isolation between environments
6. **Monitor queue depths** to detect consumer bottlenecks
7. **Set message TTL** to prevent queue buildup

## Troubleshooting

### Connection Issues

```bash
# Test connectivity
telnet localhost 5672

# Check broker status
mockforge amqp status

# View active connections
curl http://localhost:3000/__mockforge/amqp/connections
```

### Message Not Delivered

1. Verify exchange exists and bindings are correct
2. Check routing key matches binding pattern
3. Ensure queue has active consumers (for non-auto-delete queues)
4. Check if message was dead-lettered

### Memory Issues

```yaml
amqp:
  limits:
    max_queues: 1000
    max_queue_size: 100000
    max_message_size: 16777216  # 16MB
```

## See Also

- [MQTT Protocol Guide](./MQTT.md)
- [WebSocket Protocol Guide](./WEBSOCKET.md)
- [Configuration Reference](../ENVIRONMENT_VARIABLES.md)
