# Journeys by Pillar

**Version:** 1.0.0
**Last Updated:** 2025-01-27

This document provides an overview of pillar-first onboarding journeys for MockForge. Each journey is designed to guide users to the features most relevant to their needs.

## Overview

MockForge is built on five foundational pillars. Depending on your role, team, or use case, you may want to start with a specific pillar. These journeys help you get started quickly with the features that matter most to you.

## The Five Pillars

- **[Reality]** – Everything that makes mocks feel like a real, evolving backend
- **[Contracts]** – Schema, drift, validation, and safety nets
- **[DevX]** – SDKs, generators, playgrounds, ergonomics
- **[Cloud]** – Registry, orgs, governance, monetization, marketplace
- **[AI]** – LLM/voice flows, AI diff/assist, generative behaviors

See [PILLARS.md](PILLARS.md) for detailed information about each pillar.

## Pillar-First Journeys

### [Reality] Reality-First Onboarding

**Start here if:** You care about realism. You want mocks that feel indistinguishable from production backends.

**Journey:** [Reality-First Onboarding](../book/src/getting-started/reality-first.md)

**Key Features:**
- Reality Continuum - Blend mock and real data
- Smart Personas - Consistent, relationship-aware data
- Reality Slider - Adjust realism levels on the fly
- Chaos Lab - Simulate network conditions and failures
- Temporal Simulation - Time travel and time-based mutations

**Perfect for:**
- Frontend teams needing realistic data
- Testing resilience and failure scenarios
- Simulating production-like network conditions
- Creating believable mock backends before real APIs exist

**Quick Start:**
```bash
mockforge serve --config mockforge.yaml
# Configure reality level and personas in your config
```

### [Contracts] Contracts-First Onboarding

**Start here if:** You're a Platform/API team. You need to ensure API contracts are correct, validated, and stay in sync.

**Journey:** [Contracts-First Onboarding](../book/src/getting-started/contracts-first.md)

**Key Features:**
- Request/Response Validation - Validate against OpenAPI schemas
- Contract Drift Detection - Monitor contract changes
- Automatic API Sync - Keep mocks in sync with real APIs
- AI Contract Diff - Intelligently analyze contract changes
- Multi-Protocol Contracts - Manage contracts across HTTP, gRPC, WebSocket, MQTT, Kafka

**Perfect for:**
- API teams managing contract evolution
- Platform teams ensuring contract consistency
- Teams needing automatic API sync and change detection
- Organizations requiring contract validation and drift monitoring

**Quick Start:**
```bash
mockforge serve --config mockforge.yaml --validate
# Configure validation and drift detection in your config
```

### [AI] AI-First Onboarding

**Start here if:** You want natural-language-driven mocks. You want to generate mocks from descriptions and use voice commands.

**Journey:** [AI-First Onboarding](../book/src/getting-started/ai-first.md)

**Key Features:**
- MockAI - Natural language mock generation
- Voice + LLM Interface - Build mocks using voice commands
- AI Contract Diff - Intelligently analyze contract changes
- Generative Schema Mode - Generate APIs from JSON examples
- AI Event Streams - Narrative-driven WebSocket events

**Perfect for:**
- Teams wanting to generate mocks from natural language
- Developers who prefer conversational interfaces
- Teams needing AI-powered contract analysis
- Organizations wanting to automate mock generation

**Quick Start:**
```bash
mockforge serve --ai-enabled
# Use MockAI to generate mocks from natural language
```

### [DevX] DevX-First Onboarding

**Start here if:** You care about developer experience. You want easy-to-use SDKs, code generators, interactive playgrounds, and ergonomic tooling.

**Journey:** [DevX-First Onboarding](../book/src/getting-started/devx-first.md)

**Key Features:**
- Multi-Language SDKs - Rust, Node.js, Python, Go, Java, .NET
- Client Code Generation - TypeScript, React, Vue, Angular, Svelte
- Interactive Playground - Visual endpoint builder and testing
- CLI Tooling - Comprehensive command-line interface
- Plugin System - Extend MockForge with custom functionality

**Perfect for:**
- Developers who want to integrate mocks into their test suites quickly
- Teams needing client code generation for their APIs
- Developers who prefer interactive playgrounds over configuration files
- Teams wanting plugin-based extensibility

**Quick Start:**
```bash
# Install SDK for your language
npm install @mockforge/sdk  # Node.js
pip install mockforge-sdk   # Python
# Or use the playground
mockforge serve --playground
```

### [Cloud] Cloud-First Onboarding

**Start here if:** You're a team or organization that needs collaboration, sharing, and governance. You want to share mock scenarios across teams and leverage the marketplace.

**Journey:** [Cloud-First Onboarding](../book/src/getting-started/cloud-first.md)

