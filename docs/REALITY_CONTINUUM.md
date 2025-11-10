# Reality Continuum

**Priority:** 🔥 Experimental
**Tags:** `#simulation` `#time` `#AI`

## Overview

The Reality Continuum feature enables gradual transition from mock to real backend data by intelligently blending responses from both sources. This allows teams to develop and test against a real backend that's still under construction, smoothly transitioning from 100% mock to 100% real over time.

## Key Features

- **Dynamic Blending**: Intelligently merges mock and real responses based on configurable blend ratios
- **Time-Based Progression**: Automatically transitions blend ratios over time using virtual clock
- **Flexible Configuration**: Supports per-route, group-level, and global blend ratio settings
- **Multiple Merge Strategies**: Field-level merge, weighted selection, or body blending
- **Fallback Handling**: Gracefully handles failures from either source

## Configuration

### Basic Configuration

```yaml
reality_continuum:
  enabled: true
  default_ratio: 0.0  # Start with 100% mock
  transition_mode: "manual"  # or "time_based" or "scheduled"
  merge_strategy: "field_level"
```

### Time-Based Progression

Configure automatic progression from mock to real over a time period:

```yaml
reality_continuum:
  enabled: true
  default_ratio: 0.0
  transition_mode: "time_based"
  time_schedule:
    start_time: "2025-01-01T00:00:00Z"
    end_time: "2025-02-01T00:00:00Z"
    start_ratio: 0.0
    end_ratio: 1.0
    curve: "linear"  # or "exponential" or "sigmoid"
```

### Per-Route Configuration

Set different blend ratios for specific routes:

```yaml
reality_continuum:
  enabled: true
  default_ratio: 0.0
  routes:
    - pattern: "/api/users/*"
      ratio: 0.5  # 50% real for user endpoints
      enabled: true
    - pattern: "/api/orders/*"
      ratio: 0.3  # 30% real for order endpoints
      group: "api-v1"
      enabled: true
```

### Group-Level Configuration

Control blend ratios for entire migration groups:

```yaml
reality_continuum:
  enabled: true
  groups:
    "api-v1": 0.0  # All api-v1 routes use 100% mock
    "api-v2": 0.5  # All api-v2 routes use 50% real
```

## Blend Ratio Priority

The blend ratio is determined in the following order (highest to lowest priority):

1. **Manual Overrides** - Set via API calls
2. **Route-Specific Rules** - Per-route configuration
3. **Group-Level Overrides** - Migration group settings
4. **Time-Based Schedule** - If time-based mode is enabled
5. **Default Ratio** - Global default setting

## Merge Strategies

### Field-Level (Default)

Deep merges JSON objects, combines arrays, and uses weighted selection for primitives:

```json
// Mock response
{
  "id": 1,
  "name": "Mock User",
  "email": "mock@example.com"
}

// Real response
{
  "id": 2,
  "name": "Real User",
  "status": "active"
}

// Blended (ratio: 0.5)
{
  "id": 1.5,  // Weighted average
  "name": "Real User",  // Selected based on ratio
  "email": "mock@example.com",  // From mock (ratio < 0.5)
  "status": "active"  // From real (ratio >= 0.5)
}
```

### Weighted Selection

Randomly selects between mock and real based on ratio (for testing/demo).

### Body Blend

Merges arrays, averages numeric fields, and deep merges objects with interleaving.

## Transition Curves

### Linear

Constant rate of progression:

```
Ratio
1.0 |                    *
    |               *
    |          *
    |     *
0.0 |*
    +------------------- Time
```

### Exponential

Slow start, fast end:

```
Ratio
1.0 |                        *
    |                  *
    |            *
    |      *
0.0 |*
    +------------------- Time
```

### Sigmoid

Slow start and end, fast middle:

```
Ratio
1.0 |                    *
    |               *
    |          *
    |     *
0.0 |*
    +------------------- Time
```

## API Endpoints

### Get Blend Ratio

```http
GET /__mockforge/continuum/ratio?path=/api/users/123
```

Response:
```json
{
  "success": true,
  "data": {
    "path": "/api/users/123",
    "blend_ratio": 0.5,
    "enabled": true,
    "transition_mode": "Manual",
    "merge_strategy": "FieldLevel",
    "default_ratio": 0.0
  }
}
```

### Set Blend Ratio

