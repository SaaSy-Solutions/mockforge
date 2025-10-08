# Protocol-Specific Chaos Engineering

MockForge provides advanced chaos engineering capabilities for protocol-specific testing across gRPC, WebSocket, and GraphQL APIs.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [gRPC Chaos Engineering](#grpc-chaos-engineering)
- [WebSocket Chaos Engineering](#websocket-chaos-engineering)
- [GraphQL Chaos Engineering](#graphql-chaos-engineering)
- [API Reference](#api-reference)
- [CLI Reference](#cli-reference)
- [Best Practices](#best-practices)
- [Examples](#examples)

## Overview

Protocol-specific chaos engineering extends MockForge's base chaos capabilities with protocol-aware fault injection:

- **gRPC**: Status code injection, stream interruption, metadata/message corruption
- **WebSocket**: Close code injection, message dropping, message corruption
- **GraphQL**: Error code injection, partial data responses, resolver-level latency

Each protocol handler reuses the core chaos components (latency, fault injection, rate limiting, traffic shaping) while providing protocol-specific error mappings and behaviors.

## Quick Start

### Enable gRPC Chaos via CLI

```bash
mockforge serve \
  --chaos \
  --chaos-grpc \
  --chaos-grpc-status-codes "13,14" \
  --chaos-grpc-stream-interruption-probability 0.1
```

### Enable WebSocket Chaos via CLI

```bash
mockforge serve \
  --chaos \
  --chaos-websocket \
  --chaos-websocket-close-codes "1008,1011" \
  --chaos-websocket-message-drop-probability 0.05
```

### Enable GraphQL Chaos via CLI

```bash
mockforge serve \
  --chaos \
  --chaos-graphql \
  --chaos-graphql-error-codes "UNAUTHENTICATED,INTERNAL_SERVER_ERROR" \
  --chaos-graphql-partial-data-probability 0.1 \
  --chaos-graphql-resolver-latency
```

## gRPC Chaos Engineering

### Features

- **Status Code Injection**: Inject gRPC status codes (INTERNAL, UNAVAILABLE, etc.)
- **Stream Interruption**: Terminate streaming RPCs mid-stream
- **Pre/Post Request Hooks**: Apply chaos before and after RPC execution
- **HTTP → gRPC Code Mapping**: Automatic conversion from HTTP error codes

### gRPC Status Code Mapping

| HTTP Code | gRPC Status Code | Description |
|-----------|-----------------|-------------|
| 400 | 3 (INVALID_ARGUMENT) | Bad request parameters |
| 401 | 16 (UNAUTHENTICATED) | Missing or invalid authentication |
| 403 | 7 (PERMISSION_DENIED) | Insufficient permissions |
| 404 | 5 (NOT_FOUND) | Resource not found |
| 429 | 8 (RESOURCE_EXHAUSTED) | Rate limit exceeded |
| 500 | 13 (INTERNAL) | Server error |
| 501 | 12 (UNIMPLEMENTED) | Method not implemented |
| 503 | 14 (UNAVAILABLE) | Service unavailable |
| 504 | 4 (DEADLINE_EXCEEDED) | Request timeout |

### Usage Example

```rust
use mockforge_chaos::protocols::grpc::GrpcChaos;
use mockforge_chaos::config::ChaosConfig;

// Create gRPC chaos handler
let chaos = GrpcChaos::new(ChaosConfig {
    enabled: true,
    fault_injection: Some(FaultInjectionConfig {
        enabled: true,
        http_errors: vec![500, 503],
        http_error_probability: 0.2,
        ..Default::default()
    }),
    ..Default::default()
});

// Apply chaos before RPC
chaos.apply_pre_request(
    "UserService",
    "GetUser",
    Some("192.168.1.1")
).await?;

// Check for status code injection
if let Some(code) = chaos.get_grpc_status_code() {
    // Return gRPC error with status code
    return Err(Status::new(code, "Chaos injected error"));
}

// Apply chaos after RPC
chaos.apply_post_response(response_size).await?;
```

### API Endpoints

#### Inject gRPC Status Codes

```bash
POST /api/chaos/protocols/grpc/status-codes
Content-Type: application/json

{
  "status_codes": [13, 14],  // INTERNAL, UNAVAILABLE
  "probability": 0.2
}
```

#### Set Stream Interruption Probability

```bash
POST /api/chaos/protocols/grpc/stream-interruption
Content-Type: application/json

{
  "probability": 0.1
}
```

## WebSocket Chaos Engineering

### Features

- **Close Code Injection**: Send specific WebSocket close codes
- **Message Dropping**: Randomly drop messages
- **Message Corruption**: Partially corrupt message payloads
- **Connection-Level Chaos**: Apply chaos during handshake
- **Message-Level Chaos**: Apply chaos per message (bidirectional)

### WebSocket Close Code Mapping

| HTTP Code | WebSocket Close Code | Description |
|-----------|---------------------|-------------|
| 400 | 1002 (PROTOCOL_ERROR) | Protocol error |
| 408 | 1001 (GOING_AWAY) | Timeout |
| 429 | 1008 (POLICY_VIOLATION) | Rate limit |
| 500 | 1011 (INTERNAL_ERROR) | Server error |
| 503 | 1001 (GOING_AWAY) | Service unavailable |

### Usage Example

```rust
use mockforge_chaos::protocols::websocket::WebSocketChaos;

let chaos = WebSocketChaos::new(config);

// Apply chaos during connection
chaos.apply_connection("/ws", Some("192.168.1.1")).await?;

// Apply chaos for message
chaos.apply_message(message.len(), "inbound").await?;

// Check for connection drop
if chaos.should_drop_connection() {
    return Err("Connection dropped");
}

// Check for close code injection
if let Some(code) = chaos.get_close_code() {
    ws.close(Some(CloseFrame {
        code: CloseCode::from(code),
        reason: "Chaos injected".into(),
    })).await?;
}
```

### API Endpoints

#### Inject WebSocket Close Codes

```bash
POST /api/chaos/protocols/websocket/close-codes
Content-Type: application/json

{
  "close_codes": [1008, 1011],  // POLICY_VIOLATION, INTERNAL_ERROR
  "probability": 0.15
}
```

#### Set Message Drop Probability

```bash
POST /api/chaos/protocols/websocket/message-drop
Content-Type: application/json

{
  "probability": 0.05
}
```

#### Set Message Corruption Probability

```bash
POST /api/chaos/protocols/websocket/message-corruption
Content-Type: application/json

{
  "probability": 0.05
}
```

## GraphQL Chaos Engineering

### Features

- **Error Code Injection**: Inject GraphQL-specific error codes
- **Partial Data Responses**: Return partial data with errors
- **Resolver-Level Latency**: Add latency to individual field resolvers
- **Query/Mutation/Subscription Support**: Apply chaos to all operation types

### GraphQL Error Code Mapping

| HTTP Code | GraphQL Error Code | Description |
|-----------|-------------------|-------------|
| 400 | BAD_USER_INPUT | Invalid input parameters |
| 401 | UNAUTHENTICATED | Not authenticated |
| 403 | FORBIDDEN | Insufficient permissions |
| 404 | NOT_FOUND | Resource not found |
| 500 | INTERNAL_SERVER_ERROR | Server error |
| 503 | SERVICE_UNAVAILABLE | Service unavailable |

### Usage Example

```rust
use mockforge_chaos::protocols::graphql::GraphQLChaos;

let chaos = GraphQLChaos::new(config);

// Apply chaos before query execution
chaos.apply_pre_query(
    "query",
    Some("getUserProfile"),
    Some("192.168.1.1")
).await?;

// Check for error injection
if let Some(error_msg) = chaos.should_inject_error() {
    let error_code = chaos.get_error_code().unwrap_or("INTERNAL_SERVER_ERROR");

    return Ok(GraphQLResponse {
        data: None,
        errors: vec![GraphQLError {
            message: error_msg,
            extensions: json!({ "code": error_code }),
        }],
    });
}

// Apply resolver-level chaos (10% of query latency)
chaos.apply_resolver("user").await?;

// Check for partial data
if chaos.should_return_partial_data() {
    // Return partial response with some null fields
}

// Apply chaos after query
chaos.apply_post_query(response_size).await?;
```

### API Endpoints

#### Inject GraphQL Error Codes

```bash
POST /api/chaos/protocols/graphql/error-codes
Content-Type: application/json

{
  "error_codes": ["UNAUTHENTICATED", "INTERNAL_SERVER_ERROR"],
  "probability": 0.2
}
```

#### Set Partial Data Probability

```bash
POST /api/chaos/protocols/graphql/partial-data
Content-Type: application/json

{
  "probability": 0.1
}
```

#### Toggle Resolver Latency

```bash
POST /api/chaos/protocols/graphql/resolver-latency
Content-Type: application/json

{
  "enabled": true
}
```

## API Reference

All protocol-specific chaos APIs are accessible under `/api/chaos/protocols/`.

### Common Response Format

```json
{
  "message": "Operation completed successfully"
}
```

### Error Responses

```json
{
  "error": "Error message"
}
```

## CLI Reference

### gRPC Chaos Flags

- `--chaos-grpc`: Enable gRPC-specific chaos engineering
- `--chaos-grpc-status-codes <CODES>`: gRPC status codes to inject (comma-separated)
- `--chaos-grpc-stream-interruption-probability <PROB>`: Stream interruption probability (0.0-1.0, default: 0.1)

### WebSocket Chaos Flags

- `--chaos-websocket`: Enable WebSocket-specific chaos engineering
- `--chaos-websocket-close-codes <CODES>`: WebSocket close codes to inject (comma-separated)
- `--chaos-websocket-message-drop-probability <PROB>`: Message drop probability (0.0-1.0, default: 0.05)
- `--chaos-websocket-message-corruption-probability <PROB>`: Message corruption probability (0.0-1.0, default: 0.05)

### GraphQL Chaos Flags

- `--chaos-graphql`: Enable GraphQL-specific chaos engineering
- `--chaos-graphql-error-codes <CODES>`: GraphQL error codes to inject (comma-separated)
- `--chaos-graphql-partial-data-probability <PROB>`: Partial data probability (0.0-1.0, default: 0.1)
- `--chaos-graphql-resolver-latency`: Enable resolver-level latency injection

## Best Practices

### 1. Start with Low Probabilities

Begin with low fault injection probabilities (5-10%) to avoid overwhelming your tests:

```bash
--chaos-grpc-stream-interruption-probability 0.05
```

### 2. Use Realistic Error Codes

Inject error codes that match real-world scenarios:

- gRPC: UNAVAILABLE (14) for service outages, RESOURCE_EXHAUSTED (8) for rate limits
- WebSocket: POLICY_VIOLATION (1008) for rate limits, INTERNAL_ERROR (1011) for server errors
- GraphQL: UNAUTHENTICATED for auth failures, INTERNAL_SERVER_ERROR for backend issues

### 3. Combine with Core Chaos

Protocol-specific chaos works best when combined with base chaos features:

```bash
mockforge serve \
  --chaos \
  --chaos-latency-ms 100 \
  --chaos-grpc \
  --chaos-grpc-status-codes "13,14"
```

### 4. Test Each Protocol Independently

Test protocols in isolation before combining:

```bash
# Test gRPC only
mockforge serve --chaos --chaos-grpc --grpc-port 50051

# Test WebSocket only
mockforge serve --chaos --chaos-websocket --ws-port 3001

# Test GraphQL only
mockforge serve --chaos --chaos-graphql --http-port 3000
```

### 5. Monitor and Adjust

Use the status API to monitor chaos effects:

```bash
curl http://localhost:3000/api/chaos/status
```

## Examples

### Example 1: Simulate gRPC Service Degradation

```bash
mockforge serve \
  --chaos \
  --chaos-latency-ms 500 \
  --chaos-grpc \
  --chaos-grpc-status-codes "14" \
  --chaos-grpc-stream-interruption-probability 0.2 \
  --grpc-port 50051
```

This simulates a degraded gRPC service with:
- 500ms latency on all requests
- 20% chance of UNAVAILABLE status
- 20% chance of stream interruption

### Example 2: Simulate WebSocket Connection Issues

```bash
mockforge serve \
  --chaos \
  --chaos-websocket \
  --chaos-websocket-close-codes "1008,1011" \
  --chaos-websocket-message-drop-probability 0.1 \
  --chaos-packet-loss 5 \
  --ws-port 3001
```

This simulates unreliable WebSocket connections with:
- Random close codes (policy violation or server error)
- 10% message drop rate
- 5% packet loss

### Example 3: Simulate GraphQL Partial Failures

```bash
mockforge serve \
  --chaos \
  --chaos-graphql \
  --chaos-graphql-error-codes "UNAUTHENTICATED,INTERNAL_SERVER_ERROR" \
  --chaos-graphql-partial-data-probability 0.15 \
  --chaos-graphql-resolver-latency \
  --http-port 3000
```

This simulates GraphQL partial failures with:
- Random authentication and server errors
- 15% chance of partial data responses
- Individual resolver latency (10% of query latency)

### Example 4: Combined Protocol Chaos

```bash
mockforge serve \
  --chaos \
  --chaos-latency-ms 200 \
  --chaos-rate-limit 50 \
  --chaos-grpc \
  --chaos-grpc-status-codes "13,14" \
  --chaos-websocket \
  --chaos-websocket-message-drop-probability 0.05 \
  --chaos-graphql \
  --chaos-graphql-partial-data-probability 0.1 \
  --http-port 3000 \
  --ws-port 3001 \
  --grpc-port 50051
```

This runs all protocols with coordinated chaos:
- 200ms base latency across all protocols
- 50 requests/sec rate limit
- gRPC: INTERNAL/UNAVAILABLE errors
- WebSocket: 5% message drops
- GraphQL: 10% partial data responses

### Example 5: API-Driven Protocol Chaos

```bash
# Start MockForge with chaos enabled
mockforge serve --chaos --http-port 3000 --grpc-port 50051

# Configure gRPC chaos via API
curl -X POST http://localhost:3000/api/chaos/protocols/grpc/status-codes \
  -H "Content-Type: application/json" \
  -d '{"status_codes": [13, 14], "probability": 0.2}'

# Configure WebSocket chaos via API
curl -X POST http://localhost:3000/api/chaos/protocols/websocket/message-drop \
  -H "Content-Type: application/json" \
  -d '{"probability": 0.1}'

# Configure GraphQL chaos via API
curl -X POST http://localhost:3000/api/chaos/protocols/graphql/error-codes \
  -H "Content-Type: application/json" \
  -d '{"error_codes": ["UNAUTHENTICATED"], "probability": 0.15}'
```

## Advanced Topics

### Custom Protocol Handlers

You can extend the protocol chaos system by implementing the `ChaosProtocol` trait:

```rust
use mockforge_chaos::protocols::ChaosProtocol;
use async_trait::async_trait;

pub struct CustomProtocolChaos {
    // Custom fields
}

#[async_trait]
impl ChaosProtocol for CustomProtocolChaos {
    async fn apply_pre_request(&self) -> Result<()> {
        // Custom pre-request logic
    }

    async fn apply_post_response(&self, response_size: usize) -> Result<()> {
        // Custom post-response logic
    }

    fn should_abort(&self) -> Option<String> {
        // Custom abort logic
    }

    fn protocol_name(&self) -> &str {
        "custom"
    }
}
```

### Integration with Observability

Protocol chaos integrates with MockForge's observability features:

```bash
mockforge serve \
  --chaos \
  --chaos-grpc \
  --metrics \
  --tracing \
  --recorder
```

This enables:
- Prometheus metrics for chaos events
- Distributed tracing of chaos-affected requests
- API flight recorder capturing chaos scenarios

## Troubleshooting

### Issue: Chaos not applying to protocol

**Solution**: Ensure both `--chaos` and the protocol-specific flag are set:

```bash
mockforge serve --chaos --chaos-grpc  # Correct
mockforge serve --chaos-grpc          # Incorrect (missing --chaos)
```

### Issue: Too many errors

**Solution**: Reduce probability values:

```bash
--chaos-grpc-stream-interruption-probability 0.05  # Start lower
```

### Issue: API returns 404 for protocol endpoints

**Solution**: Ensure chaos is enabled in the configuration:

```bash
curl -X POST http://localhost:3000/api/chaos/enable
```

## See Also

- [Chaos Engineering Guide](./CHAOS_ENGINEERING.md) - Base chaos capabilities
- [gRPC Documentation](./grpc/README.md) - gRPC server setup
- [WebSocket Documentation](./websocket/README.md) - WebSocket server setup
- [GraphQL Documentation](./graphql/README.md) - GraphQL server setup
