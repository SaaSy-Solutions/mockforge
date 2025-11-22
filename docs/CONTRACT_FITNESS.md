# Contract Fitness Functions

**Version:** 0.3.6
**Theme:** Contracts as a first-class 'fitness & impact' system

## Overview

Contract Fitness Functions are custom rules that evaluate contract quality and evolution patterns. They move beyond basic contract diff analysis into "fitness functions" that enforce architectural constraints, performance characteristics, and evolution rules.

Fitness functions are evaluated for each new contract version and drift diff, with results surfaced in the Admin UI and Drift Incident views.

## Configuration

Fitness rules are configured in your MockForge configuration file under `contracts.fitness_rules`:

```yaml
contracts:
  fitness_rules:
    - name: "Response size stable"
      scope: "/v1/mobile/*"
      type: response_size_delta
      max_percent_increase: 25

    - name: "No new required fields"
      scope: "/v1/public/*"
      type: no_new_required_fields

    - name: "Field Count Limit"
      scope: "service:user-service"
      type: field_count
      max_fields: 50

    - name: "Schema Complexity Limit"
      scope: "workspace:prod"
      type: schema_complexity
      max_depth: 5
```

**Note:** The `type` field uses snake_case format (`response_size_delta`, `no_new_required_fields`, `field_count`, `schema_complexity`).

## Rule Types

### 1. Response Size Delta (`response_size_delta`)

Enforces that response sizes don't increase beyond a specified percentage threshold.

**Parameters:**
- `max_percent_increase` (required): Maximum allowed percentage increase in response size

**Example:**
```yaml
- name: "Keep responses lean"
  scope: "global"
  type: response_size_delta
  max_percent_increase: 15.0
```

**Use Cases:**
- Preventing API bloat
- Maintaining mobile app performance
- Controlling bandwidth costs

### 2. No New Required Fields (`no_new_required_fields`)

Prevents adding new required fields to existing endpoints, which would break backward compatibility.

**Parameters:**
- None (rule is binary: pass if no new required fields, fail otherwise)

**Example:**
```yaml
- name: "Backward compatibility for mobile clients"
  scope: "/v1/mobile/*"
  type: no_new_required_fields
```

**Use Cases:**
- Ensuring backward compatibility
- Protecting mobile SDK consumers
- Maintaining API stability

### 3. Field Count (`field_count`)

Limits the total number of fields in a response schema.

**Parameters:**
- `max_fields` (required): Maximum number of fields allowed

**Example:**
```yaml
- name: "Keep schemas focused"
  scope: "service:user-service"
  type: field_count
  max_fields: 30
```

**Use Cases:**
- Preventing schema bloat
- Maintaining API simplicity
- Improving client-side parsing performance

### 4. Schema Complexity (`schema_complexity`)

Limits the maximum nesting depth of schema structures.

**Parameters:**
- `max_depth` (required): Maximum allowed nesting depth

**Example:**
```yaml
- name: "Limit nested structures"
  scope: "workspace:prod"
  type: schema_complexity
  max_depth: 4
```

**Use Cases:**
- Preventing overly complex schemas
- Improving code generation quality
- Maintaining API clarity

## Scope Patterns

Fitness rules can be scoped to different levels:

### Global Scope
Applies to all endpoints across all services:
```yaml
scope: "global"
```

### Workspace Scope
Applies to all endpoints in a specific workspace:
```yaml
scope: "workspace:prod"
scope: "workspace:staging"
```

### Service Scope
Applies to all endpoints in a specific service:
```yaml
scope: "service:user-service"
scope: "service:payment-service"
```

### Endpoint Pattern Scope
Applies to endpoints matching a pattern:
```yaml
scope: "/v1/mobile/*"        # All mobile v1 endpoints
scope: "/api/users/*"         # All user endpoints
scope: "/v2/*/public"         # All public v2 endpoints
```

