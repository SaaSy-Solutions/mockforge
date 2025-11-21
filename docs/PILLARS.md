# The Five Pillars of MockForge

**Version:** 1.0.0
**Last Updated:** 2025-01-27

MockForge is built on five foundational pillars that define our product vision and guide every feature we build. These pillars ensure that MockForge delivers a cohesive, powerful mocking experience that scales from solo developers to enterprise teams.

## Overview

Every feature in MockForge maps to one or more pillars. This structure helps us:
- **Communicate value** clearly in changelogs, docs, and marketing
- **Prioritize development** based on pillar investment
- **Maintain consistency** across features and releases
- **Guide users** to the right features for their needs

---

## The Five Pillars

### [Reality] – Everything that makes mocks feel like a real, evolving backend

**Purpose:** Make mocks indistinguishable from production backends through realistic behavior, state management, and dynamic data generation.

**Key Capabilities:**
- Realistic data generation with relationships and constraints
- Stateful behavior and persistence
- Network condition simulation (latency, packet loss, failures)
- Time-based mutations and temporal simulation
- Progressive data evolution and drift
- Multi-protocol support (HTTP, gRPC, WebSocket, Kafka, MQTT, AMQP, SMTP, FTP, TCP)

**Example Features:**
- **Reality Continuum**: Blend mock and real data with configurable reality levels
- **Reality Slider**: Hot-reload reality level adjustments
- **Smart Personas**: Consistent cross-endpoint data generation
- **Generative Schema Mode**: Dynamic mock data generation without seed data
- **Chaos Lab**: Interactive network condition simulation
- **Deceptive Deploy**: Advanced testing scenarios with realistic failures
- **Virtual Backend Reality (VBR)**: Virtual database layer with CRUD operations
- **Temporal Simulation**: Time travel and time-based data mutations
- **Latency Injection**: Configurable latency profiles and recording
- **Response Selection Modes**: Sequential, random, and weighted response selection
- **Template Expansion**: Dynamic data generation with faker functions
- **Capture Scrubbing**: Deterministic replay with data sanitization

**Use Cases:**
- Frontend teams need realistic data that evolves over time
- Testing resilience and failure scenarios
- Simulating production-like network conditions
- Creating believable mock backends before real APIs exist

---

### [Contracts] – Schema, drift, validation, and safety nets

**Purpose:** Ensure API contracts are correct, validated, and stay in sync with real backends.

**Key Capabilities:**
- OpenAPI/GraphQL schema validation
- Request/response validation with detailed error reporting
- Contract drift detection and monitoring
- Automatic API sync and change detection
- Schema-driven mock generation
- Cross-endpoint validation and referential integrity
- Multi-protocol contract support (HTTP, gRPC, WebSocket, MQTT, Kafka)
- Contract fitness functions for quality enforcement
- Consumer impact analysis for downstream dependencies

**Example Features:**
- **AI Contract Diff**: Compare and visualize API contract changes
- **Automatic API Sync & Change Detection**: Periodic polling and sync for upstream API changes
- **Request/Response Validation**: Built-in validation with configurable modes (disabled, warn, enforce)
- **OpenAPI Integration**: Full OpenAPI v3 support with deep $ref resolution
- **Schema Validation**: Composite schemas (oneOf/anyOf/allOf) support
- **Validation Modes**: Runtime admin UI to view/toggle validation mode
- **Contract Testing**: Verify API contracts match specifications
- **Protocol Contracts**: gRPC, WebSocket, MQTT, and Kafka contract management
- **Fitness Functions**: Custom tests to enforce contract quality and evolution rules
- **Consumer Impact Analysis**: Map endpoints to SDK methods and consuming applications
- **Drift Budgets**: Configurable thresholds for acceptable contract changes

**Use Cases:**
- Catch breaking changes before they reach production
- Ensure mocks stay in sync with real APIs
- Validate API contracts during development
- Detect schema drift automatically

---

### [DevX] – SDKs, generators, playgrounds, ergonomics

