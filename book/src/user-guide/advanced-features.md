# Advanced Features

MockForge includes a comprehensive set of advanced features that enable sophisticated mocking scenarios, intelligent behavior simulation, and production-like testing environments. This section provides an overview of all advanced features with links to detailed documentation.

## Overview

MockForge's advanced features are organized into several categories:

- **Simulation & State Management**: Virtual Backend Reality (VBR), Temporal Simulation, Scenario State Machines
- **Intelligence & Automation**: MockAI, Generative Schema Mode, AI Contract Diff
- **Chaos & Realism**: Chaos Lab, Reality Slider
- **Collaboration & Cloud**: Cloud Workspaces, Data Scenario Marketplace
- **Developer Experience**: ForgeConnect SDK
- **Experimental Features**: Deceptive Deploys, Voice + LLM Interface, Reality Continuum, Smart Personas

## Simulation & State Management

### Virtual Backend Reality (VBR) Engine

The VBR Engine provides a virtual "database" layer that automatically generates CRUD operations from OpenAPI specifications. It supports relationship mapping, data persistence, and state management.

**Key Features:**
- Automatic CRUD generation from OpenAPI specs
- Support for 1:N and N:N relationships
- Multiple storage backends (JSON, SQLite, in-memory)
- Data seeding and state snapshots
- Realistic ID generation

**Learn More:** [VBR Engine Documentation](vbr-engine.md)

### Temporal Simulation (Time Travel)

Temporal Simulation allows you to control time in your mock environment, enabling time-based data mutations, scheduled events, and time-travel debugging.

**Key Features:**
- Virtual clock abstraction
- Time advancement controls
- Data mutation rules triggered by time
- Scheduler for simulated cron events
- UI controls for time travel

**Learn More:** [Temporal Simulation Documentation](temporal-simulation.md)

### Scenario State Machines 2.0

Advanced state machine system for modeling complex workflows and multi-step scenarios with visual flow editing and conditional transitions.

**Key Features:**
- Visual flow editor for state transitions
- Conditional transitions with if/else logic
- Reusable sub-scenarios
- Real-time preview of active state
- Programmatic state manipulation

**Learn More:** [Scenario State Machines Documentation](scenario-state-machines.md)

## Intelligence & Automation

### MockAI (Intelligent Mocking)

MockAI uses artificial intelligence to generate contextually appropriate, realistic API responses. It learns from OpenAPI specifications and example payloads.

**Key Features:**
- Trainable rule engine from examples or schema
- Context-aware conditional logic generation
- LLM-based dynamic response option
- Automatic fake data consistency
- Realistic validation error simulation

**Learn More:** [MockAI Documentation](mockai.md)

### Generative Schema Mode

Generate complete API ecosystems from JSON payloads, automatically creating routes, schemas, and entity relationships.

**Key Features:**
- Complete "JSON → entire API ecosystem" generation
- Auto-route generation with realistic CRUD mapping
- One-click environment creation from JSON payloads
- Entity relation inference
- Schema merging from multiple examples

**Learn More:** [Generative Schema Mode Documentation](generative-schema.md)

### AI Contract Diff

Automatically detect and analyze differences between API contracts and live requests, providing contextual recommendations for mismatches.

**Key Features:**
- Contract diff analysis between schema and live requests
- Contextual recommendations for mismatches
- Inline schema correction proposals
- CI/CD integration (contract verification step)
- Dashboard visualization of mismatches

**Learn More:** [AI Contract Diff Documentation](ai-contract-diff.md)

## Chaos & Realism

### Chaos Lab

Interactive network condition simulation with real-time latency visualization, network profiles, and error pattern scripting.

**Key Features:**
- Real-time latency visualization
- Network profile management (slow 3G, flaky Wi-Fi, etc.)
- Error pattern scripting (burst, random, sequential)
- Profile export/import
- CLI integration

**Learn More:** [Chaos Lab Documentation](chaos-lab.md)

### Reality Slider

Unified control mechanism that adjusts mock environment realism from simple static stubs to full production-level chaos.

**Key Features:**
- Configurable realism levels (1–5)
- Automated toggling of chaos, latency, and MockAI behaviors
- Persistent slider state per environment
- Export/import of realism presets
- Keyboard shortcuts for quick changes

**Learn More:** [Reality Slider Documentation](reality-slider.md)