Pattern matching supports:
- `*` - Matches any sequence of characters
- `**` - Matches across path segments
- Exact path matching for specific endpoints

## Evaluation

Fitness functions are evaluated:

1. **During Contract Comparison**: When comparing two contract versions, all applicable fitness rules are evaluated
2. **During Drift Detection**: When drift is detected, fitness rules are checked
3. **On Demand**: Fitness rules can be tested manually via the Admin UI

### Evaluation Results

Each fitness function evaluation produces a `FitnessTestResult`:

```json
{
  "function_id": "config-rule-0",
  "function_name": "Response Size Limit",
  "passed": false,
  "message": "Response size increased by 25.3%, exceeding limit of 10.0%",
  "metrics": {
    "old_size": 1024,
    "new_size": 1283,
    "percent_increase": 25.3,
    "threshold": 10.0
  }
}
```

## Integration with Drift Incidents

Fitness test results are automatically included in drift incidents:

- **Per-Endpoint View**: See fitness results for each endpoint in the drift view
- **Global Summary**: View aggregate fitness results across all endpoints
- **Incident Details**: Each drift incident shows pass/fail status for each fitness rule

### Incident Creation

If a fitness rule fails, it may trigger a drift incident (depending on your drift budget configuration):

```yaml
# Fitness rule failure can contribute to incident creation
drift_budgets:
  - endpoint: "/api/users"
    method: "GET"
    max_breaking_changes: 0
    # Fitness failures are tracked separately
```

## Protocol-Specific Fitness Rules

Fitness rules work across all protocols (HTTP, gRPC, WebSocket, MQTT, Kafka):

### gRPC Fitness Rules

Fitness rules evaluate gRPC service methods using operation IDs in the format `service.method`:

```yaml
contracts:
  fitness_rules:
    - name: "gRPC response size limit"
      scope: "user.UserService.*"  # All methods in UserService
      type: response_size_delta
      max_percent_increase: 20.0

    - name: "No breaking changes in payment service"
      scope: "payment.PaymentService.*"
      type: no_new_required_fields
```

### WebSocket Fitness Rules

For WebSocket contracts, scope by message type:

```yaml
contracts:
  fitness_rules:
    - name: "WebSocket message size limit"
      scope: "user_*"  # All user-related message types
      type: response_size_delta
      max_percent_increase: 15.0
```

### MQTT/Kafka Fitness Rules

For message queue contracts, scope by topic:

```yaml
contracts:
  fitness_rules:
    - name: "IoT telemetry schema limit"
      scope: "devices/+/telemetry"  # MQTT topic pattern
      type: field_count
      max_fields: 20
```

## Best Practices

### 1. Start Conservative
Begin with lenient thresholds and tighten them over time:

```yaml
# Start with 50% increase allowed
max_percent_increase: 50.0

# Gradually reduce to 10%
max_percent_increase: 10.0
```

### 2. Scope Appropriately
Use specific scopes for rules that only apply to certain endpoints:

```yaml
# Mobile endpoints need stricter rules
- name: "Mobile response size"
  scope: "/v1/mobile/*"
  type: response_size_delta
  max_percent_increase: 5.0

# Internal APIs can be more flexible
- name: "Internal API response size"
  scope: "/internal/*"
  type: response_size_delta
  max_percent_increase: 20.0
```

### 3. Combine Rules
Use multiple rules together for comprehensive coverage:

```yaml
# Prevent both size and complexity issues
- name: "Response size"
  scope: "global"
  type: response_size_delta
  max_percent_increase: 10.0

- name: "Schema complexity"
  scope: "global"
  type: schema_complexity
  max_depth: 5
```

### 4. Document Intent
Use descriptive names and consider adding comments:

```yaml
# Prevent breaking changes for mobile SDK v2.0
- name: "Mobile SDK v2.0 compatibility"
  scope: "/v1/mobile/*"
  type: no_new_required_fields
```

## Protocol Support