**Purpose:** Make MockForge effortless to use, integrate, and extend for developers.

**Key Capabilities:**
- Multi-language SDKs (Rust, Node.js, Python, Go, Java, .NET)
- Client code generation (React, Vue, Angular, Svelte)
- Interactive playgrounds and admin UI
- CLI tooling and configuration management
- Comprehensive documentation and examples
- Plugin system for extensibility

**Example Features:**
- **ForgeConnect SDK**: Complete SDK implementation with full feature set
- **GraphQL + REST Playground**: Interactive playground with workspace filtering
- **Multi-Language SDKs**: Native SDKs for 6 languages
- **Client Generators**: React, Vue, Angular, Svelte client code generation
- **CLI Tool**: Full-featured command-line interface
- **Admin UI**: Modern React-based admin interface
- **Configuration Management**: YAML/JSON config files with profiles
- **Browser Proxy Mode**: Seamless integration with browser workflows
- **Git Sync**: Workspace synchronization via Git
- **Plugin System**: WASM-based plugin architecture
- **E2E Test Suite**: Comprehensive end-to-end testing tools
- **Custom Routes**: Flexible routing configuration
- **Voice + LLM Interface**: Voice interface with Speech-to-Text support
- **WireMock-Inspired Features**: Familiar patterns for WireMock users

**Use Cases:**
- Embed mock servers directly in test suites
- Generate type-safe API clients automatically
- Integrate with existing development workflows
- Extend functionality with custom plugins

---

### [Cloud] – Registry, orgs, governance, monetization, marketplace

**Purpose:** Enable team collaboration, sharing, and scaling from solo developers to enterprise organizations.

**Key Capabilities:**
- Organization and user management
- Scenario marketplace and sharing
- Registry server for mock distribution
- Cloud workspaces and synchronization
- Governance and access controls
- Monetization infrastructure

**Example Features:**
- **Cloud Workspaces**: Shared workspaces with team collaboration
- **Scenario Marketplace**: Browse and share mock scenarios
- **Registry Server**: Centralized mock distribution and discovery
- **Organization Management**: Multi-tenant organization support
- **User Management**: Team member and permission management
- **Cloud Sync**: Synchronize workspaces across devices
- **Security Controls**: Enterprise-grade access controls and audit trails
- **Cloud Monetization**: Pricing models and subscription management
- **Enhanced Metrics**: Team-level analytics and monitoring

**Use Cases:**
- Share mock scenarios across teams
- Manage enterprise deployments
- Discover and reuse community scenarios
- Scale from individual to team usage

---

### [AI] – LLM/voice flows, AI diff/assist, generative behaviors

**Purpose:** Leverage artificial intelligence to automate mock generation, enhance data realism, and assist developers.

**Key Capabilities:**
- LLM-powered mock generation
- AI-driven data synthesis
- Voice interface for mock creation
- Intelligent contract analysis
- Generative data behaviors
- Natural language to mock conversion

**Example Features:**
- **MockAI**: Intelligent mock generation from natural language
- **AI Response Generation**: LLM-powered realistic response generation
- **AI Contract Diff**: AI-assisted contract comparison and analysis
- **Voice + LLM Interface**: Voice-driven mock creation and management
- **Generative Schema Mode**: AI-powered schema extrapolation
- **AI Event Streams**: LLM-generated narrative-driven WebSocket events
- **Data Drift Simulation**: AI-driven evolving mock data
- **Smart Data Generation**: RAG-powered synthetic data with relationship awareness

**Use Cases:**
- Generate mocks from natural language descriptions
- Create realistic data without manual configuration
- Analyze and compare API contracts intelligently
- Build mocks through voice commands

---

## Pillar Feature Matrix

This matrix shows how key MockForge features map to the five pillars:

