# MockForge CLI

Command-line interface for MockForge - the comprehensive API mocking framework.

MockForge CLI provides a powerful command-line interface to manage MockForge servers, generate synthetic data, perform load testing, and orchestrate chaos experiments. It's the primary tool for interacting with MockForge in development, testing, and CI/CD environments.

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

## Quick Start

### Start Mock Servers

```bash
# Start all servers with default configuration
mockforge serve

# Start with custom ports and progress indicators
mockforge serve --http-port 8080 --ws-port 8081 --grpc-port 9090 --progress

# Start with OpenAPI spec and verbose logging
mockforge serve --spec api.yaml --verbose

# Start with admin UI, metrics, and progress bar
mockforge serve --admin --metrics --progress
```

### Generate Mock Servers

```bash
# Generate mock server from OpenAPI spec
mockforge generate --spec api.yaml --output ./generated

# Generate with progress bar and verbose output
mockforge generate --spec api.json --output ./mocks --progress --verbose

# Watch mode for development (auto-regenerate on file changes)
mockforge generate --spec api.yaml --watch --progress

# Dry run to validate configuration
mockforge generate --config mockforge.toml --dry-run
```

### Generate Test Data

```bash
# Generate user data
mockforge data template user --rows 100 --format json

# Generate from JSON schema
mockforge data schema my-schema.json --rows 50 --output data.json
```

### Load Testing

```bash
# Load test an API
mockforge bench --spec api.yaml --target https://api.example.com --vus 50 --duration 30s
```

## Enhanced Features

### Progress Indicators & Feedback

MockForge CLI provides comprehensive progress tracking and user feedback:

- **Progress Bars**: Visual progress indicators for long-running operations
- **Spinners**: Animated spinners for indeterminate operations
- **Step Indicators**: Clear step-by-step progress for multi-step processes
- **Verbose Logging**: Detailed output with `--verbose` flag
- **Error Handling**: Structured error messages with helpful suggestions

### Watch Mode

Development-friendly watch mode automatically regenerates mock servers when files change:

```bash
# Watch OpenAPI spec for changes
mockforge generate --spec api.yaml --watch --progress

# Custom debounce time (default: 500ms)
mockforge generate --spec api.yaml --watch --watch-debounce 1000
```

### Exit Codes

Consistent exit codes for programmatic integration:

- `0`: Success
- `1`: General error
- `2`: Invalid arguments
- `3`: File not found
- `4`: Permission denied
- `5`: Network error
- `6`: Configuration error
- `7`: Generation error
- `8`: Server error

## Core Commands

### Server Management (`serve`)

Start MockForge servers with comprehensive configuration options:

```bash
mockforge serve [OPTIONS]
```

#### Key Options

- **Ports**: `--http-port`, `--ws-port`, `--grpc-port`, `--admin-port`, `--metrics-port`
- **Protocols**: `--spec` (OpenAPI), `--ws-replay-file` (WebSocket replay)
- **Observability**: `--metrics`, `--tracing`, `--recorder`
- **Chaos Engineering**: `--chaos`, `--traffic-shaping`
- **AI Features**: `--ai-enabled`, `--rag-provider`
- **Progress & Feedback**: `--progress`, `--verbose`, `--dry-run`

#### Examples

```bash
# Basic HTTP mock server
mockforge serve --http-port 3000

# Full-stack with all protocols and progress indicators
mockforge serve \
  --http-port 3000 \
  --ws-port 3001 \
  --grpc-port 50051 \
  --admin \
  --metrics

# With chaos engineering
mockforge serve \
  --chaos \
  --chaos-scenario network_degradation \
  --chaos-latency-ms 200

# With traffic shaping
mockforge serve \
  --traffic-shaping \
  --bandwidth-limit 1000000 \
  --network-profile 3g
```

### Protocol-Specific Commands

#### MQTT Broker (`mqtt`)

```bash
# Start MQTT broker
mockforge mqtt serve --port 1883

# Publish message
mockforge mqtt publish --topic "sensors/temp" --payload '{"temp": 22.5}'

# Subscribe to topic
mockforge mqtt subscribe --topic "sensors/#"

# Manage topics
mockforge mqtt topics list
```

#### FTP Server (`ftp`)

```bash
# Start FTP server
mockforge ftp serve --port 2121

# Manage virtual filesystem
mockforge ftp vfs add /test.txt --content "Hello World"

# Load fixtures
mockforge ftp fixtures load ./fixtures/ftp/
```

#### Kafka Broker (`kafka`) [requires kafka feature]

