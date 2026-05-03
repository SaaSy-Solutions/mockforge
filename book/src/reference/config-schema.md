# Configuration Schema

MockForge supports comprehensive configuration through YAML files. This schema reference documents all available configuration options, their types, defaults, and usage examples.

## Complete Configuration Template

For a **fully annotated configuration template** with all options documented inline, see:

**[config.template.yaml](https://github.com/SaaSy-Solutions/mockforge/blob/main/config.template.yaml)**

This template includes:
- Every configuration field with inline documentation
- Default values and valid ranges
- Example configurations for common scenarios
- Comments explaining each option's purpose

## Quick Start

```bash
# Initialize a new configuration
mockforge init my-project

# Validate your configuration
mockforge config validate

# Start with validated config
mockforge serve --config mockforge.yaml
```

See the [Configuration Validation Guide](config-validation.md) for validation best practices.

## File Format

Configuration files use YAML format with the following structure:

```yaml
# Top-level configuration sections
server:        # Server port and binding configuration
admin:         # Admin UI settings
validation:    # Request validation settings
response:      # Response processing options
chaos:         # Chaos engineering features
grpc:          # gRPC-specific settings
websocket:     # WebSocket-specific settings
logging:       # Logging configuration
```

## Server Configuration

### `server.http_port` (integer, default: 3000)
HTTP server port for REST API endpoints.

```yaml
server:
  http_port: 9080
```

### `server.ws_port` (integer, default: 3001)
WebSocket server port for real-time connections.

```yaml
server:
  ws_port: 8081
```

### `server.grpc_port` (integer, default: 50051)
gRPC server port for protocol buffer services.

```yaml
server:
  grpc_port: 9090
```

### `server.bind` (string, default: "0.0.0.0")
Network interface to bind servers to.

```yaml
server:
  bind: "127.0.0.1"  # Bind to localhost only
```

## Admin UI Configuration

### `admin.enabled` (boolean, default: false)
Enable the web-based admin interface.

```yaml
admin:
  enabled: true
```

### `admin.port` (integer, default: 9080)
Port for the admin UI server.

```yaml
admin:
  port: 9090
```

### `admin.embedded` (boolean, default: false)
Embed admin UI under the main HTTP server instead of running standalone.

```yaml
admin:
  embedded: true
```

### `admin.mount_path` (string, default: "/admin")
URL path where embedded admin UI is accessible.

```yaml
admin:
  embedded: true
  mount_path: "/mockforge-admin"
```

### `admin.standalone` (boolean, default: true)
Force standalone admin UI server (overrides embedded setting).

```yaml
admin:
  standalone: true
```

### `admin.disable_api` (boolean, default: false)
Disable admin API endpoints while keeping the UI interface.

```yaml
admin:
  disable_api: false
```

## Validation Configuration

### `validation.mode` (string, default: "enforce")
Request validation mode. Options: "off", "warn", "enforce"

```yaml
validation:
  mode: warn  # Log warnings but allow invalid requests
```

### `validation.aggregate_errors` (boolean, default: false)
Combine multiple validation errors into a single JSON array response.

```yaml
validation:
  aggregate_errors: true
```

### `validation.validate_responses` (boolean, default: false)
Validate response payloads against OpenAPI schemas (warn-only).

```yaml
validation:
  validate_responses: true
```

### `validation.status_code` (integer, default: 400)
HTTP status code to return for validation errors.

```yaml
validation:
  status_code: 422  # Use 422 Unprocessable Entity
```

### `validation.skip_admin_validation` (boolean, default: true)
Skip validation for admin UI routes.

```yaml
validation:
  skip_admin_validation: true
```

### `validation.overrides` (object)
Per-route validation overrides.

```yaml
validation:
  overrides:
    "/api/users": "off"      # Disable validation for this route
    "/api/admin/**": "warn"  # Warning mode for admin routes
```

## Response Configuration

### `response.template_expand` (boolean, default: false)
Enable template variable expansion in responses.

```yaml
response:
  template_expand: true
```

### `response.caching` (object)
Response caching configuration.

```yaml
response:
  caching:
    enabled: true
    ttl_seconds: 300
    max_size_mb: 100
```

## Chaos Engineering

### `chaos.latency_enabled` (boolean, default: false)
Enable response latency simulation.

```yaml
chaos:
  latency_enabled: true
```

### `chaos.latency_min_ms` (integer, default: 0)
Minimum response latency in milliseconds.

```yaml
chaos:
  latency_min_ms: 100
```

### `chaos.latency_max_ms` (integer, default: 1000)
Maximum response latency in milliseconds.

```yaml
chaos:
  latency_max_ms: 2000
```

### `chaos.failures_enabled` (boolean, default: false)
Enable random failure injection.

```yaml
chaos:
  failures_enabled: true
```

### `chaos.failure_rate` (float, default: 0.0)
Probability of random failures (0.0 to 1.0).

```yaml
chaos:
  failure_rate: 0.05  # 5% failure rate
```

### `chaos.failure_status_codes` (array of integers)
HTTP status codes to return for injected failures.

```yaml
chaos:
  failure_status_codes: [500, 502, 503, 504]
```

### Full Chaos Engineering Surface (v0.3.125+)

The simple flat fields above are kept for back-compat. The richer chaos
surface is configured under `observability.chaos` with these blocks:

#### `observability.chaos.fault_injection`

```yaml
observability:
  chaos:
    enabled: true
    fault_injection:
      enabled: true

      # HTTP errors
      http_errors: [500, 502, 503, 504]
      http_error_probability: 0.1
      error_pattern:               # optional, takes precedence over flat probability
        type: burst                # burst | random | sequential
        count: 3
        interval_ms: 1000

      # Connection errors
      connection_errors: false
      connection_error_probability: 0.05
      connection_error_kind: http_503   # http_503 | tcp_reset | tcp_close

      # Real timeouts (sleep then 504)
      timeout_errors: false
      timeout_ms: 5000
      timeout_probability: 0.05

      # Truncated responses (chunked-aware)
      partial_responses: false
      partial_response_probability: 0.05

      # Body corruption
      payload_corruption: false
      payload_corruption_probability: 0.05
      corruption_type: none        # none | random_bytes | truncate | bit_flip

      # Per-request matcher: gate fault injection on request properties.
      # AND across fields, OR within a list. Empty matcher = match all.
      request_matcher:
        source_ips:
          - "10.0.0.0/8"
          - "192.168.1.42"
        headers:
          - name: "x-test"
            value: "yes"            # omit `value` for presence-only
        min_body_size_bytes: 1048576
        max_body_size_bytes: 10485760
        chunked_only: true
```

#### `observability.chaos.rate_limit`

```yaml
rate_limit:
  enabled: true
  requests_per_second: 100
  burst_size: 10
  per_ip: true
  per_endpoint: false
```

#### `observability.chaos.traffic_shaping`

```yaml
traffic_shaping:
  enabled: true
  bandwidth_limit_bps: 1000000   # 1 MB/s
  packet_loss_percent: 2.0
  max_connections: 100
  connection_timeout_ms: 30000
```

#### `observability.chaos.circuit_breaker` and `bulkhead`

```yaml
circuit_breaker:
  enabled: true
  failure_threshold: 5
  success_threshold: 2
  timeout_ms: 60000

bulkhead:
  enabled: true
  max_concurrent_requests: 100
  max_queue_size: 10
```

For a tour of the resulting behavior — including which faults get injected
per-request vs per-connection, and how `connection_error_kind` interacts
with the chaos listener wrapper — see the
[Chaos Engineering chapter](../user-guide/chaos-engineering.md) and the
[reference doc](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/CHAOS_ENGINEERING.md).

## gRPC Configuration

### `grpc.proto_dir` (string, default: "proto/")
Directory containing Protocol Buffer files.

```yaml
grpc:
  proto_dir: "my-protos/"
```

### `grpc.enable_reflection` (boolean, default: true)
Enable gRPC server reflection for service discovery.

```yaml
grpc:
  enable_reflection: true
```

### `grpc.excluded_services` (array of strings)
gRPC services to exclude from automatic registration.

```yaml
grpc:
  excluded_services:
    - "grpc.reflection.v1alpha.ServerReflection"
```

### `grpc.max_message_size` (integer, default: 4194304)
Maximum message size in bytes (4MB default).

```yaml
grpc:
  max_message_size: 8388608  # 8MB
```

### `grpc.concurrency_limit` (integer, default: 32)
Maximum concurrent requests per connection.

```yaml
grpc:
  concurrency_limit: 64
```

## WebSocket Configuration

### `websocket.replay_file` (string)
Path to WebSocket replay file for scripted interactions.

```yaml
websocket:
  replay_file: "examples/ws-demo.jsonl"
```

### `websocket.max_connections` (integer, default: 1000)
Maximum concurrent WebSocket connections.

```yaml
websocket:
  max_connections: 500
```

### `websocket.message_timeout` (integer, default: 30000)
Timeout for WebSocket messages in milliseconds.

```yaml
websocket:
  message_timeout: 60000
```

### `websocket.heartbeat_interval` (integer, default: 30000)
Heartbeat interval for long-running connections.

```yaml
websocket:
  heartbeat_interval: 45000
```

## Logging Configuration

### `logging.level` (string, default: "info")
Log level. Options: "error", "warn", "info", "debug", "trace"

```yaml
logging:
  level: debug
```

### `logging.format` (string, default: "text")
Log output format. Options: "text", "json"

```yaml
logging:
  format: json
```

### `logging.file` (string)
Path to log file (if not specified, logs to stdout).

```yaml
logging:
  file: "/var/log/mockforge.log"
```

### `logging.max_size_mb` (integer, default: 10)
Maximum log file size in megabytes before rotation.

```yaml
logging:
  max_size_mb: 50
```

### `logging.max_files` (integer, default: 5)
Maximum number of rotated log files to keep.

```yaml
logging:
  max_files: 10
```

## Complete Configuration Example

```yaml
# Complete MockForge configuration example
server:
  http_port: 3000
  ws_port: 3001
  grpc_port: 50051
  bind: "0.0.0.0"

admin:
  enabled: true
  port: 9080
  embedded: false
  standalone: true

validation:
  mode: enforce
  aggregate_errors: false
  validate_responses: false
  status_code: 400

response:
  template_expand: true

chaos:
  latency_enabled: false
  failures_enabled: false

grpc:
  proto_dir: "proto/"
  enable_reflection: true
  max_message_size: 4194304

websocket:
  replay_file: "examples/ws-demo.jsonl"
  max_connections: 1000

logging:
  level: info
  format: text
```

## Configuration Precedence

Configuration values are applied in order of priority (highest to lowest):

1. **Command-line arguments** - Override all other settings
2. **Environment variables** - Override config file settings
3. **Configuration file** - Default values from YAML file
4. **Compiled defaults** - Built-in fallback values

## Environment Variable Mapping

A subset of config options can be overridden via env vars. The full list
is in [Environment Variables](../configuration/environment.md). Common ones:

```bash
# Server configuration
export MOCKFORGE_HTTP_PORT=9080
export MOCKFORGE_HTTP_HOST="127.0.0.1"
export MOCKFORGE_GRPC_PORT=50051
export MOCKFORGE_WS_PORT=3001

# Admin UI
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_ADMIN_PORT=9080

# Validation / templating
export MOCKFORGE_REQUEST_VALIDATION=warn
export MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true

# Protocol-specific (paths)
export MOCKFORGE_WS_REPLAY_FILE="replay.jsonl"
# Use --grpc-proto-dir or grpc.proto_dir in YAML for proto paths
```

## Validation

MockForge validates configuration files at startup and reports errors clearly:

```bash
# Validate configuration without starting server
mockforge-cli validate-config config.yaml

# Check for deprecated options
mockforge-cli validate-config --check-deprecated config.yaml
```

## Hot Reloading

Some configuration options support runtime updates without restart:

- Validation mode changes
- Template expansion toggle
- Admin UI settings
- Logging level adjustments

```bash
# Update validation mode at runtime
curl -X POST http://localhost:9080/__mockforge/config \
  -H "Content-Type: application/json" \
  -d '{"validation": {"mode": "warn"}}'
```

## Best Practices

### Development Configuration

```yaml
# development.yaml
server:
  http_port: 3000
  ws_port: 3001

admin:
  enabled: true
  embedded: true

validation:
  mode: warn

response:
  template_expand: true

logging:
  level: debug
```

### Production Configuration

```yaml
# production.yaml
server:
  http_port: 9080
  bind: "127.0.0.1"

admin:
  enabled: true
  standalone: true
  port: 9090

validation:
  mode: enforce

chaos:
  latency_enabled: false
  failures_enabled: false

logging:
  level: warn
  file: "/var/log/mockforge.log"
```

### Testing Configuration

```yaml
# test.yaml
server:
  http_port: 3000

validation:
  mode: off

response:
  template_expand: true

logging:
  level: debug
```

## Migration Guide

### Upgrading from CLI-only Configuration

If migrating from command-line only configuration:

1. Create a `config.yaml` file with your current settings
2. Test the configuration with `mockforge-cli validate-config`
3. Gradually move settings from environment variables to the config file
4. Update deployment scripts to use the config file

### Version Compatibility

Configuration options may change between versions. Check the changelog for breaking changes and use the validation command to identify deprecated options:

```bash
mockforge-cli validate-config --check-deprecated config.yaml
```

This schema provides comprehensive control over MockForge's behavior across all protocols and features.
