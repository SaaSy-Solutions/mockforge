# World State Engine

**Pillars:** [Reality][DevX]

The World State Engine unifies all MockForge state systems into a single "world state" visualization. Think of it as a miniature game engine for your backend—a unified view of personas, lifecycle, reality, time, multi-protocol state, behavior trees, generative schemas, recorded data, and AI modifiers.

## Overview

The World State Engine aggregates and visualizes:

- **Personas**: Persona profiles, relationships, and graphs
- **Lifecycle**: Lifecycle states, transitions, and time-based changes
- **Reality**: Reality levels, continuum ratios, and chaos rules
- **Time**: Virtual clock state, scheduled events, and time scale
- **Multi-Protocol**: Protocol-specific state, sessions, and entity state
- **Behavior**: Behavior trees, rules, and AI modifiers
- **Schemas**: Generative schema definitions and entity relationships
- **Recorded Data**: Recorded requests/responses, fixtures, and replay state
- **AI Modifiers**: AI response configurations and modifiers

## Key Features

- **Unified State Aggregation**: Collects state from all MockForge subsystems
- **Graph Visualization**: Represents state as nodes and edges for visualization
- **Real-time Updates**: Streams state changes in real-time
- **Time Travel**: View state at any point in time
- **Query Interface**: Flexible querying of state with filters
- **Export Capabilities**: Export state in various formats (JSON, GraphML, DOT)

## Usage

### Create Snapshot

```bash
# Create world state snapshot
mockforge world-state snapshot create

# Or via API
POST /api/v1/world-state/snapshots
{
  "workspace_id": "workspace-123",
  "include_layers": ["personas", "reality", "time"]
}
```

### Query State

```bash
# Query world state
mockforge world-state query \
  --layers personas,reality \
  --filter "persona_id=premium-customer"

# Or via API
GET /api/v1/world-state/query?layers=personas,reality&filter=persona_id=premium-customer
```

### Visualize State

```bash
# Export as graph
mockforge world-state export --format graphml --output state.graphml

# View in UI
# Navigate to World State page in Admin UI
```

## State Layers

### Personas Layer

Persona profiles and relationships:

```json
{
  "layer": "personas",
  "nodes": [
    {
      "id": "persona:premium-001",
      "type": "persona",
      "data": {
        "traits": {...},
        "relationships": [...]
      }
    }
  ],
  "edges": [
    {
      "from": "persona:premium-001",
      "to": "order:123",
      "type": "has_orders"
    }
  ]
}
```

### Reality Layer

Reality levels and continuum ratios:

```json
{
  "layer": "reality",
  "state": {
    "reality_level": 3,
    "continuum_ratio": 0.5,
    "chaos_rules": [...]
  }
}
```

### Time Layer

Virtual clock and scheduled events:

```json
{
  "layer": "time",
  "state": {
    "virtual_time": "2025-01-27T10:00:00Z",
    "time_scale": 1.0,
    "scheduled_events": [...]
  }
}
```

### Protocols Layer

Protocol-specific state:

```json
{
  "layer": "protocols",
  "state": {
    "http": {
      "sessions": [...],
      "connections": [...]
    },
    "websocket": {
      "connections": [...],
      "subscriptions": [...]
    }
  }
}
```

## Query Interface

### Filter by Layer

```bash
# Query specific layers
mockforge world-state query --layers personas,reality
```

### Filter by Criteria

```bash
# Filter by persona
mockforge world-state query --filter "persona_id=premium-customer"

# Filter by reality level
mockforge world-state query --filter "reality_level>=3"

# Filter by time
mockforge world-state query --filter "time>=2025-01-27"
```

### Complex Queries

```bash
# Multiple filters
mockforge world-state query \
  --layers personas,reality \
  --filter "persona_id=premium-customer" \
  --filter "reality_level>=3"
```

## Time Travel

View state at any point in time:

```bash
# View state at specific time
mockforge world-state query \
  --time "2025-01-27T10:00:00Z" \
  --layers personas,reality
```

## Export Formats

### JSON

```bash
mockforge world-state export --format json --output state.json
```

### GraphML

```bash
mockforge world-state export --format graphml --output state.graphml
```

### DOT

```bash
mockforge world-state export --format dot --output state.dot
```

## Real-World Example

### E-Commerce World State

```json
{
  "workspace_id": "ecommerce-123",
  "snapshot_time": "2025-01-27T10:00:00Z",
  "layers": {
    "personas": {
      "nodes": [
        {
          "id": "persona:premium-001",
          "type": "persona",
          "data": {
            "traits": {"tier": "premium"},
            "lifecycle_state": "active"
          }
        },
        {
          "id": "order:456",
          "type": "order",
          "data": {
            "status": "pending",
            "total": 99.99
          }
        }
      ],
      "edges": [
        {
          "from": "persona:premium-001",
          "to": "order:456",
          "type": "has_orders"
        }
      ]
    },
    "reality": {
      "reality_level": 3,
      "continuum_ratio": 0.5,
      "chaos_rules": []
    },
    "time": {
      "virtual_time": "2025-01-27T10:00:00Z",
      "time_scale": 1.0
    }
  }
}
```

## Best Practices

1. **Regular Snapshots**: Take snapshots at key points
2. **Query Efficiently**: Use filters to query only needed layers
3. **Visualize**: Use graph visualization to understand relationships
4. **Time Travel**: Use time travel to debug temporal issues
5. **Export for Analysis**: Export state for external analysis

## Related Documentation

- [Smart Personas](smart-personas.md) - Persona system
- [Reality Continuum](reality-continuum.md) - Reality levels
- [Temporal Simulation](temporal-simulation.md) - Time travel