Fitness functions work across all supported protocols:

- **HTTP/REST**: Full support for OpenAPI contracts
- **gRPC**: Evaluates protobuf message sizes and field counts
- **WebSocket**: Evaluates message schema complexity
- **MQTT/Kafka**: Evaluates topic message schemas

## Examples

### Example 1: Mobile API Constraints

```yaml
contracts:
  fitness_rules:
    # Keep mobile responses small
    - name: "Mobile response size"
      scope: "/v1/mobile/*"
      type: response_size_delta
      max_percent_increase: 5.0

    # No breaking changes for mobile
    - name: "Mobile backward compatibility"
      scope: "/v1/mobile/*"
      type: no_new_required_fields

    # Limit mobile schema complexity
    - name: "Mobile schema simplicity"
      scope: "/v1/mobile/*"
      type: schema_complexity
      max_depth: 3
```

### Example 2: Service-Specific Rules

```yaml
contracts:
  fitness_rules:
    # User service should stay focused
    - name: "User service field limit"
      scope: "service:user-service"
      type: field_count
      max_fields: 25

    # Payment service needs strict compatibility
    - name: "Payment service compatibility"
      scope: "service:payment-service"
      type: no_new_required_fields
```

### Example 3: Production Workspace Rules

```yaml
contracts:
  fitness_rules:
    # Stricter rules for production
    - name: "Production response size"
      scope: "workspace:prod"
      type: response_size_delta
      max_percent_increase: 5.0

    - name: "Production schema complexity"
      scope: "workspace:prod"
      type: schema_complexity
      max_depth: 4
```

### Example 4: gRPC Service Fitness Rules

```yaml
contracts:
  fitness_rules:
    # Limit gRPC response sizes for mobile clients
    - name: "gRPC mobile response size"
      scope: "user.UserService.*"
      type: response_size_delta
      max_percent_increase: 10.0

    # Prevent breaking changes in payment gRPC service
    - name: "Payment gRPC compatibility"
      scope: "payment.PaymentService.*"
      type: no_new_required_fields

    # Keep gRPC message schemas focused
    - name: "gRPC message field limit"
      scope: "order.OrderService.*"
      type: field_count
      max_fields: 30

    # Limit gRPC message complexity
    - name: "gRPC schema depth limit"
      scope: "global"
      type: schema_complexity
      max_depth: 5
```

**Fitness Test Result Example for gRPC:**

When a gRPC contract change violates a fitness rule, the result includes protocol-specific context:

```json
{
  "function_id": "config-rule-3",
  "function_name": "gRPC mobile response size",
  "passed": false,
  "message": "Response size increased by 15.2%, exceeding limit of 10.0%",
  "metrics": {
    "old_size": 2048,
    "new_size": 2359,
    "percent_increase": 15.2,
    "threshold": 10.0,
    "service": "user.UserService",
    "method": "GetUser"
  },
  "context": {
    "protocol": "grpc",
    "operation_id": "user.UserService.GetUser"
  }
}
```

### Example 5: WebSocket Message Fitness Rules

```yaml
contracts:
  fitness_rules:
    # Keep WebSocket messages small for real-time performance
    - name: "WebSocket message size limit"
      scope: "user_*"
      type: response_size_delta
      max_percent_increase: 20.0

    # Prevent breaking changes in presence messages
    - name: "Presence message compatibility"
      scope: "presence:*"
      type: no_new_required_fields

    # Limit WebSocket message schema complexity
    - name: "WebSocket schema depth"
      scope: "global"
      type: schema_complexity
      max_depth: 4
```

**Fitness Test Result Example for WebSocket:**

```json
{
  "function_id": "config-rule-5",
  "function_name": "WebSocket message size limit",
  "passed": false,
  "message": "Message size increased by 25.5%, exceeding limit of 20.0%",
  "metrics": {
    "old_size": 512,
    "new_size": 642,
    "percent_increase": 25.5,
    "threshold": 20.0,
    "message_type": "user_joined",
    "schema_format": "json_schema"
  },
  "context": {
    "protocol": "websocket",
    "operation_id": "user_joined"
  }
}
```

