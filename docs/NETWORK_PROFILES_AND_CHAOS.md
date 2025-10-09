# Enhanced Traffic Simulation Modes

This document describes the new network profiles and random chaos testing features added to MockForge.

## Overview

MockForge now provides two major enhancements to traffic simulation:

1. **Network Condition Profiles**: Pre-configured latency and traffic shaping profiles for common network scenarios
2. **Random Chaos Mode**: Randomly inject errors and delays for adversarial testing

## Network Condition Profiles

### Available Profiles

Network profiles package latency, bandwidth throttling, and packet loss settings into user-friendly presets:

| Profile | Description | Latency | Bandwidth | Use Case |
|---------|-------------|---------|-----------|----------|
| `perfect` | No degradation | 0ms | Unlimited | Baseline testing |
| `5g` | 5G mobile network | 10-30ms | ~100 Mbps | Modern mobile |
| `4g` | 4G/LTE mobile | 30-70ms | ~20 Mbps | Common mobile |
| `3g` | 3G mobile | 100-250ms | ~1 Mbps | Degraded mobile |
| `2g` | 2G/EDGE | 300-600ms | ~250 Kbps | Poor mobile |
| `edge` | EDGE (worst case) | 500-1000ms | ~100 Kbps | Extremely poor mobile |
| `satellite_leo` | Low Earth Orbit | 20-150ms | ~100 Mbps | Starlink-like |
| `satellite_geo` | Geostationary | 550-850ms | ~15 Mbps | Traditional satellite |
| `congested` | Congested network | 100-800ms | ~2 Mbps | Peak hours |
| `lossy` | High packet loss | 50-120ms | ~10 Mbps | Unreliable connection |
| `high_latency` | Very high latency | 500-1200ms | ~10 Mbps | Intercontinental |
| `intermittent` | Frequent drops | 100-400ms | ~5 Mbps | Unstable connection |
| `extremely_poor` | Worst case | 800-3000ms | ~50 Kbps | Emergency scenarios |

### Usage

#### List Available Profiles

```bash
mockforge serve --list-network-profiles
```

Output:
```
📡 Available Network Profiles:

  • 2g                   2G/EDGE mobile network (300-500ms latency, ~250 Kbps)
  • 3g                   3G mobile network (100-200ms latency, ~1 Mbps)
  • 4g                   4G/LTE mobile network (30-60ms latency, ~20 Mbps)
  • 5g                   5G mobile network (10-30ms latency, ~100 Mbps)
  • congested            Congested network (100-500ms latency, ~2 Mbps, high jitter)
  • edge                 EDGE mobile network (500-800ms latency, ~100 Kbps)
  • extremely_poor       Extremely poor network (1000ms+ latency, <50 Kbps, high loss)
  • high_latency         High latency network (500-1000ms latency, normal bandwidth)
  • intermittent         Intermittent connection (100-300ms latency, frequent drops)
  • lossy                Lossy network (50-100ms latency, 20% packet loss)
  • perfect              Perfect network with no degradation
  • satellite_geo        GEO satellite (550-750ms latency, ~15 Mbps)
  • satellite_leo        LEO satellite (20-40ms latency, ~100 Mbps, variable)
```

#### Apply a Profile

```bash
mockforge serve --network-profile 3g
```

This automatically configures:
- Latency with appropriate distribution
- Bandwidth throttling
- Packet loss simulation
- Burst loss patterns

#### Example: Test Mobile User Experience

```bash
# Simulate a 4G mobile user
mockforge serve --network-profile 4g --spec api.yaml

# Simulate poor satellite connection
mockforge serve --network-profile satellite_geo --spec api.yaml

# Test under congested network conditions
mockforge serve --network-profile congested --spec api.yaml
```

### Profile Characteristics

Each profile includes:

1. **Latency Configuration**:
   - Base latency (mean)
   - Distribution type (Fixed, Normal, or Pareto)
   - Min/max bounds
   - Statistical parameters (standard deviation, shape)

2. **Bandwidth Throttling**:
   - Maximum bytes per second
   - Burst capacity
   - Token bucket algorithm

3. **Packet Loss Simulation**:
   - Burst probability
   - Loss rate during bursts
   - Burst duration
   - Recovery time

## Random Chaos Mode

The random chaos mode provides probabilistic error and delay injection for adversarial testing.

### Features

