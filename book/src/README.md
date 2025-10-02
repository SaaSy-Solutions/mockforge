# MockForge

[![Crates.io](https://img.shields.io/crates/v/mockforge.svg)](https://crates.io/crates/mockforge)
[![Documentation](https://docs.rs/mockforge/badge.svg)](https://docs.rs/mockforge)
[![CI](https://github.com/SaaSy-Solutions/mockforge/workflows/CI/badge.svg)](https://github.com/SaaSy-Solutions/mockforge/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/SaaSy-Solutions/mockforge/blob/main/LICENSE)

MockForge is a comprehensive mocking framework for APIs, gRPC services, and WebSockets. It provides a unified interface for creating, managing, and deploying mock servers across different protocols.

## Features

- **Multi-Protocol Support**: HTTP REST APIs, gRPC services, and WebSocket connections
- **Dynamic Response Generation**: Create realistic mock responses with configurable latency and failure rates
- **Scenario Management**: Define complex interaction scenarios with state management
- **CLI Tool**: Easy-to-use command-line interface for local development
- **Admin UI**: Web-based interface for managing mock servers
- **Extensible Architecture**: Plugin system for custom response generators

## Quick Start

### Installation

```bash
cargo install mockforge-cli
```

### Basic Usage

```bash
# Start a mock server with an OpenAPI spec
cargo run -p mockforge-cli -- serve --spec examples/openapi-demo.json --http-port 3000

# Add WebSocket support with replay file
MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl cargo run -p mockforge-cli -- serve --ws-port 3001

# Full configuration with Admin UI
MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl \
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
cargo run -p mockforge-cli -- serve --spec examples/openapi-demo.json --admin --admin-port 9080

# Use configuration file
cargo run -p mockforge-cli -- serve --config demo-config.yaml
```

### Docker

```bash
docker run -p 3000:3000 -p 3001:3001 -p 50051:50051 SaaSy-Solutions/mockforge
```

## Documentation Structure

- [Getting Started](getting-started.md) - Installation and basic setup
- [HTTP Mocking](http-mocking.md) - REST API mocking guide
- [gRPC Mocking](grpc-mocking.md) - gRPC service mocking
- [WebSocket Mocking](websocket-mocking.md) - WebSocket connection mocking
- [Configuration](configuration.md) - Advanced configuration options
- [API Reference](api-reference.md) - Complete API documentation
- [Contributing](contributing.md) - How to contribute to MockForge
- [FAQ](faq.md) - Frequently asked questions

## Examples

Check out the [`examples/`](../examples/) directory for sample configurations and use cases.

## Community

- [GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues) - Report bugs and request features
- [GitHub Discussions](https://github.com/SaaSy-Solutions/mockforge/discussions) - Ask questions and share ideas
- [Discord](https://discord.gg/mockforge) - Join our community chat

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/SaaSy-Solutions/mockforge/blob/main/LICENSE-APACHE))
- MIT License ([LICENSE-MIT](https://github.com/SaaSy-Solutions/mockforge/blob/main/LICENSE-MIT))

at your option.
