# MockForge CLI Reference

The MockForge CLI provides a comprehensive command-line interface for managing mock servers, generating test data, and orchestrating API mocking workflows. This document covers all available commands, options, and usage patterns.

## Installation

### From Source
```bash
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge
cargo build --release --bin mockforge
# Binary will be available at target/release/mockforge
```

### From Crates.io (when published)
```bash
cargo install mockforge-cli
```

## Global Options

All MockForge commands support the following global options:

- `-h, --help`: Display help information
- `-v, --log-level <LEVEL>`: Set log level (error, warn, info, debug, trace) [default: info]
- `--version`: Display version information

## Commands

### `serve` - Start Mock Servers

Start MockForge's mock servers with support for HTTP, WebSocket, gRPC, and other protocols.

```bash
mockforge serve [OPTIONS]
```

#### Server Options

**Port Configuration:**
- `--http-port <PORT>`: HTTP server port (default: 3000)
- `--ws-port <PORT>`: WebSocket server port (default: 3001)
- `--grpc-port <PORT>`: gRPC server port (default: 50051)
- `--smtp-port <PORT>`: SMTP server port (default: 1025)
- `--mqtt-port <PORT>`: MQTT server port (default: 1883)
- `--kafka-port <PORT>`: Kafka broker port (default: 9092)
- `--amqp-port <PORT>`: AMQP broker port (default: 5672)

**Configuration:**
- `-c, --config <PATH>`: Path to configuration file
- `-p, --profile <NAME>`: Configuration profile to use (dev, ci, demo, etc.)

**API Specification:**
- `--spec <PATH>`: OpenAPI spec file for HTTP server (JSON or YAML format)
- `--ws-replay-file <PATH>`: WebSocket replay file for message simulation
- `--graphql <PATH>`: GraphQL schema file
- `--graphql-port <PORT>`: GraphQL server port (default: 4000)
- `--graphql-upstream <URL>`: Upstream GraphQL server URL

#### Admin UI Options

- `--admin`: Enable admin UI
- `--admin-port <PORT>`: Admin UI port (default: 9080)

#### Observability Options

- `--metrics`: Enable Prometheus metrics endpoint
- `--metrics-port <PORT>`: Metrics server port (default: 9090)
- `--tracing`: Enable OpenTelemetry distributed tracing
- `--tracing-service-name <NAME>`: Service name for traces (default: mockforge)
- `--tracing-environment <ENV>`: Tracing environment (default: development)
- `--jaeger-endpoint <URL>`: Jaeger endpoint for trace export
- `--tracing-sampling-rate <RATE>`: Tracing sampling rate 0.0-1.0 (default: 1.0)

#### API Flight Recorder Options

- `--recorder`: Enable API Flight Recorder
- `--recorder-db <PATH>`: Recorder database file path (default: ./mockforge-recordings.db)
- `--recorder-no-api`: Disable recorder management API
- `--recorder-api-port <PORT>`: Recorder management API port
- `--recorder-max-requests <COUNT>`: Maximum number of recorded requests (default: 10000)
- `--recorder-retention-days <DAYS>`: Auto-delete recordings older than N days (default: 7)

#### Chaos Engineering Options

- `--chaos`: Enable chaos engineering (fault injection and reliability testing)
- `--chaos-scenario <SCENARIO>`: Predefined chaos scenario (network_degradation, service_instability, cascading_failure, peak_traffic, slow_backend)
- `--chaos-latency-ms <MS>`: Chaos latency: fixed delay in milliseconds
- `--chaos-latency-range <RANGE>`: Chaos latency: random delay range (min-max) in milliseconds
- `--chaos-latency-probability <PROB>`: Chaos latency probability 0.0-1.0 (default: 1.0)
- `--chaos-http-errors <CODES>`: Chaos fault injection: HTTP error codes (comma-separated)
- `--chaos-http-error-probability <PROB>`: Chaos fault injection: HTTP error probability 0.0-1.0 (default: 0.1)
- `--chaos-rate-limit <RPS>`: Chaos rate limit: requests per second
- `--chaos-bandwidth-limit <BYTES>`: Chaos: bandwidth limit in bytes/sec
- `--chaos-packet-loss <PERCENT>`: Chaos: packet loss percentage 0-100

#### Traffic Shaping Options

- `--traffic-shaping`: Enable traffic shaping
- `--bandwidth-limit <BYTES>`: Maximum bandwidth in bytes per second (default: 1000000)
- `--burst-size <BYTES>`: Maximum burst size in bytes (default: 10000)
- `--network-profile <PROFILE>`: Network condition profile (3g, 4g, 5g, satellite_leo, satellite_geo, congested, lossy, high_latency, intermittent, extremely_poor, perfect)
- `--list-network-profiles`: List all available network profiles with descriptions

#### AI Features Options

