# Performance Mode (Load Simulation)

**Pillars:** [DevX]

Performance Mode provides lightweight load simulation for running scenarios at N RPS, simulating bottlenecks, recording latencies, and observing how responses change under load. This is NOT true load testing—it's realistic behavior simulation under stress testing conditions.

## Overview

Performance Mode enables:

- **Run scenarios at n RPS**: Control request rate
- **Simulate bottlenecks**: Add artificial delays
- **Record latencies**: Track response times
- **Observe behavior changes**: See how responses change under load

## Quick Start

### Start Performance Mode

```bash
# Start performance mode
mockforge performance start --rps 100

# Start with bottlenecks
mockforge performance start \
  --rps 100 \
  --bottleneck checkout:500ms \
  --bottleneck payments:1000ms
```

### Via API

```bash
# Start performance mode
POST /api/performance/start
{
  "initial_rps": 100,
  "rps_profile": "constant",
  "bottlenecks": [
    {
      "endpoint": "/api/checkout/*",
      "delay_ms": 500
    }
  ]
}
```

## RPS Profiles

### Constant RPS

Maintain constant requests per second:

```yaml
rps_profile:
  type: constant
  rps: 100
```

### Ramp Profile

Gradually increase RPS:

```yaml
rps_profile:
  type: ramp
  start_rps: 10
  end_rps: 100
  duration_seconds: 60
```

### Spike Profile

Sudden spike in RPS:

```yaml
rps_profile:
  type: spike
  base_rps: 50
  spike_rps: 200
  spike_duration_seconds: 10
  spike_interval_seconds: 60
```

## Bottleneck Simulation

### Endpoint Bottlenecks

Add delays to specific endpoints:

```yaml
bottlenecks:
  - endpoint: "/api/checkout/*"
    delay_ms: 500
    probability: 1.0  # Always delay
  - endpoint: "/api/payments/*"
    delay_ms: 1000
    probability: 0.5  # 50% chance
```

### Database Bottlenecks

Simulate database slowdowns:

```yaml
bottlenecks:
  - type: database
    delay_ms: 200
    probability: 0.3
```

### Network Bottlenecks

Simulate network conditions:

```yaml
bottlenecks:
  - type: network
    latency_ms: 100
    packet_loss: 0.01
```

## Latency Recording

### Automatic Recording

Latencies are automatically recorded:

```json
{
  "endpoint": "/api/users/{id}",
  "method": "GET",
  "latency_ms": 150,
  "status_code": 200,
  "timestamp": "2025-01-27T10:00:00Z"
}
```

### Latency Analysis

Analyze recorded latencies:

```bash
# Get latency statistics
GET /api/performance/snapshot

# Response:
{
  "stats": {
    "mean_latency_ms": 150,
    "p50_latency_ms": 140,
    "p95_latency_ms": 250,
    "p99_latency_ms": 400,
    "max_latency_ms": 500
  }
}
```

## Response Changes Under Load

### Behavioral Economics Integration

Responses may change under load due to behavioral economics:

```yaml
# Under normal load
GET /api/checkout → 200 OK, conversion: 90%

# Under high load (latency > 400ms)
GET /api/checkout → 200 OK, conversion: 70%  # 20% drop
```

### Error Rate Increases

Error rates may increase under load:

```yaml
# Normal load
Error rate: 1%

# High load
Error rate: 5%  # Increased errors
```

## Usage Examples

### Example 1: Constant Load

```bash
# Run at constant 100 RPS
mockforge performance start --rps 100

# Monitor
GET /api/performance/snapshot
```

### Example 2: Ramp Load

```bash
# Ramp from 10 to 100 RPS over 60 seconds
mockforge performance start \
  --rps-profile ramp \
  --start-rps 10 \
  --end-rps 100 \
  --duration 60
```

### Example 3: With Bottlenecks

```bash
# Run with checkout bottleneck
mockforge performance start \
  --rps 100 \
  --bottleneck "/api/checkout/*:500ms"
```

## Configuration

### Performance Mode Config

```yaml
# mockforge.yaml
performance:
  enabled: true
  default_rps: 100
  max_latency_samples: 10000
  max_latency_age_seconds: 300
```

### RPS Controller

```yaml
performance:
  rps_controller:
    type: token_bucket
    capacity: 1000
    refill_rate: 100  # per second
```

## Best Practices

1. **Start Low**: Begin with low RPS and increase gradually
2. **Monitor Latencies**: Watch latency percentiles
3. **Test Bottlenecks**: Simulate realistic bottlenecks
4. **Observe Behavior**: Watch how responses change
5. **Not True Load Testing**: This is simulation, not production load testing

## Related Documentation

- [Behavioral Economics Engine](behavioral-economics.md) - Behavior under load
- [Chaos Lab](chaos-lab.md) - Chaos engineering
- [Reality Slider](reality-slider.md) - Reality levels