## Collaboration & Cloud

### Cloud Workspaces

Multi-user collaborative editing with real-time state synchronization, version control, and role-based permissions.

**Key Features:**
- User authentication and access control
- Multi-user environment editing
- State synchronization between clients
- Git-style version control for mocks and data
- Role-based permissions (Owner, Editor, Viewer)

**Learn More:** [Cloud Workspaces Documentation](cloud-workspaces.md)

### Data Scenario Marketplace

Marketplace for downloadable mock templates with tags, ratings, versioning, and one-click import/export.

**Key Features:**
- Marketplace for downloadable mock templates
- Tags, ratings, and versioning
- One-click import/export
- Domain-specific packs (e-commerce, fintech, IoT)
- Automatic schema and route alignment

**Learn More:** [Scenario Marketplace Documentation](scenario-marketplace.md)

## Developer Experience

### ForgeConnect SDK

Browser extension and SDK for capturing network traffic, auto-generating mocks, and integrating with popular frameworks.

**Key Features:**
- Browser extension to capture network traffic
- Auto-mock generation for unhandled requests
- Local mock preview in browser
- SDK for framework bindings (React, Vue, Angular)
- Auth passthrough support for OAuth flows

**Learn More:** [ForgeConnect SDK Documentation](forgeconnect-sdk.md)

## Experimental Features

### Deceptive Deploys

Deploy mock APIs that look identical to production endpoints, perfect for demos, PoCs, and client presentations.

**Key Features:**
- Production-like headers and response patterns
- Production-like CORS configuration
- Production-like rate limiting
- OAuth flow simulation
- Auto-tunnel deployment

**Learn More:** [Deceptive Deploys Documentation](deceptive-deploys.md)

### Voice + LLM Interface

Generate OpenAPI specifications and mock APIs from natural language voice commands.

**Key Features:**
- Voice command parsing with LLM
- OpenAPI spec generation from voice commands
- Conversational mode for multi-turn interactions
- Single-shot mode for complete commands
- CLI and Web UI integration

**Learn More:** [Voice + LLM Interface Documentation](voice-llm-interface.md)

### Reality Continuum

Gradually transition from mock to real backend data by intelligently blending responses from both sources.

**Key Features:**
- Dynamic blending of mock and real responses
- Time-based progression with virtual clock integration
- Per-route, group-level, and global blend ratios
- Multiple merge strategies
- Fallback handling for failures

**Learn More:** [Reality Continuum Documentation](reality-continuum.md)

### Smart Personas

Generate coherent, consistent mock data using persona profiles with unique backstories and deterministic generation.

**Key Features:**
- Persona profile system with unique IDs and domains
- Coherent backstories with template-based generation
- Persona relationships (connections between personas)
- Deterministic data generation (same persona = same data)
- Domain-specific persona templates

**Learn More:** [Smart Personas Documentation](smart-personas.md)

## Getting Started

To get started with advanced features:

1. **Review the feature documentation** linked above for detailed information
2. **Check configuration examples** in the [Configuration Guide](../configuration/files.md)
3. **Try the tutorials** in the [Tutorials section](../tutorials/README.md)
4. **Explore examples** in the `examples/` directory

## Feature Comparison

| Feature | Use Case | Complexity |
|---------|----------|------------|
| VBR Engine | Stateful CRUD operations | Medium |
| Temporal Simulation | Time-based testing | Medium |
| MockAI | Intelligent responses | High |
| Chaos Lab | Resilience testing | Low |
| Reality Slider | Quick realism adjustment | Low |
| Cloud Workspaces | Team collaboration | Medium |
| ForgeConnect SDK | Browser-based development | Low |

## Best Practices

1. **Start Simple**: Begin with basic features (Chaos Lab, Reality Slider) before moving to advanced features
2. **Read Documentation**: Each feature has detailed documentation with examples
3. **Use Examples**: Check the `examples/` directory for working configurations
4. **Test Incrementally**: Enable features one at a time to understand their impact
5. **Monitor Performance**: Some features (like MockAI) may add latency

## Related Documentation

- [Advanced Behavior and Simulation](advanced-behavior.md) - Basic advanced features
- [Configuration Guide](../configuration/files.md) - How to configure features
- [API Reference](../api/rust.md) - Programmatic API access
- [Tutorials](../tutorials/README.md) - Step-by-step guides