| Feature | Reality | Contracts | DevX | Cloud | AI |
|---------|---------|-----------|------|-------|-----|
| **Reality Continuum** | ✅ | | | | |
| **Reality Slider** | ✅ | | ✅ | | |
| **Smart Personas** | ✅ | | | | ✅ |
| **Generative Schema Mode** | ✅ | ✅ | | | ✅ |
| **Chaos Lab** | ✅ | | ✅ | | |
| **Deceptive Deploy** | ✅ | | | | |
| **VBR Engine** | ✅ | ✅ | | | |
| **Temporal Simulation** | ✅ | | ✅ | | |
| **AI Contract Diff** | | ✅ | ✅ | | ✅ |
| **Automatic API Sync** | | ✅ | | | |
| **Request/Response Validation** | | ✅ | ✅ | | |
| **ForgeConnect SDK** | | | ✅ | | |
| **Client Generators** | | ✅ | ✅ | | |
| **GraphQL + REST Playground** | | | ✅ | | |
| **Admin UI** | | | ✅ | | |
| **Plugin System** | | | ✅ | | |
| **Cloud Workspaces** | | | | ✅ | |
| **Scenario Marketplace** | | | | ✅ | |
| **Registry Server** | | | | ✅ | |
| **Organization Management** | | | | ✅ | |
| **MockAI** | | | ✅ | | ✅ |
| **Voice + LLM Interface** | | | ✅ | | ✅ |
| **AI Event Streams** | ✅ | | | | ✅ |
| **Data Drift Simulation** | ✅ | | | | ✅ |

---

## Using Pillars in Changelog Entries

When adding features to the changelog, tag them with one or more relevant pillars:

```markdown
- **[Reality] Smart Personas** with array generation and relationship inference

- **[Contracts][DevX] AI Contract Diff** with interactive playground integration

- **[Cloud] Organization management endpoints** for team collaboration
```

**Guidelines:**
- Use `[Pillar]` format with square brackets
- Multiple pillars: `[Pillar1][Pillar2]`
- Tag the primary pillar first, then secondary pillars
- Every major feature should have at least one pillar tag
- Minor fixes and internal changes may not need tags

---

## Pillar Investment by Release

Understanding which pillars receive investment in each release helps users understand the product direction:

- **Reality-heavy releases**: Focus on making mocks more realistic and production-like
- **Contracts-heavy releases**: Emphasis on validation, sync, and contract safety
- **DevX-heavy releases**: Improved ergonomics, SDKs, and developer experience
- **Cloud-heavy releases**: Team features, collaboration, and enterprise capabilities
- **AI-heavy releases**: Intelligent automation and AI-powered features

---

## Documentation by Pillar

This section organizes MockForge documentation by pillar to help you find relevant guides quickly.

**New to MockForge?** Start with [Journeys by Pillar](JOURNEYS_BY_PILLAR.md) to choose the onboarding path that best fits your needs.

### [Reality] Documentation

**Getting Started:**
- [Reality-First Onboarding](../book/src/getting-started/reality-first.md) - Start here for reality features

**Core Features:**
- [Reality Continuum](REALITY_CONTINUUM.md) - Blend mock and real data
- [Smart Personas](PERSONAS.md) - Consistent cross-endpoint data generation
- [Reality Slider](REALITY_SLIDER.md) - Hot-reload reality level adjustments
- [Chaos Lab](CHAOS_LAB.md) - Interactive network condition simulation
- [Behavioral Cloning](BEHAVIORAL_CLONING.md) - Record and replay realistic flows
- [Reality Trace](REALITY_TRACE.md) - Observability for mock behavior

**Advanced:**
- [Lifecysles and Time](LIFECYCLES_AND_TIME.md) - Time-based mutations and temporal simulation
- [Deceptive Deploy](DECEPTIVE_DEPLOY.md) - Advanced testing scenarios

### [Contracts] Documentation

**Getting Started:**
- [Contracts-First Onboarding](../book/src/getting-started/contracts-first.md) - Start here for contract features