- **Random Error Injection**: Randomly return HTTP 5xx errors at a configurable rate
- **Random Delay Injection**: Randomly inject delays within a configurable range
- **Independent Control**: Separately configure error and delay rates
- **Customizable Parameters**: Full control over rates and delay ranges

### Usage

#### Basic Random Chaos

```bash
mockforge serve --chaos-random --spec api.yaml
```

Default behavior:
- 10% error rate (returns random 5xx errors)
- 30% delay rate (injects 100-2000ms delays)

#### Custom Error Rate

```bash
# 20% chance of errors
mockforge serve --chaos-random --chaos-random-error-rate 0.2 --spec api.yaml
```

#### Custom Delay Configuration

```bash
# 50% chance of 500-1500ms delays
mockforge serve --chaos-random \
  --chaos-random-delay-rate 0.5 \
  --chaos-random-min-delay 500 \
  --chaos-random-max-delay 1500 \
  --spec api.yaml
```

#### Combined Configuration

```bash
# High chaos: 30% errors, 70% delays
mockforge serve --chaos-random \
  --chaos-random-error-rate 0.3 \
  --chaos-random-delay-rate 0.7 \
  --chaos-random-min-delay 200 \
  --chaos-random-max-delay 5000 \
  --spec api.yaml
```

### Parameters

| Parameter | Description | Default | Range |
|-----------|-------------|---------|-------|
| `--chaos-random` | Enable random chaos mode | false | - |
| `--chaos-random-error-rate` | Error injection probability | 0.1 | 0.0-1.0 |
| `--chaos-random-delay-rate` | Delay injection probability | 0.3 | 0.0-1.0 |
| `--chaos-random-min-delay` | Minimum delay (ms) | 100 | 0+ |
| `--chaos-random-max-delay` | Maximum delay (ms) | 2000 | 0+ |

## Combining Features

You can combine network profiles with random chaos mode:

```bash
# Simulate 3G network with additional random failures
mockforge serve \
  --network-profile 3g \
  --chaos-random \
  --chaos-random-error-rate 0.15 \
  --spec api.yaml
```

This creates:
- 3G network characteristics (100-250ms latency, ~1 Mbps bandwidth)
- 15% additional random errors
- 30% additional random delays (default)

## Use Cases

### 1. Mobile App Testing

Test how your app behaves under various mobile network conditions:

```bash
# Test on 4G
mockforge serve --network-profile 4g --spec api.yaml

# Test on degraded 3G
mockforge serve --network-profile 3g --spec api.yaml

# Test on poor connection
mockforge serve --network-profile edge --spec api.yaml
```

### 2. Satellite/Remote Connection Testing

```bash
# Test Starlink-like connection
mockforge serve --network-profile satellite_leo --spec api.yaml

# Test traditional satellite
mockforge serve --network-profile satellite_geo --spec api.yaml
```

### 3. Chaos/Reliability Testing

```bash
# High error rate for resilience testing
mockforge serve --chaos-random --chaos-random-error-rate 0.5 --spec api.yaml

# Extreme delay variation
mockforge serve --chaos-random \
  --chaos-random-delay-rate 0.8 \
  --chaos-random-min-delay 1000 \
  --chaos-random-max-delay 10000 \
  --spec api.yaml
```

### 4. Load Testing Under Network Constraints

```bash
# Simulate congested network during load test
mockforge serve --network-profile congested --spec api.yaml

# Add random failures to load test
mockforge serve \
  --network-profile congested \
  --chaos-random \
  --chaos-random-error-rate 0.2 \
  --spec api.yaml
```

## Implementation Details

### Network Profiles

Network profiles are defined in `mockforge-core/src/network_profiles.rs`:

```rust
use mockforge_core::{NetworkProfile, NetworkProfileCatalog};

// Create catalog with built-in profiles
let catalog = NetworkProfileCatalog::new();

// Get a profile
let profile = catalog.get("3g").unwrap();

// Apply the profile
let (latency, traffic_shaping) = profile.apply();
```

### Random Chaos Engine

The chaos engine is defined in `mockforge-core/src/chaos_utilities.rs`:

```rust
use mockforge_core::{ChaosConfig, ChaosEngine};

// Create chaos engine
let config = ChaosConfig::new(0.1, 0.3); // 10% errors, 30% delays
let engine = ChaosEngine::new(config);

// Process request
let result = engine.process_request(&tags).await;
match result {
    ChaosResult::Success => { /* proceed normally */ }
    ChaosResult::Error { status_code, message } => { /* return error */ }
    ChaosResult::Delay { delay_ms } => { /* inject delay */ }
    ChaosResult::Timeout { timeout_ms } => { /* inject timeout */ }
}
```

