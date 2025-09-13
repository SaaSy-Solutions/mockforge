# gRPC Mocking

MockForge provides comprehensive gRPC service mocking with dynamic Protocol Buffer discovery, streaming support, and flexible service registration. This enables testing of gRPC-based microservices and APIs with realistic mock responses.

## Overview

MockForge's gRPC mocking system offers:

- **Dynamic Proto Discovery**: Automatically discovers and compiles `.proto` files from configurable directories
- **Flexible Service Registration**: Register and mock any gRPC service without hardcoding
- **Streaming Support**: Full support for unary, server streaming, client streaming, and bidirectional streaming
- **Reflection Support**: Built-in gRPC reflection for service discovery and testing
- **Template Integration**: Use MockForge's template system for dynamic response generation

## Quick Start

### Basic gRPC Server

Start a gRPC mock server with default configuration:

```bash
# Start with default proto directory (proto/)
mockforge serve --grpc-port 50051
```

### With Custom Proto Directory

```bash
# Specify custom proto directory
MOCKFORGE_PROTO_DIR=my-protos mockforge serve --grpc-port 50051
```

### Complete Example

```bash
# Start MockForge with HTTP, WebSocket, and gRPC support
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl \
MOCKFORGE_PROTO_DIR=examples/grpc-protos \
mockforge serve \
  --spec examples/openapi-demo.json \
  --http-port 3000 \
  --ws-port 3001 \
  --grpc-port 50051 \
  --admin --admin-port 8080
```

## Proto File Setup

### Directory Structure

MockForge automatically discovers `.proto` files in a configurable directory:

```
your-project/
├── proto/                    # Default proto directory
│   ├── user_service.proto   # Will be discovered
│   ├── payment.proto        # Will be discovered
│   └── subdir/
│       └── analytics.proto  # Will be discovered (recursive)
└── examples/
    └── grpc-protos/         # Custom proto directory
        └── service.proto
```

### Sample Proto File

```protobuf
syntax = "proto3";
package mockforge.user;

service UserService {
  rpc GetUser(GetUserRequest) returns (UserResponse);
  rpc ListUsers(ListUsersRequest) returns (stream UserResponse);
  rpc CreateUser(stream CreateUserRequest) returns (UserResponse);
  rpc Chat(stream ChatMessage) returns (stream ChatMessage);
}

message GetUserRequest {
  string user_id = 1;
}

message UserResponse {
  string user_id = 1;
  string name = 2;
  string email = 3;
  int64 created_at = 4;
  Status status = 5;
}

message ListUsersRequest {
  int32 limit = 1;
  string filter = 2;
}

message CreateUserRequest {
  string name = 1;
  string email = 2;
}

message ChatMessage {
  string user_id = 1;
  string content = 2;
  int64 timestamp = 3;
}

enum Status {
  UNKNOWN = 0;
  ACTIVE = 1;
  INACTIVE = 2;
  SUSPENDED = 3;
}
```

## Dynamic Response Generation

MockForge generates responses automatically based on your proto message schemas, with support for templates and custom logic.

### Automatic Response Generation

For basic use cases, MockForge generates responses from proto schemas:

- **Strings**: Random realistic values
- **Integers**: Random numbers in appropriate ranges
- **Timestamps**: Current time or future dates
- **Enums**: Random valid enum values
- **Messages**: Nested objects with generated data
- **Repeated fields**: Arrays with multiple generated items

### Template-Enhanced Responses

Use MockForge templates in proto comments for custom responses:

```protobuf
message UserResponse {
  string user_id = 1; // {{uuid}}
  string name = 2; // {{request.user_id == "123" ? "John Doe" : "Jane Smith"}}
  string email = 3; // {{name | replace(" ", ".") | lower}}@example.com
  int64 created_at = 4; // {{now}}
  Status status = 5; // ACTIVE
}
```

### Request Context Access

Access request data in templates:

```protobuf
message UserResponse {
  string user_id = 1; // {{request.user_id}}
  string requested_by = 2; // {{request.metadata.user_id}}
  string message = 3; // User {{request.user_id}} was retrieved
}
```

## Testing gRPC Services

### Using gRPC CLI Tools

#### grpcurl (Recommended)

```bash
# Install grpcurl
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest

# List available services
grpcurl -plaintext localhost:50051 list

# Call a unary method
grpcurl -plaintext -d '{"user_id": "123"}' \
  localhost:50051 mockforge.user.UserService/GetUser

# Call a server streaming method
grpcurl -plaintext -d '{"limit": 5}' \
  localhost:50051 mockforge.user.UserService/ListUsers

# Call a client streaming method
echo '{"name": "Alice", "email": "alice@example.com"}' | \
grpcurl -plaintext -d @ \
  localhost:50051 mockforge.user.UserService/CreateUser
```

