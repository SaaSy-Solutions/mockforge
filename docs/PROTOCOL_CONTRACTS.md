# Protocol Contracts

**Pillars:** [Contracts]

[Contracts] - Schema, drift, validation, and safety nets for multi-protocol API contracts

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

## Drift Detection Examples

### gRPC Drift Detection

When comparing gRPC contracts, MockForge detects changes at the service and method level with clear classification as **additive** vs **breaking**:

**Breaking Changes (is_breaking: true):**
- Method removed from a service
- Method signature changed (input/output message types)
- Streaming configuration changed (unary → streaming)
- Required field added to request/response messages
- Service removed

**Additive Changes (is_additive: true):**
- New method added to a service
- Optional field added to messages
- New service added

**Classification Metadata:**

Each mismatch in a gRPC diff includes classification metadata in the `context` field:

```json
{
  "mismatch_type": "endpoint_not_found",
  "path": "user.UserService.GetUser",
  "description": "Method user.UserService.GetUser was removed",
  "severity": "critical",
  "context": {
    "is_additive": false,
    "is_breaking": true,
    "change_category": "method_removed",
    "service": "user.UserService",
    "method": "GetUser"
  }
}
```

**Per-Service+Method Drift Reports:**

The drift evaluation response includes a detailed breakdown per service and method:

```json
{
  "drift_evaluation": {
    "operation_id": "user.UserService.GetUser",
    "breaking_changes": 1,
    "non_breaking_changes": 0,
    "fitness_test_results": [...],
    "consumer_impact": {...}
  }
}
```

Use the `generate_grpc_drift_report()` helper function to generate a comprehensive report:

```rust
use mockforge_core::contract_drift::generate_grpc_drift_report;

let report = generate_grpc_drift_report(&diff_result);
// Returns JSON with per-service and per-method breakdown
```

**Example Drift Incident for gRPC:**

```json
{
  "id": "incident-123",
  "protocol": "grpc",
  "endpoint": "user.UserService.GetUser",
  "method": "grpc",
  "incident_type": "breaking_change",
  "severity": "high",
  "details": {
    "operation_id": "user.UserService.GetUser",
    "operation_type": "GrpcMethod",
    "breaking_changes": 1,
    "non_breaking_changes": 0
  },
  "fitness_test_results": [
    {
      "function_name": "No New Required Fields",
      "passed": false,
      "message": "Required field 'email' added to GetUserResponse"
    }
  ],
  "affected_consumers": {
    "affected_sdk_methods": [
      {
        "sdk_name": "user-client-go",
        "method_name": "GetUser",
        "consuming_apps": [
          {
            "app_name": "Mobile App iOS",
            "app_type": "mobile_ios"
          }
        ]
      }
    ]
  }
}
```

### WebSocket Drift Detection

For WebSocket contracts, drift is detected at the message type level with message shape diff detection:

**Breaking Changes (is_breaking: true):**
- Message type removed
- Required field added to message schema
- Message direction changed (outbound → inbound)
- Topic/channel changed
- Property type changed in message schema
- Property removed from message schema

**Additive Changes (is_additive: true):**
- New message type added
- Optional field added to message schema
- Required field removed (field is now optional)
- New property added to message schema
- Description or metadata updated

**Message Shape Diff Detection:**

WebSocket contracts support JSON Schema diff detection for message payloads. Changes are classified and tracked:

```json
{
  "mismatch_type": "missing_required_field",
  "path": "user_joined.email",
  "description": "Field email became required",
  "severity": "critical",
  "context": {
    "is_additive": false,
    "is_breaking": true,
    "change_category": "required_field_added",
    "field_name": "email"
  }
}
```

**Schema Format Support:**

WebSocket contracts support JSON Schema for message validation and diff detection. The schema is validated and cached for efficient comparison.

**JSON Schema Example:**
```json
{
  "type": "object",
  "properties": {
    "user_id": { "type": "string" },
    "username": { "type": "string" },
    "timestamp": { "type": "number" }
  },
  "required": ["user_id", "username"]
}
```

**Avro Schema Support (Future):**
Avro schema support is planned for future releases. Currently, JSON Schema is the primary format for WebSocket message validation.

**Example Drift Incident for WebSocket:**

```json
{
  "id": "incident-456",
  "protocol": "websocket",
  "endpoint": "user_joined",
  "method": "websocket",
  "incident_type": "breaking_change",
  "severity": "medium",
  "details": {
    "operation_id": "user_joined",
    "operation_type": "WebSocketMessage",
    "breaking_changes": 1
  },
  "fitness_test_results": [
    {
      "function_name": "Response Size Limit",
      "passed": false,
      "message": "Message size increased by 35%, exceeding limit of 10%"
    }
  ]
}
```

### MQTT/Kafka Drift Detection

