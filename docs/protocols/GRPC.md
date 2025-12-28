# gRPC Protocol Guide

This guide covers MockForge's gRPC server implementation for testing RPC-based microservices.

## Overview

MockForge provides comprehensive gRPC mocking capabilities:
- Proto file parsing and reflection
- Unary, server streaming, client streaming, and bidirectional streaming
- gRPC-Web support
- TLS and mTLS
- Deadline and cancellation handling
- Metadata propagation
- Error code simulation

## Quick Start

### Basic Configuration

```yaml
# mockforge.yaml
grpc:
  enabled: true
  port: 50051
  host: "0.0.0.0"
  proto_paths:
    - "./protos"
  reflection: true
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MOCKFORGE_GRPC_ENABLED` | `false` | Enable gRPC server |
| `MOCKFORGE_GRPC_PORT` | `50051` | gRPC server port |
| `MOCKFORGE_GRPC_HOST` | `0.0.0.0` | Bind address |
| `MOCKFORGE_GRPC_PROTO_PATHS` | `./protos` | Comma-separated proto paths |
| `MOCKFORGE_GRPC_REFLECTION` | `true` | Enable gRPC reflection |
| `MOCKFORGE_GRPC_MAX_MESSAGE_SIZE` | `4194304` | Max message size (4MB) |

### Starting the Server

```bash
# Via CLI
mockforge serve --grpc --proto-path ./protos

# With custom port
mockforge serve --grpc --grpc-port 9090

# With TLS
mockforge serve --grpc --grpc-tls
```

## Proto File Configuration

### Service Definition

Given this proto file:

```protobuf
// protos/user.proto
syntax = "proto3";

package user.v1;

service UserService {
  rpc GetUser(GetUserRequest) returns (GetUserResponse);
  rpc ListUsers(ListUsersRequest) returns (stream User);
  rpc CreateUsers(stream CreateUserRequest) returns (CreateUsersResponse);
  rpc Chat(stream ChatMessage) returns (stream ChatMessage);
}

message GetUserRequest {
  string user_id = 1;
}

message GetUserResponse {
  User user = 1;
}

message User {
  string id = 1;
  string name = 2;
  string email = 3;
  int64 created_at = 4;
}
```

### Mock Configuration

```yaml
grpc:
  enabled: true
  proto_paths:
    - "./protos"
  services:
    - package: "user.v1"
      service: "UserService"
      methods:
        - name: "GetUser"
          response:
            user:
              id: "{{request.user_id}}"
              name: "John Doe"
              email: "john@example.com"
              created_at: 1703721600

        - name: "ListUsers"
          stream:
            - user: { id: "1", name: "Alice" }
            - user: { id: "2", name: "Bob" }
            - user: { id: "3", name: "Charlie" }
          delay_between_ms: 100
```

## Response Templating

### Dynamic Responses

Use Handlebars templates in responses:

```yaml
grpc:
  services:
    - package: "order.v1"
      service: "OrderService"
      methods:
        - name: "CreateOrder"
          response:
            order_id: "{{uuid}}"
            status: "CREATED"
            created_at: "{{timestamp}}"
            items_count: "{{request.items | length}}"
            total: "{{sum request.items 'price'}}"
```

### Conditional Responses

```yaml
grpc:
  services:
    - package: "user.v1"
      service: "UserService"
      methods:
        - name: "GetUser"
          rules:
            - when:
                request:
                  user_id: "not-found"
              error:
                code: NOT_FOUND
                message: "User not found"

            - when:
                request:
                  user_id: "error"
              error:
                code: INTERNAL
                message: "Internal server error"

            - response:
                user:
                  id: "{{request.user_id}}"
                  name: "Default User"
```

### Metadata-Based Routing

```yaml
grpc:
  services:
    - package: "api.v1"
      service: "ApiService"
      methods:
        - name: "Process"
          rules:
            - when:
                metadata:
                  x-tenant-id: "premium"
              response:
                priority: "HIGH"
                rate_limit: 10000

            - when:
                metadata:
                  x-tenant-id: "basic"
              response:
                priority: "LOW"
                rate_limit: 100
```

## Streaming

### Server Streaming

```yaml
grpc:
  services:
    - package: "feed.v1"
      service: "FeedService"
      methods:
        - name: "Subscribe"
          type: server_streaming
          stream:
            items:
              - event: { type: "POST", id: "1" }
              - event: { type: "COMMENT", id: "2" }
              - event: { type: "LIKE", id: "3" }
            delay_between_ms: 500
            repeat: true  # Infinite stream
```

