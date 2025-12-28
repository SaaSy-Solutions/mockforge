# MQTT Protocol Guide

This guide covers MockForge's MQTT broker implementation for testing IoT and messaging applications.

## Overview

MockForge includes a full-featured MQTT broker that supports:
- MQTT 3.1.1 and 5.0 protocols
- QoS levels 0, 1, and 2
- TLS/mTLS encryption
- Retained messages
- Last Will and Testament (LWT)
- Session persistence
- Topic wildcards (`+` and `#`)

## Quick Start

### Basic Configuration

```yaml
# mockforge.yaml
mqtt:
  enabled: true
  port: 1883
  host: "0.0.0.0"
  max_connections: 1000
  max_packet_size: 65536
  keep_alive_secs: 60
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_MQTT_ENABLED` | `false` | Enable MQTT broker |
| `MOCKFORGE_MQTT_PORT` | `1883` | MQTT broker port |
| `MOCKFORGE_MQTT_HOST` | `0.0.0.0` | Bind address |
| `MOCKFORGE_MQTT_MAX_CONNECTIONS` | `1000` | Maximum concurrent connections |
| `MOCKFORGE_MQTT_MAX_PACKET_SIZE` | `65536` | Maximum packet size in bytes |
| `MOCKFORGE_MQTT_KEEP_ALIVE_SECS` | `60` | Keep-alive timeout |

### Starting the Broker

```bash
# Via CLI
mockforge serve --mqtt

# With custom port
mockforge serve --mqtt --mqtt-port 1884

# With TLS enabled
mockforge serve --mqtt --mqtt-tls
```

## TLS Configuration

### Self-Signed Certificates (Development)

```bash
# Generate CA
openssl genrsa -out ca.key 4096
openssl req -new -x509 -days 365 -key ca.key -out ca.crt -subj "/CN=MockForge CA"

# Generate server certificate
openssl genrsa -out server.key 2048
openssl req -new -key server.key -out server.csr -subj "/CN=localhost"
openssl x509 -req -days 365 -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt
```

### TLS Configuration

```yaml
mqtt:
  enabled: true
  port: 1883
  tls:
    enabled: true
    port: 8883
    cert_path: "./certs/server.crt"
    key_path: "./certs/server.key"
    ca_path: "./certs/ca.crt"  # For client certificate verification
    client_auth: false          # Set to true for mTLS
```

### Environment Variables (TLS)

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_MQTT_TLS_ENABLED` | `false` | Enable TLS |
| `MOCKFORGE_MQTT_TLS_PORT` | `8883` | TLS port |
| `MOCKFORGE_MQTT_TLS_CERT_PATH` | - | Server certificate path |
| `MOCKFORGE_MQTT_TLS_KEY_PATH` | - | Server key path |
| `MOCKFORGE_MQTT_TLS_CA_PATH` | - | CA certificate for client auth |
| `MOCKFORGE_MQTT_TLS_CLIENT_AUTH` | `false` | Require client certificates |

## Mocking MQTT Messages

### Pre-configured Subscriptions

Define mock responses for specific topic patterns:

```yaml
mqtt:
  enabled: true
  mocks:
    - topic: "sensors/+/temperature"
      qos: 1
      responses:
        - payload: '{"value": 23.5, "unit": "celsius"}'
          delay_ms: 100
        - payload: '{"value": 24.1, "unit": "celsius"}'
          delay_ms: 100

    - topic: "devices/#"
      qos: 0
      response:
        payload: '{"status": "acknowledged"}'
```

### Dynamic Response Templates

Use Handlebars templates for dynamic responses:

```yaml
mqtt:
  mocks:
    - topic: "sensors/{{device_id}}/data"
      response:
        payload: |
          {
            "device_id": "{{topic_parts.1}}",
            "timestamp": "{{now}}",
            "value": {{random_float 0 100}}
          }
```

### Recording and Replay

Record MQTT traffic for later replay:

```bash
# Start recording
mockforge mqtt record --output mqtt-session.json

# Replay recorded session
mockforge mqtt replay --input mqtt-session.json --speed 2.0
```

## Testing Patterns

### Connection Testing

```rust
use rumqttc::{Client, MqttOptions, QoS};

