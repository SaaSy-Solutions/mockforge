# WebSocket Protocol Guide

This guide covers MockForge's WebSocket server implementation for testing real-time applications.

## Overview

MockForge provides full WebSocket support:
- RFC 6455 compliant WebSocket server
- Text and binary message support
- Ping/pong heartbeat handling
- Per-connection state management
- Message recording and replay
- Proxy mode for upstream forwarding
- Hot-reload of message handlers

## Quick Start

### Basic Configuration

```yaml
# mockforge.yaml
websocket:
  enabled: true
  port: 8080
  path: "/ws"
  max_connections: 10000
  max_message_size: 65536
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_WS_ENABLED` | `false` | Enable WebSocket server |
| `MOCKFORGE_WS_PORT` | `8080` | WebSocket server port |
| `MOCKFORGE_WS_PATH` | `/ws` | WebSocket endpoint path |
| `MOCKFORGE_WS_MAX_CONNECTIONS` | `10000` | Maximum concurrent connections |
| `MOCKFORGE_WS_MAX_MESSAGE_SIZE` | `65536` | Maximum message size in bytes |
| `MOCKFORGE_WS_PING_INTERVAL` | `30` | Ping interval in seconds |
| `MOCKFORGE_WS_HOTRELOAD` | `false` | Enable hot reload |

### Starting the Server

```bash
# Via CLI
mockforge serve --websocket

# With custom path
mockforge serve --websocket --ws-path /api/realtime

# With TLS
mockforge serve --websocket --ws-tls
```

## Message Handlers

### Echo Handler

Simple echo server:

```yaml
websocket:
  handlers:
    - path: "/echo"
      type: echo
```

### Static Response

Return fixed responses:

```yaml
websocket:
  handlers:
    - path: "/status"
      type: static
      messages:
        - on_connect: '{"type": "connected", "server": "mockforge"}'
        - on_message: '{"type": "ack", "received": true}'
        - on_close: '{"type": "goodbye"}'
```

### Pattern Matching

Route based on message content:

```yaml
websocket:
  handlers:
    - path: "/api"
      type: pattern
      rules:
        - match:
            type: "ping"
          response:
            type: "pong"
            timestamp: "{{now}}"

        - match:
            type: "subscribe"
            channel: "*"
          response:
            type: "subscribed"
            channel: "{{message.channel}}"
            id: "{{uuid}}"

        - match:
            type: "message"
          response:
            type: "ack"
            id: "{{message.id}}"
            processed: true
```

### JSON-RPC Handler

Support for JSON-RPC 2.0:

```yaml
websocket:
  handlers:
    - path: "/jsonrpc"
      type: jsonrpc
      methods:
        - name: "user.get"
          response:
            result:
              id: "{{params.id}}"
              name: "John Doe"
              email: "john@example.com"

        - name: "user.create"
          response:
            result:
              id: "{{uuid}}"
              created: true

        - name: "unknown"
          error:
            code: -32601
            message: "Method not found"
```

### Scripted Handler

Custom JavaScript logic:

```yaml
websocket:
  handlers:
    - path: "/custom"
      type: script
      script: |
        // Access connection state
        const state = connection.state || { messageCount: 0 };
        state.messageCount++;
        connection.state = state;

        // Parse incoming message
        const msg = JSON.parse(message.text);

        // Generate response
        return {
          type: "response",
          request_id: msg.id,
          message_number: state.messageCount,
          timestamp: Date.now()
        };
```

## Broadcast and Rooms

### Room-Based Broadcasting

```yaml
websocket:
  handlers:
    - path: "/chat"
      type: rooms
      rules:
        - match:
            action: "join"
          handler: |
            rooms.join(message.room);
            broadcast(message.room, {
              type: "user_joined",
              user: connection.id
            });

        - match:
            action: "leave"
          handler: |
            broadcast(message.room, {
              type: "user_left",
              user: connection.id
            });
            rooms.leave(message.room);

        - match:
            action: "message"
          handler: |
            broadcast(message.room, {
              type: "chat",
              from: connection.id,
              text: message.text,
              timestamp: Date.now()
            });
```

### Global Broadcasting

```yaml
websocket:
  handlers:
    - path: "/announcements"
      broadcast:
        interval_ms: 5000
        message:
          type: "heartbeat"
          timestamp: "{{now}}"
          connections: "{{connection_count}}"
```

## Scheduled Messages

### Periodic Updates

```yaml
websocket:
  handlers:
    - path: "/ticker"
      scheduled:
        - interval_ms: 1000
          message:
            type: "price_update"
            symbol: "BTC"
            price: "{{random_float 40000 50000}}"
            timestamp: "{{now}}"

        - interval_ms: 5000
          message:
            type: "stats"
            volume: "{{random_int 1000000 5000000}}"
```

### Delayed Responses