**Key Features:**
- Organization Management - Multi-tenant workspaces and team collaboration
- Scenario Marketplace - Discover and publish pre-built scenarios
- Registry Server - Centralized mock distribution
- Cloud Workspaces - Synchronization and backup
- Governance & Access Control - RBAC and audit logging

**Perfect for:**
- Teams needing to share mock scenarios
- Organizations requiring workspace management
- Teams wanting to discover and use marketplace scenarios
- Organizations needing governance and access controls

**Quick Start:**
```bash
# Login to MockForge Cloud
mockforge cloud login

# Create or join an organization
mockforge cloud org create my-org

# Browse marketplace scenarios
mockforge marketplace list
```

## Choosing Your Journey

### By Role

- **Frontend Developer** → Start with [Reality-First](../book/src/getting-started/reality-first.md)
- **API/Platform Team** → Start with [Contracts-First](../book/src/getting-started/contracts-first.md)
- **Rapid Prototyper** → Start with [AI-First](../book/src/getting-started/ai-first.md)
- **SDK/CLI Developer** → Start with [DevX-First](../book/src/getting-started/devx-first.md)
- **DevOps Engineer** → Start with [Contracts-First](../book/src/getting-started/contracts-first.md) or [Cloud-First](../book/src/getting-started/cloud-first.md)
- **Team Lead** → Start with [Cloud-First](../book/src/getting-started/cloud-first.md)

### By Use Case

- **Need realistic test data** → [Reality-First](../book/src/getting-started/reality-first.md)
- **Need contract validation** → [Contracts-First](../book/src/getting-started/contracts-first.md)
- **Want to generate mocks quickly** → [AI-First](../book/src/getting-started/ai-first.md)
- **Want better developer experience** → [DevX-First](../book/src/getting-started/devx-first.md)
- **Need team collaboration** → [Cloud-First](../book/src/getting-started/cloud-first.md)

### By Team Size

- **Solo Developer** → Start with [Reality-First](../book/src/getting-started/reality-first.md) or [AI-First](../book/src/getting-started/ai-first.md)
- **Small Team (2-10)** → Start with [Contracts-First](../book/src/getting-started/contracts-first.md) or [Reality-First](../book/src/getting-started/reality-first.md)
- **Large Team (10+)** → Start with [Contracts-First](../book/src/getting-started/contracts-first.md) and explore [Cloud-First](../book/src/getting-started/cloud-first.md)

## Cross-Pillar Exploration

After mastering one pillar, explore complementary pillars:

### From Reality

- **Add validation** → Explore [Contracts](../book/src/getting-started/contracts-first.md)
- **Improve workflow** → Explore [DevX](../book/src/getting-started/devx-first.md)
- **Enable collaboration** → Explore [Cloud](../book/src/getting-started/cloud-first.md)
- **Enhance with AI** → Explore [AI](../book/src/getting-started/ai-first.md)

### From Contracts

- **Add realism** → Explore [Reality](../book/src/getting-started/reality-first.md)
- **Improve workflow** → Explore [DevX](../book/src/getting-started/devx-first.md)
- **Enable collaboration** → Explore [Cloud](../book/src/getting-started/cloud-first.md)
- **Enhance analysis** → Explore [AI](../book/src/getting-started/ai-first.md)

### From DevX

- **Add realism** → Explore [Reality](../book/src/getting-started/reality-first.md)
- **Add validation** → Explore [Contracts](../book/src/getting-started/contracts-first.md)
- **Enable collaboration** → Explore [Cloud](../book/src/getting-started/cloud-first.md)
- **Enhance with AI** → Explore [AI](../book/src/getting-started/ai-first.md)

### From Cloud

- **Add realism** → Explore [Reality](../book/src/getting-started/reality-first.md)
- **Add validation** → Explore [Contracts](../book/src/getting-started/contracts-first.md)
- **Improve workflow** → Explore [DevX](../book/src/getting-started/devx-first.md)
- **Enhance with AI** → Explore [AI](../book/src/getting-started/ai-first.md)

### From AI

- **Add realism** → Explore [Reality](../book/src/getting-started/reality-first.md)
- **Add validation** → Explore [Contracts](../book/src/getting-started/contracts-first.md)
- **Improve workflow** → Explore [DevX](../book/src/getting-started/devx-first.md)
- **Enable collaboration** → Explore [Cloud](../book/src/getting-started/cloud-first.md)

## Next Steps

1. **Choose your journey** based on your role, use case, or team size
2. **Complete the quick start** in your chosen journey
3. **Explore key features** for your pillar
4. **Cross-pollinate** by exploring complementary pillars
5. **Master the platform** by understanding all five pillars

## Resources

- [Complete Pillars Documentation](PILLARS.md)
- [Getting Started Guide](../book/src/getting-started/getting-started.md)
- [API Reference](../book/src/api/rust.md)
- [Examples Repository](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples)

---

**Ready to start?** Choose your journey above or explore the [complete pillars documentation](PILLARS.md).
