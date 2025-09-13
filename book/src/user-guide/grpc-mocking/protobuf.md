# Protocol Buffers

Protocol Buffers (protobuf) are the interface definition language used by gRPC services. MockForge provides comprehensive support for working with protobuf files, including automatic discovery, compilation, and dynamic service generation.

## Understanding Proto Files

### Basic Structure

A `.proto` file defines the service interface and message formats:

```protobuf
syntax = "proto3";

package myapp.user;

import "google/protobuf/timestamp.proto";

// Service definition
service UserService {
  rpc GetUser(GetUserRequest) returns (User);
  rpc ListUsers(ListUsersRequest) returns (stream User);
  rpc CreateUser(CreateUserRequest) returns (User);
  rpc UpdateUser(UpdateUserRequest) returns (User);
  rpc DeleteUser(DeleteUserRequest) returns (google.protobuf.Empty);
}

// Message definitions
message GetUserRequest {
  string user_id = 1;
}

message User {
  string user_id = 1;
  string email = 2;
  string name = 3;
  google.protobuf.Timestamp created_at = 4;
  google.protobuf.Timestamp updated_at = 5;
  UserStatus status = 6;
  repeated string roles = 7;
}

message ListUsersRequest {
  int32 page_size = 1;
  string page_token = 2;
  string filter = 3;
}

message CreateUserRequest {
  string email = 1;
  string name = 2;
  repeated string roles = 3;
}

message UpdateUserRequest {
  string user_id = 1;
  string email = 2;
  string name = 3;
  repeated string roles = 4;
}

message DeleteUserRequest {
  string user_id = 1;
}

enum UserStatus {
  UNKNOWN = 0;
  ACTIVE = 1;
  INACTIVE = 2;
  SUSPENDED = 3;
}
```

### Key Components

#### Syntax Declaration
```protobuf
syntax = "proto3";
```
Declares the protobuf version. MockForge supports proto3.

#### Package Declaration
```protobuf
package myapp.user;
```
Defines the namespace for the service and messages.

#### Imports
```protobuf
import "google/protobuf/timestamp.proto";
```
Imports common protobuf types and other proto files.

#### Service Definition
```protobuf
service UserService {
  rpc GetUser(GetUserRequest) returns (User);
  // ... more methods
}
```
Defines the RPC methods available in the service.

#### Message Definitions
```protobuf
message User {
  string user_id = 1;
  string email = 2;
  // ... more fields
}
```
Defines the structure of data exchanged between client and server.

#### Enum Definitions
```protobuf
enum UserStatus {
  UNKNOWN = 0;
  ACTIVE = 1;
  // ... more values
}
```
Defines enumerated types with named constants.

## Field Types

### Scalar Types

| Proto Type | Go Type | Java Type | C++ Type | Notes |
|------------|---------|-----------|----------|-------|
| double | float64 | double | double | |
| float | float32 | float | float | |
| int32 | int32 | int | int32 | Uses variable-length encoding |
| int64 | int64 | long | int64 | Uses variable-length encoding |
| uint32 | uint32 | int | uint32 | Uses variable-length encoding |
| uint64 | uint64 | long | uint64 | Uses variable-length encoding |
| sint32 | int32 | int | int32 | Uses zigzag encoding |
| sint64 | int64 | long | int64 | Uses zigzag encoding |
| fixed32 | uint32 | int | uint32 | Always 4 bytes |
| fixed64 | uint64 | long | uint64 | Always 8 bytes |
| sfixed32 | int32 | int | int32 | Always 4 bytes |
| sfixed64 | int64 | long | int64 | Always 8 bytes |
| bool | bool | boolean | bool | |
| string | string | String | string | UTF-8 encoded |
| bytes | []byte | ByteString | string | |

### Repeated Fields

```protobuf
message SearchResponse {
  repeated Result results = 1;
}
```

Creates an array/list of the specified type.

### Nested Messages

```protobuf
message Address {
  string street = 1;
  string city = 2;
  string country = 3;
}

message Person {
  string name = 1;
  Address address = 2;
}
```