```yaml
websocket:
  handlers:
    - path: "/slow"
      rules:
        - match:
            type: "request"
          response:
            type: "processing"
          then:
            delay_ms: 2000
            response:
              type: "complete"
              result: "done"
```

## Testing Patterns

### Basic Connection Test

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};

#[tokio::test]
async fn test_websocket_connection() {
    let (ws_stream, _) = connect_async("ws://localhost:8080/ws")
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Send message
    write.send(Message::Text(r#"{"type": "ping"}"#.into())).await.unwrap();

    // Receive response
    let msg = read.next().await.unwrap().unwrap();
    assert!(msg.is_text());

    let response: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(response["type"], "pong");
}
```

### Echo Test

```rust
#[tokio::test]
async fn test_echo() {
    let (ws_stream, _) = connect_async("ws://localhost:8080/echo").await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let test_messages = vec!["hello", "world", "test"];

    for msg in &test_messages {
        write.send(Message::Text((*msg).into())).await.unwrap();

        let response = read.next().await.unwrap().unwrap();
        assert_eq!(response.to_text().unwrap(), *msg);
    }
}
```

### Binary Message Test

```rust
#[tokio::test]
async fn test_binary_messages() {
    let (ws_stream, _) = connect_async("ws://localhost:8080/binary").await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    let binary_data = vec![0u8, 1, 2, 3, 4, 5];
    write.send(Message::Binary(binary_data.clone())).await.unwrap();

    let response = read.next().await.unwrap().unwrap();
    assert!(response.is_binary());
    assert_eq!(response.into_data(), binary_data);
}
```

### Concurrent Connections Test

```rust
#[tokio::test]
async fn test_concurrent_connections() {
    let mut handles = vec![];

    for i in 0..100 {
        let handle = tokio::spawn(async move {
            let (ws_stream, _) = connect_async("ws://localhost:8080/ws").await.unwrap();
            let (mut write, mut read) = ws_stream.split();

            write.send(Message::Text(format!(r#"{{"id": {}}}"#, i))).await.unwrap();

            let msg = read.next().await.unwrap().unwrap();
            let response: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();

            assert!(response["id"].as_i64().is_some());
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
```

### Reconnection Test

```rust
#[tokio::test]
async fn test_reconnection() {
    // First connection
    let (ws1, _) = connect_async("ws://localhost:8080/ws").await.unwrap();
    drop(ws1); // Disconnect

    // Small delay
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Reconnect
    let (ws2, _) = connect_async("ws://localhost:8080/ws").await.unwrap();
    let (mut write, mut read) = ws2.split();

    write.send(Message::Text("test".into())).await.unwrap();
    let msg = read.next().await.unwrap().unwrap();
    assert!(msg.is_text());
}
```

### Ping/Pong Test

```rust
#[tokio::test]
async fn test_ping_pong() {
    let (ws_stream, _) = connect_async("ws://localhost:8080/ws").await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Send ping
    write.send(Message::Ping(vec![1, 2, 3])).await.unwrap();

    // Should receive pong
    let msg = read.next().await.unwrap().unwrap();
    assert!(matches!(msg, Message::Pong(_)));
}
```

### Close Handling Test

```rust
#[tokio::test]
async fn test_graceful_close() {
    let (ws_stream, _) = connect_async("ws://localhost:8080/ws").await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Send close frame
    write.send(Message::Close(Some(CloseFrame {
        code: CloseCode::Normal,
        reason: "test complete".into(),
    }))).await.unwrap();

    // Should receive close acknowledgment
    let msg = read.next().await.unwrap().unwrap();
    assert!(matches!(msg, Message::Close(_)));
}
```

## Recording and Replay

### Record Session

```bash
# Start recording
mockforge ws record --output session.json

# Record with filters
mockforge ws record --output session.json --path /api --exclude-ping
```

### Session Format

```json
{
  "metadata": {
    "recorded_at": "2024-01-15T10:30:00Z",
    "duration_ms": 5000,
    "message_count": 42
  },
  "messages": [
    {
      "timestamp_ms": 0,
      "direction": "incoming",
      "type": "text",
      "data": "{\"type\": \"subscribe\", \"channel\": \"orders\"}"
    },
    {
      "timestamp_ms": 50,
      "direction": "outgoing",
      "type": "text",
      "data": "{\"type\": \"subscribed\", \"channel\": \"orders\"}"
    }
  ]
}
```

### Replay Session

```bash
# Replay at recorded speed
mockforge ws replay --input session.json

# Replay faster
mockforge ws replay --input session.json --speed 2.0

# Replay with modifications
mockforge ws replay --input session.json --transform ./transform.js
```

## Proxy Mode

### Forward to Upstream

```yaml
websocket:
  handlers:
    - path: "/proxy"
      type: proxy
      upstream: "wss://api.example.com/ws"
      on_connect:
        headers:
          Authorization: "Bearer {{env.API_TOKEN}}"
      transform:
        incoming: |
          // Modify messages from client before forwarding
          msg.client_id = connection.id;
          return msg;
        outgoing: |
          // Modify messages from upstream before sending to client
          delete msg.internal_id;
          return msg;
```

### Record and Forward

```yaml
websocket:
  handlers:
    - path: "/recorded-proxy"
      type: proxy
      upstream: "wss://api.example.com/ws"
      record: true
      record_path: "./recordings/ws-{{date}}.json"
```

## Chaos Testing

### Connection Instability

```yaml
websocket:
  chaos:
    enabled: true
    disconnect_probability: 0.02  # 2% random disconnect
    delay:
      min_ms: 10
      max_ms: 100
```

### Message Corruption

```yaml
websocket:
  chaos:
    message_drop_rate: 0.01       # 1% message drop
    message_duplicate_rate: 0.005  # 0.5% duplicate
    message_reorder_rate: 0.005    # 0.5% reorder
```

### Slow Consumer Simulation

```yaml
websocket:
  chaos:
    slow_consumer:
      enabled: true
      delay_ms: 1000
      buffer_size: 10
      drop_when_full: true
```

## TLS Configuration

```yaml
websocket:
  tls:
    enabled: true
    cert_path: "./certs/server.crt"
    key_path: "./certs/server.key"
```

```bash
# Connect with wss://
wscat -c wss://localhost:8080/ws --ca ./certs/ca.crt
```

## Metrics and Monitoring

### Available Metrics

```
# Connections
ws_connections_active 150
ws_connections_total 10000
ws_connections_rejected 5

# Messages
ws_messages_received_total 500000
ws_messages_sent_total 500000
ws_messages_bytes_received 104857600
ws_messages_bytes_sent 104857600

# Errors
ws_errors_total{type="parse"} 10
ws_errors_total{type="timeout"} 5
```

### REST API

```bash
# List active connections
curl http://localhost:3000/__mockforge/ws/connections

# Get connection details
curl http://localhost:3000/__mockforge/ws/connections/conn-123

# Send message to connection
curl -X POST http://localhost:3000/__mockforge/ws/connections/conn-123/send \
  -H "Content-Type: application/json" \
  -d '{"type": "notification", "message": "Hello"}'

# Broadcast to all
curl -X POST http://localhost:3000/__mockforge/ws/broadcast \
  -H "Content-Type: application/json" \
  -d '{"type": "announcement", "message": "Server maintenance in 5 minutes"}'

# Close connection
curl -X DELETE http://localhost:3000/__mockforge/ws/connections/conn-123
```

## State Management

### Per-Connection State

```yaml
websocket:
  handlers:
    - path: "/stateful"
      state:
        initial:
          authenticated: false
          subscriptions: []
      rules:
        - match:
            type: "auth"
          handler: |
            if (message.token === "valid-token") {
              state.authenticated = true;
              return { type: "auth_success" };
            }
            return { type: "auth_failed" };

        - match:
            type: "subscribe"
          when: "state.authenticated"
          handler: |
            state.subscriptions.push(message.channel);
            return { type: "subscribed", channel: message.channel };
```

### Shared State

```yaml
websocket:
  shared_state:
    enabled: true
    redis_url: "redis://localhost:6379"

  handlers:
    - path: "/shared"
      rules:
        - match:
            type: "get_counter"
          handler: |
            const count = await shared.get("counter") || 0;
            return { type: "counter", value: count };

        - match:
            type: "increment"
          handler: |
            const count = await shared.incr("counter");
            broadcast("/shared", { type: "counter_updated", value: count });
            return { type: "incremented", value: count };
```

## Best Practices

1. **Handle connection lifecycle** - Implement on_connect, on_message, on_close handlers
2. **Use heartbeats** - Detect stale connections with ping/pong
3. **Set message size limits** - Prevent memory exhaustion
4. **Implement backpressure** - Handle slow consumers gracefully
5. **Use rooms for scaling** - Group related connections
6. **Test reconnection logic** - Ensure clients handle disconnects
7. **Monitor connection counts** - Set alerts for unusual patterns

## Troubleshooting

### Connection Refused

```bash
# Check server is listening
netstat -an | grep 8080

# Test with wscat
wscat -c ws://localhost:8080/ws
```

### Messages Not Received

1. Check message format matches expected pattern
2. Verify connection is still active
3. Check for message size limits
4. Look for parsing errors in logs

### High Memory Usage

1. Monitor connection count
2. Check for connection leaks
3. Review message buffer sizes
4. Implement connection timeouts

## See Also

- [MQTT Protocol Guide](./MQTT.md)
- [AMQP Protocol Guide](./AMQP.md)
- [gRPC Protocol Guide](./GRPC.md)
