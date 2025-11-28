# Pillar Tagging Guide

**Version:** 1.0.0
**Last Updated:** 2025-01-27

This guide explains how to tag modules, functions, and features with MockForge pillars for better code organization, test coverage tracking, and production usage analysis.

## Overview

Every feature in MockForge maps to one or more pillars. Tagging code with pillars enables:

- **Test Coverage by Pillar**: "Show me test coverage by pillar"
- **Production Usage Analysis**: "Which pillars are most used in production?"
- **Code Organization**: Quickly identify which pillar a module belongs to
- **Documentation**: Automatically generate pillar-based documentation

## The Five Pillars

- **[Reality]** – Everything that makes mocks feel like a real, evolving backend
- **[Contracts]** – Schema, drift, validation, and safety nets
- **[DevX]** – SDKs, generators, playgrounds, ergonomics
- **[Cloud]** – Registry, orgs, governance, monetization, marketplace
- **[AI]** – LLM/voice flows, AI diff/assist, generative behaviors

See [PILLARS.md](../PILLARS.md) for detailed information about each pillar.

## Tagging Format

### Module-Level Tagging

Add pillar tags to module documentation comments using the `Pillars:` format:

```rust
//! Pillars: [Reality][AI]
//!
//! This module implements Smart Personas with relationship graphs
//! and AI-powered data generation.
pub mod personas {
    // ...
}
```

### Multiple Pillars

A module can belong to multiple pillars. List them all:

```rust
//! Pillars: [Reality][DevX]
//!
//! Reality Slider with hot-reload capabilities for developer experience.
pub mod reality_slider {
    // ...
}
```

### Single Pillar

For modules that belong to a single pillar:

```rust
//! Pillars: [Contracts]
//!
//! Request/response validation against OpenAPI schemas.
pub mod validation {
    // ...
}
```

## Programmatic Tagging

You can also use the `PillarMetadata` type programmatically:

```rust
use mockforge_core::pillars::{Pillar, PillarMetadata};

// Create metadata with multiple pillars
let metadata = PillarMetadata::new()
    .with_pillar(Pillar::Reality)
    .with_pillar(Pillar::Ai);

// Check if a pillar is present
if metadata.has_pillar(Pillar::Reality) {
    // ...
}

// Format as changelog tags
let tags = metadata.to_changelog_tags(); // "[AI][Reality]"
```

## Tagging Guidelines

### When to Tag

- **Always tag** major modules and features
- **Tag** when a module clearly belongs to one or more pillars
- **Don't tag** utility modules that don't map to a specific pillar
- **Don't tag** test modules (tests inherit tags from the modules they test)

### Which Pillars to Tag

1. **Reality**: Does it make mocks feel more like real backends?
   - Examples: `reality.rs`, `reality_continuum/`, `chaos_utilities.rs`, `personas/`

2. **Contracts**: Does it validate, sync, or monitor API contracts?
   - Examples: `validation.rs`, `contract_drift/`, `contract_validation.rs`, `schema_diff.rs`

3. **DevX**: Does it improve developer experience or ergonomics?
   - Examples: `cli/`, `plugin-*/`, `observability/`, `templating.rs`

4. **Cloud**: Does it enable team collaboration or sharing?
   - Examples: `analytics/`, `registry-server/`, `multi_tenant/`, `workspace/`

5. **AI**: Does it use AI/LLM to automate or enhance functionality?
   - Examples: `ai_studio/`, `ai_contract_diff/`, `voice/`, `generative_schema/`

### Primary vs Secondary Pillars

- **Primary pillar**: The main pillar the module belongs to (list first)
- **Secondary pillars**: Additional pillars the module supports (list after primary)

Example:
```rust
//! Pillars: [Reality][DevX]
//!
//! Primary: Reality (makes mocks realistic)
//! Secondary: DevX (hot-reload improves developer experience)
```

## Examples

### Reality Pillar

```rust
//! Pillars: [Reality]
//!
//! Reality engine for configuring reality levels and presets.
pub mod reality {
    // ...
}
```

### Contracts Pillar

```rust
//! Pillars: [Contracts]
//!
//! Contract drift detection and monitoring.
pub mod contract_drift {
    // ...
}
```

### Multi-Pillar Module

```rust
//! Pillars: [Reality][AI]
//!
//! Smart Personas with AI-powered data generation and relationship graphs.
pub mod personas {
    // ...
}
```

### DevX Pillar

```rust
//! Pillars: [DevX]
//!
//! Plugin system for extending MockForge functionality.
pub mod plugin_core {
    // ...
}
```

## Extracting Pillar Tags

The `PillarMetadata` type can parse tags from documentation comments:

```rust
use mockforge_core::pillars::PillarMetadata;

let doc = "//! Pillars: [Reality][AI]\n//! This module does something";
let metadata = PillarMetadata::from_doc_comment(doc).unwrap();
assert!(metadata.has_pillar(Pillar::Reality));
assert!(metadata.has_pillar(Pillar::Ai));
```

## Querying by Pillar

### Test Coverage

Use the `scripts/pillar-coverage.sh` script to generate test coverage reports by pillar:

```bash
./scripts/pillar-coverage.sh
```

This will:
1. Parse pillar tags from source files
2. Map test files to modules to pillars
3. Generate coverage reports grouped by pillar

### Production Usage

Use the `scripts/pillar-usage.sh` script to query production pillar usage:

```bash
./scripts/pillar-usage.sh --prometheus-url http://localhost:9090
```

This queries Prometheus metrics with pillar labels to show which pillars are most used in production.

## Common Patterns

### Protocol Implementations

Protocol implementations typically span multiple pillars:

```rust
//! Pillars: [Contracts][DevX]
//!
//! HTTP protocol implementation with OpenAPI validation and developer tools.
pub mod http {
    // ...
}
```

### Core Features

Core features often belong to a single primary pillar:

```rust
//! Pillars: [Reality]
//!
//! Latency injection for simulating realistic network conditions.
pub mod latency {
    // ...
}
```

### Cross-Cutting Concerns

Some modules support multiple pillars:

```rust
//! Pillars: [DevX][Cloud]
//!
//! Observability features for both local development and cloud deployments.
pub mod observability {
    // ...
}
```

## Verification

After tagging modules, verify your tags:

1. **Check documentation**: Ensure tags appear in generated docs
2. **Run coverage script**: Verify pillar coverage reports work
3. **Check telemetry**: Ensure pillar labels appear in Prometheus metrics

## Questions?

If you're unsure which pillar(s) a module belongs to:

1. **Reality**: Does it make mocks feel more like real backends?
2. **Contracts**: Does it validate, sync, or monitor API contracts?
3. **DevX**: Does it improve developer experience or ergonomics?
4. **Cloud**: Does it enable team collaboration or sharing?
5. **AI**: Does it use AI/LLM to automate or enhance functionality?

Many modules span multiple pillars—that's intentional and encouraged!

## References

- [PILLARS.md](../PILLARS.md) - Complete pillar documentation
- [Pillar Queries Guide](PILLAR_QUERIES.md) - How to query by pillar
- [Contributing Guide](../CONTRIBUTING.md) - General contribution guidelines