### Client Streaming

```yaml
grpc:
  services:
    - package: "upload.v1"
      service: "UploadService"
      methods:
        - name: "UploadChunks"
          type: client_streaming
          response:
            bytes_received: "{{stream_count * 1024}}"
            status: "COMPLETE"
```

### Bidirectional Streaming

```yaml
grpc:
  services:
    - package: "chat.v1"
      service: "ChatService"
      methods:
        - name: "Chat"
          type: bidirectional
          echo: true  # Echo back received messages
          transform: |
            {
              "from": "bot",
              "message": "You said: {{message}}",
              "timestamp": "{{now}}"
            }
```

## Testing Patterns

### Unary RPC Testing

```rust
use tonic::Request;
use user::v1::{user_service_client::UserServiceClient, GetUserRequest};

#[tokio::test]
async fn test_get_user() {
    let mut client = UserServiceClient::connect("http://localhost:50051")
        .await
        .unwrap();

    let request = Request::new(GetUserRequest {
        user_id: "user-123".to_string(),
    });

    let response = client.get_user(request).await.unwrap();
    let user = response.into_inner().user.unwrap();

    assert_eq!(user.id, "user-123");
    assert!(!user.name.is_empty());
}
```

### Server Streaming Testing

```rust
#[tokio::test]
async fn test_list_users_stream() {
    let mut client = UserServiceClient::connect("http://localhost:50051")
        .await
        .unwrap();

    let request = Request::new(ListUsersRequest { page_size: 10 });

    let mut stream = client.list_users(request).await.unwrap().into_inner();

    let mut users = Vec::new();
    while let Some(user) = stream.message().await.unwrap() {
        users.push(user);
    }

    assert!(!users.is_empty());
}
```

### Client Streaming Testing

```rust
#[tokio::test]
async fn test_bulk_create() {
    let mut client = UserServiceClient::connect("http://localhost:50051")
        .await
        .unwrap();

    let users = vec![
        CreateUserRequest { name: "Alice".into(), email: "alice@example.com".into() },
        CreateUserRequest { name: "Bob".into(), email: "bob@example.com".into() },
    ];

    let request = Request::new(tokio_stream::iter(users));

    let response = client.create_users(request).await.unwrap();
    assert_eq!(response.into_inner().created_count, 2);
}
```

### Bidirectional Streaming Testing

```rust
#[tokio::test]
async fn test_chat_stream() {
    let mut client = ChatServiceClient::connect("http://localhost:50051")
        .await
        .unwrap();

    let (tx, rx) = tokio::sync::mpsc::channel(32);

    // Send messages
    tokio::spawn(async move {
        tx.send(ChatMessage { text: "Hello".into() }).await.unwrap();
        tx.send(ChatMessage { text: "World".into() }).await.unwrap();
    });

    let request = Request::new(tokio_stream::wrappers::ReceiverStream::new(rx));
    let mut response_stream = client.chat(request).await.unwrap().into_inner();

    let mut responses = Vec::new();
    while let Some(msg) = response_stream.message().await.unwrap() {
        responses.push(msg);
    }

    assert_eq!(responses.len(), 2);
}
```

### Error Handling Testing

```rust
#[tokio::test]
async fn test_not_found_error() {
    let mut client = UserServiceClient::connect("http://localhost:50051")
        .await
        .unwrap();

    let request = Request::new(GetUserRequest {
        user_id: "not-found".to_string(),
    });

    let result = client.get_user(request).await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
}
```

### Metadata Testing

```rust
#[tokio::test]
async fn test_metadata_propagation() {
    let mut client = UserServiceClient::connect("http://localhost:50051")
        .await
        .unwrap();

    let mut request = Request::new(GetUserRequest { user_id: "123".into() });
    request.metadata_mut().insert("x-request-id", "req-456".parse().unwrap());
    request.metadata_mut().insert("authorization", "Bearer token".parse().unwrap());

    let response = client.get_user(request).await.unwrap();

    // Check response metadata
    let metadata = response.metadata();
    assert!(metadata.get("x-request-id").is_some());
}
```

### Deadline Testing

```rust
#[tokio::test]
async fn test_deadline_exceeded() {
    let mut client = UserServiceClient::connect("http://localhost:50051")
        .await
        .unwrap();

    let mut request = Request::new(GetUserRequest { user_id: "slow".into() });
    request.set_timeout(Duration::from_millis(10)); // Very short timeout

    let result = client.get_user(request).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::DeadlineExceeded);
}
```

## Chaos Testing

