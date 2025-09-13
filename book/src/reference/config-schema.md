# Configuration Schema

MockForge supports comprehensive configuration through YAML files. This schema reference documents all available configuration options, their types, defaults, and usage examples.

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
  http_port: 8080
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

### `admin.port` (integer, default: 8080)
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
  port: 8080
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

All configuration options can be set via environment variables using the `MOCKFORGE_` prefix with underscore-separated paths:

```bash
# Server configuration
export MOCKFORGE_SERVER_HTTP_PORT=8080
export MOCKFORGE_SERVER_BIND="127.0.0.1"

# Admin UI
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_ADMIN_PORT=9090

# Validation
export MOCKFORGE_VALIDATION_MODE=warn
export MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true

# Protocol-specific
export MOCKFORGE_GRPC_PROTO_DIR="my-protos/"
export MOCKFORGE_WEBSOCKET_REPLAY_FILE="replay.jsonl"
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
curl -X POST http://localhost:8080/__mockforge/config \
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
  http_port: 8080
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