**Core Features:**
- [Drift Budgets](DRIFT_BUDGETS.md) - Configurable thresholds for contract changes
- [Protocol Contracts](PROTOCOL_CONTRACTS.md) - Multi-protocol contract management
- [Contract Fitness](CONTRACT_FITNESS.md) - Quality enforcement functions
- [Consumer Impact Analysis](CONSUMER_IMPACT_ANALYSIS.md) - Map endpoints to consumers

**Advanced:**
- [Drift Budget Setup](DRIFT_BUDGET_SETUP.md) - Configuration guide

### [DevX] Documentation

**Getting Started:**
- [DevX-First Onboarding](../book/src/getting-started/devx-first.md) - Start here for developer experience features

**Core Features:**
- [Admin UI Quickstart](ADMIN_UI_QUICKSTART.md) - Interactive playground
- [Plugin Marketplace](PLUGIN_MARKETPLACE_PRODUCTION.md) - Extend MockForge
- [Multi-Framework Client Generation](MULTI_FRAMEWORK_CLIENT_GENERATION.md) - Generate clients

**Advanced:**
- [Developer Workflow Integration](DEVELOPER_WORKFLOW_INTEGRATION.md) - Integrate into workflows
- [Reality Trace](REALITY_TRACE.md) - Developer debugging tools

### [Cloud] Documentation

**Getting Started:**
- [Cloud-First Onboarding](../book/src/getting-started/cloud-first.md) - Start here for cloud features

**Core Features:**
- [Cloud Environments](CLOUD_ENVIRONMENTS.md) - Multi-tenant workspaces
- [Scenario Marketplace](SCENARIOS_MARKETPLACE.md) - Discover and share scenarios
- [Marketplace Monitoring](MARKETPLACE_MONITORING.md) - Analytics and dashboards
- [Cloud Sync Implementation](CLOUD_SYNC_IMPLEMENTATION_GUIDE.md) - Synchronization guide
- [RBAC Guide](RBAC_GUIDE.md) - Access control and governance

**Advanced:**
- [Enterprise Deployment Guide](ENTERPRISE_DEPLOYMENT_GUIDE.md) - Enterprise features
- [Cloud SaaS MVP Guide](CLOUD_SAAS_MVP_GUIDE.md) - SaaS implementation

### [AI] Documentation

**Getting Started:**
- [AI-First Onboarding](../book/src/getting-started/ai-first.md) - Start here for AI features

**Core Features:**
- [AI Studio](AI_STUDIO.md) - Unified AI interface
- [MockAI Usage](MOCKAI_USAGE.md) - Natural language mock generation
- [AI Contract Diff](PROTOCOL_CONTRACTS.md#ai-contract-diff) - Intelligent contract analysis

**Advanced:**
- [AI Features README](AI_FEATURES_README.md) - Complete AI feature overview
- [AI-Driven Mocking](AI_DRIVEN_MOCKING.md) - AI-powered mock generation

### Cross-Pillar Documentation

- [Journeys by Pillar](JOURNEYS_BY_PILLAR.md) - Choose your onboarding journey
- [Pillar Tagging Guide](contributing/PILLAR_TAGGING.md) - How to tag code with pillars
- [Pillar Queries Guide](contributing/PILLAR_QUERIES.md) - Query by pillar

---

## References

- [Changelog](../CHANGELOG.md) - See pillar tags in action
- [Getting Started Guide](../book/src/getting-started/getting-started.md) - Learn about MockForge
- [Contributing Guide](../CONTRIBUTING.md) - How to contribute with pillar tagging
- [Release Process](../book/src/contributing/release.md) - Release process with pillar requirements

---

## Questions?

If you're unsure which pillar(s) a feature belongs to:

1. **Reality**: Does it make mocks feel more like real backends?
2. **Contracts**: Does it validate, sync, or monitor API contracts?
3. **DevX**: Does it improve developer experience or ergonomics?
4. **Cloud**: Does it enable team collaboration or sharing?
5. **AI**: Does it use AI/LLM to automate or enhance functionality?

Many features span multiple pillars—that's intentional and encouraged!