#### grpcui (Web Interface)

```bash
# Install grpcui
go install github.com/fullstorydev/grpcui/cmd/grpcui@latest

# Start web interface
grpcui -plaintext localhost:50051

# Open http://localhost:2633 in your browser
```

### Programmatic Testing

#### Node.js with grpc-js

```javascript
const grpc = require('@grpc/grpc-js');
const protoLoader = require('@grpc/proto-loader');

const packageDefinition = protoLoader.loadSync(
  'proto/user_service.proto',
  {
    keepCase: true,
    longs: String,
    enums: String,
    defaults: true,
    oneofs: true
  }
);

const protoDescriptor = grpc.loadPackageDefinition(packageDefinition);
const client = new protoDescriptor.mockforge.user.UserService(
  'localhost:50051',
  grpc.credentials.createInsecure()
);

// Unary call
client.GetUser({ user_id: '123' }, (error, response) => {
  if (error) {
    console.error('Error:', error);
  } else {
    console.log('Response:', response);
  }
});

// Server streaming
const stream = client.ListUsers({ limit: 5 });
stream.on('data', (response) => {
  console.log('User:', response);
});
stream.on('end', () => {
  console.log('Stream ended');
});
```

#### Python with grpcio

```python
import grpc
from user_service_pb2 import GetUserRequest
from user_service_pb2_grpc import UserServiceStub

channel = grpc.insecure_channel('localhost:50051')
stub = UserServiceStub(channel)

# Unary call
request = GetUserRequest(user_id='123')
response = stub.GetUser(request)
print(f"User: {response.name}, Email: {response.email}")

# Streaming
for user in stub.ListUsers(ListUsersRequest(limit=5)):
    print(f"User: {user.name}")
```

## Advanced Configuration

### Custom Response Mappings

Create custom response logic by implementing service handlers:

```rust
use mockforge_grpc::{ServiceRegistry, ServiceImplementation};
use std::collections::HashMap;

struct CustomUserService {
    user_data: HashMap<String, UserResponse>,
}

impl ServiceImplementation for CustomUserService {
    fn handle_unary(&self, method: &str, request: &[u8]) -> Vec<u8> {
        match method {
            "GetUser" => {
                let req: GetUserRequest = prost::Message::decode(request).unwrap();
                let response = self.user_data.get(&req.user_id)
                    .cloned()
                    .unwrap_or_else(|| UserResponse {
                        user_id: req.user_id,
                        name: "Unknown User".to_string(),
                        email: "unknown@example.com".to_string(),
                        created_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap().as_secs() as i64,
                        status: Status::Unknown as i32,
                    });
                let mut buf = Vec::new();
                response.encode(&mut buf).unwrap();
                buf
            }
            _ => Vec::new(),
        }
    }
}
```

### Environment Variables

```bash
# Proto file configuration
MOCKFORGE_PROTO_DIR=proto/              # Directory containing .proto files
MOCKFORGE_GRPC_PORT=50051               # gRPC server port

# Service behavior
MOCKFORGE_GRPC_LATENCY_ENABLED=true     # Enable response latency
MOCKFORGE_GRPC_LATENCY_MIN_MS=10        # Minimum latency
MOCKFORGE_GRPC_LATENCY_MAX_MS=100       # Maximum latency

# Reflection settings
MOCKFORGE_GRPC_REFLECTION_ENABLED=true  # Enable gRPC reflection
```

### Configuration File

```yaml
grpc:
  port: 50051
  proto_dir: "proto/"
  enable_reflection: true
  latency:
    enabled: true
    min_ms: 10
    max_ms: 100
  services:
    - name: "mockforge.user.UserService"
      implementation: "dynamic"
    - name: "custom.Service"
      implementation: "custom_handler"
```

## Streaming Support

MockForge supports all gRPC streaming patterns:

### Unary (Request → Response)

```protobuf
rpc GetUser(GetUserRequest) returns (UserResponse);
```

Standard request-response pattern used for simple operations.

### Server Streaming (Request → Stream of Responses)

```protobuf
rpc ListUsers(ListUsersRequest) returns (stream UserResponse);
```

Single request that returns multiple responses over time.

### Client Streaming (Stream of Requests → Response)

```protobuf
rpc CreateUsers(stream CreateUserRequest) returns (UserSummary);
```

Multiple requests sent as a stream, single response returned.

### Bidirectional Streaming (Stream ↔ Stream)

```protobuf
rpc Chat(stream ChatMessage) returns (stream ChatMessage);
```

Both client and server can send messages independently.

## Error Handling

### gRPC Status Codes

MockForge supports all standard gRPC status codes:

```protobuf
// In proto comments for custom error responses
rpc GetUser(GetUserRequest) returns (UserResponse);
// @error NOT_FOUND User not found
// @error INVALID_ARGUMENT Invalid user ID format
// @error INTERNAL Server error occurred
```

### Custom Error Responses

```rust
// Custom error handling
fn handle_unary(&self, method: &str, request: &[u8]) -> Result<Vec<u8>, tonic::Status> {
    match method {
        "GetUser" => {
            let req: GetUserRequest = prost::Message::decode(request)?;

            if !is_valid_user_id(&req.user_id) {
                return Err(tonic::Status::invalid_argument("Invalid user ID"));
            }

            match self.get_user(&req.user_id) {
                Some(user) => {
                    let mut buf = Vec::new();
                    user.encode(&mut buf)?;
                    Ok(buf)
                }
                None => Err(tonic::Status::not_found("User not found")),
            }
        }
        _ => Err(tonic::Status::unimplemented("Method not implemented")),
    }
}
```

## Integration Patterns

### Microservices Testing

```bash
# Start multiple gRPC services
MOCKFORGE_PROTO_DIR=user-proto mockforge serve --grpc-port 50051 &
MOCKFORGE_PROTO_DIR=payment-proto mockforge serve --grpc-port 50052 &
MOCKFORGE_PROTO_DIR=inventory-proto mockforge serve --grpc-port 50053 &

# Test service communication
grpcurl -plaintext localhost:50051 mockforge.user.UserService/GetUser \
  -d '{"user_id": "123"}'
```

### Load Testing

```bash
# Simple load test with hey
hey -n 1000 -c 10 \
  grpcurl -plaintext -d '{"user_id": "123"}' \
    localhost:50051 mockforge.user.UserService/GetUser

# Advanced load testing with ghz
ghz --insecure \
    --proto proto/user_service.proto \
    --call mockforge.user.UserService.GetUser \
    --data '{"user_id": "123"}' \
    --concurrency 10 \
    --total 1000 \
    localhost:50051
```

### CI/CD Integration

```yaml
# .github/workflows/test.yml
name: gRPC Tests
on: [push, pull_request]

jobs:
  grpc-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
      - name: Start MockForge
        run: |
          cargo run --bin mockforge-cli -- serve --grpc-port 50051 &
          sleep 5
      - name: Run gRPC Tests
        run: |
          npm install -g grpcurl
          grpcurl -plaintext localhost:50051 list
          # Add your test commands here
```

## Best Practices

### Proto File Organization

1. **Clear Package Names**: Use descriptive package names that reflect service domains
2. **Consistent Naming**: Follow protobuf naming conventions
3. **Versioning**: Include version information in package names when appropriate
4. **Documentation**: Add comments to proto files for better API documentation

### Service Design

1. **Appropriate Streaming**: Choose the right streaming pattern for your use case
2. **Error Handling**: Define clear error conditions and status codes
3. **Pagination**: Implement pagination for large result sets
4. **Backwards Compatibility**: Design for evolution and backwards compatibility

### Testing Strategies

1. **Unit Tests**: Test individual service methods
2. **Integration Tests**: Test service interactions
3. **Load Tests**: Verify performance under load
4. **Chaos Tests**: Test failure scenarios and recovery

### Performance Optimization

1. **Response Caching**: Cache frequently requested data
2. **Connection Pooling**: Reuse gRPC connections
3. **Async Processing**: Use async operations for I/O bound tasks
4. **Memory Management**: Monitor and optimize memory usage

## Troubleshooting

### Common Issues

**Proto files not found**: Check `MOCKFORGE_PROTO_DIR` environment variable and directory permissions

**Service not available**: Verify proto compilation succeeded and service names match

**Connection refused**: Ensure gRPC port is accessible and not blocked by firewall

**Template errors**: Check template syntax and available context variables

### Debug Commands

```bash
# Check proto compilation
cargo build --verbose

# List available services
grpcurl -plaintext localhost:50051 list

# Check service methods
grpcurl -plaintext localhost:50051 describe mockforge.user.UserService

# Test with verbose output
grpcurl -plaintext -v -d '{"user_id": "123"}' \
  localhost:50051 mockforge.user.UserService/GetUser
```

### Log Analysis

```bash
# View gRPC logs
tail -f mockforge.log | grep -i grpc

# Count requests by service
grep "grpc.*call" mockforge.log | cut -d' ' -f5 | sort | uniq -c

# Monitor errors
grep -i "grpc.*error" mockforge.log
```

For detailed implementation guides, see:
- [Protocol Buffers](grpc-mocking/protobuf.md) - Working with .proto files
- [Streaming](grpc-mocking/streaming.md) - Advanced streaming patterns