### Integration with Server Configuration

The chaos engine is integrated into MockForge's `Config` structure:

```rust
use mockforge_core::{Config, ChaosConfig};

// Create configuration with chaos mode
let mut config = Config::default();
config.chaos_random = Some(
    ChaosConfig::new(0.1, 0.3)
        .with_delay_range(100, 500)
);

// Create chaos engine from config
let engine = config.create_chaos_engine(); // Returns Option<ChaosEngine>

// Check if chaos is enabled
if config.is_chaos_random_enabled() {
    println!("Chaos mode is active!");
}
```

### Middleware Integration Pattern

To integrate the chaos engine into request processing, use this pattern:

```rust
use axum::{middleware, extract::State, http::Request, body::Body, response::Response};
use mockforge_core::{ChaosEngine, ChaosResult};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    chaos_engine: Option<Arc<ChaosEngine>>,
}

async fn chaos_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if let Some(engine) = &state.chaos_engine {
        match engine.process_request(&[]).await {
            ChaosResult::Success => next.run(request).await,
            ChaosResult::Error { status_code, message } => {
                // Return error response
                (StatusCode::from_u16(status_code).unwrap(), message).into_response()
            }
            ChaosResult::Delay { delay_ms } => {
                // Inject delay
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                next.run(request).await
            }
            ChaosResult::Timeout { .. } => {
                (StatusCode::GATEWAY_TIMEOUT, "Request timeout").into_response()
            }
        }
    } else {
        next.run(request).await
    }
}

// Apply middleware to router
let app = Router::new()
    .route("/hello", get(handler))
    .layer(middleware::from_fn_with_state(state.clone(), chaos_middleware))
    .with_state(state);
```

See `examples/chaos_engine_integration.rs` for a complete working example.

## Configuration File Support

Network profiles and chaos settings can also be specified in configuration files:

```yaml
# mockforge.yaml
core:
  latency_enabled: true
  traffic_shaping_enabled: true

# Apply network profile programmatically
# or use --network-profile flag
```

## Performance Considerations

- **Network Profiles**: Minimal overhead, uses efficient token bucket algorithm for bandwidth throttling
- **Random Chaos**: Very low overhead, random number generation only
- **Combined Mode**: Both features can be used simultaneously with minimal impact

## Future Enhancements

Potential future additions:

1. **Custom Profiles**: Allow users to define custom network profiles in config files
2. **Profile Transitions**: Gradually transition between profiles (e.g., 4G → 3G → 2G)
3. **Geographic Profiles**: Region-specific network characteristics
4. **Time-Based Profiles**: Change profiles based on time of day
5. **Advanced Chaos**: Support for more sophisticated failure patterns
6. **WebSocket Support**: Extend random chaos to WebSocket connections
7. **gRPC Support**: Extend random chaos to gRPC streams

## Related Features

These features complement existing MockForge capabilities:

- **Chaos Engineering** (`--chaos`): Scenario-based chaos testing
- **Traffic Shaping** (`--traffic-shaping`): Manual bandwidth control
- **Latency Profiles**: Existing latency configuration
- **Failure Injection**: Tag-based failure injection

## Troubleshooting

### Profile Not Found

```bash
⚠️  Warning: Unknown network profile 'unknown'. Use --list-network-profiles to see available profiles.
```

Solution: Use `--list-network-profiles` to see available options.

### High Latency Impact

If profiles cause too much latency:
- Choose a less restrictive profile (e.g., `4g` instead of `3g`)
- Adjust chaos rates if using `--chaos-random`
- Use `--network-profile perfect` to disable

### Bandwidth Too Restrictive

If bandwidth is too limited:
- Use a higher bandwidth profile (e.g., `5g` instead of `4g`)
- Adjust `--bandwidth-limit` manually instead of using profiles

## Summary

The network profiles and random chaos features provide:

1. **Easy Simulation**: Pre-configured profiles for common scenarios
2. **Realistic Testing**: Based on real-world network characteristics
3. **Adversarial Testing**: Random injection for resilience testing
4. **Flexible Configuration**: Fine-grained control over all parameters
5. **Minimal Overhead**: Efficient implementation with low performance impact

Use these features to thoroughly test your application under realistic and adversarial network conditions!
