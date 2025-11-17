# Protocol Contracts

## Overview

MockForge supports contract management and drift detection across multiple protocols, not just HTTP/REST. This enables teams to maintain contract consistency across all transport layers, ensuring mocks and contracts stay aligned for gRPC, WebSocket, MQTT, and Kafka services.

## Supported Protocols

### gRPC

gRPC contracts are defined using Protocol Buffers (protobuf). MockForge supports:
- Service and method definitions
- Message type schemas
- Streaming methods (unary, server streaming, client streaming, bidirectional)
- Per-method drift detection

### WebSocket

WebSocket contracts define:
- Message types and their schemas
- Topics or channels
- Message direction (inbound, outbound, bidirectional)
- JSON schema validation for message payloads

### MQTT

MQTT contracts define:
- Topic schemas
- Quality of Service (QoS) levels
- Retained message configuration
- JSON schema validation for message payloads

### Kafka

Kafka contracts define:
- Topic schemas (key and value)
- Schema formats (JSON, Avro, Protobuf)
- Partition and replication configuration
- Evolution rules for schema changes

## Creating Contracts

### gRPC Contract

Create a gRPC contract from a protobuf descriptor set:

```http
POST /api/v1/contracts/grpc
Content-Type: application/json

{
  "contract_id": "user-service",
  "version": "1.0.0",
  "descriptor_set": "<base64-encoded-protobuf-descriptor-set>"
}
```

The descriptor set is a compiled protobuf `FileDescriptorSet` that contains all service and message definitions.

### WebSocket Contract

Create a WebSocket contract with message types:

```http
POST /api/v1/contracts/websocket
Content-Type: application/json

{
  "contract_id": "realtime-service",
  "version": "1.0.0",
  "message_types": [
    {
      "message_type": "user_joined",
      "topic": "presence",
      "schema": {
        "type": "object",
        "properties": {
          "user_id": { "type": "string" },
          "username": { "type": "string" },
          "timestamp": { "type": "number" }
        },
        "required": ["user_id", "username"]
      },
      "direction": "outbound",
      "description": "Sent when a user joins a channel"
    }
  ]
}
```

### MQTT Contract

Create an MQTT contract with topic schemas:

```http
POST /api/v1/contracts/mqtt
Content-Type: application/json

{
  "contract_id": "iot-device-service",
  "version": "1.0.0",
  "topics": [
    {
      "topic": "devices/+/telemetry",
      "qos": 1,
      "schema": {
        "type": "object",
        "properties": {
          "device_id": { "type": "string" },
          "temperature": { "type": "number" },
          "humidity": { "type": "number" }
        },
        "required": ["device_id", "temperature"]
      },
      "retained": false,
      "description": "Device telemetry data"
    }
  ]
}
```

### Kafka Contract

Create a Kafka contract with topic schemas:

```http
POST /api/v1/contracts/kafka
Content-Type: application/json

{
  "contract_id": "event-stream-service",
  "version": "1.0.0",
  "topics": [
    {
      "topic": "user-events",
      "key_schema": {
        "format": "json",
        "schema": {
          "type": "string"
        }
      },
      "value_schema": {
        "format": "avro",
        "schema": {
          "type": "record",
          "name": "UserEvent",
          "fields": [
            { "name": "user_id", "type": "string" },
            { "name": "event_type", "type": "string" },
            { "name": "timestamp", "type": "long" }
          ]
        }
      },
      "partitions": 3,
      "replication_factor": 2,
      "evolution_rules": {
        "allow_backward_compatible": true,
        "allow_forward_compatible": true,
        "require_version_bump": true
      }
    }
  ]
}
```

## Contract Comparison

Compare two versions of a contract to detect drift:

```http
POST /api/v1/contracts/compare
Content-Type: application/json

{
  "old_contract_id": "user-service",
  "new_contract_id": "user-service-v2"
}
```