```http
PUT /__mockforge/continuum/ratio
Content-Type: application/json

{
  "path": "/api/users/*",
  "ratio": 0.75
}
```

### Get Time Schedule

```http
GET /__mockforge/continuum/schedule
```

### Update Time Schedule

```http
PUT /__mockforge/continuum/schedule
Content-Type: application/json

{
  "start_time": "2025-01-01T00:00:00Z",
  "end_time": "2025-02-01T00:00:00Z",
  "start_ratio": 0.0,
  "end_ratio": 1.0,
  "curve": "linear"
}
```

### Manually Advance Ratio

```http
POST /__mockforge/continuum/advance
Content-Type: application/json

{
  "increment": 0.1
}
```

### Enable/Disable

```http
PUT /__mockforge/continuum/enabled
Content-Type: application/json

{
  "enabled": true
}
```

### Get Manual Overrides

```http
GET /__mockforge/continuum/overrides
```

### Clear Manual Overrides

```http
DELETE /__mockforge/continuum/overrides
```

## Integration with Time Travel

The Reality Continuum integrates seamlessly with MockForge's time travel system. When virtual time is enabled, blend ratios automatically progress based on the virtual clock:

```rust
use mockforge_core::{RealityContinuumEngine, VirtualClock, TimeSchedule};
use std::sync::Arc;

let clock = Arc::new(VirtualClock::new());
clock.enable_and_set(start_time);

let schedule = TimeSchedule::new(start_time, end_time, 0.0, 1.0);
let config = ContinuumConfig {
    enabled: true,
    transition_mode: TransitionMode::TimeBased,
    time_schedule: Some(schedule),
    ..Default::default()
};

let engine = RealityContinuumEngine::with_virtual_clock(config, clock);
```

## Use Cases

### Gradual Backend Migration

Start with 100% mock responses and gradually increase real backend usage as endpoints are implemented:

```yaml
reality_continuum:
  enabled: true
  transition_mode: "time_based"
  time_schedule:
    start_time: "2025-01-01T00:00:00Z"
    end_time: "2025-03-01T00:00:00Z"  # 2 months transition
    start_ratio: 0.0
    end_ratio: 1.0
    curve: "sigmoid"  # Slow start and end
```

### Per-Endpoint Rollout

Different endpoints migrate at different rates:

```yaml
reality_continuum:
  enabled: true
  routes:
    - pattern: "/api/users/*"
      ratio: 0.9  # Almost fully migrated
    - pattern: "/api/orders/*"
      ratio: 0.3  # Still mostly mock
    - pattern: "/api/payments/*"
      ratio: 0.0  # Not yet migrated
```

### A/B Testing

Compare mock and real responses by blending them:

```yaml
reality_continuum:
  enabled: true
  default_ratio: 0.5  # 50/50 split
  merge_strategy: "field_level"
```

## Fallback Behavior

When continuum is enabled:

- **Both sources succeed**: Responses are blended according to the blend ratio
- **Only proxy succeeds**: Real response is returned (fallback to real)
- **Only mock succeeds**: Mock response is returned (fallback to mock)
- **Both fail**: Error is returned (unless migration mode is Real, which fails hard)

## Observability

The continuum engine logs blend operations:

```
INFO Reality Continuum: blended mock and real responses path=/api/users/123 blend_ratio=0.5
```

Response metadata includes blend information:

```json
{
  "source": {
    "priority": "Proxy",
    "name": "continuum",
    "metadata": {
      "blend_ratio": "0.5",
      "upstream_url": "https://api.example.com"
    }
  }
}
```

## Best Practices

1. **Start Conservative**: Begin with `default_ratio: 0.0` (100% mock)
2. **Use Time-Based Progression**: Automate the transition with time schedules
3. **Monitor Both Sources**: Ensure both mock and real backends are healthy
4. **Test Fallback Behavior**: Verify graceful degradation when one source fails
5. **Use Groups for Batch Control**: Group related routes for coordinated migration
6. **Leverage Virtual Clock**: Use time travel to simulate weeks of development in minutes

## Limitations

- Currently supports JSON responses only
- Merge strategies may not handle all edge cases perfectly
- Time-based progression requires time travel to be enabled for full effect
- Blending adds slight latency (both responses must be fetched)

## Future Enhancements

- Support for XML, binary, and other response formats
- More sophisticated merge strategies
- Automatic health-based ratio adjustment
- Per-field blend ratios
- Response caching to reduce latency