### Latency Injection

```yaml
grpc:
  chaos:
    enabled: true
    latency:
      min_ms: 50
      max_ms: 200
    methods:
      - pattern: "*.Get*"
        latency:
          min_ms: 10
          max_ms: 50
```

### Error Injection

```yaml
grpc:
  chaos:
    error_rate: 0.05  # 5% error rate
    errors:
      - code: UNAVAILABLE
        weight: 70
      - code: RESOURCE_EXHAUSTED
        weight: 20
      - code: INTERNAL
        weight: 10
```

### Stream Interruption

```yaml
grpc:
  chaos:
    stream_interrupt:
      probability: 0.1
      after_messages: 5  # Interrupt after 5 messages
      error: CANCELLED
```

## TLS Configuration

### Server TLS

```yaml
grpc:
  tls:
    enabled: true
    cert_path: "./certs/server.crt"
    key_path: "./certs/server.key"
```

### Mutual TLS (mTLS)

```yaml
grpc:
  tls:
    enabled: true
    cert_path: "./certs/server.crt"
    key_path: "./certs/server.key"
    ca_path: "./certs/ca.crt"
    require_client_cert: true
```

## gRPC-Web Support

```yaml
grpc:
  grpc_web:
    enabled: true
    cors:
      allowed_origins:
        - "http://localhost:3000"
      allowed_methods:
        - POST
      allowed_headers:
        - content-type
        - x-grpc-web
```

## Health Checking

MockForge implements the gRPC health checking protocol:

```yaml
grpc:
  health:
    enabled: true
    services:
      - name: ""  # Overall health
        status: SERVING
      - name: "user.v1.UserService"
        status: SERVING
```

```bash
# Check health via grpcurl
grpcurl -plaintext localhost:50051 grpc.health.v1.Health/Check
```

## Metrics and Monitoring

### Available Metrics

```
# RPCs
grpc_server_started_total{method="GetUser",service="UserService"} 1234
grpc_server_handled_total{method="GetUser",code="OK"} 1200
grpc_server_handled_total{method="GetUser",code="NOT_FOUND"} 34

# Latency
grpc_server_handling_seconds_bucket{method="GetUser",le="0.1"} 1100
grpc_server_handling_seconds_bucket{method="GetUser",le="0.5"} 1200

# Streams
grpc_server_msg_received_total{method="Chat"} 5000
grpc_server_msg_sent_total{method="Chat"} 5000
```

### REST API

```bash
# List services
curl http://localhost:3000/__mockforge/grpc/services

# Get service methods
curl http://localhost:3000/__mockforge/grpc/services/user.v1.UserService

# Mock a method
curl -X POST http://localhost:3000/__mockforge/grpc/mock \
  -H "Content-Type: application/json" \
  -d '{
    "service": "user.v1.UserService",
    "method": "GetUser",
    "response": {
      "user": { "id": "test", "name": "Test User" }
    }
  }'
```

## Reflection

Enable server reflection for tools like grpcurl:

```yaml
grpc:
  reflection: true
```

```bash
# List services
grpcurl -plaintext localhost:50051 list

# Describe service
grpcurl -plaintext localhost:50051 describe user.v1.UserService

# Call method
grpcurl -plaintext -d '{"user_id": "123"}' localhost:50051 user.v1.UserService/GetUser
```

## Best Practices

1. **Use proto files** - Define services with proto files for type safety
2. **Enable reflection** in development for easier debugging
3. **Set appropriate deadlines** - All RPCs should have deadlines
4. **Handle all error codes** - Test for UNAVAILABLE, DEADLINE_EXCEEDED, etc.
5. **Use interceptors** for cross-cutting concerns (logging, auth)
6. **Implement health checks** for service discovery
7. **Monitor stream lifecycle** - Handle cancellation properly

## Troubleshooting

### Connection Refused

```bash
# Check server is running
grpcurl -plaintext localhost:50051 list

# Check port
netstat -an | grep 50051
```

### Proto Parsing Errors

```bash
# Validate proto files
protoc --proto_path=./protos --descriptor_set_out=/dev/null ./protos/*.proto

# Check imports are resolvable
mockforge grpc validate --proto-path ./protos
```

### Streaming Issues

1. Check client handles stream completion
2. Verify server isn't holding stream open unnecessarily
3. Monitor for memory leaks in long-running streams

## See Also

- [MQTT Protocol Guide](./MQTT.md)
- [AMQP Protocol Guide](./AMQP.md)
- [WebSocket Protocol Guide](./WEBSOCKET.md)