Messages can contain other messages as fields.

### Oneof Fields

```protobuf
message Person {
  string name = 1;
  oneof contact_info {
    string email = 2;
    string phone = 3;
  }
}
```

Only one of the specified fields can be set at a time.

### Maps

```protobuf
message Config {
  map<string, string> settings = 1;
}
```

Creates a key-value map structure.

## Service Patterns

### Unary RPC

```protobuf
service Calculator {
  rpc Add(AddRequest) returns (AddResponse);
}
```

Standard request-response pattern.

### Server Streaming

```protobuf
service NotificationService {
  rpc Subscribe(SubscribeRequest) returns (stream Notification);
}
```

Server sends multiple responses for a single request.

### Client Streaming

```protobuf
service UploadService {
  rpc Upload(stream UploadChunk) returns (UploadResponse);
}
```

Client sends multiple requests, server responds once.

### Bidirectional Streaming

```protobuf
service ChatService {
  rpc Chat(stream ChatMessage) returns (stream ChatMessage);
}
```

Both client and server can send messages independently.

## Proto File Organization

### Directory Structure

```
proto/
├── user/
│   ├── v1/
│   │   ├── user.proto
│   │   └── user_service.proto
│   └── v2/
│       ├── user.proto
│       └── user_service.proto
├── payment/
│   ├── payment.proto
│   └── payment_service.proto
└── common/
    ├── types.proto
    └── errors.proto
```

### Versioning

```protobuf
// user/v1/user.proto
syntax = "proto3";
package myapp.user.v1;

// Version-specific message
message User {
  string id = 1;
  string name = 2;
  string email = 3;
}
```

```protobuf
// user/v2/user.proto
syntax = "proto3";
package myapp.user.v2;

// Extended version with new fields
message User {
  string id = 1;
  string name = 2;
  string email = 3;
  string phone = 4;  // New field
  repeated string tags = 5;  // New field
}
```

## MockForge Integration

### Automatic Discovery

MockForge automatically discovers `.proto` files in the configured directory:

```bash
# Default proto directory
mockforge serve --grpc-port 50051

# Custom proto directory
MOCKFORGE_PROTO_DIR=my-protos mockforge serve --grpc-port 50051
```

### Service Registration

MockForge automatically registers all discovered services:

```bash
# List available services
grpcurl -plaintext localhost:50051 list

# Output:
# grpc.reflection.v1alpha.ServerReflection
# myapp.user.UserService
# myapp.payment.PaymentService
```

### Dynamic Response Generation

MockForge generates responses based on proto message schemas:

```protobuf
message UserResponse {
  string user_id = 1;    // Generates UUID
  string name = 2;       // Generates random name
  string email = 3;      // Generates valid email
  int64 created_at = 4;  // Generates timestamp
  UserStatus status = 5; // Random enum value
}
```

### Template Support

Use MockForge templates for custom responses:

```protobuf
message UserResponse {
  string user_id = 1;    // {{uuid}}
  string name = 2;       // {{request.user_id == "123" ? "John Doe" : "Jane Smith"}}
  string email = 3;      // {{name | replace(" ", ".") | lower}}@example.com
  int64 created_at = 4;  // {{now}}
  UserStatus status = 5; // ACTIVE
}
```

## Best Practices

### Naming Conventions

1. **Packages**: Use lowercase with dots (e.g., `myapp.user.v1`)
2. **Services**: Use PascalCase with "Service" suffix (e.g., `UserService`)
3. **Messages**: Use PascalCase (e.g., `UserProfile`)
4. **Fields**: Use snake_case (e.g., `user_id`, `created_at`)
5. **Enums**: Use PascalCase for type, SCREAMING_SNAKE_CASE for values

### Field Numbering

1. **Reserve numbers**: Don't reuse field numbers from deleted fields
2. **Start from 1**: Field numbers start from 1
3. **Gap for extensions**: Leave gaps for future extensions
4. **Document reservations**: Comment reserved field numbers

