# Advanced Features

MockForge includes a comprehensive set of advanced features that enable sophisticated mocking scenarios, intelligent behavior simulation, and production-like testing environments. This section provides an overview of all advanced features with links to detailed documentation.

## Overview

MockForge's advanced features are organized into several categories:

- **Simulation & State Management**: Virtual Backend Reality (VBR), Temporal Simulation, Scenario State Machines, World State Engine
- **Intelligence & Automation**: MockAI, Generative Schema Mode, AI Contract Diff, API Architecture Critique, System Generation, Behavioral Simulation
- **Chaos & Realism**: Chaos Lab, Reality Slider, Reality Profiles Marketplace, Behavioral Economics Engine, Performance Mode
- **Collaboration & Cloud**: Cloud Workspaces, Data Scenario Marketplace, MockOps Pipelines, Federation, Analytics Dashboard
- **Developer Experience**: ForgeConnect SDK, Zero-Config Mode, Snapshot Diff, Mock-Oriented Development
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

### API Architecture Critique

LLM-powered analysis of API schemas to detect anti-patterns, redundancies, naming issues, emotional tone problems, and restructuring recommendations.

**Key Features:**
- Anti-pattern detection
- Redundancy detection
- Naming quality assessment
- Emotional tone analysis
- Restructuring recommendations

**Learn More:** [API Architecture Critique Documentation](../ai/api-architecture-critique.md)

### System Generation

Generate complete backend systems from natural language descriptions, including endpoints, personas, lifecycles, WebSocket topics, and more.

**Key Features:**
- 20-30 REST endpoints from description
- 4-5 personas based on roles
- 6-10 lifecycle states
- WebSocket topics
- Full OpenAPI spec
- CI pipeline templates

**Learn More:** [System Generation Documentation](../ai/system-generation.md)

### Behavioral Simulation

Model users as narrative agents that react to app state, form intentions, respond to errors, and trigger multi-step interactions.

**Key Features:**
- Narrative agents
- React to app state
- Form intentions (shop, browse, buy, abandon)
- Respond to errors
- Multi-step interactions

**Learn More:** [Behavioral Simulation Documentation](../ai/behavioral-simulation.md)

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

### Reality Profiles Marketplace

Pre-tuned "realism packs" that bundle personas, scenarios, chaos rules, latency curves, error distributions, and protocol behaviors into ready-to-use packages.

**Key Features:**
- E-Commerce Peak Season Pack
- Fintech Fraud Pack
- Healthcare HL7/Insurance Edge Cases Pack
- IoT Device Fleet Chaos Pack
- Custom pack creation

**Learn More:** [Reality Profiles Marketplace Documentation](advanced-features/reality-profiles-marketplace.md)

### Behavioral Economics Engine

Makes mocks react to real-world pressures like latency, load, pricing changes, fraud suspicion, and customer segments.

**Key Features:**
- Cart conversion drops if latency > 400ms
- Bank declines transactions if prior balance checks failed
- User churn increases after multiple 500s
- Declarative and scriptable rules

**Learn More:** [Behavioral Economics Engine Documentation](advanced-features/behavioral-economics.md)

### World State Engine

Unified visualization of all MockForge state systems—like a "miniature game engine for your backend."

**Key Features:**
- Unified state aggregation from all subsystems
- Graph visualization
- Real-time updates
- Time travel
- Query interface

**Learn More:** [World State Engine Documentation](advanced-features/world-state-engine.md)

### Performance Mode

Lightweight load simulation for running scenarios at N RPS, simulating bottlenecks, and recording latencies.

**Key Features:**
- Run scenarios at n RPS
- Simulate bottlenecks
- Record latencies
- Observe response changes under load

**Learn More:** [Performance Mode Documentation](advanced-features/performance-mode.md)

### Drift Learning

Mocks learn from recorded traffic patterns and adapt their behavior over time.

**Key Features:**
- Traffic pattern learning from recorded requests
- Persona behavior adaptation
- Configurable learning modes (behavioral, statistical, hybrid)
- Opt-in per endpoint/persona learning

**Learn More:** [Drift Learning Documentation](advanced-features/drift-learning.md)

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

### MockOps Pipelines

GitHub Actions-like automation for mock lifecycle management with event-driven pipelines.

**Key Features:**
- Schema change → auto-regenerate SDK
- Scenario published → auto-promote to test → notify teams
- Drift threshold exceeded → auto-generate Git PR
- Event-driven automation

**Learn More:** [MockOps Pipelines Documentation](../cloud/mockops-pipelines.md)

### Multi-Workspace Federation

Compose multiple mock workspaces into one federated "virtual system" for large organizations with microservices.

**Key Features:**
- Service boundary definition
- Compose workspaces into virtual systems
- System-wide scenarios
- Per-service reality level control

**Learn More:** [Federation Documentation](../cloud/federation.md)

### Analytics Dashboard

Leadership insight into coverage, risk, and usage with heatmaps, CI tracking, and coverage analysis.

**Key Features:**
- Scenario usage heatmaps
- Persona CI hit tracking
- Endpoint coverage analysis
- Reality level staleness detection
- Drift percentage tracking

**Learn More:** [Analytics Dashboard Documentation](../cloud/analytics-dashboard.md)

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

### Zero-Config Mode (Runtime Daemon)

The "invisible mock server" experience—automatically creates mocks, generates types, and sets up scenarios when you hit non-existent endpoints.

**Key Features:**
- Auto-detection of 404 responses
- Automatic mock creation
- Type generation
- Client stub generation
- OpenAPI schema updates

**Learn More:** [Zero-Config Mode Documentation](../devx/zero-config-mode.md)

### Snapshot Diff

Side-by-side visualization for comparing mock behavior between environments, personas, scenarios, or reality levels.

**Key Features:**
- Compare test vs prod
- Compare personas
- Compare reality levels
- Side-by-side visualization

**Learn More:** [Snapshot Diff Documentation](../devx/snapshot-diff.md)

### Mock-Oriented Development (MOD)

A software development methodology that places mocks at the center of the development workflow.

**Key Features:**
- Mock-first design
- Contract-driven development
- Reality progression
- Scenario-driven testing

**Learn More:** [MOD Documentation](../devx/mock-oriented-development.md)

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

