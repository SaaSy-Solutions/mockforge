# Summary

[Introduction](README.md)

## Getting Started

- [Getting Started](getting-started/getting-started.md)
- [Installation](getting-started/installation.md)
- [Your First Mock API in 5 Minutes](getting-started/five-minute-api.md)
- [Quick Start](getting-started/quick-start.md)
- [Basic Concepts](getting-started/concepts.md)
- [The Five Pillars](../../docs/PILLARS.md)

### Choose Your Path

- [Reality-First Onboarding](getting-started/reality-first.md) - Start here if you care about realism
- [Contracts-First Onboarding](getting-started/contracts-first.md) - Start here if you're a Platform/API team
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

## Protocols

- [MQTT](protocols/mqtt/getting-started.md)
  - [Configuration](protocols/mqtt/configuration.md)
  - [Fixtures](protocols/mqtt/fixtures.md)
  - [Examples](protocols/mqtt/examples.md)
- [SMTP](protocols/smtp/getting-started.md)
   - [Configuration](protocols/smtp/configuration.md)
   - [Fixtures](protocols/smtp/fixtures.md)
   - [Examples](protocols/smtp/examples.md)
- [FTP](protocols/ftp/getting-started.md)
   - [Configuration](protocols/ftp/configuration.md)
   - [Fixtures](protocols/ftp/fixtures.md)
   - [Examples](protocols/ftp/examples.md)

## User Guide

- [HTTP Mocking](user-guide/http-mocking.md)
  - [OpenAPI Integration](user-guide/http-mocking/openapi.md)
  - [Custom Responses](user-guide/http-mocking/custom-responses.md)
  - [Dynamic Data](user-guide/http-mocking/dynamic-data.md)
- [Advanced Behavior and Simulation](user-guide/advanced-behavior.md)
- [gRPC Mocking](user-guide/grpc-mocking.md)
  - [Protocol Buffers](user-guide/grpc-mocking/protobuf.md)
  - [Streaming](user-guide/grpc-mocking/streaming.md)
  - [Advanced Data Synthesis](user-guide/grpc-mocking/advanced-data-synthesis.md)
- [GraphQL Mocking](user-guide/graphql-mocking.md)
- [WebSocket Mocking](user-guide/websocket-mocking.md)
  - [Replay Mode](user-guide/websocket-mocking/replay.md)
  - [Interactive Mode](user-guide/websocket-mocking/interactive.md)
- [Plugin System](user-guide/plugins.md)
- [Security & Encryption](user-guide/security.md)
- [Directory Synchronization](user-guide/sync.md)
- [Admin UI](user-guide/admin-ui.md)
- [IDE Integration](user-guide/ide-integration.md)
- [Advanced Features](user-guide/advanced-features.md)
  - [VBR Engine](user-guide/vbr-engine.md)
  - [Temporal Simulation](user-guide/temporal-simulation.md)
  - [Scenario State Machines](user-guide/scenario-state-machines.md)
  - [MockAI](user-guide/mockai.md)
  - [Generative Schema Mode](user-guide/generative-schema.md)
  - [AI Contract Diff](user-guide/ai-contract-diff.md)
  - [Chaos Lab](user-guide/chaos-lab.md)
  - [Reality Slider](user-guide/reality-slider.md)
  - [Reality Profiles Marketplace](user-guide/advanced-features/reality-profiles-marketplace.md)
  - [Behavioral Economics Engine](user-guide/advanced-features/behavioral-economics.md)
  - [World State Engine](user-guide/advanced-features/world-state-engine.md)
  - [Performance Mode](user-guide/advanced-features/performance-mode.md)
  - [Drift Learning](user-guide/advanced-features/drift-learning.md)
  - [Cloud Workspaces](user-guide/cloud-workspaces.md)
  - [Scenario Marketplace](user-guide/scenario-marketplace.md)
  - [ForgeConnect SDK](user-guide/forgeconnect-sdk.md)
  - [Deceptive Deploys](user-guide/deceptive-deploys.md)
  - [Voice + LLM Interface](user-guide/voice-llm-interface.md)
  - [Reality Continuum](user-guide/reality-continuum.md)
  - [Smart Personas](user-guide/smart-personas.md)
  - [API Change Forecasting](user-guide/contracts/api-change-forecasting.md)
  - [Semantic Drift Notifications](user-guide/contracts/semantic-drift.md)
  - [Contract Threat Modeling](user-guide/contracts/threat-modeling.md)
  - [Zero-Config Mode](user-guide/devx/zero-config-mode.md)
  - [Mock-Oriented Development](user-guide/devx/mock-oriented-development.md)
  - [Snapshot Diff](user-guide/devx/snapshot-diff.md)
  - [MockOps Pipelines](user-guide/cloud/mockops-pipelines.md)
  - [Multi-Workspace Federation](user-guide/cloud/federation.md)
  - [Analytics Dashboard](user-guide/cloud/analytics-dashboard.md)
  - [API Architecture Critique](user-guide/ai/api-architecture-critique.md)
  - [System Generation](user-guide/ai/system-generation.md)
  - [Behavioral Simulation](user-guide/ai/behavioral-simulation.md)

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

- [Configuration Schema](reference/config-schema.md)
- [Configuration Validation](reference/config-validation.md)
- [Supported Formats](reference/formats.md)
- [Templating Reference](reference/templating.md)
- [Request Chaining](reference/chaining.md)
- [Fixtures and Smoke Testing](reference/fixtures.md)
- [Troubleshooting](reference/troubleshooting.md)
- [Common Issues & Solutions](reference/common-issues.md)
- [FAQ](reference/faq.md)
- [Changelog](reference/changelog.md)
