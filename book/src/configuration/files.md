# Configuration Files

MockForge supports comprehensive configuration through YAML files as an alternative to environment variables. This page documents the configuration file format, options, and usage.

## Quick Start

### Initialize a New Configuration

```bash
# Create a new project with template configuration
mockforge init my-project

# Or initialize in current directory
mockforge init .
```

This creates a `mockforge.yaml` file with sensible defaults and example configurations.

### Validate Your Configuration

```bash
# Validate configuration file
mockforge config validate

# Validate specific file
mockforge config validate --config my-config.yaml
```

See the [Configuration Validation Guide](../reference/config-validation.md) for detailed validation instructions.

## Complete Configuration Template

For a **fully documented configuration template** with all available options, see:
**[config.template.yaml](https://github.com/SaaSy-Solutions/mockforge/blob/main/config.template.yaml)**

This template includes:
- Every configuration option with inline comments
- Default values and valid ranges
- Example configurations for common scenarios
- Links to detailed documentation

## Configuration File Location

MockForge looks for configuration files in the following order:

1. Path specified by `--config` CLI flag
2. Path specified by `MOCKFORGE_CONFIG_FILE` environment variable
3. Default location: `./mockforge.yaml` or `./mockforge.yml`
4. No configuration file (uses defaults)

## Basic Configuration Structure

```yaml
# MockForge Configuration Example
# This file demonstrates all available configuration options

# HTTP server configuration
http:
  port: 3000
  host: "0.0.0.0"
  openapi_spec: "examples/openapi-demo.json"
  cors_enabled: true
  request_timeout_secs: 30
  request_validation: "enforce"
  aggregate_validation_errors: true
  validate_responses: false
  response_template_expand: true
  skip_admin_validation: true

# WebSocket server configuration
websocket:
  port: 3001
  host: "0.0.0.0"
  replay_file: "examples/ws-demo.jsonl"
  connection_timeout_secs: 300

# gRPC server configuration
grpc:
  port: 50051
  host: "0.0.0.0"

# Admin UI configuration
admin:
  enabled: true
  port: 9080
  host: "127.0.0.1"
  mount_path: null
  api_enabled: true

# Core MockForge configuration
core:
  latency_enabled: true
  failures_enabled: false

# Logging configuration
logging:
  level: "info"
  json_format: false
  file_path: null
  max_file_size_mb: 10
  max_files: 5

# Data generation configuration
data:
  default_rows: 100
  default_format: "json"
  locale: "en"
```

## HTTP Server Configuration

### Basic Settings

```yaml
http:
  port: 3000                    # Server port
  host: "0.0.0.0"              # Bind address (0.0.0.0 for all interfaces)
  cors_enabled: true           # Enable CORS headers
  request_timeout_secs: 30     # Request timeout in seconds
```

### OpenAPI Integration

```yaml
http:
  openapi_spec: "path/to/spec.json"  # Path to OpenAPI specification
  # Alternative: use URL
  openapi_spec: "https://example.com/api-spec.yaml"
```

### Validation and Response Handling

```yaml
http:
  request_validation: "enforce"      # off|warn|enforce
  aggregate_validation_errors: true  # Combine multiple errors
  validate_responses: false          # Validate generated responses
  response_template_expand: true     # Enable {{uuid}}, {{now}} etc.
  skip_admin_validation: true        # Skip validation for admin endpoints
```

### Validation Overrides

```yaml
http:
  validation_overrides:
    "POST /users/{id}": "warn"      # Override validation level per endpoint
    "GET /internal/health": "off"  # Skip validation for specific endpoints
```

## WebSocket Server Configuration

```yaml
websocket:
  port: 3001                          # Server port
  host: "0.0.0.0"                    # Bind address
  replay_file: "path/to/replay.jsonl" # WebSocket replay file
  connection_timeout_secs: 300       # Connection timeout in seconds
```

## gRPC Server Configuration

```yaml
grpc:
  port: 50051       # Server port
  host: "0.0.0.0"  # Bind address
  proto_dir: null  # Directory containing .proto files
  tls: null        # TLS configuration (optional)
```

## Admin UI Configuration

### Standalone Mode (Default)

```yaml
admin:
  enabled: true
  port: 9080
  host: "127.0.0.1"
  api_enabled: true
```

### Embedded Mode

```yaml
admin:
  enabled: true
  mount_path: "/admin"  # Mount under HTTP server
  api_enabled: true     # Enable API endpoints
  # Note: port/host ignored when mount_path is set
```

## Core Configuration

### Latency Simulation

```yaml
core:
  latency_enabled: true
  default_latency:
    base_ms: 50
    jitter_ms: 20
    distribution: "fixed"  # fixed, normal, or pareto

  # For normal distribution
  # std_dev_ms: 10.0

  # For pareto distribution
  # pareto_shape: 2.0

  min_ms: 10      # Minimum latency
  max_ms: 5000    # Maximum latency (optional)

  # Per-operation overrides
  tag_overrides:
    auth: 100
    payments: 200
```

### Failure Injection

```yaml
core:
  failures_enabled: true
  failure_config:
    global_error_rate: 0.05  # 5% global error rate

    # Default status codes for failures
    default_status_codes: [500, 502, 503, 504]

    # Per-tag error rates and status codes
    tag_configs:
      auth:
        error_rate: 0.1      # 10% error rate for auth operations
        status_codes: [401, 403]
        error_message: "Authentication failed"
      payments:
        error_rate: 0.02     # 2% error rate for payments
        status_codes: [402, 503]
        error_message: "Payment processing failed"

    # Tag filtering
    include_tags: []         # Empty means all tags included
    exclude_tags: ["health", "metrics"]  # Exclude these tags
```

### Proxy Configuration

```yaml
core:
  proxy:
    upstream_url: "http://api.example.com"
    timeout_seconds: 30
```

## Logging Configuration

```yaml
logging:
  level: "info"           # debug|info|warn|error
  json_format: false      # Use JSON format for logs
  file_path: "logs/mockforge.log"  # Optional log file
  max_file_size_mb: 10    # Rotate when file reaches this size
  max_files: 5           # Keep this many rotated log files
```

## Data Generation Configuration

```yaml
data:
  default_rows: 100       # Default number of rows to generate
  default_format: "json"  # Default output format
  locale: "en"           # Locale for generated data

  # Custom faker templates
  templates:
    custom_user:
      name: "{{faker.name}}"
      email: "{{faker.email}}"
      department: "{{faker.word}}"

  # RAG (Retrieval-Augmented Generation) configuration
  rag:
    enabled: false
    api_endpoint: null
    api_key: null
    model: null
    context_window: 4000
```

## Advanced Configuration

### Request/Response Overrides

```yaml
# YAML patch overrides for requests/responses
overrides:
  - targets: ["operation:getUser"]     # Target specific operations
    patch:
      - op: add
        path: /metadata/requestId
        value: "{{uuid}}"
      - op: replace
        path: /user/createdAt
        value: "{{now}}"
      - op: add
        path: /user/score
        value: "{{rand.float}}"

  - targets: ["tag:Payments"]          # Target by tags
    patch:
      - op: replace
        path: /payment/status
        value: "FAILED"
```

### Latency Profiles

```yaml
# External latency profiles file
latency_profiles: "config/latency.yaml"

# Example latency configuration:
# operation:getUser:
#   fixed_ms: 120
#   jitter_ms: 80
#   fail_p: 0.0
#
# tag:Payments:
#   fixed_ms: 200
#   jitter_ms: 300
#   fail_p: 0.05
#   fail_status: 503
```

## Configuration Examples

### Development Configuration

```yaml
# Development setup with debugging and fast responses
http:
  port: 3000
  response_template_expand: true
  request_validation: "warn"

admin:
  enabled: true
  port: 9080

core:
  latency_enabled: false  # Disable latency for faster development

logging:
  level: "debug"
  json_format: false
```

### Testing Configuration

```yaml
# Testing setup with deterministic responses
http:
  port: 3000
  response_template_expand: false  # Disable random tokens for determinism

core:
  latency_enabled: false

data:
  rag:
    enabled: false  # Disable RAG for consistent test data
```

### Production Configuration

```yaml
# Production setup with monitoring and reliability
http:
  port: 80
  host: "0.0.0.0"
  request_validation: "enforce"
  cors_enabled: false

admin:
  enabled: false  # Disable admin UI in production

core:
  latency_enabled: true
  failures_enabled: false

logging:
  level: "warn"
  json_format: true
  file_path: "/var/log/mockforge.log"
```

## Configuration File Validation

MockForge validates configuration files at startup. Common issues:

1. **Invalid YAML syntax** - Check indentation and quotes
2. **Missing required fields** - Some fields like `request_timeout_secs` are required
3. **Invalid file paths** - Ensure OpenAPI spec and replay files exist
4. **Port conflicts** - Choose unique ports for each service

## Configuration Precedence

Configuration values are resolved in this priority order:

1. **CLI flags** (highest priority)
2. **Environment variables**
3. **Configuration file**
4. **Default values** (lowest priority)

This allows you to override specific values without changing your configuration file.

## Hot Reloading

Configuration changes require a server restart to take effect. For development, you can use:

```bash
# Watch for changes and auto-restart
cargo watch -x "run -p mockforge-cli -- serve --config config.yaml"
```

For more information on environment variables, see the [Environment Variables](environment.md) documentation.
