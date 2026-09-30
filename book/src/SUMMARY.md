# Summary

[Introduction](README.md)

## Getting Started

- [Getting Started](getting-started/getting-started.md)
- [Installation](getting-started/installation.md)
- [Your First Mock API in 5 Minutes](getting-started/five-minute-api.md)
- [Quick Start](getting-started/quick-start.md)
- [Basic Concepts](getting-started/concepts.md)

### Choose Your Path

- [Open Source Builder](getting-started/devx-first.md) - Start here for local CLI workflows and fast setup
- [Product Team](getting-started/reality-first.md) - Start here for realistic mocks across frontend, backend, and QA
- [Platform / API Team](getting-started/contracts-first.md) - Start here for spec-driven and contract-heavy workflows
- [Cloud / Enterprise](getting-started/cloud-first.md) - Start here for shared environments and managed rollout
- [AI-First Onboarding](getting-started/ai-first.md) - Start here if you want natural-language-driven mocks

## Tutorials

- [Overview](tutorials/README.md)
- [The Golden Path: Blueprint → Dev-Setup → Integration](tutorials/golden-path.md) ⭐ **Start Here**
- [Mock a REST API from OpenAPI](tutorials/mock-openapi-spec.md)
- [React + MockForge Workflow](tutorials/react-workflow.md)
- [Vue + MockForge Workflow](tutorials/vue-workflow.md)
- [Admin UI Walkthrough](tutorials/admin-ui-walkthrough.md)
- [Plugin Starter Guide](tutorials/plugin-starter.md) - Create your first plugin
- [IDE Extension Guide](tutorials/ide-extension-guide.md) - VS Code extension walkthrough
- [Add a Custom Plugin](tutorials/add-custom-plugin.md)

## Core Workflows

- [HTTP Mocking](user-guide/http-mocking.md)
  - [OpenAPI Integration](user-guide/http-mocking/openapi.md)
  - [Custom Responses](user-guide/http-mocking/custom-responses.md)
  - [Dynamic Data](user-guide/http-mocking/dynamic-data.md)
- [Advanced Behavior and Simulation](user-guide/advanced-behavior.md)
- [gRPC Mocking](user-guide/grpc-mocking.md)
  - [Protocol Buffers](user-guide/grpc-mocking/protobuf.md)
  - [Streaming](user-guide/grpc-mocking/streaming.md)
  - [Advanced Data Synthesis](user-guide/grpc-mocking/advanced-data-synthesis.md)
- [Kafka Mocking](user-guide/kafka-mocking.md)
- [GraphQL Mocking](user-guide/graphql-mocking.md)
- [WebSocket Mocking](user-guide/websocket-mocking.md)
  - [Replay Mode](user-guide/websocket-mocking/replay.md)
  - [Interactive Mode](user-guide/websocket-mocking/interactive.md)
- [Plugin System](user-guide/plugins.md)
- [Security & Encryption](user-guide/security.md)
- [Directory Synchronization](user-guide/sync.md)
- [Admin UI](user-guide/admin-ui.md)
- [IDE Integration](user-guide/ide-integration.md)

## Testing & Resilience

- [Load Testing](user-guide/load-testing.md)
- [WAF Testing](user-guide/waf-testing.md)
- [Chaos Engineering](user-guide/chaos-engineering.md)
- [Rate Limiting & Traffic Shaping](user-guide/rate-limiting.md)
- [Observability & Metrics](user-guide/observability.md)
- [TUI Dashboard](user-guide/tui-dashboard.md)

## Team and Cloud

- [Cloud Workspaces](user-guide/cloud-workspaces.md)
- [MockOps Pipelines](user-guide/cloud/mockops-pipelines.md)
- [Multi-Workspace Federation](user-guide/cloud/federation.md)
- [Analytics Dashboard](user-guide/cloud/analytics-dashboard.md)

## Advanced and Labs