For message queue contracts, drift is detected at the topic level with message shape diff detection:

**Breaking Changes (is_breaking: true):**
- Topic removed
- Required field added to topic schema
- Schema format changed (JSON → Avro)
- QoS level changed (for MQTT)
- Property type changed in topic schema
- Property removed from topic schema

**Additive Changes (is_additive: true):**
- New topic added
- Optional field added to topic schema
- Required field removed (field is now optional)
- New property added to topic schema
- Description updated

**Message Shape Diff Detection:**

MQTT and Kafka contracts support schema diff detection for topic messages. The schema format can be:
- **JSON Schema**: Standard JSON Schema format (fully supported)
- **Avro**: Apache Avro schema format (for Kafka) - format defined, parsing planned
- **JSON-shape**: Simplified JSON shape format

**Schema Format Support Details:**

**JSON Schema (Currently Supported):**
- Full validation and diff detection
- Required field tracking with classification (additions = breaking, removals = additive)
- Property type change detection (breaking)
- Property addition/removal tracking (additions = additive, removals = breaking)

**Avro Schema (Format Defined):**
- Schema format is recognized in contract definitions
- Full Avro parsing and validation is planned for future releases
- Currently, JSON Schema is the primary supported format for MQTT/Kafka

**Example MQTT Topic Schema Diff:**

```json
{
  "mismatch_type": "type_mismatch",
  "path": "devices/+/telemetry.temperature",
  "description": "Property temperature type changed from number to string",
  "severity": "high",
  "context": {
    "is_additive": false,
    "is_breaking": true,
    "change_category": "property_type_changed",
    "field_name": "temperature",
    "old_type": "number",
    "new_type": "string"
  }
}
```

**Example Drift Incident for MQTT:**

```json
{
  "id": "incident-789",
  "protocol": "mqtt",
  "endpoint": "devices/+/telemetry",
  "method": "mqtt",
  "incident_type": "threshold_exceeded",
  "severity": "low",
  "details": {
    "operation_id": "devices/+/telemetry",
    "operation_type": "MqttTopic",
    "non_breaking_changes": 3
  },
  "fitness_test_results": [
    {
      "function_name": "Schema Complexity Limit",
      "passed": false,
      "message": "Schema depth increased to 6, exceeding limit of 4"
    }
  ]
}
```

### Drift Evaluation Response

When comparing protocol contracts, the response includes drift evaluation:

```json
{
  "matches": 8,
  "confidence": 0.95,
  "mismatches": [
    {
      "path": "user.UserService.GetUser",
      "severity": "critical",
      "type": "missing_required_field",
      "message": "Required field 'email' added to response"
    }
  ],
  "recommendations": [
    "Consider making 'email' optional to maintain backward compatibility"
  ],
  "corrections": [],
  "drift_evaluation": {
    "operation_id": "user.UserService.GetUser",
    "endpoint": "user.UserService.GetUser",
    "method": "grpc",
    "budget_exceeded": true,
    "breaking_changes": 1,
    "fitness_test_results": [
      {
        "function_name": "No New Required Fields",
        "passed": false,
        "message": "Required field 'email' added to GetUserResponse"
      }
    ],
    "consumer_impact": {
      "affected_sdk_methods": [
        {
          "sdk_name": "user-client-go",
          "method_name": "GetUser",
          "consuming_apps": [
            {
              "app_name": "Mobile App iOS",
              "app_type": "mobile_ios"
            }
          ]
        }
      ],
      "impact_summary": "1 SDK method affected across 1 application"
    }
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

### 5. gRPC Proto Diff Best Practices

**Classifying Changes:**
- Always review `is_additive` and `is_breaking` flags in mismatch context
- Use `generate_grpc_drift_report()` to get per-service+method breakdown
- Monitor breaking changes closely as they affect all consumers

**Example Workflow:**
1. Register gRPC contract version 1.0.0
2. Update proto file and register version 2.0.0
3. Compare contracts to detect drift
4. Review per-service+method report
5. Address breaking changes before deployment

### 6. WebSocket/MQ Message Shape Diff

**Schema Design:**
- Use JSON Schema for WebSocket messages
- Prefer optional fields for new additions
- Document message type evolution

**Diff Detection:**
- Monitor required field additions (breaking)
- Track property type changes (breaking)
- Allow new optional properties (additive)

**Example Schema Evolution:**
```json
// Version 1.0.0
{
  "type": "object",
  "properties": {
    "user_id": { "type": "string" },
    "username": { "type": "string" }
  },
  "required": ["user_id"]
}

// Version 2.0.0 (additive - safe)
{
  "type": "object",
  "properties": {
    "user_id": { "type": "string" },
    "username": { "type": "string" },
    "avatar_url": { "type": "string" }  // New optional field
  },
  "required": ["user_id"]
}

