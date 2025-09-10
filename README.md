# MockForge

[![Crates.io](https://img.shields.io/crates/v/mockforge.svg)](https://crates.io/crates/mockforge)
[![Documentation](https://docs.rs/mockforge/badge.svg)](https://docs.rs/mockforge)
[![CI](https://github.com/SaaSy-Solutions/mockforge/workflows/CI/badge.svg)](https://github.com/SaaSy-Solutions/mockforge/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/SaaSy-Solutions/mockforge/blob/main/LICENSE)

MockForge is a comprehensive mocking framework for APIs, gRPC services, and WebSockets. It provides a unified interface for creating, managing, and deploying mock servers across different protocols with advanced data generation capabilities.

## ✨ Features

- **Multi-Protocol Support**: HTTP REST APIs, gRPC services, and WebSocket connections
- **Synthetic Data Generation**: Generate realistic test data with faker primitives and RAG (Retrieval-Augmented Generation)
- **Dynamic Response Generation**: Create realistic mock responses with configurable latency and failure rates
- **Admin UI**: Modern web-based interface for managing mock servers and viewing metrics
- **Configuration Management**: Flexible configuration via YAML/JSON files with environment variable overrides
- **Built-in Data Templates**: Pre-configured schemas for common data types (users, products, orders)
- **Extensible Architecture**: Plugin system for custom response generators and data sources
- **Production Ready**: Comprehensive testing, security audits, and automated releases

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

### Basic Usage

```bash
# Build the project
cargo build

# Start all mock servers with Admin UI (separate port)
cargo run -p mockforge-cli -- serve --admin --admin-port 8080

# Start with custom configuration
cargo run -p mockforge-cli -- serve --config config.yaml --admin

# Generate test data
cargo run -p mockforge-cli -- data template user --rows 50 --output users.json

# Start Admin UI only (standalone server)
cargo run -p mockforge-cli -- admin --port 8080

# Access Admin Interface

- Standalone Admin: http://localhost:8080/
- Admin embedded under HTTP (when configured): http://localhost:3000/admin/

# Quick development setup with environment variables
MOCKFORGE_ADMIN_ENABLED=true MOCKFORGE_HTTP_PORT=3000 cargo run -p mockforge-cli -- serve
```

## HTTP

curl <http://localhost:3000/ping>

## WS (scripted replay)

export MOCKFORGE_WS_REPLAY_FILE=mockforge/examples/ws-demo.jsonl

## then connect to ws://localhost:3001/ws and send "CLIENT_READY"

Using websocat (command line tool):
websocat ws://localhost:3001/ws
Then type CLIENT_READY and press Enter.

Using wscat (Node.js tool):
wscat -c ws://localhost:3001/ws
Then type CLIENT_READY and press Enter.

Using JavaScript in browser console:
const ws = new WebSocket('ws://localhost:3001/ws');
ws.onopen = () => ws.send('CLIENT_READY');
ws.onmessage = (event) => console.log('Received:', event.data);

Using curl (if server supports it):
curl --include --no-buffer --header "Connection: Upgrade" --header "Upgrade: websocket" --header
"Sec-WebSocket-Key: SGVsbG8sIHdvcmxkIQ==" --header "Sec-WebSocket-Version: 13" ws://localhost:3001/ws

## gRPC

grpcurl -plaintext -proto crates/mockforge-grpc/proto/gretter.proto -d '{"name":"Ray"}' localhost:50051 mockforge.greeter.Greeter/SayHello

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

MockForge ships a built-in Admin UI that can run as either:

- A standalone server (default when `--admin` is used): `http://localhost:8080/`.
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
- Switching back to standalone mode: remove `mount_path` (or unset env) and run with `--admin --admin-port 8080`.

### Admin Mode Flags (CLI)

You can control how the Admin UI runs via flags on `serve`:

```bash
# Force embedded mode (default mount at /admin)
cargo run -p mockforge-cli -- serve --admin-embed

# Embedded with explicit mount
cargo run -p mockforge-cli -- serve --admin-embed --admin-mount-path /tools

# Force standalone mode on port 8080 (overrides embed)
cargo run -p mockforge-cli -- serve --admin --admin-standalone --admin-port 8080

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
export MOCKFORGE_HTTP_PORT=8080
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
├── examples/                   # Example configurations
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

- [User Guide](https://SaaSy-Solutions.github.io/mockforge/) - Complete documentation
- [API Reference](https://docs.rs/mockforge) - Rust API documentation
- [Contributing](CONTRIBUTING.md) - How to contribute
- [Changelog](CHANGELOG.md) - Release notes

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
