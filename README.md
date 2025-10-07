# MockForge

[![Crates.io](https://img.shields.io/crates/v/mockforge.svg)](https://crates.io/crates/mockforge)
[![Documentation](https://docs.rs/mockforge/badge.svg)](https://docs.rs/mockforge)
[![Book](https://img.shields.io/badge/book-read%20online-blue.svg)](https://docs.mockforge.dev/)
[![CI](https://github.com/SaaSy-Solutions/mockforge/workflows/CI/badge.svg)](https://github.com/SaaSy-Solutions/mockforge/actions)
[![Tests](https://github.com/SaaSy-Solutions/mockforge/workflows/Tests/badge.svg)](https://github.com/SaaSy-Solutions/mockforge/actions)
[![Coverage](https://codecov.io/gh/SaaSy-Solutions/mockforge/branch/main/graph/badge.svg)](https://codecov.io/gh/SaaSy-Solutions/mockforge)
[![Benchmarks](https://img.shields.io/badge/benchmarks-criterion-blue)](https://github.com/SaaSy-Solutions/mockforge/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/SaaSy-Solutions/mockforge/blob/main/LICENSE)

MockForge is a comprehensive mocking framework for APIs, gRPC services, and WebSockets. It provides a unified interface for creating, managing, and deploying mock servers across different protocols with advanced data generation capabilities.

## ✨ Features

- **Multi-Protocol Support**: HTTP REST APIs, gRPC services, GraphQL APIs, and WebSocket connections
- **Advanced Data Synthesis**: Intelligent mock data generation with:
  - **Smart Field Inference**: Automatic data type detection from field names
  - **Deterministic Seeding**: Reproducible test fixtures for stable testing
  - **RAG-Driven Generation**: Context-aware data using domain knowledge
  - **Relationship Awareness**: Foreign key detection and cross-reference validation
  - **Schema Graph Extraction**: Automatic relationship discovery from protobuf schemas
- **Plugin System**: WebAssembly-based extensible architecture with:
  - **Custom Response Generators**: Build plugins for specialized mock data
  - **Authentication Providers**: JWT, OAuth2, and custom auth plugins
  - **Data Source Connectors**: CSV, database, and external API integrations
  - **Template Extensions**: Custom template functions and filters
  - **Security Sandbox**: Isolated plugin execution with resource limits
- **End-to-End Encryption**: Enterprise-grade security features:
  - **Multi-Algorithm Support**: AES-256-GCM and ChaCha20-Poly1305 encryption
  - **Key Management**: Hierarchical key system with secure storage
  - **Auto-Encryption**: Automatic encryption of sensitive configuration data
  - **Template Functions**: Built-in encryption/decryption in templates
- **Workspace Synchronization**: Bidirectional sync with version control:
  - **File System Watching**: Real-time sync between workspaces and directories
  - **Git Integration**: Version control your mock configurations
  - **Team Collaboration**: Shared workspaces with conflict resolution
- **Dynamic Response Generation**: Create realistic mock responses with configurable latency and failure rates
- **Cross-Endpoint Validation**: Ensure referential integrity across different endpoints
- **Admin UI v2**: Modern React-based interface with:
  - **Role-Based Authentication**: Admin and viewer access control
  - **Real-time Monitoring**: Live logs, metrics, and performance tracking
  - **Visual Configuration**: Drag-and-drop fixture management
  - **Advanced Search**: Full-text search across services and logs
- **Configuration Management**: Flexible configuration via YAML/JSON files with environment variable overrides
- **Built-in Data Templates**: Pre-configured schemas for common data types (users, products, orders)
- **Production Ready**: Comprehensive testing, security audits, and automated releases

## 📖 Documentation

For comprehensive documentation, tutorials, and guides:

**[📚 Read the MockForge Book](https://docs.mockforge.dev/)**

The documentation covers:
- Getting started guide and installation
- Detailed configuration options
- API reference for all protocols (HTTP, gRPC, WebSocket)  
- Advanced features and examples
- Contributing guidelines

## 🚀 Quick Start

### Installation

```bash
# Install from crates.io
cargo install mockforge-cli

# Or build from source
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge
make setup
make build
make install
```

### Try the Examples

MockForge comes with comprehensive examples to get you started quickly:

```bash
# Run with the included examples
make run-example

# Or use the configuration file
cargo run -p mockforge-cli -- serve --config demo-config.yaml

# Or run manually with environment variables
MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl \
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
cargo run -p mockforge-cli -- serve --spec examples/openapi-demo.json --admin
```

See `examples/README.md` for detailed documentation on the example files.

### Docker (Alternative Installation)

MockForge can also be run using Docker for easy deployment:

#### Quick Docker Start

```bash
# Using Docker Compose (recommended)
make docker-compose-up

# Or using Docker directly
make docker-build && make docker-run
```

#### Manual Docker Commands

```bash
# Build the image
docker build -t mockforge .

# Run with examples
docker run -p 3000:3000 -p 3001:3001 -p 50051:50051 -p 9080:9080 \
  -v $(pwd)/examples:/app/examples:ro \
  -e MOCKFORGE_ADMIN_ENABLED=true \
  -e MOCKFORGE_HTTP_OPENAPI_SPEC=examples/openapi-demo.json \
  mockforge
```

See [DOCKER.md](DOCKER.md) for comprehensive Docker documentation and deployment options.

### Basic Usage

```bash
# Build the project
cargo build

# Start all mock servers with Admin UI (separate port)
cargo run -p mockforge-cli -- serve --admin --admin-port 9080

# Start with custom configuration
cargo run -p mockforge-cli -- serve --config config.yaml --admin

# Generate test data
cargo run -p mockforge-cli -- data template user --rows 50 --output users.json

# Start Admin UI only (standalone server)
cargo run -p mockforge-cli -- admin --port 9080

# Start workspace synchronization daemon
cargo run -p mockforge-cli -- sync start --directory ./workspace-sync

# Access Admin Interface

- Standalone Admin: http://localhost:9080/
- Admin embedded under HTTP (when configured): http://localhost:3000/admin/

# Quick development setup with environment variables
MOCKFORGE_ADMIN_ENABLED=true MOCKFORGE_HTTP_PORT=3000 cargo run -p mockforge-cli -- serve
```

## HTTP

curl <http://localhost:3000/ping>

## WebSocket (Scripted Replay)

MockForge supports scripted WebSocket interactions with template expansion and conditional responses.

### Quick Start

```bash
# Set the replay file environment variable
export MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl

# Start the WebSocket server
cargo run -p mockforge-cli -- serve --ws-port 3001
```

### Connect and Test

**Using Node.js:**
```javascript
const WebSocket = require('ws');
const ws = new WebSocket('ws://localhost:3001/ws');

ws.on('open', () => {
  console.log('Connected! Sending CLIENT_READY...');
  ws.send('CLIENT_READY');
});

ws.on('message', (data) => {
  console.log('Received:', data.toString());

  // Auto-respond to expected prompts
  if (data.toString().includes('ACK')) {
    ws.send('ACK');
  }
  if (data.toString().includes('CONFIRMED')) {
    ws.send('CONFIRMED');
  }
});

ws.on('close', () => console.log('Connection closed'));
```

**Using websocat:**
```bash
websocat ws://localhost:3001/ws
# Then type: CLIENT_READY
# The server will respond with scripted messages
```

**Using wscat:**
```bash
wscat -c ws://localhost:3001/ws
# Then type: CLIENT_READY
```

**Browser Console:**
```javascript
const ws = new WebSocket('ws://localhost:3001/ws');
ws.onopen = () => ws.send('CLIENT_READY');
ws.onmessage = (event) => console.log('Received:', event.data);
```

### Advanced Message Matching with JSONPath

MockForge supports JSONPath queries for sophisticated WebSocket message matching:

```json
[
  {"waitFor": "^CLIENT_READY$", "text": "Welcome!"},
  {"waitFor": "$.type", "text": "Type received"},
  {"waitFor": "$.user.id", "text": "User authenticated"},
  {"waitFor": "$.order.status", "text": "Order status updated"}
]
```

**JSONPath Examples:**
- `$.type` - Wait for any message with a `type` property
- `$.user.id` - Wait for messages with user ID
- `$.order.status` - Wait for order status updates
- `$.items[0].name` - Wait for first item name

**JSON Message Testing:**
```javascript
const ws = new WebSocket('ws://localhost:3001/ws');

// Send JSON messages that match JSONPath patterns
ws.onopen = () => {
  ws.send(JSON.stringify({type: 'login'}));           // Matches $.type
  ws.send(JSON.stringify({user: {id: '123'}}));       // Matches $.user.id
  ws.send(JSON.stringify({order: {status: 'paid'}})); // Matches $.order.status
};

ws.onmessage = (event) => console.log('Response:', event.data);
```

See `examples/README-websocket-jsonpath.md` for complete documentation.

### Replay File Format

WebSocket replay files use JSON Lines format with the following structure:

```json
{"ts":0,"dir":"out","text":"HELLO {{uuid}}","waitFor":"^CLIENT_READY$"}
{"ts":10,"dir":"out","text":"{\\"type\\":\\"welcome\\",\\"sessionId\\":\\"{{uuid}}\\"}"}
{"ts":20,"dir":"out","text":"{\\"type\\":\\"data\\",\\"value\\":\\"{{randInt 1 100}}\\"}","waitFor":"^ACK$"}
```

- `ts`: Timestamp in milliseconds for message timing
- `dir`: Direction ("in" for received, "out" for sent)
- `text`: Message content (supports template expansion)
- `waitFor`: Optional regex pattern to wait for before sending

### Template Expansion

WebSocket messages support the same template expansion as HTTP responses:
- `{{uuid}}` → Random UUID
- `{{now}}` → Current timestamp
- `{{now+1h}}` → Future timestamp
- `{{randInt 1 100}}` → Random integer

## gRPC

grpcurl -plaintext -proto crates/mockforge-grpc/proto/gretter.proto -d '{"name":"Ray"}' localhost:50051 mockforge.greeter.Greeter/SayHello

### 🚀 HTTP Bridge for gRPC Services

MockForge now includes an advanced **HTTP Bridge** that automatically converts gRPC services to REST APIs, eliminating the need for separate gRPC and HTTP implementations.

#### Features

- **Automatic Discovery**: Scans `.proto` files and creates REST endpoints for all gRPC services
- **JSON ↔ Protobuf Conversion**: Full bidirectional conversion between JSON and protobuf messages
- **OpenAPI Documentation**: Auto-generated OpenAPI/Swagger specs for all bridged services
- **Streaming Support**: Server-Sent Events (SSE) for server streaming and bidirectional communication
- **Statistics & Monitoring**: Built-in request metrics and health checks

#### Quick Start

```bash
# Start gRPC server with HTTP bridge
cargo run -p mockforge-cli -- serve --config config.dev.yaml --admin
```

The bridge will automatically:
1. Discover services from proto files
2. Create REST endpoints at `/api/{service}/{method}`
3. Generate OpenAPI docs at `/api/docs`
4. Provide health monitoring at `/api/health`

#### Example Usage

**gRPC Service:**
```protobuf
service UserService {
  rpc CreateUser(CreateUserRequest) returns (CreateUserResponse);
  rpc GetUser(GetUserRequest) returns (GetUserResponse);
}
```

**HTTP Bridge Endpoints:**
```bash
# Create user (POST)
curl -X POST http://localhost:3000/api/userservice/createuser \
  -H "Content-Type: application/json" \
  -d '{"name": "John Doe", "email": "john@example.com"}'

# Get user (POST - gRPC semantics)
curl -X POST http://localhost:3000/api/userservice/getuser \
  -H "Content-Type: application/json" \
  -d '{"user_id": "123"}'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "user_id": "123",
    "name": "John Doe",
    "email": "john@example.com",
    "created_at": "2025-01-01T00:00:00Z"
  },
  "error": null,
  "metadata": {
    "x-mockforge-service": "userservice",
    "x-mockforge-method": "createuser"
  }
}
```

#### Configuration

Enable the HTTP bridge by modifying your config:

```yaml
grpc:
  dynamic:
    enabled: true
    proto_dir: "proto"          # Directory containing .proto files
    enable_reflection: true     # Enable gRPC reflection
    http_bridge:
      enabled: true             # Enable HTTP bridge
      base_path: "/api"         # Base path for REST endpoints
      enable_cors: true         # Enable CORS
      timeout_seconds: 30       # Request timeout
```

Or via environment variables:
```bash
export MOCKFORGE_GRPC_DYNAMIC_ENABLED=true
export MOCKFORGE_GRPC_HTTP_BRIDGE_ENABLED=true
export MOCKFORGE_GRPC_PROTO_DIR=proto
```

#### Bridge Endpoints

- **`GET /api/health`** - Health check
- **`GET /api/stats`** - Request statistics and metrics
- **`GET /api/services`** - List available gRPC services
- **`GET /api/docs`** - OpenAPI 3.0 documentation
- **`/api/{service}/{method}`** - Automatically generated REST endpoints

#### Streaming Support

For gRPC streaming methods, the bridge provides:

```bash
# Server streaming endpoint
curl -N http://localhost:3000/api/chat/streammessages \
  -H "Content-Type: application/json" \
  -d '{"topic": "tech"}'
```

Returns server-sent events:
```javascript
data: {"event_type":"message","data":{"text":"Hello!"},"metadata":{}}
event: message

data: {"event_type":"message","data":{"text":"How can I help?"},"metadata":{}}
event: message
```

#### OpenAPI Integration

The bridge auto-generates comprehensive OpenAPI documentation:

```bash
# Access interactive API docs
open http://localhost:3000/api/docs

# Get OpenAPI JSON spec
curl http://localhost:3000/api/docs
```

Features:
- Automatic schema generation from protobuf definitions
- Example requests and responses
- Streaming method documentation
- Method tags and descriptions

#### Advanced Features

- **Bidirectional Streaming**: Full support for client ↔ server streaming via WebSockets-in-disguise
- **Metadata Preservation**: Passes gRPC metadata as HTTP headers
- **Error Handling**: Comprehensive error responses with detailed messages
- **Metrics**: Request counting, latency tracking, and failure rates
- **Security**: Configurable CORS and request validation

#### Use Cases

1. **Frontend Development**: Test gRPC APIs with familiar HTTP tools
2. **API Gateways**: Expose gRPC services as REST APIs
3. **Mixed Environments**: Support for both gRPC and HTTP clients
4. **Development Tools**: Use Postman, curl, or any HTTP client
5. **Documentation**: Auto-generated API docs for gRPC services

## 🎯 Data Generation

MockForge includes powerful synthetic data generation capabilities:

```bash
# Generate user data using built-in templates
cargo run -p mockforge-cli -- data template user --rows 100 --output users.json

# Generate product data
cargo run -p mockforge-cli -- data template product --rows 50 --format csv --output products.csv

# Generate data from JSON schema
cargo run -p mockforge-cli -- data schema schema.json --rows 200 --output custom_data.json

# Enable RAG mode for enhanced data generation
cargo run -p mockforge-cli -- data template user --rows 100 --rag --output users.json
```

### Built-in Templates

- **User**: Complete user profiles with emails, names, addresses
- **Product**: Product catalog with pricing, categories, descriptions
- **Order**: Customer orders with relationships to users and products

### Advanced Features

- **RAG Integration**: Use LLM-powered generation for more realistic data
- **Multiple Formats**: JSON, JSON Lines, CSV output
- **Custom Schemas**: Generate data from your own JSON schemas
- **Relationship Support**: Maintain referential integrity between entities

echo -e '{"name":"one"}\n{"name":"two"}' | grpcurl -plaintext -proto crates/mockforge-grpc/proto/gretter.proto -d @ localhost:50051 mockforge.greeter.Greeter/SayHelloClientStream

echo -e '{"name":"first"}\n{"name":"second"}' | grpcurl -plaintext -proto crates/mockforge-grpc/proto/gretter.proto -d @ localhost:50051 mockforge.greeter.Greeter/Chat

## 🎛️ Admin Interface

### Dashboard

![Dashboard](docs/images/mockforge-dashboard.png)

MockForge ships a built-in Admin UI that can run as either:

- A standalone server (default when `--admin` is used): `http://localhost:9080/`.
- Embedded under the HTTP server at a mount path, e.g. `http://localhost:3000/admin/` when `admin.mount_path: "/admin"` is configured.

The Admin UI provides:

- **📊 Modern dashboard** with real-time server status
- **⚙️ Configuration management** for latency, faults, and proxy settings
- **📝 Request logging** with filtering and monitoring
- **📈 Metrics visualization** with performance insights
- **🎯 Fixture management** with record/replay capabilities
- **🎨 Professional UI** with tabbed interface and responsive design

### Embedded Admin Mode

You can embed the Admin UI under the HTTP server instead of running it on a separate port. This is handy when you want a single endpoint to expose mocks and admin controls.

- Configure via file (config.yaml):

```yaml
admin:
  enabled: true
  mount_path: "/admin"
```

- Or via environment:

```bash
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_ADMIN_MOUNT_PATH=/admin
```

- Start servers:

```bash
cargo run -p mockforge-cli -- serve
```

- Access URLs:
  - UI: http://localhost:3000/admin/
  - Health: http://localhost:3000/admin/__mockforge/health
  - Dashboard: http://localhost:3000/admin/__mockforge/dashboard

Notes:
- Static assets are served relative to the mount path (e.g., `/admin/admin.css`).
- Switching back to standalone mode: remove `mount_path` (or unset env) and run with `--admin --admin-port 9080`.

### Admin Mode Flags (CLI)

You can control how the Admin UI runs via flags on `serve`:

```bash
# Force embedded mode (default mount at /admin)
cargo run -p mockforge-cli -- serve --admin-embed

# Embedded with explicit mount
cargo run -p mockforge-cli -- serve --admin-embed --admin-mount-path /tools

# Force standalone mode on port 9080 (overrides embed)
cargo run -p mockforge-cli -- serve --admin --admin-standalone --admin-port 9080

# Disable Admin APIs (UI loads but __mockforge/* endpoints are absent)
cargo run -p mockforge-cli -- serve --admin-embed --disable-admin-api

# Equivalent env-based control
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_ADMIN_MOUNT_PATH=/admin
export MOCKFORGE_ADMIN_API_ENABLED=false
cargo run -p mockforge-cli -- serve
```

### API Endpoints

Admin API endpoints are namespaced under `__mockforge`:

- Standalone Admin (default):
  - `GET /__mockforge/dashboard`
  - `GET /__mockforge/health`
  - `GET /__mockforge/logs`
  - `GET /__mockforge/metrics`
  - `GET /__mockforge/fixtures`
  - `POST /__mockforge/config/*`
- Embedded under a mount path (e.g., `/admin`):
  - `GET /admin/__mockforge/dashboard`
  - `GET /admin/__mockforge/health`
  - ... (same suffixes under the mount prefix)

## ⚙️ Configuration

MockForge supports flexible configuration through YAML or JSON files:

```bash
# Use a configuration file
cargo run -p mockforge-cli -- serve --config my-config.yaml

# Configuration file example
cp config.example.yaml my-config.yaml
```

### Environment Variables

Override any configuration setting with environment variables:

```bash
# Server ports
export MOCKFORGE_HTTP_PORT=9080
export MOCKFORGE_WS_PORT=8081
export MOCKFORGE_GRPC_PORT=9090
export MOCKFORGE_ADMIN_PORT=9091

# Enable features
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_LATENCY_ENABLED=true

# Logging
export MOCKFORGE_LOG_LEVEL=debug
```

### Configuration Options

- **HTTP Server**: Port, host, OpenAPI spec, CORS settings
- **WebSocket Server**: Port, host, replay files, timeouts
- **gRPC Server**: Port, host, proto files, TLS configuration
- **Admin UI**: Enable/disable, authentication, custom port
- **Core Features**: Latency profiles, failure injection, proxy settings
- **Data Generation**: Default settings, RAG configuration, custom templates

## 🛠️ Development

### Prerequisites

- Rust 1.70 or later
- Make
- Python 3 (for some tooling)

### Setup

```bash
# Clone the repository
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge

# Set up development environment (installs all tools and hooks)
make setup

# Build the project
make build

# Run all tests
make test

# Run all quality checks
make check-all
```

### Development Workflow

```bash
# Start development mode with file watching
make dev

# Format code
make fmt

# Run lints
make clippy

# Run security audit
make audit

# Generate documentation
make doc

# Build user docs
make book
```

### Project Structure

```text
mockforge/
├── crates/                     # Workspace crates
│   ├── mockforge-cli/          # Command-line interface
│   ├── mockforge-core/         # Shared logic (routing, validation, latency, proxy)
│   ├── mockforge-http/         # HTTP mocking library
│   ├── mockforge-ws/           # WebSocket mocking library
│   ├── mockforge-grpc/         # gRPC mocking library
│   ├── mockforge-data/         # Synthetic data generation (faker + RAG)
│   └── mockforge-ui/           # Admin UI (Axum routes + static assets)
├── config.example.yaml         # Configuration example
├── docs/                       # Project documentation
├── book/                       # mdBook documentation
├── examples/                   # Example configurations and test files
├── tools/                      # Development tools
├── scripts/                    # Setup and utility scripts
├── .github/                    # GitHub Actions and templates
└── tools/                      # Development utilities
```

### Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Release Process

This project uses automated releases with [cargo-release](https://github.com/crate-ci/cargo-release):

```bash
# Patch release (bug fixes)
make release-patch

# Minor release (new features)
make release-minor

# Major release (breaking changes)
make release-major
```

## 📚 Documentation

- [User Guide](https://docs.mockforge.dev/) - Complete documentation
- [API Reference](https://docs.rs/mockforge) - Rust API documentation
- [Contributing](CONTRIBUTING.md) - How to contribute
- [Changelog](CHANGELOG.md) - Release notes

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
## Validation Modes

You can control request/response validation via CLI, environment, or config.

- Environment:
- `MOCKFORGE_REQUEST_VALIDATION=off|warn|enforce` (default: enforce)
- `MOCKFORGE_AGGREGATE_ERRORS=true|false` (default: true)
- `MOCKFORGE_RESPONSE_VALIDATION=true|false` (default: false)
- `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true|false` (default: false)
  - When true, mock responses (including media-level `example` bodies) expand tokens:
    - `{{uuid}}` → random UUID v4
    - `{{now}}` → RFC3339 timestamp
    - `{{now±Nd|Nh|Nm|Ns}}` → timestamp offset by days/hours/minutes/seconds, e.g., `{{now+2h}}`, `{{now-30m}}`
    - `{{rand.int}}` → random integer
    - `{{rand.float}}` → random float
  - Also supports ranged and faker tokens when enabled:
    - `{{randInt 10 99}}`, `{{rand.int -5 5}}`
    - `{{faker.uuid}}`, `{{faker.email}}`, `{{faker.name}}`, `{{faker.address}}`, `{{faker.phone}}`, `{{faker.company}}`, `{{faker.url}}`, `{{faker.ip}}`, `{{faker.color}}`, `{{faker.word}}`, `{{faker.sentence}}`, `{{faker.paragraph}}`
  - Determinism: set `MOCKFORGE_FAKE_TOKENS=false` to disable faker token expansion (uuid/now/rand tokens still expand).
  
 - `MOCKFORGE_VALIDATION_STATUS=400|422` (default: 400)
   - Status code returned on request validation failure in enforce mode.

- CLI (serve):
  - `--validation off|warn|enforce`
  - `--aggregate-errors`
  - `--validate-responses`

- Config (config.yaml):

```yaml
http:
  request_validation: "enforce"   # off|warn|enforce
  aggregate_validation_errors: true
  validate_responses: false
  skip_admin_validation: true
  validation_overrides:
    "POST /users/{id}": "warn"
    "GET /internal/health": "off"
```

When aggregation is enabled, 400 responses include both a flat `errors` list and a `details` array with structured items:

```json
{
  "error": "request validation failed",
  "details": [
    { "path": "query.q", "code": "type", "message": "query.q: expected number, got \"abc\"", "value": "abc" }
  ]
}
```
