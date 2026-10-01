# Snapshot Diff Between Environments

> **Experimental.** This feature ships in MockForge but is early: the comparison API is served at `/__mockforge/api/snapshot-diff/snapshots/compare` with in-memory storage; there is no `mockforge snapshot diff` command yet, and environment/persona/reality comparisons do not yet capture distinct states. Behavior, commands, and configuration may change between releases. Report issues at <https://github.com/SaaSy-Solutions/mockforge/issues>.

**Pillars:** [DevX]

Snapshot Diff provides side-by-side visualization for comparing mock behavior between different environments, personas, scenarios, or "realities" (Reality 0.1 vs Reality 0.9). This is amazing for demos and debugging.

## Overview

Snapshot Diff enables you to:

- **Compare Test vs Prod** mock behavior
- **Compare Persona A vs Persona B** responses
- **Compare Reality 0.1 vs Reality 0.9** behavior
- **Compare Scenarios** side-by-side
- **Visualize Differences** with highlighted changes

## Usage

### Browser Extension

1. Open DevTools
2. Navigate to "MockForge" tab
3. Select "Snapshot Diff" panel
4. Choose comparison type
5. View side-by-side diff

### API Usage

```bash
# Compare snapshots (management API on the HTTP server)
POST /__mockforge/api/snapshot-diff/snapshots/compare
{
  "left_environment_id": "test",
  "right_environment_id": "prod"
}

# Capture and list snapshots
POST /__mockforge/api/snapshot-diff/snapshots
GET  /__mockforge/api/snapshot-diff/snapshots
GET  /__mockforge/api/snapshot-diff/snapshots/{id}
```

The compare request also accepts `left_snapshot_id`/`right_snapshot_id`, `left_persona_id`/`right_persona_id`, `left_scenario_id`/`right_scenario_id`, and `left_reality_level`/`right_reality_level`.

### CLI Usage

There is no `mockforge snapshot diff` command yet. The `mockforge snapshot` command manages time-travel state snapshots (`save`, `load`, `list`, `info`, `delete`, `validate`) and does not compare them; use the API above instead.

## Comparison Types

### Environment Comparison

Compare mock behavior between environments:

```
Test Environment              Prod Environment
─────────────────            ─────────────────
GET /api/users/123           GET /api/users/123
Status: 200                  Status: 200
Body: {...}                  Body: {...}
  id: "123"                    id: "123"
  name: "Test User"             name: "Prod User"  ⚠️
  email: "test@..."             email: "prod@..."  ⚠️
```

### Persona Comparison

Compare responses for different personas:

```
Premium Customer              Regular Customer
─────────────────            ─────────────────
GET /api/users/123           GET /api/users/123
Status: 200                  Status: 200
Body: {...}                  Body: {...}
  tier: "premium"              tier: "regular"  ⚠️
  features: [...]               features: [...]  ⚠️
```

### Reality Level Comparison

Compare behavior at different reality levels:

```
Reality 0.1 (Low)            Reality 0.9 (High)
─────────────────            ─────────────────
GET /api/users/123           GET /api/users/123
Status: 200                  Status: 200
Body: {...}                  Body: {...}
  # Synthetic data             # Blended with real
  id: "generated-123"          id: "real-user-123"  ⚠️
  name: "Generated Name"        name: "Real Name"  ⚠️
```

### Scenario Comparison

Compare different scenarios:

```
Happy Path                   Error Path
─────────────────            ─────────────────
POST /api/orders             POST /api/orders
Status: 201                  Status: 400
Body: {...}                  Body: {...}
  status: "created"            error: "Invalid..."  ⚠️
```

## Diff Visualization

### Side-by-Side View

```
Left Snapshot                Right Snapshot
─────────────────            ─────────────────
Status: 200                  Status: 200
Headers:                     Headers:
  Content-Type: json            Content-Type: json
Body:                         Body:
  {                             {
    "id": "123",                "id": "123",
    "name": "User A",           "name": "User B",  ⚠️
    "email": "a@..."            "email": "b@..."   ⚠️
  }                             }
```

### Difference Types

- **Missing in Right**: Fields present in left but not right
- **Missing in Left**: Fields present in right but not left
- **Status Code Mismatch**: Different status codes
- **Body Mismatch**: Different response bodies
- **Headers Mismatch**: Different headers

## Use Cases

### Demo Preparation

Compare scenarios to prepare demos:

```bash
# Compare demo scenarios
POST /__mockforge/api/snapshot-diff/snapshots/compare
{
  "left_scenario_id": "demo-basic",
  "right_scenario_id": "demo-premium"
}
```

### Debugging

Compare behavior to debug issues:

```bash
# Compare test vs prod to find differences
POST /__mockforge/api/snapshot-diff/snapshots/compare
{
  "left_environment_id": "test",
  "right_environment_id": "prod"
}
```

### Reality Progression

Compare reality levels to understand progression:

```bash
# Compare low vs high reality
POST /__mockforge/api/snapshot-diff/snapshots/compare
{
  "left_reality_level": 0.1,
  "right_reality_level": 0.9
}
```

## Configuration

### Snapshot Storage

```yaml
# mockforge.yaml
snapshots:
  enabled: true
  storage:
    type: database  # or file
    retention_days: 30
```

### Comparison Options

```yaml
snapshots:
  comparison:
    include_headers: true
    include_timing: true
    diff_format: unified  # or side-by-side
```

## Best Practices

1. **Take Snapshots Regularly**: Capture snapshots at key points
2. **Compare Before Deploy**: Compare test vs prod before deployment
3. **Document Differences**: Document expected differences
4. **Use for Demos**: Use comparisons in demos and presentations
5. **Track Over Time**: Compare snapshots over time to track changes

## Related Documentation

- [Zero-Config Mode](zero-config-mode.md) - Auto-mock generation
- [DevTools Integration](devtools-integration.md) - Browser integration
- [ForgeConnect SDK](../forgeconnect-sdk.md) - Browser SDK

