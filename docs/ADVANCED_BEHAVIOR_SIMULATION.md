# Advanced Behavior and Simulation Features

This guide covers MockForge's advanced behavior and simulation features including record & playback, stateful behavior simulation, fault injection, latency simulation, and conditional proxying.

## Table of Contents

- [Record & Playback](#record--playback)
- [Stateful Behavior Simulation](#stateful-behavior-simulation)
- [Per-Route Fault Injection](#per-route-fault-injection)
- [Per-Route Latency Simulation](#per-route-latency-simulation)
- [Conditional Proxying](#conditional-proxying)
- [Browser Proxy with Conditional Forwarding](#browser-proxy-with-conditional-forwarding)

---

## Record & Playback

MockForge can automatically record API interactions and convert them into replayable stub mappings (fixtures) for later playback.

### Overview

The record & playback feature allows you to:
- **Proxy to a real service** and capture all requests/responses
- **Automatically convert** recorded exchanges into stub mappings
- **Replay** recorded interactions as fixtures
- **Generate fixtures** in JSON or YAML format

### Configuration

```yaml
core:
  recorder:
    enabled: true
    record_proxy: true  # Record proxied requests (default: true)
    auto_convert: true  # Automatically convert recordings to stub mappings
    output_dir: "./fixtures/recorded"
    format: "yaml"  # or "json"
    filters:
      min_status_code: 200
      max_status_code: 299
      exclude_paths:
        - "/health"
        - "/metrics"
```

When `record_proxy` is enabled, proxied requests will be recorded with metadata indicating they came from a proxy source. This metadata is included in the generated stub mappings.

### CLI Usage

#### Convert a Single Recording

```bash
# Convert a specific recording to a stub mapping
mockforge recorder convert --recording-id abc123 --output fixtures/user-api.yaml

# Convert to JSON format
mockforge recorder convert --recording-id abc123 --output fixtures/user-api.json --format json
```

#### Batch Conversion

```bash
# Convert all recordings from a database
mockforge recorder convert --input recordings.db --output fixtures/ --format yaml

# Convert with filters
mockforge recorder convert \
  --input recordings.db \
  --output fixtures/ \
  --format yaml \
  --min-status 200 \
  --max-status 299 \
  --exclude-paths "/health,/metrics"
```

### API Usage

```bash
# Convert a specific recording via API
curl -X POST http://localhost:9080/api/recorder/convert/abc123 \
  -H "Content-Type: application/json" \
  -d '{"format": "yaml"}'

# Batch convert recordings
curl -X POST http://localhost:9080/api/recorder/convert/batch \
  -H "Content-Type: application/json" \
  -d '{
    "format": "yaml",
    "filters": {
      "min_status_code": 200,
      "max_status_code": 299
    }
  }'
```

### Generated Stub Format

The converter automatically:
- Extracts request matchers (method, path, headers, body)
- Generates response templates with dynamic values replaced
- Detects UUIDs, timestamps, and other dynamic values
- Creates flexible matchers (regex for paths, JSONPath for bodies)

Example generated stub:

```yaml
request:
  method: POST
  urlPathPattern: "/api/users"
  headers:
    Content-Type:
      equalTo: "application/json"
  bodyPatterns:
    - matchesJsonPath: "$.name"
    - matchesJsonPath: "$.email"
response:
  status: 201
  headers:
    Content-Type: "application/json"
  body: |
    {
      "id": "{{uuid}}",
      "name": "{{request.body.name}}",
      "email": "{{request.body.email}}",
      "created_at": "{{timestamp}}"
    }
```

---

## Stateful Behavior Simulation

MockForge supports stateful behavior simulation where responses change based on previous requests, using state machines.

### Overview

Stateful simulation allows you to:
- **Track state** per resource (e.g., order ID, user ID)
- **Transition states** based on HTTP requests
- **Generate responses** based on current state
- **Support complex workflows** like order processing, user onboarding, etc.

### Configuration

```yaml
core:
  stateful:
    enabled: true
    state_machines:
      - name: "order_workflow"
        initial_state: "pending"
        states:
          - name: "pending"
            response:
              status_code: 200
              body_template: '{"order_id": "{{resource_id}}", "status": "pending"}'
              content_type: "application/json"
          - name: "processing"
            response:
              status_code: 200
              body_template: '{"order_id": "{{resource_id}}", "status": "processing"}'
              content_type: "application/json"
          - name: "shipped"
            response:
              status_code: 200
              body_template: '{"order_id": "{{resource_id}}", "status": "shipped", "tracking": "{{uuid}}"}'
              content_type: "application/json"
        # Resource ID extraction
        resource_id_extract:
          type: "path_param"
          param: "order_id"
        # State transitions
        transitions:
          - method: "POST"
            path_pattern: "/api/orders"
            from_state: "initial"
            to_state: "pending"
          - method: "PUT"
            path_pattern: "/api/orders/{order_id}/process"
            from_state: "pending"
            to_state: "processing"
          - method: "PUT"
            path_pattern: "/api/orders/{order_id}/ship"
            from_state: "processing"
            to_state: "shipped"
            condition: "$.ready == true"  # Optional condition
```

### Resource ID Extraction

You can extract resource IDs from various sources:

```yaml
# From path parameter
resource_id_extract:
  type: "path_param"
  param: "order_id"

# From header
resource_id_extract:
  type: "header"
  name: "X-Resource-ID"

# From query parameter
resource_id_extract:
  type: "query_param"
  param: "id"

# From JSONPath in request body
resource_id_extract:
  type: "json_path"
  path: "$.order.id"

# Composite (tries multiple sources)
resource_id_extract:
  type: "composite"
  extractors:
    - type: "path_param"
      param: "order_id"
    - type: "header"
      name: "X-Order-ID"
```

### State Transitions

State transitions are triggered by HTTP method + path combinations:

```yaml
transitions:
  - method: "POST"
    path_pattern: "/api/orders"
    from_state: "initial"
    to_state: "pending"
  - method: "PUT"
    path_pattern: "/api/orders/{order_id}/ship"
    from_state: "processing"
    to_state: "shipped"
    condition: "$.ready == true"  # Optional JSONPath condition
```

### Response Templates

Response templates support dynamic values:

- `{{resource_id}}` - The extracted resource ID
- `{{state}}` - Current state name
- `{{state_data.key}}` - Access state machine data
- `{{uuid}}`, `{{timestamp}}`, etc. - Standard template variables

### State Machine Configuration in Stubs

You can also configure state machines directly in response stubs using the SDK:

```rust
use mockforge_sdk::{ResponseStub, StateMachineConfig, ResourceIdExtractConfig, StateResponseOverride};
use serde_json::json;
use std::collections::HashMap;

let stub = ResponseStub::new("GET", "/orders/{id}", json!({"status": "pending"}))
    .with_state_machine(StateMachineConfig {
        resource_type: "order".to_string(),
        resource_id_extract: ResourceIdExtractConfig::PathParam {
            param: "id".to_string(),
        },
        initial_state: "pending".to_string(),
        state_responses: Some({
            let mut responses = HashMap::new();
            responses.insert("pending".to_string(), StateResponseOverride {
                status: Some(200),
                body: Some(json!({"status": "pending", "message": "Order is being processed"})),
                headers: None,
            });
            responses.insert("shipped".to_string(), StateResponseOverride {
                status: Some(200),
                body: Some(json!({"status": "shipped", "tracking": "{{uuid}}"})),
                headers: None,
            });
            responses
        }),
    });
```

The stateful handler will automatically process stubs with state machine configuration and apply state-based response overrides.

---

## Per-Route Fault Injection

Configure fault injection on a per-route basis with multiple fault types and probability-based injection.

### Configuration

```yaml
core:
  routes:
    - path: "/api/payments/process"
      method: "POST"
      response:
        status: 200
        body:
          message: "Payment processed"
      fault_injection:
        enabled: true
        probability: 0.05  # 5% chance of failure
        fault_types:
          - type: "http_error"
            status_code: 503
            message: "Payment service temporarily unavailable"
          - type: "timeout"
            duration_ms: 5000
            message: "Payment processing timeout"
          - type: "connection_error"
            message: "Connection refused"
          - type: "partial_response"
            truncate_percent: 50.0
          - type: "payload_corruption"
            corruption_type: "random_bytes"
```

### Fault Types

#### HTTP Error

```yaml
- type: "http_error"
  status_code: 500
  message: "Internal server error"
```

#### Connection Error

```yaml
- type: "connection_error"
  message: "Connection refused"
```

#### Timeout

```yaml
- type: "timeout"
  duration_ms: 5000
  message: "Request timeout after 5000ms"
```

#### Partial Response

```yaml
- type: "partial_response"
  truncate_percent: 50.0  # Truncate at 50% of response
```

#### Payload Corruption

```yaml
- type: "payload_corruption"
  corruption_type: "random_bytes"  # or "truncate", "bit_flip"
```

### Fault Injection in Stubs

You can also configure fault injection directly in response stubs using the SDK:

```rust
use mockforge_sdk::{ResponseStub, StubFaultInjectionConfig};
use serde_json::json;

// HTTP error injection
let stub = ResponseStub::new("POST", "/api/payments", json!({"status": "success"}))
    .with_fault_injection(StubFaultInjectionConfig::http_error(vec![500, 503]));

// Timeout error injection
let stub = ResponseStub::new("GET", "/api/slow", json!({}))
    .with_fault_injection(StubFaultInjectionConfig::timeout(5000));

// Connection error injection
let stub = ResponseStub::new("GET", "/api/unavailable", json!({}))
    .with_fault_injection(StubFaultInjectionConfig::connection_error());

// Custom fault injection with probabilities
let stub = ResponseStub::new("POST", "/api/unreliable", json!({}))
    .with_fault_injection(StubFaultInjectionConfig {
        enabled: true,
        http_errors: Some(vec![500, 502, 503]),
        http_error_probability: Some(0.1),  // 10% chance
        timeout_error: true,
        timeout_ms: Some(3000),
        timeout_probability: Some(0.05),  // 5% chance
        connection_error: false,
        connection_error_probability: None,
    });
```

---

## Per-Route Latency Simulation

Configure latency injection on a per-route basis with various delay distributions.

### Configuration

```yaml
core:
  routes:
    - path: "/api/search"
      method: "GET"
      response:
        status: 200
        body:
          results: []
      latency:
        enabled: true
        probability: 0.8  # 80% of requests get latency
        distribution: "exponential"  # fixed, normal, exponential, uniform
        lambda: 0.001  # For exponential: mean delay = 1000ms
        jitter_percent: 15.0
```

### Latency Distributions

#### Fixed Delay

```yaml
latency:
  enabled: true
  probability: 1.0
  fixed_delay_ms: 500
  jitter_percent: 20.0
  distribution: "fixed"
```

#### Normal Distribution

```yaml
latency:
  enabled: true
  probability: 1.0
  distribution: "normal"
  mean_ms: 500.0
  std_dev_ms: 100.0
  jitter_percent: 10.0
```

#### Exponential Distribution

```yaml
latency:
  enabled: true
  probability: 1.0
  distribution: "exponential"
  lambda: 0.001  # Mean delay = 1000ms
  jitter_percent: 15.0
```

#### Uniform Distribution

```yaml
latency:
  enabled: true
  probability: 1.0
  distribution: "uniform"
  random_delay_range_ms: [100, 500]
  jitter_percent: 10.0
```

---

## Conditional Proxying

Proxy requests conditionally based on request attributes using JSONPath, header checks, query parameters, and complex logical expressions.

### Overview

Conditional proxying allows you to:
- **Route based on request body** (JSONPath expressions)
- **Route based on headers** (authentication, user-agent, etc.)
- **Route based on query parameters** (environment, feature flags, etc.)
- **Combine conditions** with logical operators (AND, OR, NOT)

### Configuration

```yaml
core:
  proxy:
    enabled: true
    target_url: "http://api.example.com"
    rules:
      # Proxy admin requests to production
      - pattern: "/api/admin/*"
        upstream_url: "https://admin-api.production.com"
        condition: "$.user.role == 'admin'"  # JSONPath condition

      # Proxy authenticated requests
      - pattern: "/api/protected/*"
        upstream_url: "https://protected-api.staging.com"
        condition: "header[authorization] != ''"  # Header condition

      # Proxy based on query parameter
      - pattern: "/api/data/*"
        upstream_url: "https://data-api.example.com"
        condition: "query[env] == 'production'"  # Query param condition

      # Complex condition
      - pattern: "/api/orders/*"
        upstream_url: "https://orders-api.example.com"
        condition: "AND(header[authorization] != '', $.order.amount > 1000)"
```

### Condition Types

#### JSONPath Expressions

```yaml
condition: "$.user.role == 'admin'"
condition: "$.order.amount > 1000"
condition: "$.payment.method == 'credit_card'"
```

#### Header Checks

```yaml
condition: "header[authorization] != ''"
condition: "header[x-forwarded-for] != ''"
condition: "header[user-agent] == 'MobileApp/1.0'"
```

#### Query Parameter Checks

```yaml
condition: "query[env] == 'production'"
condition: "query[version] == 'v2'"
condition: "query[feature] == 'enabled'"
```

#### Logical Operators

```yaml
# AND - all conditions must be true
condition: "AND(header[authorization] != '', $.user.role == 'admin')"

# OR - any condition can be true
condition: "OR(query[env] == 'production', query[env] == 'staging')"

# NOT - negate a condition
condition: "NOT(query[env] == 'development')"

# Complex nested conditions
condition: "AND(header[authorization] != '', OR($.user.role == 'admin', $.user.role == 'moderator'))"
```

### Use Cases

#### A/B Testing

```yaml
- pattern: "/api/experiments/*"
  upstream_url: "https://experiment-a.example.com"
  condition: "$.user.experiment_group == 'A'"
- pattern: "/api/experiments/*"
  upstream_url: "https://experiment-b.example.com"
  condition: "$.user.experiment_group == 'B'"
```

#### Environment-Based Routing

```yaml
- pattern: "/api/*"
  upstream_url: "https://api.production.com"
  condition: "query[env] == 'production'"
- pattern: "/api/*"
  upstream_url: "https://api.staging.com"
  condition: "query[env] == 'staging'"
```

#### Feature Flag Routing

```yaml
- pattern: "/api/v2/*"
  upstream_url: "https://api-v2.example.com"
  condition: "$.user.features.v2_enabled == true"
```

---

## Browser Proxy with Conditional Forwarding

The browser proxy mode also supports conditional forwarding rules, allowing you to intercept and route browser/mobile app requests based on conditions.

### Configuration

The browser proxy uses the same conditional proxying configuration as the regular proxy:

```yaml
proxy:
  enabled: true
  target_url: "http://api.example.com"
  rules:
    - pattern: "/api/users/*"
      upstream_url: "https://users-api.example.com"
      condition: "header[authorization] != ''"
```

### Usage

Start the browser proxy:

```bash
mockforge proxy --port 8081 --config config.yaml
```

Configure your browser/mobile app to use `127.0.0.1:8081` as the HTTP proxy. All requests will be evaluated against conditional rules before proxying.

### Example: Testing Different Environments

```yaml
proxy:
  enabled: true
  target_url: "http://api.example.com"
  rules:
    # Route admin users to production
    - pattern: "/api/admin/*"
      upstream_url: "https://admin-api.production.com"
      condition: "$.user.role == 'admin'"

    # Route authenticated users to staging
    - pattern: "/api/*"
      upstream_url: "https://api.staging.com"
      condition: "header[authorization] != ''"

    # Route unauthenticated requests to mock
    - pattern: "/api/*"
      upstream_url: "http://localhost:3000"  # Local mock server
      condition: "header[authorization] == ''"
```

---

## Priority Chain

MockForge processes requests through a priority chain:

1. **Replay** - Check for recorded fixtures
2. **Stateful** - Check for stateful response handling
3. **Route Chaos** - Apply per-route fault injection and latency
4. **Global Fail** - Apply global/tag-based failure injection
5. **Proxy** - Check for conditional proxying
6. **Mock** - Generate mock response from OpenAPI spec
7. **Record** - Record request for future replay

This ensures that recorded fixtures take precedence, followed by stateful behavior, then chaos engineering features, then proxying, and finally mock generation.

---

## Best Practices

### Record & Playback

1. **Filter recordings** to exclude health checks and metrics
2. **Review generated stubs** before using in production
3. **Use batch conversion** for large datasets
4. **Version control** your generated fixtures

### Stateful Behavior

1. **Use clear state names** that reflect business logic
2. **Extract resource IDs** from stable sources (path params preferred)
3. **Test state transitions** thoroughly
4. **Use conditions** for complex transition logic

### Fault Injection

1. **Start with low probabilities** (1-5%) and increase gradually
2. **Test multiple fault types** to ensure resilience
3. **Monitor error rates** in production-like environments
4. **Use per-route configuration** for targeted testing

### Latency Simulation

1. **Match real-world distributions** (normal for most APIs)
2. **Use jitter** to avoid synchronized delays
3. **Test with various distributions** to find realistic patterns
4. **Consider probability** to simulate intermittent issues

### Conditional Proxying

1. **Use JSONPath** for request body-based routing
2. **Use headers** for authentication-based routing
3. **Use query params** for environment/feature flag routing
4. **Test conditions** thoroughly with various request types
5. **Document conditions** clearly for team understanding

---

## Troubleshooting

### Record & Playback

**Issue**: Generated stubs don't match requests
- **Solution**: Check that dynamic values are properly templated
- **Solution**: Review path patterns and ensure they're flexible enough

### Stateful Behavior

**Issue**: State not transitioning
- **Solution**: Verify resource ID extraction is working
- **Solution**: Check that path patterns match exactly
- **Solution**: Ensure conditions (if any) evaluate correctly

### Fault Injection

**Issue**: Faults not being injected
- **Solution**: Check that probability is > 0
- **Solution**: Verify route pattern matches
- **Solution**: Ensure fault injection is enabled

### Conditional Proxying

**Issue**: Requests not being proxied
- **Solution**: Verify condition syntax is correct
- **Solution**: Check that request attributes match condition
- **Solution**: Test condition in isolation first
- **Solution**: Check logs for condition evaluation results

---

## Examples

See `tests/fixtures/configs/example-advanced-features.yaml` for comprehensive examples of all features.