```protobuf
message User {
  string user_id = 1;
  string name = 2;
  string email = 3;
  // reserved 4, 5, 6;  // Reserved for future use
  int64 created_at = 7;
}
```

### Import Organization

1. **Standard imports**: Import well-known protobuf types first
2. **Local imports**: Import project-specific proto files
3. **Relative paths**: Use relative paths for local imports

```protobuf
syntax = "proto3";

import "google/protobuf/timestamp.proto";
import "google/protobuf/empty.proto";

import "common/types.proto";
import "user/profile.proto";

package myapp.user;
```

### Documentation

1. **Service comments**: Document what each service does
2. **Method comments**: Explain each RPC method
3. **Field comments**: Describe field purposes and constraints
4. **Enum comments**: Document enum value meanings

```protobuf
// User management service
service UserService {
  // Get a user by ID
  rpc GetUser(GetUserRequest) returns (User);

  // List users with pagination
  rpc ListUsers(ListUsersRequest) returns (ListUsersResponse);
}

message User {
  string user_id = 1;  // Unique identifier for the user
  string email = 2;    // User's email address (must be valid)
  UserStatus status = 3; // Current account status
}

enum UserStatus {
  UNKNOWN = 0;   // Default value
  ACTIVE = 1;    // Account is active
  INACTIVE = 2;  // Account is deactivated
  SUSPENDED = 3; // Account is temporarily suspended
}
```

## Migration and Evolution

### Adding Fields

```protobuf
// Original
message User {
  string user_id = 1;
  string name = 2;
}

// Extended (backwards compatible)
message User {
  string user_id = 1;
  string name = 2;
  string email = 3;      // New field
  bool active = 4;       // New field
}
```

### Reserved Fields

```protobuf
message User {
  reserved 5, 6, 7;        // Reserved for future use
  reserved "old_field";    // Reserved field name

  string user_id = 1;
  string name = 2;
  string email = 3;
}
```

### Versioning Strategy

1. **Package versioning**: Include version in package name
2. **Service evolution**: Extend services with new methods
3. **Deprecation notices**: Mark deprecated fields
4. **Breaking changes**: Create new service versions

## Validation

### Proto File Validation

```bash
# Validate proto syntax
protoc --proto_path=. --error_format=json myproto.proto

# Generate descriptors
protoc --proto_path=. --descriptor_set_out=descriptor.pb myproto.proto
```

### MockForge Integration Testing

```bash
# Test proto compilation
MOCKFORGE_PROTO_DIR=my-protos cargo build

# Verify service discovery
mockforge serve --grpc-port 50051 &
sleep 2
grpcurl -plaintext localhost:50051 list
```

### Cross-Language Compatibility

```bash
# Generate code for multiple languages
protoc --proto_path=. \
  --go_out=. \
  --java_out=. \
  --python_out=. \
  --cpp_out=. \
  myproto.proto
```

## Troubleshooting

### Common Proto Issues

**Import resolution**: Ensure all imported proto files are available in the proto path

**Field conflicts**: Check for duplicate field numbers or names within messages

**Circular imports**: Avoid circular dependencies between proto files

**Syntax errors**: Use `protoc` to validate proto file syntax

### MockForge-Specific Issues

**Services not discovered**: Check proto directory configuration and file permissions

**Invalid responses**: Verify proto message definitions match expected schemas

**Compilation failures**: Check for proto syntax errors and missing dependencies

**Template errors**: Ensure template variables are properly escaped in proto comments

### Debug Commands

```bash
# Check proto file discovery
find proto/ -name "*.proto" -type f

# Validate proto files
for file in $(find proto/ -name "*.proto"); do
  echo "Validating $file..."
  protoc --proto_path=. --error_format=json "$file" > /dev/null
done

# Test service compilation
MOCKFORGE_PROTO_DIR=proto/ cargo check -p mockforge-grpc

# Inspect generated code
cargo doc --open --package mockforge-grpc
```

Protocol Buffers provide a robust foundation for gRPC service definitions. By following these guidelines and leveraging MockForge's dynamic discovery capabilities, you can create well-structured, maintainable, and testable gRPC services.