- `--ai-enabled`: Enable AI-powered features
- `--rag-provider <PROVIDER>`: RAG provider (openai, anthropic, local)
- `--rag-model <MODEL>`: RAG model name
- `--rag-api-key <KEY>`: AI/RAG API key (or set MOCKFORGE_RAG_API_KEY)

#### Validation Options

- `--dry-run`: Validate configuration and check port availability without starting servers
- `--progress`: Show progress indicators during server startup
- `--verbose`: Enable verbose logging output

#### Examples

```bash
# Basic HTTP mock server
mockforge serve --http-port 3000

# Full-stack with all protocols
mockforge serve \
  --http-port 3000 \
  --ws-port 3001 \
  --grpc-port 50051 \
  --admin \
  --metrics

# With OpenAPI specification
mockforge serve --spec api.yaml --admin

# With chaos engineering
mockforge serve --chaos --chaos-scenario network_degradation

# With traffic shaping
mockforge serve --traffic-shaping --bandwidth-limit 500000

# Dry run to validate configuration
mockforge serve --dry-run --verbose
```

### `generate` - Generate Mock Servers

Generate mock server code from OpenAPI specifications.

```bash
mockforge generate [OPTIONS]
```

#### Options

- `-c, --config <PATH>`: Path to mockforge.toml configuration file
- `-s, --spec <PATH>`: OpenAPI specification file (JSON or YAML)
- `-o, --output <PATH>`: Output directory path
- `--verbose`: Generate verbose output
- `--dry-run`: Validate config without generating
- `--watch`: Watch mode - regenerate when files change
- `--watch-debounce <MS>`: Watch debounce time in milliseconds (default: 500)
- `--progress`: Show progress bar during generation

#### Examples

```bash
# Generate from OpenAPI spec
mockforge generate --spec api.yaml --output ./generated

# Generate with progress bar and verbose output
mockforge generate --spec api.json --output ./mocks --progress --verbose

# Watch mode for development
mockforge generate --spec api.yaml --watch --progress

# Dry run to validate configuration
mockforge generate --config mockforge.toml --dry-run
```

### `data` - Generate Test Data

Generate synthetic test data using templates or schemas.

```bash
mockforge data <SUBCOMMAND>
```

#### Subcommands

**`template`** - Generate data from predefined templates
```bash
mockforge data template <TEMPLATE> [OPTIONS]
```

Options:
- `--rows <COUNT>`: Number of rows to generate (default: 10)
- `--format <FORMAT>`: Output format (json, csv, yaml) (default: json)
- `--output <PATH>`: Output file path
- `--rag`: Enable RAG mode for AI-powered generation
- `--rag-provider <PROVIDER>`: RAG provider (openai, anthropic, local)
- `--rag-model <MODEL>`: RAG model name
- `--rag-endpoint <URL>`: RAG endpoint URL
- `--rag-timeout <SECONDS>`: RAG request timeout (default: 30)
- `--rag-max-retries <COUNT>`: Maximum RAG retry attempts (default: 3)

**`schema`** - Generate data from JSON schema
```bash
mockforge data schema <SCHEMA> [OPTIONS]
```

Options:
- `--rows <COUNT>`: Number of rows to generate (default: 10)
- `--output <PATH>`: Output file path
- `--format <FORMAT>`: Output format (json, csv, yaml) (default: json)

#### Examples

```bash
# Generate user data using template
mockforge data template user --rows 100 --format json --output users.json

# Generate from JSON schema
mockforge data schema user-schema.json --rows 50 --output data.json

# Generate with AI/RAG
mockforge data template product --rows 20 --rag --rag-provider openai
```

### `plugin` - Plugin Management

Manage MockForge plugins.

```bash
mockforge plugin <SUBCOMMAND>
```

#### Subcommands

- `install <SOURCE>`: Install a plugin from various sources
- `uninstall <PLUGIN_ID>`: Uninstall a plugin
- `list`: List installed plugins
- `info <PLUGIN_ID>`: Show plugin information
- `update [PLUGIN_ID]`: Update a plugin to the latest version
- `validate <SOURCE>`: Validate a plugin without installing
- `clear-cache`: Clear plugin download cache
- `cache-stats`: Show cache statistics
- `search <QUERY>`: Search for plugins in the registry

#### Examples

```bash
# Install a plugin
mockforge plugin install auth-jwt@1.0.0

# List installed plugins
mockforge plugin list

# Update all plugins
mockforge plugin update --all
```

### `workspace` - Multi-tenant Workspace Management

Manage multi-tenant workspaces.

```bash
mockforge workspace <SUBCOMMAND>
```

#### Subcommands

- `list`: List all workspaces
- `create <NAME>`: Create a new workspace
- `delete <NAME>`: Delete a workspace
- `info <NAME>`: Show workspace information
- `switch <NAME>`: Switch to a workspace
- `export <NAME>`: Export workspace configuration
- `import <PATH>`: Import workspace configuration