### Example 6: MQTT/Kafka Topic Fitness Rules

```yaml
contracts:
  fitness_rules:
    # Keep IoT telemetry messages lean
    - name: "IoT telemetry size limit"
      scope: "devices/+/telemetry"
      type: response_size_delta
      max_percent_increase: 15.0

    # Limit field count for device messages
    - name: "Device message field limit"
      scope: "devices/*"
      type: field_count
      max_fields: 20

    # Prevent breaking changes in event streams
    - name: "Event stream compatibility"
      scope: "events/*"
      type: no_new_required_fields

    # Keep Kafka message schemas simple
    - name: "Kafka schema complexity"
      scope: "user-events"
      type: schema_complexity
      max_depth: 3
```

**Fitness Test Result Example for MQTT:**

```json
{
  "function_id": "config-rule-7",
  "function_name": "IoT telemetry size limit",
  "passed": false,
  "message": "Message size increased by 18.3%, exceeding limit of 15.0%",
  "metrics": {
    "old_size": 256,
    "new_size": 303,
    "percent_increase": 18.3,
    "threshold": 15.0,
    "topic": "devices/device1/telemetry",
    "schema_format": "json_schema"
  },
  "context": {
    "protocol": "mqtt",
    "operation_id": "devices/+/telemetry"
  }
}
```

### Example 7: Multi-Protocol Fitness Rules

```yaml
contracts:
  fitness_rules:
    # Global rule for all protocols
    - name: "Global response size limit"
      scope: "global"
      type: response_size_delta
      max_percent_increase: 25.0

    # HTTP-specific rules
    - name: "HTTP mobile API size"
      scope: "/v1/mobile/*"
      type: response_size_delta
      max_percent_increase: 5.0

    # gRPC-specific rules
    - name: "gRPC service compatibility"
      scope: "user.UserService.*"
      type: no_new_required_fields

    # WebSocket-specific rules
    - name: "WebSocket message complexity"
      scope: "presence:*"
      type: schema_complexity
      max_depth: 3

    # MQTT-specific rules
    - name: "MQTT topic field limit"
      scope: "devices/*"
      type: field_count
      max_fields: 15
```

## Troubleshooting

### Rule Not Evaluating

If a fitness rule isn't being evaluated:

1. Check the scope pattern matches your endpoints
2. Verify the rule is enabled (default: enabled)
3. Check that contract comparison is being performed

### False Positives

If a rule is failing incorrectly:

1. Review the metrics in the test result
2. Adjust thresholds if they're too strict
3. Consider narrowing the scope if the rule doesn't apply broadly

### Performance Impact

Fitness evaluation is designed to be fast:

- Rules are evaluated in parallel
- Results are cached when possible
- Evaluation happens asynchronously during contract comparison

## API Reference

### Fitness Function Registry

Fitness functions can be managed via the Admin UI or API:

- `GET /api/v1/drift/fitness-functions` - List all fitness functions
- `POST /api/v1/drift/fitness-functions` - Create a new fitness function
- `GET /api/v1/drift/fitness-functions/{id}` - Get a specific function
- `PATCH /api/v1/drift/fitness-functions/{id}` - Update a function
- `DELETE /api/v1/drift/fitness-functions/{id}` - Delete a function
- `POST /api/v1/drift/fitness-functions/{id}/test` - Test a function

## See Also

- [Protocol Contracts Documentation](./PROTOCOL_CONTRACTS.md) - Contract definitions across protocols
- [Drift Budgets](./DRIFT_BUDGETS.md) - Managing drift budgets and thresholds
- [Consumer Impact Analysis](./CONSUMER_IMPACT_ANALYSIS.md) - Understanding consumer impact
