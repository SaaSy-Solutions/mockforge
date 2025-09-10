# MockForge

[![Crates.io](https://img.shields.io/crates/v/mockforge.svg)](https://crates.io/crates/mockforge)
[![Documentation](https://docs.rs/mockforge/badge.svg)](https://docs.rs/mockforge)
[![CI](https://github.com/SaaSy-Solutions/mockforge/workflows/CI/badge.svg)](https://github.com/SaaSy-Solutions/mockforge/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/SaaSy-Solutions/mockforge/blob/main/LICENSE)

MockForge is a comprehensive mocking framework for APIs, gRPC services, and WebSockets. It provides a unified interface for creating, managing, and deploying mock servers across different protocols.

## ✨ Features

- **Multi-Protocol Support**: HTTP REST APIs, gRPC services, and WebSocket connections
- **Dynamic Response Generation**: Create realistic mock responses with configurable latency and failure rates
- **Scenario Management**: Define complex interaction scenarios with state management
- **CLI Tool**: Easy-to-use command-line interface for local development
- **Admin UI**: Web-based interface for managing mock servers
- **Extensible Architecture**: Plugin system for custom response generators
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
# Quick development setup
make run-example

# Or manually:
cargo build
MOCKFORGE_LATENCY_ENABLED=true MOCKFORGE_FAILURES_ENABLED=false cargo watch -x 'run -p mockforge-cli -- --spec examples/openapi-demo.json --http-port 3000 --ws-port 3001 --grpc-port 50051'
```

## HTTP

curl http://localhost:3000/ping

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

echo -e '{"name":"one"}\n{"name":"two"}' | grpcurl -plaintext -proto crates/mockforge-grpc/proto/gretter.proto -d @ localhost:50051 mockforge.greeter.Greeter/SayHelloClientStream

echo -e '{"name":"first"}\n{"name":"second"}' | grpcurl -plaintext -proto crates/mockforge-grpc/proto/gretter.proto -d @ localhost:50051 mockforge.greeter.Greeter/Chat

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

```
mockforge/
├── crates/                    # Workspace crates
│   ├── mockforge-cli/        # Command-line interface
│   ├── mockforge-http/       # HTTP mocking library
│   ├── mockforge-ws/         # WebSocket mocking library
│   └── mockforge-grpc/       # gRPC mocking library
├── admin-ui/                 # Web-based admin interface
├── docs/                     # Project documentation
├── book/                     # mdBook documentation
├── examples/                 # Example configurations
├── tools/                    # Development tools
├── scripts/                  # Setup and utility scripts
├── .github/                  # GitHub Actions and templates
└── tools/                    # Development utilities
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