```bash
# Start Kafka broker
mockforge kafka serve --port 9092

# Create topic
mockforge kafka topic create orders --partitions 3

# Produce message
mockforge kafka produce --topic orders --value '{"id": "123"}'

# Consume messages
mockforge kafka consume --topic orders --group test-group
```

#### AMQP Broker (`amqp`)

```bash
# Start AMQP broker
mockforge amqp serve --port 5672

# Declare exchange
mockforge amqp exchange declare orders --type topic --durable

# Publish message
mockforge amqp publish --exchange orders --routing-key "order.created" --body '{"id": "123"}'

# Consume messages
mockforge amqp consume --queue orders.new
```

#### SMTP Server (`smtp`) [requires smtp feature]

```bash
# Send test email
mockforge smtp send --to user@example.com --subject "Test" --body "Hello World"

# Manage mailbox
mockforge smtp mailbox list
mockforge smtp mailbox show email-123
```

### Data Generation (`data`)

Generate synthetic test data using various methods:

```bash
mockforge data [SUBCOMMAND]
```

#### Subcommands

- **template**: Generate from built-in templates (user, product, order)
- **schema**: Generate from JSON schema files

#### Examples

```bash
# Generate users
mockforge data template user --rows 100 --format json --output users.json

# Generate products with RAG enhancement
mockforge data template product --rows 50 --rag --rag-provider openai --output products.json

# Generate from custom schema
mockforge data schema my-schema.json --rows 200 --format csv --output data.csv
```

### Load Testing (`bench`)

Perform load testing using OpenAPI specifications:

```bash
mockforge bench --spec API.yaml --target https://api.example.com [OPTIONS]
```

#### Options

- **Load Profile**: `--vus` (virtual users), `--duration`, `--scenario`
- **Target**: `--target` (API endpoint), `--auth`, `--headers`
- **Filters**: `--operations`, `--method`, `--path`
- **Thresholds**: `--threshold-percentile`, `--threshold-ms`, `--max-error-rate`

#### Examples

```bash
# Basic load test
mockforge bench --spec api.yaml --target https://api.example.com --vus 10 --duration 1m

# Ramp-up scenario
mockforge bench --spec api.yaml --target https://staging.api.com --scenario ramp-up --vus 100

# Test specific endpoints
mockforge bench --spec api.yaml --target https://api.com --operations "GET /users,POST /users"
```

### Test Generation (`generate-tests`)

Generate test suites from recorded API interactions:

```bash
mockforge generate-tests --database recordings.db --format rust_reqwest [OPTIONS]
```

#### Supported Formats

- `rust_reqwest` - Rust with reqwest
- `python_pytest` - Python with pytest
- `javascript_jest` - JavaScript with Jest
- `go_test` - Go with testing
- `http_file` - HTTP files
- `curl` - cURL commands
- `postman` - Postman collection
- `k6` - k6 load testing script

#### Examples

```bash
# Generate Rust tests
mockforge generate-tests --format rust_reqwest --output tests.rs

# Generate with AI descriptions
mockforge generate-tests --format python_pytest --ai-descriptions --llm-provider openai

# Filter by endpoint
mockforge generate-tests --path "/api/users/*" --status-code 200
```

### AI-Powered Features (`suggest`, `test-ai`)

#### API Specification Suggestion (`suggest`)

Generate complete OpenAPI specs from minimal input:

```bash
# From text description
mockforge suggest --from-description "A blog API with posts and comments" --output api.yaml

# From example endpoint
mockforge suggest --from example.json --num-suggestions 10 --domain e-commerce
```

#### AI Testing (`test-ai`)

Test AI-powered features:

```bash
# Test intelligent mock generation
mockforge test-ai intelligent-mock --prompt "Generate a REST API for a blog" --output mock.json

# Test event stream generation
mockforge test-ai event-stream --narrative "User login flow" --event-count 10
```

### Workspace Management (`workspace`)

Multi-tenant workspace management:

```bash
# List workspaces
mockforge workspace list

# Create workspace
mockforge workspace create my-workspace --name "My Workspace"

# Workspace info
mockforge workspace info my-workspace
```

### Plugin Management (`plugin`)

Manage MockForge plugins:

```bash
# List installed plugins
mockforge plugin list

# Install plugin
mockforge plugin install my-plugin

# Build plugin
mockforge plugin build ./my-plugin
```

### Chaos Orchestration (`orchestrate`)

Orchestrate chaos experiments:

```bash
# Start orchestration
mockforge orchestrate start --file orchestration.yaml --base-url http://localhost:3000

# Check status
mockforge orchestrate status --base-url http://localhost:3000

# Stop orchestration
mockforge orchestrate stop --base-url http://localhost:3000
```