// Version 3.0.0 (breaking - avoid)
{
  "type": "object",
  "properties": {
    "user_id": { "type": "string" },
    "username": { "type": "string" },
    "avatar_url": { "type": "string" },
    "email": { "type": "string" }  // New required field - BREAKING!
  },
  "required": ["user_id", "email"]
}
```

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

## gRPC Proto Diff Classification

### Additive vs Breaking Classification

All gRPC contract diffs include classification metadata indicating whether changes are additive or breaking:

**Breaking Changes:**
- Method removed: `change_category: "method_removed"`, `is_breaking: true`
- Input type changed: `change_category: "input_type_changed"`, `is_breaking: true`
- Output type changed: `change_category: "output_type_changed"`, `is_breaking: true`
- Streaming config changed: `change_category: "streaming_config_changed"`, `is_breaking: true`
- Service removed: `change_category: "service_removed"`, `is_breaking: true`

**Additive Changes:**
- Method added: `change_category: "method_added"`, `is_additive: true`
- Service added: `change_category: "service_added"`, `is_additive: true`

### Per-Service+Method Drift Reports

Generate detailed drift reports per service and method:

```rust
use mockforge_core::contract_drift::generate_grpc_drift_report;

let diff_result = old_contract.diff(&new_contract).await?;
let report = generate_grpc_drift_report(&diff_result);
```

The report structure:
```json
{
  "services": {
    "user.UserService": {
      "additive_changes": 2,
      "breaking_changes": 1,
      "methods": {
        "GetUser": {
          "additive_changes": 0,
          "breaking_changes": 1,
          "total_changes": 1,
          "changes": [
            {
              "description": "Input type changed from GetUserRequest to GetUserRequestV2",
              "path": "user.UserService.GetUser.input",
              "severity": "high",
              "is_additive": false,
              "is_breaking": true,
              "change_category": "input_type_changed"
            }
          ]
        },
        "CreateUser": {
          "additive_changes": 1,
          "breaking_changes": 0,
          "total_changes": 1,
          "changes": [
            {
              "description": "New method CreateUser was added",
              "path": "user.UserService.CreateUser",
              "severity": "low",
              "is_additive": true,
              "is_breaking": false,
              "change_category": "method_added"
            }
          ]
        }
      }
    },
    "order.OrderService": {
      "additive_changes": 1,
      "breaking_changes": 0,
      "methods": {
        "ListOrders": {
          "additive_changes": 1,
          "breaking_changes": 0,
          "total_changes": 1,
          "changes": [
            {
              "description": "New optional field 'filter' added to ListOrdersRequest",
              "path": "order.OrderService.ListOrders.request.filter",
              "severity": "low",
              "is_additive": true,
              "is_breaking": false,
              "change_category": "property_added"
            }
          ]
        }
      }
    }
  },
  "total_mismatches": 3
}
```

**Using the Report:**

The per-service+method report helps you:
1. **Identify Breaking Changes**: Quickly see which services/methods have breaking changes
2. **Plan Rollouts**: Additive changes can be deployed immediately; breaking changes need coordination
3. **Consumer Impact**: Focus on breaking changes in methods with known consumers
4. **Version Planning**: Use breaking change counts to determine version bumps

## WebSocket/MQ Message Shape Diff

### Schema Format Support

WebSocket and MQTT contracts support multiple schema formats for message validation and diff detection:

**JSON Schema:**
```json
{
  "type": "object",
  "properties": {
    "user_id": { "type": "string" },
    "timestamp": { "type": "number" }
  },
  "required": ["user_id"]
}
```

**Avro (for Kafka):**
```json
{
  "type": "record",
  "name": "UserEvent",
  "fields": [
    { "name": "user_id", "type": "string" },
    { "name": "timestamp", "type": "long" }
  ]
}
```

**JSON-shape (simplified):**
```json
{
  "user_id": "string",
  "timestamp": "number"
}
```

### Schema Format Detection

MockForge automatically detects the schema format when comparing contracts:

1. **Avro Detection**: Detects Avro schemas by checking for `type: "record"` or `fields` array
2. **JSON Schema Detection**: Detects JSON Schema by checking for `$schema`, `properties`, or `required` fields
3. **JSON-shape Detection**: Detects simplified JSON-shape format (simple object with type strings)

The detected format is included in mismatch context as `schema_format`:

```json
{
  "mismatch_type": "type_mismatch",
  "path": "user_joined.email",
  "description": "Property email type changed from string to number",
  "severity": "high",
  "context": {
    "is_additive": false,
    "is_breaking": true,
    "change_category": "property_type_changed",
    "field_name": "email",
    "old_type": "string",
    "new_type": "number",
    "schema_format": "json_schema"
  }
}
```

### Schema Format Changes

Changing the schema format itself is detected as a breaking change:

```json
{
  "mismatch_type": "schema_format_changed",
  "path": "user-events",
  "description": "Schema format changed from json_schema to avro",
  "severity": "critical",
  "context": {
    "is_additive": false,
    "is_breaking": true,
    "change_category": "schema_format_changed",
    "old_format": "json_schema",
    "new_format": "avro"
  }
}
```

### Diff Detection

Message shape diffs detect:
- Required field additions (breaking)
- Required field removals (additive - field now optional)
- Property type changes (breaking)
- Property additions (additive)
- Property removals (breaking)
- Schema format changes (breaking)

All changes include classification metadata in the mismatch context with `schema_format` information.

### WebSocket Message Shape Diff Example

**Before (Version 1.0.0):**
```json
{
  "message_type": "user_joined",
  "schema": {
    "type": "object",
    "properties": {
      "user_id": { "type": "string" },
      "username": { "type": "string" },
      "timestamp": { "type": "number" }
    },
    "required": ["user_id", "username"]
  }
}
```

**After (Version 2.0.0):**
```json
{
  "message_type": "user_joined",
  "schema": {
    "type": "object",
    "properties": {
      "user_id": { "type": "string" },
      "username": { "type": "string" },
      "timestamp": { "type": "number" },
      "email": { "type": "string" },
      "avatar_url": { "type": "string" }
    },
    "required": ["user_id", "username", "email"]
  }
}
```

**Detected Changes:**
```json
{
  "mismatches": [
    {
      "mismatch_type": "missing_required_field",
      "path": "user_joined.email",
      "description": "Field email became required",
      "severity": "critical",
      "context": {
        "is_additive": false,
        "is_breaking": true,
        "change_category": "required_field_added",
        "field_name": "email",
        "schema_format": "json_schema"
      }
    },
    {
      "mismatch_type": "property_added",
      "path": "user_joined.avatar_url",
      "description": "New optional property avatar_url added",
      "severity": "low",
      "context": {
        "is_additive": true,
        "is_breaking": false,
        "change_category": "property_added",
        "field_name": "avatar_url",
        "schema_format": "json_schema"
      }
    }
  ]
}
```

### MQTT/Kafka Topic Schema Diff Example

**Before (Version 1.0.0 - JSON Schema):**
```json
{
  "topic": "devices/+/telemetry",
  "schema": {
    "type": "object",
    "properties": {
      "device_id": { "type": "string" },
      "temperature": { "type": "number" },
      "humidity": { "type": "number" }
    },
    "required": ["device_id", "temperature"]
  }
}
```

**After (Version 2.0.0 - JSON Schema with new field):**
```json
{
  "topic": "devices/+/telemetry",
  "schema": {
    "type": "object",
    "properties": {
      "device_id": { "type": "string" },
      "temperature": { "type": "number" },
      "humidity": { "type": "number" },
      "pressure": { "type": "number" }
    },
    "required": ["device_id", "temperature", "pressure"]
  }
}
```

**Detected Changes:**
```json
{
  "mismatches": [
    {
      "mismatch_type": "missing_required_field",
      "path": "devices/+/telemetry.pressure",
      "description": "Field pressure became required",
      "severity": "high",
      "context": {
        "is_additive": false,
        "is_breaking": true,
        "change_category": "required_field_added",
        "field_name": "pressure",
        "schema_format": "json_schema"
      }
    }
  ]
}
```

### Kafka Avro Schema Diff Example

**Before (Version 1.0.0 - Avro):**
```json
{
  "topic": "user-events",
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
  }
}
```

**After (Version 2.0.0 - Avro with new field):**
```json
{
  "topic": "user-events",
  "value_schema": {
    "format": "avro",
    "schema": {
      "type": "record",
      "name": "UserEvent",
      "fields": [
        { "name": "user_id", "type": "string" },
        { "name": "event_type", "type": "string" },
        { "name": "timestamp", "type": "long" },
        { "name": "metadata", "type": ["null", "string"], "default": null }
      ]
    }
  }
}
```

**Detected Changes:**
```json
{
  "mismatches": [
    {
      "mismatch_type": "property_added",
      "path": "user-events.metadata",
      "description": "New optional field metadata added",
      "severity": "low",
      "context": {
        "is_additive": true,
        "is_breaking": false,
        "change_category": "property_added",
        "field_name": "metadata",
        "schema_format": "avro"
      }
    }
  ]
}
```

## See Also

- [Drift Budgets](./DRIFT_BUDGETS.md) - Configure drift thresholds
- [Consumer Impact Analysis](./CONSUMER_IMPACT_ANALYSIS.md) - Understand downstream impact
- [Contract Fitness Functions](./CONTRACT_FITNESS.md) - Define contract quality rules and fitness functions
