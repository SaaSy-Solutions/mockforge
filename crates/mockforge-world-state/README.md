# MockForge World State Engine

**Pillars:** [Reality][DevX]

Unified visualization of all MockForge state systems - like a "miniature game engine for your backend."

## Overview

The World State Engine aggregates and visualizes all state systems in MockForge, including:

- **Personas**: Persona profiles, relationships, and graphs
- **Lifecycle**: Lifecycle states, transitions, and time-based changes
- **Reality**: Reality levels, continuum ratios, and chaos rules
- **Time**: Virtual clock state, scheduled events, and time scale
- **Multi-Protocol**: Protocol-specific state, sessions, and entity state
- **Behavior**: Behavior trees, rules, and AI modifiers
- **Schemas**: Generative schema definitions and entity relationships
- **Recorded Data**: Recorded requests/responses, fixtures, and replay state
- **AI Modifiers**: AI response configurations and modifiers

## Features

- **Unified State Aggregation**: Collects state from all MockForge subsystems
- **Graph Visualization**: Represents state as nodes and edges for visualization
- **Real-time Updates**: Streams state changes in real-time
- **Time Travel**: View state at any point in time
- **Query Interface**: Flexible querying of state with filters
- **Export Capabilities**: Export state in various formats (JSON, GraphML, DOT)

## Usage

```rust
use mockforge_world_state::{WorldStateEngine, WorldStateQuery};
use std::collections::HashSet;

// Create engine
let mut engine = WorldStateEngine::new();

// Register aggregators (typically done by the main application)
// engine.register_aggregator(Arc::new(PersonaAggregator::new(...)));

// Create a snapshot
let snapshot = engine.create_snapshot().await?;

// Query with filters
let query = WorldStateQuery::new()
    .with_layers(HashSet::from([StateLayer::Personas, StateLayer::Protocols]));
let filtered = engine.query(&query).await?;
```

## Architecture

- **Engine**: Central coordinator that aggregates state
- **Aggregators**: Collect state from specific subsystems
- **Model**: Core data structures (nodes, edges, layers, snapshots)
- **Query**: Flexible querying interface

## Integration

The World State Engine integrates with:

- `mockforge-core`: Reality, time, consistency, behavior
- `mockforge-data`: Personas, lifecycle, schemas
- `mockforge-recorder`: Recorded data and replay state

## API

See the [API documentation](https://docs.rs/mockforge-world-state) for detailed API reference.