### Synchronization (`sync`)

Bidirectional workspace synchronization:

```bash
# Start sync daemon
mockforge sync --workspace-dir ./workspaces
```

### Project Initialization (`init`)

Initialize new MockForge projects:

```bash
# Initialize in current directory
mockforge init

# Initialize with custom name
mockforge init my-project

# Skip example files
mockforge init --no-examples
```

## Configuration

### Configuration Files

MockForge supports YAML configuration files:

```yaml
# mockforge.yaml
http:
  port: 3000
  openapi_spec: "api.yaml"

websocket:
  port: 3001
  replay_file: "events.json"

grpc:
  port: 50051

admin:
  enabled: true
  port: 9080

observability:
  prometheus:
    enabled: true
    port: 9090

data:
  rag:
    enabled: true
    provider: "openai"
    model: "gpt-4"
```

### Environment Variables

Override configuration with environment variables:

```bash
export MOCKFORGE_HTTP_PORT=8080
export MOCKFORGE_RAG_API_KEY="sk-..."
export MOCKFORGE_LOG_LEVEL=debug
```

### Command-Line Completions

Generate shell completions:

```bash
# Bash
mockforge completions bash > mockforge.bash

# Zsh
mockforge completions zsh > _mockforge

# Fish
mockforge completions fish > mockforge.fish
```

## Advanced Features

### Chaos Engineering

Inject failures and test resilience:

```bash
# Network degradation
mockforge serve --chaos --chaos-scenario network_degradation

# Custom latency
mockforge serve --chaos --chaos-latency-ms 500 --chaos-latency-probability 0.8

# HTTP errors
mockforge serve --chaos --chaos-http-errors "500,502,503" --chaos-http-error-probability 0.1

# Rate limiting
mockforge serve --chaos --chaos-rate-limit 10

# Random chaos
mockforge serve --chaos-random --chaos-random-error-rate 0.05
```

### Traffic Shaping

Simulate network conditions:

```bash
# Bandwidth limiting
mockforge serve --traffic-shaping --bandwidth-limit 1000000

# Network profiles
mockforge serve --network-profile 3g
mockforge serve --network-profile satellite_leo

# List available profiles
mockforge serve --list-network-profiles
```

### AI Integration

Leverage AI for enhanced mocking:

```bash
# Enable AI features
mockforge serve --ai-enabled --rag-provider openai --rag-model gpt-4

# Generate intelligent mocks
mockforge test-ai intelligent-mock --prompt "Create a user management API"

# AI-powered test generation
mockforge generate-tests --ai-descriptions --llm-provider ollama
```

### Observability

Monitor and trace requests:

```bash
# Enable metrics
mockforge serve --metrics --metrics-port 9090

# Enable tracing
mockforge serve --tracing --tracing-service-name my-service --jaeger-endpoint http://localhost:14268/api/traces

# Enable API recorder
mockforge serve --recorder --recorder-db ./recordings.db
```

## Examples

### Development Workflow

```bash
# 1. Initialize project
mockforge init my-api

# 2. Start development server
mockforge serve --admin --metrics

# 3. Generate test data
mockforge data template user --rows 100 --output users.json

# 4. Load test your API
mockforge bench --spec api.yaml --target http://localhost:3000 --vus 20 --duration 30s
```

### CI/CD Integration

```bash
# Validate configuration
mockforge serve --dry-run --config mockforge.yaml

# Generate tests from recordings
mockforge generate-tests --database recordings.db --format rust_reqwest --output integration_tests.rs

# Run chaos experiments
mockforge orchestrate start --file chaos.yaml --base-url $API_URL
```

### Testing Workflow

```bash
# Start mock servers for testing
mockforge serve --spec api.yaml --chaos-random

# Generate test data
mockforge data schema test-schema.json --rows 1000 --output test-data.json

# Run load tests
mockforge bench --spec api.yaml --target $TEST_API_URL --scenario stress
```

## Troubleshooting

### Common Issues

**Port already in use:**
```bash
# Check what's using the port
lsof -i :3000

# Use different ports
mockforge serve --http-port 3001 --ws-port 3002
```

**Configuration validation:**
```bash
# Validate config before starting
mockforge config validate --config mockforge.yaml
```

**Performance issues:**
```bash
# Enable metrics to monitor performance
mockforge serve --metrics

# Check metrics at http://localhost:9090
```

## Contributing

See the main [MockForge repository](https://github.com/SaaSy-Solutions/mockforge) for contribution guidelines.

## License

Licensed under MIT OR Apache-2.0