- [Advanced Features](user-guide/advanced-features.md)
  - [Temporal Simulation](user-guide/temporal-simulation.md)
  - [Scenario State Machines](user-guide/scenario-state-machines.md)
  - [MockAI](user-guide/mockai.md)
  - [AI / RAG-Driven Mock Data](user-guide/ai-rag.md)
  - [AI Contract Diff](user-guide/ai-contract-diff.md)
  - [Chaos Lab](user-guide/chaos-lab.md)
  - [Reality Slider](user-guide/reality-slider.md)
  - [Scenario Marketplace](user-guide/scenario-marketplace.md)
  - [Mock-Oriented Development](user-guide/devx/mock-oriented-development.md)
  - [VBR Engine (Experimental)](user-guide/vbr-engine.md)
  - [Reality Profiles Marketplace (Experimental)](user-guide/advanced-features/reality-profiles-marketplace.md)
  - [ForgeConnect SDK (Experimental)](user-guide/forgeconnect-sdk.md)
  - [Reality Continuum (Experimental)](user-guide/reality-continuum.md)
  - [Smart Personas (Experimental)](user-guide/smart-personas.md)
  - [API Change Forecasting (Experimental)](user-guide/contracts/api-change-forecasting.md)
  - [Semantic Drift Notifications (Experimental)](user-guide/contracts/semantic-drift.md)
  - [Contract Threat Modeling (Experimental)](user-guide/contracts/threat-modeling.md)
  - [Snapshot Diff (Experimental)](user-guide/devx/snapshot-diff.md)
  - [Deceptive Deploys (Experimental)](user-guide/deceptive-deploys.md)
  - [Voice + LLM Interface (Experimental)](user-guide/voice-llm-interface.md)

## Experimental / Roadmap

- [Generative Schema Mode (Planned)](user-guide/generative-schema.md)
- [Behavioral Economics Engine (Planned)](user-guide/advanced-features/behavioral-economics.md)
- [World State Engine (Planned)](user-guide/advanced-features/world-state-engine.md)
- [Performance Mode (Planned)](user-guide/advanced-features/performance-mode.md)
- [Drift Learning (Planned)](user-guide/advanced-features/drift-learning.md)
- [Zero-Config Mode (Planned)](user-guide/devx/zero-config-mode.md)
- [API Architecture Critique (Planned)](user-guide/ai/api-architecture-critique.md)
- [System Generation (Planned)](user-guide/ai/system-generation.md)
- [Behavioral Simulation (Planned)](user-guide/ai/behavioral-simulation.md)

## Additional Protocols

- [SMTP](protocols/smtp/getting-started.md)
  - [Configuration](protocols/smtp/configuration.md)
  - [Fixtures](protocols/smtp/fixtures.md)
  - [Examples](protocols/smtp/examples.md)
- [MQTT](protocols/mqtt/getting-started.md)
  - [Configuration](protocols/mqtt/configuration.md)
  - [Fixtures](protocols/mqtt/fixtures.md)
  - [Examples](protocols/mqtt/examples.md)
- [FTP](protocols/ftp/getting-started.md)
  - [Configuration](protocols/ftp/configuration.md)
  - [Fixtures](protocols/ftp/fixtures.md)
  - [Examples](protocols/ftp/examples.md)
- [TCP](protocols/tcp/getting-started.md)

## Configuration

- [Environment Variables](configuration/environment.md)
- [Configuration Files](configuration/files.md)
- [Advanced Options](configuration/advanced.md)

## Development

- [Building from Source](development/building.md)
- [Testing](development/testing.md)
- [Architecture](development/architecture.md)
  - [CLI Crate](development/architecture/cli.md)
  - [HTTP Crate](development/architecture/http.md)
  - [gRPC Crate](development/architecture/grpc.md)
  - [WebSocket Crate](development/architecture/ws.md)

## API Reference

- [CLI Reference](api/cli.md)
- [Admin UI REST API](api/admin-ui-rest.md)
- [Rust API](api/rust.md)
  - [HTTP Module](api/rust/http.md)
  - [gRPC Module](api/rust/grpc.md)
  - [WebSocket Module](api/rust/ws.md)

## Contributing

- [Development Setup](contributing/setup.md)
- [Code Style](contributing/style.md)
- [Testing Guidelines](contributing/testing.md)
- [Release Process](contributing/release.md)

## Reference

- [The Five Pillars](../../docs/PILLARS.md)
- [Configuration Schema](reference/config-schema.md)
- [Configuration Validation](reference/config-validation.md)
- [Supported Formats](reference/formats.md)
- [Templating Reference](reference/templating.md)
- [Request Chaining](reference/chaining.md)
- [Fixtures and Smoke Testing](reference/fixtures.md)
- [Conformance Self-Test Probes](reference/conformance-self-test-probes.md)
- [Bench Custom Checks (Uploads + Chains)](reference/bench-custom-checks.md)
- [Bench Capacity Sizing](reference/bench-capacity-sizing.md)
- [Agent / LLM / MCP Traffic (Packet-Level)](reference/agent-llm-mcp-traffic.md)
- [Troubleshooting](reference/troubleshooting.md)
- [Common Issues & Solutions](reference/common-issues.md)
- [FAQ](reference/faq.md)
- [Changelog](reference/changelog.md)
