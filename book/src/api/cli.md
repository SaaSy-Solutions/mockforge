# CLI Reference

MockForge provides a comprehensive command-line interface for managing mock servers and generating test data. This reference covers all available commands, options, and usage patterns.

## Global Options

All MockForge commands support the following global options:

```bash
mockforge-cli [OPTIONS] <COMMAND>
```

### Global Options

- `-h, --help`: Display help information

## Commands

### `serve` - Start Mock Servers

The primary command for starting MockForge's mock servers with support for HTTP, WebSocket, and gRPC protocols.

```bash
mockforge-cli serve [OPTIONS]
```

#### Server Options

**Port Configuration:**
- `--http-port <PORT>`: HTTP server port (default: 3000)
- `--ws-port <PORT>`: WebSocket server port (default: 3001)
- `--grpc-port <PORT>`: gRPC server port (default: 50051)

**API Specification:**
- `--spec <PATH>`: Path to OpenAPI specification file (JSON or YAML format)

**Configuration:**
- `-c, --config <PATH>`: Path to configuration file

#### Admin UI Options

**Admin UI Control:**
- `--admin`: Enable admin UI
- `--admin-port <PORT>`: Admin UI port (default: 8080)
- `--admin-embed`: Force embedding Admin UI under HTTP server
- `--admin-mount-path <PATH>`: Explicit mount path for embedded Admin UI (implies `--admin-embed`)
- `--admin-standalone`: Force standalone Admin UI on separate port (overrides embed)
- `--disable-admin-api`: Disable Admin API endpoints (UI loads but API routes are absent)

#### Validation Options

**Request Validation:**
- `--validation <MODE>`: Request validation mode (default: enforce)
  - `off`: Disable validation
  - `warn`: Log warnings but allow requests
  - `enforce`: Reject invalid requests
- `--aggregate-errors`: Aggregate request validation errors into JSON array
- `--validate-responses`: Validate responses (warn-only)
- `--validation-status <CODE>`: Validation error HTTP status code (default: 400)

#### Response Processing

**Template Expansion:**
- `--response-template-expand`: Expand templating tokens in responses/examples

#### Chaos Engineering

**Latency Simulation:**
- `--latency-enabled`: Enable latency simulation

**Failure Injection:**
- `--failures-enabled`: Enable failure injection

#### Examples

**Basic HTTP Server:**
```bash
mockforge-cli serve --spec examples/openapi-demo.json --http-port 3000
```

**Full Multi-Protocol Setup:**
```bash
mockforge-cli serve \
  --spec examples/openapi-demo.json \
  --http-port 3000 \
  --ws-port 3001 \
  --grpc-port 50051 \
  --admin \
  --admin-port 8080 \
  --response-template-expand
```

**Development Configuration:**
```bash
mockforge-cli serve \
  --config demo-config.yaml \
  --validation warn \
  --response-template-expand \
  --latency-enabled
```

**Production Configuration:**
```bash
mockforge-cli serve \
  --config production-config.yaml \
  --validation enforce \
  --admin-standalone
```

### `data` - Generate Synthetic Data

Generate synthetic test data using various templates and schemas.

```bash
mockforge-cli data <SUBCOMMAND>
```

#### Subcommands

##### `template` - Generate from Built-in Templates

Generate data using MockForge's built-in data generation templates.

```bash
mockforge-cli data template [OPTIONS]
```

**Options:**
- `--count <N>`: Number of items to generate (default: 1)
- `--format <FORMAT>`: Output format (json, yaml, csv)
- `--template <NAME>`: Template name (user, product, order, etc.)
- `--output <PATH>`: Output file path

**Examples:**

```bash
# Generate 10 user records as JSON
mockforge-cli data template --template user --count 10 --format json

# Generate product data to file
mockforge-cli data template --template product --count 50 --output products.json
```

##### `schema` - Generate from JSON Schema

Generate data conforming to a JSON Schema specification.

```bash
mockforge-cli data schema [OPTIONS] <SCHEMA>
```

**Parameters:**
- `<SCHEMA>`: Path to JSON Schema file

**Options:**
- `--count <N>`: Number of items to generate (default: 1)
- `--format <FORMAT>`: Output format (json, yaml)
- `--output <PATH>`: Output file path

**Examples:**

```bash
# Generate data from user schema
mockforge-cli data schema --count 5 user-schema.json

# Generate and save to file
mockforge-cli data schema --count 100 --output generated-data.json api-schema.json
```

##### `open-api` - Generate from OpenAPI Spec

Generate mock data based on OpenAPI specification schemas.

```bash
mockforge-cli data open-api [OPTIONS] <SPEC>
```

**Parameters:**
- `<SPEC>`: Path to OpenAPI specification file

**Options:**
- `--endpoint <PATH>`: Specific endpoint to generate data for
- `--method <METHOD>`: HTTP method (get, post, put, delete)
- `--count <N>`: Number of items to generate (default: 1)
- `--format <FORMAT>`: Output format (json, yaml)
- `--output <PATH>`: Output file path

**Examples:**

```bash
# Generate data for all endpoints in OpenAPI spec
mockforge-cli data open-api api-spec.yaml

# Generate data for specific endpoint
mockforge-cli data open-api --endpoint /users --method get --count 20 api-spec.yaml

# Generate POST request body data
mockforge-cli data open-api --endpoint /users --method post api-spec.yaml
```

### `admin` - Admin UI Server

Start the Admin UI as a standalone server without the main mock servers.

```bash
mockforge-cli admin [OPTIONS]
```