Response:
```json
{
  "breaking_changes": [
    {
      "operation_id": "GetUser",
      "change_type": "method_removed",
      "description": "Method GetUser was removed"
    }
  ],
  "non_breaking_changes": [
    {
      "operation_id": "ListUsers",
      "change_type": "field_added",
      "description": "Optional field 'metadata' added to response"
    }
  ],
  "summary": {
    "total_operations": 10,
    "breaking_count": 1,
    "non_breaking_count": 1
  }
}
```

## Message Validation

Validate a message against a contract:

```http
POST /api/v1/contracts/{contract_id}/validate
Content-Type: application/json

{
  "operation_id": "user_joined",
  "message": {
    "user_id": "123",
    "username": "alice",
    "timestamp": 1234567890
  },
  "message_format": "json"
}
```

Response:
```json
{
  "valid": true,
  "errors": [],
  "warnings": []
}
```

## Protocol-Specific Features

### gRPC

- **Service Discovery**: Automatically extract services and methods from descriptor sets
- **Method Signatures**: Track input/output message types
- **Streaming Support**: Detect changes in streaming configuration
- **Per-Method Drift**: Identify which specific methods changed

### WebSocket

- **Topic-Based Routing**: Validate messages based on topic/channel
- **Directional Validation**: Enforce inbound/outbound message rules
- **Schema Evolution**: Track changes to message schemas over time
- **Example Payloads**: Store example messages for documentation

### MQTT

- **QoS Configuration**: Track Quality of Service levels per topic
- **Retained Messages**: Validate retained message schemas
- **Wildcard Topics**: Support MQTT topic wildcards (+ and #)
- **Topic Hierarchy**: Validate topic structure

### Kafka

- **Key/Value Schemas**: Separate schemas for message keys and values
- **Schema Formats**: Support JSON, Avro, and Protobuf formats
- **Evolution Rules**: Define backward/forward compatibility rules
- **Partition Configuration**: Track partition and replication settings

## Integration with Drift Budgets

Protocol contracts integrate with the drift budget system:

1. **Contract Registration**: Register protocol contracts alongside HTTP contracts
2. **Drift Detection**: Compare contract versions to detect changes
3. **Budget Evaluation**: Evaluate changes against configured drift budgets
4. **Incident Creation**: Create incidents when budgets are exceeded
5. **Consumer Impact**: Analyze impact on consuming applications

## Admin UI

### Protocol Selector

The Contract Diff page includes a protocol selector to switch between:
- HTTP/REST
- gRPC
- WebSocket
- MQTT
- Kafka

### Contract Editor

The Protocol Contract Editor provides:
- Protocol-specific forms for each contract type
- Schema editors with JSON validation
- File upload for protobuf descriptor sets
- Topic and message type management

### Contract List

View all contracts for a selected protocol:
- Contract ID and version
- Protocol type badge
- Quick actions (view, delete, compare)

## Best Practices

### 1. Version Management

- Use semantic versioning for contracts
- Increment versions when making breaking changes
- Document version changes in release notes

### 2. Schema Evolution

- Design schemas with backward compatibility in mind
- Use optional fields for new additions
- Document deprecation timelines for removed fields

### 3. Contract Testing

- Validate messages against contracts in tests
- Use contract comparison to detect unintended changes
- Integrate contract validation into CI/CD pipelines

### 4. Multi-Protocol Consistency

- Keep contracts aligned across protocols
- Use consistent naming conventions
- Document protocol-specific differences

## API Reference

### List Contracts

```http
GET /api/v1/contracts?protocol=grpc
```

### Get Contract

```http
GET /api/v1/contracts/{contract_id}
```

### Delete Contract

```http
DELETE /api/v1/contracts/{contract_id}
```

### Compare Contracts

```http
POST /api/v1/contracts/compare
```

### Validate Message

```http
POST /api/v1/contracts/{contract_id}/validate
```

## See Also

- [Drift Budgets](./DRIFT_BUDGETS.md) - Configure drift thresholds
- [Consumer Impact Analysis](./CONSUMER_IMPACT_ANALYSIS.md) - Understand downstream impact
- [Fitness Functions](./DRIFT_BUDGETS.md#fitness-functions) - Define contract quality rules