#[tokio::test]
async fn test_mqtt_connection() {
    let mut mqttoptions = MqttOptions::new("test-client", "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = Client::new(mqttoptions, 10);

    // Connection should succeed
    let event = eventloop.poll().await.unwrap();
    assert!(matches!(event, Event::Incoming(Packet::ConnAck(_))));
}
```

### Publish/Subscribe Testing

```rust
#[tokio::test]
async fn test_publish_subscribe() {
    let (client, mut eventloop) = create_client();

    // Subscribe
    client.subscribe("test/topic", QoS::AtLeastOnce).await.unwrap();

    // Publish
    client.publish("test/topic", QoS::AtLeastOnce, false, "hello").await.unwrap();

    // Verify message received
    loop {
        match eventloop.poll().await.unwrap() {
            Event::Incoming(Packet::Publish(p)) => {
                assert_eq!(p.topic, "test/topic");
                assert_eq!(p.payload, Bytes::from("hello"));
                break;
            }
            _ => continue,
        }
    }
}
```

### QoS Testing

```rust
#[tokio::test]
async fn test_qos2_exactly_once() {
    let (client, mut eventloop) = create_client();

    client.subscribe("qos2/test", QoS::ExactlyOnce).await.unwrap();
    client.publish("qos2/test", QoS::ExactlyOnce, false, "important").await.unwrap();

    // Verify full QoS 2 handshake
    let mut received_publish = false;
    let mut received_pubcomp = false;

    while !received_publish || !received_pubcomp {
        match eventloop.poll().await.unwrap() {
            Event::Incoming(Packet::Publish(_)) => received_publish = true,
            Event::Incoming(Packet::PubComp(_)) => received_pubcomp = true,
            _ => {}
        }
    }

    assert!(received_publish && received_pubcomp);
}
```

### Retained Message Testing

```rust
#[tokio::test]
async fn test_retained_messages() {
    let (client1, mut eventloop1) = create_client();

    // Publish retained message
    client1.publish("retained/topic", QoS::AtLeastOnce, true, "retained-value").await.unwrap();

    // New client should receive retained message on subscribe
    let (client2, mut eventloop2) = create_client();
    client2.subscribe("retained/topic", QoS::AtLeastOnce).await.unwrap();

    loop {
        match eventloop2.poll().await.unwrap() {
            Event::Incoming(Packet::Publish(p)) => {
                assert!(p.retain);
                assert_eq!(p.payload, Bytes::from("retained-value"));
                break;
            }
            _ => continue,
        }
    }
}
```

### Last Will Testing

```rust
#[tokio::test]
async fn test_last_will() {
    let mut options = MqttOptions::new("will-client", "localhost", 1883);
    options.set_last_will(LastWill::new(
        "clients/status",
        "offline",
        QoS::AtLeastOnce,
        false,
    ));

    let (client, mut eventloop) = Client::new(options, 10);

    // Subscribe to will topic from another client
    let (observer, mut obs_eventloop) = create_client();
    observer.subscribe("clients/status", QoS::AtLeastOnce).await.unwrap();

    // Abruptly disconnect will-client (simulating crash)
    drop(client);
    drop(eventloop);

    // Observer should receive LWT message
    // ...
}
```

## Chaos Testing

### Connection Disruption

```yaml
mqtt:
  chaos:
    enabled: true
    disconnect_probability: 0.05  # 5% chance of random disconnect
    packet_loss: 0.01             # 1% packet loss
    latency:
      min_ms: 10
      max_ms: 100
```

### Slow Broker Simulation

```yaml
mqtt:
  chaos:
    slow_subscriber:
      enabled: true
      delay_ms: 5000          # Simulate slow consumer
      affected_topics:
        - "slow/#"
```

## Metrics and Monitoring

### Available Metrics

MockForge exposes MQTT metrics at `/__mockforge/metrics`:

```
# Connections
mqtt_connections_total{status="active"} 42
mqtt_connections_total{status="closed"} 156

# Messages
mqtt_messages_published_total{qos="0"} 1234
mqtt_messages_published_total{qos="1"} 567
mqtt_messages_published_total{qos="2"} 89

# Subscriptions
mqtt_subscriptions_active 128
mqtt_subscriptions_total 256

# Bytes
mqtt_bytes_received_total 1048576
mqtt_bytes_sent_total 2097152
```

### Programmatic Access

```rust
use mockforge::mqtt::metrics;

let stats = metrics::get_broker_stats().await;
println!("Active connections: {}", stats.active_connections);
println!("Messages/sec: {}", stats.messages_per_second);
```

## Integration with HTTP

### Webhook on Message

Trigger HTTP webhooks when MQTT messages are received:

```yaml
mqtt:
  webhooks:
    - topic: "orders/+/created"
      url: "http://localhost:8080/webhook/order"
      method: POST
      headers:
        X-Source: "mqtt"
```

### REST API for MQTT

Publish messages via HTTP API:

```bash
# Publish a message
curl -X POST http://localhost:3000/__mockforge/mqtt/publish \
  -H "Content-Type: application/json" \
  -d '{
    "topic": "sensors/device1/temp",
    "payload": {"value": 25.5},
    "qos": 1,
    "retain": false
  }'

# Get subscriptions
curl http://localhost:3000/__mockforge/mqtt/subscriptions

# Get broker stats
curl http://localhost:3000/__mockforge/mqtt/stats
```

## Best Practices

1. **Use appropriate QoS levels**
   - QoS 0 for telemetry that can be lost
   - QoS 1 for important but idempotent messages
   - QoS 2 for critical exactly-once delivery

2. **Set reasonable keep-alive**
   - Too short: excessive ping traffic
   - Too long: slow failure detection

3. **Use topic wildcards carefully**
   - `+` matches single level: `sensors/+/temp` matches `sensors/device1/temp`
   - `#` matches multiple levels: `sensors/#` matches all under `sensors/`

4. **Test edge cases**
   - Maximum message size
   - Many concurrent connections
   - Rapid connect/disconnect cycles
   - Network partition scenarios

## Troubleshooting

### Connection Refused

```bash
# Check if broker is running
mockforge mqtt status

# Check port availability
netstat -an | grep 1883
```

### TLS Handshake Failures

```bash
# Test TLS connection
openssl s_client -connect localhost:8883 -CAfile ca.crt

# Verify certificate
openssl x509 -in server.crt -text -noout
```

### Message Not Received

1. Verify subscription topic matches publish topic
2. Check QoS levels are compatible
3. Ensure client is connected when message is published (unless retained)
4. Check for topic ACL restrictions

## See Also

- [AMQP Protocol Guide](./AMQP.md)
- [WebSocket Protocol Guide](./WEBSOCKET.md)
- [Configuration Reference](../ENVIRONMENT_VARIABLES.md)