#### Options

- `--port <PORT>`: Server port (default: 8080)

#### Examples

```bash
# Start admin UI on default port
mockforge-cli admin

# Start admin UI on custom port
mockforge-cli admin --port 9090
```

## Configuration File Format

MockForge supports YAML configuration files that can be used instead of command-line options.

### Basic Configuration Structure

```yaml
# Server configuration
server:
  http_port: 3000
  ws_port: 3001
  grpc_port: 50051

# API specification
spec: examples/openapi-demo.json

# Admin UI configuration
admin:
  enabled: true
  port: 8080
  embedded: false
  mount_path: "/admin"
  standalone: true
  disable_api: false

# Validation settings
validation:
  mode: enforce
  aggregate_errors: false
  validate_responses: false
  status_code: 400

# Response processing
response:
  template_expand: true

# Chaos engineering
chaos:
  latency_enabled: false
  failures_enabled: false

# Protocol-specific settings
grpc:
  proto_dir: "proto/"
  enable_reflection: true

websocket:
  replay_file: "examples/ws-demo.jsonl"
```

### Configuration Precedence

Configuration values are applied in the following order (later sources override earlier ones):

1. **Default values** (compiled into the binary)
2. **Configuration file** (`-c/--config` option)
3. **Environment variables**
4. **Command-line arguments** (highest priority)

### Environment Variables

All configuration options can be set via environment variables using the `MOCKFORGE_` prefix:

```bash
# Server ports
export MOCKFORGE_HTTP_PORT=3000
export MOCKFORGE_WS_PORT=3001
export MOCKFORGE_GRPC_PORT=50051

# Admin UI
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_ADMIN_PORT=8080

# Validation
export MOCKFORGE_VALIDATION_MODE=enforce
export MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true

# gRPC settings
export MOCKFORGE_PROTO_DIR=proto/
export MOCKFORGE_GRPC_REFLECTION_ENABLED=true

# WebSocket settings
export MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl
```

## Exit Codes

MockForge uses standard exit codes:

- **0**: Success
- **1**: General error
- **2**: Configuration error
- **3**: Validation error
- **4**: File I/O error
- **5**: Network error

## Logging

MockForge provides configurable logging output to help with debugging and monitoring.

### Log Levels

- `error`: Only error messages
- `warn`: Warnings and errors
- `info`: General information (default)
- `debug`: Detailed debugging information
- `trace`: Very verbose tracing information

### Log Configuration

```bash
# Set log level via environment variable
export RUST_LOG=mockforge=debug

# Or via configuration file
logging:
  level: debug
  format: json
```

### Log Output

Logs include structured information about:
- HTTP requests/responses
- WebSocket connections and messages
- gRPC calls and streaming
- Configuration loading
- Template expansion
- Validation errors

## Examples

### Complete Development Setup

```bash
# Start all servers with admin UI
mockforge-cli serve \
  --spec examples/openapi-demo.json \
  --http-port 3000 \
  --ws-port 3001 \
  --grpc-port 50051 \
  --admin \
  --admin-port 8080 \
  --response-template-expand \
  --validation warn
```

### CI/CD Testing Pipeline

```bash
#!/bin/bash
# test-mockforge.sh

# Start MockForge in background
mockforge-cli serve --spec api-spec.yaml --http-port 3000 &
MOCKFORGE_PID=$!

# Wait for server to start
sleep 5

# Run API tests
npm test

# Generate test data
mockforge-cli data open-api --endpoint /users --count 100 api-spec.yaml > test-users.json

# Stop MockForge
kill $MOCKFORGE_PID
```

### Load Testing Setup

```bash
#!/bin/bash
# load-test-setup.sh

# Start MockForge with minimal validation for performance
MOCKFORGE_VALIDATION_MODE=off \
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=false \
mockforge-cli serve \
  --spec load-test-spec.yaml \
  --http-port 3000 \
  --validation off

# Now run your load testing tool against localhost:3000
# Example: hey -n 10000 -c 100 http://localhost:3000/api/test
```

### Docker Integration

```bash
# Run MockForge in Docker with CLI commands
docker run --rm -v $(pwd)/examples:/examples \
  mockforge \
  serve --spec /examples/openapi-demo.json --http-port 3000
```

## Troubleshooting

### Common Issues

**Server won't start:**
```bash
# Check if ports are available
lsof -i :3000
lsof -i :3001

# Try different ports
mockforge-cli serve --http-port 3001 --ws-port 3002
```

**Configuration not loading:**
```bash
# Validate YAML syntax
yamllint config.yaml

# Check file permissions
ls -la config.yaml
```

**OpenAPI spec not found:**
```bash
# Verify file exists and path is correct
ls -la examples/openapi-demo.json

# Use absolute path
mockforge-cli serve --spec /full/path/to/examples/openapi-demo.json
```

**Template expansion not working:**
```bash
# Ensure template expansion is enabled
mockforge-cli serve --response-template-expand --spec api-spec.yaml
```

### Debug Mode

Run with debug logging for detailed information:

```bash
RUST_LOG=mockforge=debug mockforge-cli serve --spec api-spec.yaml
```

### Health Checks

Test basic functionality:

```bash
# HTTP health check
curl http://localhost:3000/health

# WebSocket connection test
websocat ws://localhost:3001/ws

# gRPC service discovery
grpcurl -plaintext localhost:50051 list
```

This CLI reference provides comprehensive coverage of MockForge's command-line interface. For programmatic usage, see the [Rust API Reference](rust.md).