### `import` - Import API Specifications

Import API specifications from various sources.

```bash
mockforge import <SUBCOMMAND>
```

#### Subcommands

- `openapi <PATH>`: Import OpenAPI specification
- `asyncapi <PATH>`: Import AsyncAPI specification
- `postman <PATH>`: Import Postman collection
- `insomnia <PATH>`: Import Insomnia workspace
- `curl <PATH>`: Import cURL commands
- `coverage <PATH>`: Analyze API coverage

### `client` - Client Code Generation

Generate client code for frontend frameworks.

```bash
mockforge client <SUBCOMMAND>
```

#### Subcommands

- `generate`: Generate client code
- `list`: List available frameworks

### `init` - Initialize Project

Initialize a new MockForge project.

```bash
mockforge init [NAME] [OPTIONS]
```

#### Options

- `--no-examples`: Skip creating example files

### `completion` - Shell Completion

Generate shell completion scripts.

```bash
mockforge completion <SHELL>
```

Supported shells: bash, zsh, fish, powershell, elvish

## Exit Codes

The MockForge CLI uses the following exit codes:

- `0`: Success
- `1`: General error
- `2`: Invalid arguments
- `3`: File not found
- `4`: Permission denied
- `5`: Network error
- `6`: Configuration error
- `7`: Generation error
- `8`: Server error

## Configuration Files

MockForge supports multiple configuration file formats and locations:

### File Formats
- `mockforge.yaml` / `mockforge.yml` (YAML format)
- `mockforge.json` (JSON format)
- `mockforge.toml` (TOML format)

### File Locations (in order of precedence)
1. Path specified with `--config` flag
2. `./mockforge.yaml` (current directory)
3. `./mockforge.yml` (current directory)
4. `./mockforge.json` (current directory)
5. `./mockforge.toml` (current directory)
6. `~/.config/mockforge/config.yaml` (user config directory)

### Environment Variables

Many configuration options can be overridden with environment variables:

- `MOCKFORGE_HTTP_PORT`: HTTP server port
- `MOCKFORGE_WS_PORT`: WebSocket server port
- `MOCKFORGE_GRPC_PORT`: gRPC server port
- `MOCKFORGE_ADMIN_PORT`: Admin UI port
- `MOCKFORGE_METRICS_PORT`: Metrics server port
- `MOCKFORGE_LOG_LEVEL`: Log level
- `MOCKFORGE_RAG_API_KEY`: RAG API key

## Progress Indicators

When using `--progress` flag, MockForge displays:

- **Progress bars**: For long-running operations with known total steps
- **Spinners**: For indeterminate operations
- **Step indicators**: For multi-step processes
- **Time estimates**: For operations with predictable duration

## Watch Mode

Watch mode (`--watch`) automatically regenerates mock servers when source files change:

- Monitors OpenAPI specification files
- Monitors configuration files
- Configurable debounce time (`--watch-debounce`)
- Supports multiple file types (YAML, JSON, TOML)

## Error Handling

MockForge provides comprehensive error handling:

- **Structured error messages**: Clear, actionable error descriptions
- **Suggestions**: Helpful suggestions for resolving common issues
- **Exit codes**: Consistent exit codes for programmatic handling
- **Validation**: Pre-flight validation of configurations and files

## Examples

### Complete Workflow Example

```bash
# 1. Initialize a new project
mockforge init my-api-project

# 2. Generate mock server from OpenAPI spec
mockforge generate --spec api.yaml --output ./generated --progress --verbose

# 3. Start the mock server with admin UI
mockforge serve --spec api.yaml --admin --metrics --progress

# 4. Generate test data
mockforge data template user --rows 100 --output test-data.json

# 5. Install plugins
mockforge plugin install auth-jwt@1.0.0
```

### Development Workflow

```bash
# Watch mode for development
mockforge generate --spec api.yaml --watch --progress

# In another terminal, start server with verbose logging
mockforge serve --spec api.yaml --admin --verbose --progress
```

### CI/CD Integration

```bash
# Validate configuration
mockforge serve --dry-run --config ci-config.yaml

# Generate mocks for testing
mockforge generate --spec api.yaml --output ./test-mocks --progress

# Start server for integration tests
mockforge serve --config ci-config.yaml --metrics --tracing
```

## Troubleshooting

### Common Issues

1. **Port already in use**: Use `--dry-run` to check port availability
2. **Configuration errors**: Use `--verbose` for detailed error information
3. **File not found**: Check file paths and permissions
4. **Network issues**: Verify network connectivity and firewall settings

### Debug Mode

Enable debug logging for detailed troubleshooting:

```bash
mockforge serve --log-level debug --verbose
```

### Getting Help

- Use `--help` for command-specific help
- Use `--dry-run` to validate configurations
- Use `--verbose` for detailed output
- Check the logs for detailed error information
