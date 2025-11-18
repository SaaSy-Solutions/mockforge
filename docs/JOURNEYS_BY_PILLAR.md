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

## Choosing Your Journey

### By Role

- **Frontend Developer** → Start with [Reality-First](reality-first.md)
- **API/Platform Team** → Start with [Contracts-First](contracts-first.md)
- **Rapid Prototyper** → Start with [AI-First](ai-first.md)
- **DevOps Engineer** → Start with [Contracts-First](contracts-first.md) or [Cloud features](../docs/PILLARS.md#cloud--registry-orgs-governance-monetization-marketplace)

### By Use Case

- **Need realistic test data** → [Reality-First](reality-first.md)
- **Need contract validation** → [Contracts-First](contracts-first.md)
- **Want to generate mocks quickly** → [AI-First](ai-first.md)
- **Need team collaboration** → [Cloud features](../docs/PILLARS.md#cloud--registry-orgs-governance-monetization-marketplace)
- **Want better developer experience** → [DevX features](../docs/PILLARS.md#devx--sdks-generators-playgrounds-ergonomics)

### By Team Size

- **Solo Developer** → Start with [Reality-First](reality-first.md) or [AI-First](ai-first.md)
- **Small Team (2-10)** → Start with [Contracts-First](contracts-first.md) or [Reality-First](reality-first.md)
- **Large Team (10+)** → Start with [Contracts-First](contracts-first.md) and explore [Cloud features](../docs/PILLARS.md#cloud--registry-orgs-governance-monetization-marketplace)

## Cross-Pillar Exploration

After mastering one pillar, explore complementary pillars:

### From Reality

- **Add validation** → Explore [Contracts](contracts-first.md)
- **Improve workflow** → Explore [DevX](../docs/PILLARS.md#devx--sdks-generators-playgrounds-ergonomics)
- **Enhance with AI** → Explore [AI](ai-first.md)

### From Contracts

- **Add realism** → Explore [Reality](reality-first.md)
- **Improve workflow** → Explore [DevX](../docs/PILLARS.md#devx--sdks-generators-playgrounds-ergonomics)
- **Enhance analysis** → Explore [AI](ai-first.md)

### From AI

- **Add realism** → Explore [Reality](reality-first.md)
- **Add validation** → Explore [Contracts](contracts-first.md)
- **Improve workflow** → Explore [DevX](../docs/PILLARS.md#devx--sdks-generators-playgrounds-ergonomics)

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
