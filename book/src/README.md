# MockForge

[![Crates.io](https://img.shields.io/crates/v/mockforge-cli.svg)](https://crates.io/crates/mockforge-cli)
[![Documentation](https://docs.rs/mockforge-cli/badge.svg)](https://docs.rs/mockforge-cli)
[![CI](https://github.com/SaaSy-Solutions/mockforge/actions/workflows/ci.yml/badge.svg)](https://github.com/SaaSy-Solutions/mockforge/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/SaaSy-Solutions/mockforge/blob/main/LICENSE)

MockForge helps teams simulate realistic backend behavior so frontend, backend, and QA work can move in parallel without waiting on live services.

Start with local open-source workflows, then add richer scenarios, protocol coverage, and team-facing tooling as your test surface grows.

## What MockForge Is Good At

- Turning OpenAPI specs into usable mock services quickly
- Unblocking frontend and integration work with realistic responses
- Testing retries, fallbacks, and edge cases with scenario-aware behavior
- Simulating more than plain REST when your stack includes gRPC, GraphQL, WebSockets, or SMTP
- Giving teams an admin surface and repeatable workflows instead of ad hoc fixture sprawl

## Start Here

- New to MockForge: [Your First Mock API in 5 Minutes](getting-started/five-minute-api.md)
- Building a real workflow: [The Golden Path: Blueprint -> Dev-Setup -> Integration](tutorials/golden-path.md)
- Evaluating fit for frontend and QA teams: [Reality-First Onboarding](getting-started/reality-first.md)
- Evaluating fit for platform and API teams: [Contracts-First Onboarding](getting-started/contracts-first.md)
- Planning hosted or multi-team rollout: [Cloud-First Onboarding](getting-started/cloud-first.md)

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

### Next Steps

- [Mock a REST API from OpenAPI](tutorials/mock-openapi-spec.md)
- [React + MockForge Workflow](tutorials/react-workflow.md)
- [Vue + MockForge Workflow](tutorials/vue-workflow.md)
- [Admin UI Walkthrough](tutorials/admin-ui-walkthrough.md)

## Core Capabilities

- **OpenAPI-first HTTP mocking** with custom responses and dynamic data
- **Scenario-aware simulation** for stateful behavior, latency, and failure paths
- **Multi-protocol coverage** across REST, gRPC, GraphQL, WebSockets, and additional protocols
- **Admin UI and CLI workflows** for local development and team usage
- **Extensibility** through plugins, templates, and deeper configuration

## Documentation Map

- [Getting Started](getting-started/getting-started.md) - Install MockForge and choose the right path
- [Tutorials](tutorials/README.md) - Follow end-to-end implementation workflows
- [Core Workflows](user-guide/http-mocking.md) - Learn the main mocking and simulation surfaces
- [Team and Cloud](user-guide/cloud-workspaces.md) - Shared workspaces, sync, and collaboration flows
- [Advanced and Labs](user-guide/advanced-features.md) - Explore deeper platform capabilities
- [Configuration](configuration/environment.md) - Configure MockForge for local and team use
- [API Reference](api/cli.md) - CLI, Admin API, and Rust API details
- [Reference](reference/faq.md) - Troubleshooting, schema, formats, and FAQ

## Examples

Check out the [`examples/`](../examples/) directory for sample configurations and use cases.

## Product Model

MockForge is organized around five product pillars: **Reality**, **Contracts**, **DevX**, **Cloud**, and **AI**. If you want the full internal model behind the docs structure, see [The Five Pillars](../../docs/PILLARS.md).

## Community

- [GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues) - Report bugs and request features
- [GitHub Discussions](https://github.com/SaaSy-Solutions/mockforge/discussions) - Ask questions and share ideas
- [Discord](https://discord.gg/2FxXqKpa) - Join our community chat

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/SaaSy-Solutions/mockforge/blob/main/LICENSE-APACHE))
- MIT License ([LICENSE-MIT](https://github.com/SaaSy-Solutions/mockforge/blob/main/LICENSE-MIT))

at your option.
