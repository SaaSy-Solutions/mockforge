# Advanced Behavior and Simulation

MockForge provides advanced behavior and simulation features that allow you to create realistic, stateful, and resilient API mocks. This guide covers record & playback, stateful behavior simulation, fault injection, latency simulation, and conditional proxying.

## Table of Contents

- [Record & Playback](#record--playback)
- [Stateful Behavior Simulation](#stateful-behavior-simulation)
- [Per-Route Fault Injection](#per-route-fault-injection)
- [Per-Route Latency Simulation](#per-route-latency-simulation)
- [Conditional Proxying](#conditional-proxying)
- [Browser Proxy with Conditional Forwarding](#browser-proxy-with-conditional-forwarding)

## Record & Playback

The record & playback feature allows you to capture real API interactions and convert them into replayable stub mappings.

### Quick Start

1. **Start recording** while proxying to a real service:

```bash
mockforge serve --spec api-spec.json --proxy --record
```

2. **Convert recordings** to stub mappings:

```bash
# Convert a specific recording
mockforge recorder convert --recording-id abc123 --output fixtures/user-api.yaml

# Batch convert all recordings
mockforge recorder convert --input recordings.db --output fixtures/ --format yaml
```

### Configuration

```yaml
core:
  recorder:
    enabled: true
    auto_convert: true
    output_dir: "./fixtures/recorded"
    format: "yaml"
    filters:
      min_status_code: 200
      max_status_code: 299
      exclude_paths:
        - "/health"
        - "/metrics"
```

### API Usage

```bash
# Convert via API
curl -X POST http://localhost:9080/api/recorder/convert/abc123 \
  -H "Content-Type: application/json" \
  -d '{"format": "yaml"}'
```

## Stateful Behavior Simulation

Stateful behavior simulation allows responses to change based on previous requests, using state machines to track resource state.

### Basic Example

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
          - name: "processing"
            response:
              status_code: 200
              body_template: '{"order_id": "{{resource_id}}", "status": "processing"}'
          - name: "shipped"
            response:
              status_code: 200
              body_template: '{"order_id": "{{resource_id}}", "status": "shipped"}'
        resource_id_extract:
          type: "path_param"
          param: "order_id"
        transitions:
          - method: "POST"
            path_pattern: "/api/orders"
            from_state: "initial"
            to_state: "pending"
          - method: "PUT"
            path_pattern: "/api/orders/{order_id}/process"
            from_state: "pending"
            to_state: "processing"
```

### Resource ID Extraction

Extract resource IDs from various sources:

```yaml
# From path parameter
resource_id_extract:
  type: "path_param"
  param: "order_id"

# From header
resource_id_extract:
  type: "header"
  name: "X-Resource-ID"

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

## Per-Route Fault Injection

Configure fault injection on specific routes with multiple fault types.

### Configuration

```yaml
core:
  routes:
    - path: "/api/payments/process"
      method: "POST"
      fault_injection:
        enabled: true
        probability: 0.05  # 5% chance
        fault_types:
          - type: "http_error"
            status_code: 503
            message: "Service unavailable"
          - type: "timeout"
            duration_ms: 5000
          - type: "connection_error"
            message: "Connection refused"
```

### Fault Types

- **HTTP Error**: Return specific status codes
- **Connection Error**: Simulate connection failures
- **Timeout**: Simulate request timeouts
- **Partial Response**: Truncate responses
- **Payload Corruption**: Corrupt response payloads

## Per-Route Latency Simulation

Simulate network latency with various distributions.

### Configuration

```yaml
core:
  routes:
    - path: "/api/search"
      method: "GET"
      latency:
        enabled: true
        probability: 0.8
        distribution: "normal"  # fixed, normal, exponential, uniform
        mean_ms: 500.0
        std_dev_ms: 100.0
        jitter_percent: 15.0
```

### Distributions

- **Fixed**: Constant delay with optional jitter
- **Normal**: Gaussian distribution (realistic for most APIs)
- **Exponential**: Exponential distribution (simulates network delays)
- **Uniform**: Random delay within a range

## Conditional Proxying

Proxy requests conditionally based on request attributes using expressions.

### Basic Examples

```yaml
core:
  proxy:
    enabled: true
    rules:
      # Proxy admin requests
      - pattern: "/api/admin/*"
        upstream_url: "https://admin-api.example.com"
        condition: "$.user.role == 'admin'"
      
      # Proxy authenticated requests
      - pattern: "/api/protected/*"
        upstream_url: "https://protected-api.example.com"
        condition: "header[authorization] != ''"
      
      # Proxy based on query parameter
      - pattern: "/api/data/*"
        upstream_url: "https://data-api.example.com"
        condition: "query[env] == 'production'"
```

### Condition Types

#### JSONPath Expressions

```yaml
condition: "$.user.role == 'admin'"
condition: "$.order.amount > 1000"
```

#### Header Checks

```yaml
condition: "header[authorization] != ''"
condition: "header[user-agent] == 'MobileApp/1.0'"
```

#### Query Parameters

```yaml
condition: "query[env] == 'production'"
condition: "query[version] == 'v2'"
```

#### Logical Operators

```yaml
# AND
condition: "AND(header[authorization] != '', $.user.role == 'admin')"

# OR
condition: "OR(query[env] == 'production', query[env] == 'staging')"

# NOT
condition: "NOT(query[env] == 'development')"
```

## Browser Proxy with Conditional Forwarding

The browser proxy mode supports the same conditional forwarding rules.

### Usage

```bash
# Start browser proxy with conditional rules
mockforge proxy --port 8081 --config config.yaml
```

Configure your browser/mobile app to use `127.0.0.1:8081` as the HTTP proxy. All requests will be evaluated against conditional rules before proxying.

### Example Configuration

```yaml
proxy:
  enabled: true
  rules:
    # Route admin users to production
    - pattern: "/api/admin/*"
      upstream_url: "https://admin-api.production.com"
      condition: "$.user.role == 'admin'"
    
    # Route authenticated users to staging
    - pattern: "/api/*"
      upstream_url: "https://api.staging.com"
      condition: "header[authorization] != ''"
```

## Priority Chain

MockForge processes requests through this priority chain:

1. **Replay** - Check for recorded fixtures
2. **Stateful** - Check for stateful response handling
3. **Route Chaos** - Apply per-route fault injection and latency
4. **Global Fail** - Apply global/tag-based failure injection
5. **Proxy** - Check for conditional proxying
6. **Mock** - Generate mock response from OpenAPI spec
7. **Record** - Record request for future replay

## Related Advanced Features

MockForge includes many additional advanced features that complement the basic advanced behavior:

- **[VBR Engine](vbr-engine.md)**: Virtual database layer with automatic CRUD generation
- **[Temporal Simulation](temporal-simulation.md)**: Time travel and time-based data mutations
- **[Scenario State Machines](scenario-state-machines.md)**: Visual flow editor for complex workflows
- **[MockAI](mockai.md)**: AI-powered intelligent response generation
- **[Chaos Lab](chaos-lab.md)**: Interactive network condition simulation
- **[Reality Slider](reality-slider.md)**: Unified control for mock environment realism

For a complete overview, see [Advanced Features](advanced-features.md).

## Best Practices

1. **Start simple** - Begin with basic configurations and add complexity gradually
2. **Test thoroughly** - Verify state transitions and conditions work as expected
3. **Monitor performance** - Latency injection can slow down tests
4. **Document conditions** - Keep conditional logic well-documented
5. **Use version control** - Track configuration changes over time

## Examples

See the [example configuration file](../../../tests/fixtures/configs/example-advanced-features.yaml) for comprehensive examples of all features.

For more details, see the [Advanced Behavior and Simulation documentation](../../../docs/ADVANCED_BEHAVIOR_SIMULATION.md).

