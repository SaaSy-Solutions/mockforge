# Advanced Resilience Patterns

MockForge provides advanced resilience patterns to help you build robust, fault-tolerant systems through controlled failure simulation and protection mechanisms.

> **🚀 New Advanced Features Available!**
>
> MockForge now includes advanced resilience features:
> - **Per-Endpoint Circuit Breakers** - Separate circuit breakers for each endpoint
> - **Per-Service Bulkheads** - Resource isolation by service
> - **Dynamic Threshold Adjustment** - Adaptive thresholds based on traffic
> - **Retry with Exponential Backoff** - Automatic retry logic
> - **Fallback Handlers** - Custom fallback responses
> - **Prometheus Metrics** - Comprehensive metrics export
> - **Health Check Integration** - Automatic circuit state management
> - **Real-time Dashboard** - Visual monitoring UI
>
> 📖 See [ADVANCED_RESILIENCE.md](./ADVANCED_RESILIENCE.md) for complete documentation

## Table of Contents

- [Overview](#overview)
- [Circuit Breaker Pattern](#circuit-breaker-pattern)
- [Bulkhead Pattern](#bulkhead-pattern)
- [Configuration](#configuration)
- [API Reference](#api-reference)
- [CLI Usage](#cli-usage)
- [Best Practices](#best-practices)
- [Integration with Chaos Engineering](#integration-with-chaos-engineering)
- [Examples](#examples)
- [Advanced Features](#advanced-features)

## Overview

Resilience patterns are design patterns that help systems remain functional in the face of failures. MockForge implements two fundamental resilience patterns:

1. **Circuit Breaker**: Prevents cascading failures by stopping requests to failing services
2. **Bulkhead**: Isolates resources to prevent failure in one component from affecting others

These patterns work seamlessly with MockForge's chaos engineering features to provide comprehensive resilience testing.

## Circuit Breaker Pattern

### What is a Circuit Breaker?

A circuit breaker acts like an electrical circuit breaker - it monitors for failures and "opens" (stops allowing requests) when failures exceed a threshold. This prevents cascading failures and allows the failing service time to recover.

### Circuit States

The circuit breaker has three states:

1. **Closed** (Normal operation)
   - Requests flow through normally
   - Failures are monitored
   - Opens if failure threshold is exceeded

2. **Open** (Failure detected)
   - All requests are immediately rejected
   - After a timeout period, transitions to half-open
   - Prevents cascading failures

3. **Half-Open** (Testing recovery)
   - Limited number of requests are allowed through
   - If successful, circuit closes
   - If failures continue, circuit reopens

### Configuration

```yaml
observability:
  chaos:
    circuit_breaker:
      enabled: true
      failure_threshold: 5          # Open after 5 consecutive failures
      success_threshold: 2          # Close after 2 consecutive successes
      timeout_ms: 60000            # Try half-open after 60 seconds
      half_open_max_requests: 3    # Allow 3 requests in half-open state
      failure_rate_threshold: 50.0  # Open if failure rate exceeds 50%
      min_requests_for_rate: 10    # Need 10 requests before calculating rate
      rolling_window_ms: 10000     # 10-second rolling window
```

### How It Works

```
┌─────────────┐
│   CLOSED    │  Normal operation, monitoring failures
└──────┬──────┘
       │ Failure threshold exceeded
       ↓
┌─────────────┐
│    OPEN     │  Reject all requests
└──────┬──────┘
       │ Timeout elapsed
       ↓
┌─────────────┐
│ HALF-OPEN   │  Test with limited requests
└──────┬──────┘
       │
       ├─ Successes → CLOSED
       └─ Failures  → OPEN
```

### API Usage

#### Get Circuit Breaker Status

```bash
GET /api/chaos/resilience/circuit-breaker/status
```

Response:
```json
{
  "state": "closed",
  "total_requests": 1000,
  "successful_requests": 950,
  "failed_requests": 50,
  "rejected_requests": 0,
  "consecutive_failures": 0,
  "consecutive_successes": 10,
  "last_state_change": "2025-10-07T12:00:00Z"
}
```

#### Update Circuit Breaker Configuration

```bash
PUT /api/chaos/config/circuit-breaker
Content-Type: application/json

{
  "enabled": true,
  "failure_threshold": 3,
  "success_threshold": 2,
  "timeout_ms": 30000,
  "half_open_max_requests": 5,
  "failure_rate_threshold": 60.0,
  "min_requests_for_rate": 20,
  "rolling_window_ms": 15000
}
```

#### Reset Circuit Breaker

```bash
POST /api/chaos/resilience/circuit-breaker/reset
```

## Bulkhead Pattern

### What is a Bulkhead?

The bulkhead pattern isolates resources (like thread pools or connections) so that if one part of the system is overwhelmed, it doesn't bring down the entire system. Named after ship bulkheads that prevent one flooded compartment from sinking the entire ship.

### How It Works

```
┌─────────────────────────────────────┐
│        Bulkhead (max: 100)          │
├─────────────────────────────────────┤
│  Active Requests: 95/100            │
│  ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐         │
│  │R1│ │R2│ │R3│ ...  │95│         │
│  └──┘ └──┘ └──┘      └──┘         │
├─────────────────────────────────────┤
│  Queue: 5/10                        │
│  ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐         │
│  │Q1│ │Q2│ │Q3│ │Q4│ │Q5│         │
│  └──┘ └──┘ └──┘ └──┘ └──┘         │
└─────────────────────────────────────┘

New Request → Rejected (bulkhead full)
```

### Configuration

```yaml
observability:
  chaos:
    bulkhead:
      enabled: true
      max_concurrent_requests: 100  # Maximum active requests
      max_queue_size: 10           # Maximum queued requests (0 = no queue)
      queue_timeout_ms: 5000       # Queue timeout (5 seconds)
```

### Features

- **Concurrent Request Limiting**: Limit active requests
- **Request Queuing**: Queue excess requests with timeout
- **Automatic Cleanup**: RAII-based resource management
- **Real-time Statistics**: Monitor active and queued requests

### API Usage

#### Get Bulkhead Status

```bash
GET /api/chaos/resilience/bulkhead/status
```

Response:
```json
{
  "active_requests": 85,
  "queued_requests": 5,
  "total_requests": 10000,
  "rejected_requests": 50,
  "timeout_requests": 10
}
```

#### Update Bulkhead Configuration

```bash
PUT /api/chaos/config/bulkhead
Content-Type: application/json

{
  "enabled": true,
  "max_concurrent_requests": 50,
  "max_queue_size": 20,
  "queue_timeout_ms": 10000
}
```

#### Reset Bulkhead Statistics

```bash
POST /api/chaos/resilience/bulkhead/reset
```

## Configuration

### YAML Configuration

Complete configuration example:

```yaml
observability:
  chaos:
    enabled: true

    # Circuit Breaker Configuration
    circuit_breaker:
      enabled: true
      failure_threshold: 5
      success_threshold: 2
      timeout_ms: 60000
      half_open_max_requests: 3
      failure_rate_threshold: 50.0
      min_requests_for_rate: 10
      rolling_window_ms: 10000

    # Bulkhead Configuration
    bulkhead:
      enabled: true
      max_concurrent_requests: 100
      max_queue_size: 10
      queue_timeout_ms: 5000

    # Works with other chaos features
    latency:
      enabled: true
      fixed_delay_ms: 100

    fault_injection:
      enabled: true
      http_errors: [500, 503]
      http_error_probability: 0.1
```

### Environment Variables

```bash
# Circuit Breaker
MOCKFORGE_CIRCUIT_BREAKER_ENABLED=true
MOCKFORGE_CIRCUIT_BREAKER_FAILURE_THRESHOLD=5
MOCKFORGE_CIRCUIT_BREAKER_SUCCESS_THRESHOLD=2
MOCKFORGE_CIRCUIT_BREAKER_TIMEOUT_MS=60000

# Bulkhead
MOCKFORGE_BULKHEAD_ENABLED=true
MOCKFORGE_BULKHEAD_MAX_CONCURRENT=100
MOCKFORGE_BULKHEAD_MAX_QUEUE=10
MOCKFORGE_BULKHEAD_QUEUE_TIMEOUT_MS=5000
```

## CLI Usage

### Enable Circuit Breaker

```bash
mockforge serve \
  --circuit-breaker \
  --circuit-breaker-failure-threshold 3 \
  --circuit-breaker-success-threshold 2 \
  --circuit-breaker-timeout-ms 30000 \
  --circuit-breaker-failure-rate 60.0
```

### Enable Bulkhead

```bash
mockforge serve \
  --bulkhead \
  --bulkhead-max-concurrent 50 \
  --bulkhead-max-queue 20 \
  --bulkhead-queue-timeout-ms 10000
```

### Combined with Chaos Engineering

```bash
mockforge serve \
  --chaos \
  --chaos-http-errors "500,503" \
  --chaos-http-error-probability 0.2 \
  --circuit-breaker \
  --circuit-breaker-failure-threshold 5 \
  --bulkhead \
  --bulkhead-max-concurrent 100
```

## Best Practices

### 1. Circuit Breaker Best Practices

#### Choose Appropriate Thresholds

```yaml
# For critical services - fail fast
circuit_breaker:
  failure_threshold: 3        # Open after 3 failures
  timeout_ms: 30000          # Try recovery after 30s

# For resilient services - more tolerance
circuit_breaker:
  failure_threshold: 10       # Open after 10 failures
  timeout_ms: 120000         # Try recovery after 2min
```

#### Use Failure Rate for High-Traffic Services

```yaml
circuit_breaker:
  failure_threshold: 100       # High threshold
  failure_rate_threshold: 25.0 # But open if 25% fail
  min_requests_for_rate: 50   # Need 50 requests minimum
  rolling_window_ms: 30000    # In 30-second window
```

#### Monitor State Changes

```javascript
// Poll circuit breaker status
setInterval(async () => {
  const status = await fetch('/api/chaos/resilience/circuit-breaker/status');
  const data = await status.json();

  if (data.state === 'open') {
    console.warn('Circuit breaker opened!', data);
  }
}, 5000);
```

### 2. Bulkhead Best Practices

#### Size the Bulkhead Appropriately

```yaml
# Calculate based on your system capacity
# Rule of thumb: max_concurrent = (cores * 2) to (cores * 4)
bulkhead:
  max_concurrent_requests: 100  # 16-core system * 4 = 64-128
  max_queue_size: 10           # 10% of max_concurrent
```

#### Use Queuing Wisely

```yaml
# For critical real-time APIs - no queue
bulkhead:
  max_concurrent_requests: 50
  max_queue_size: 0           # Reject immediately

# For batch processing - larger queue
bulkhead:
  max_concurrent_requests: 20
  max_queue_size: 100
  queue_timeout_ms: 30000     # Longer timeout
```

#### Monitor Queue Depth

```bash
# Check bulkhead status
curl http://localhost:3000/api/chaos/resilience/bulkhead/status

# Alert if queue is consistently full
if queued_requests > (max_queue_size * 0.8):
    alert("Bulkhead queue nearly full - scale up!")
```

### 3. Combining Patterns

#### Layer Defenses

```yaml
# Layer 1: Rate limiting
chaos:
  rate_limit:
    enabled: true
    requests_per_second: 1000

  # Layer 2: Bulkhead (resource isolation)
  bulkhead:
    enabled: true
    max_concurrent_requests: 100

  # Layer 3: Circuit breaker (failure detection)
  circuit_breaker:
    enabled: true
    failure_threshold: 5
```

#### Test with Chaos

```bash
# Simulate service degradation
mockforge serve \
  --chaos \
  --chaos-http-errors "500" \
  --chaos-http-error-probability 0.3 \
  --circuit-breaker \
  --circuit-breaker-failure-threshold 5 \
  --bulkhead \
  --bulkhead-max-concurrent 50

# Circuit breaker should open after 5 failures
# Bulkhead should prevent overload
```

## Integration with Chaos Engineering

### Automatic Failure Detection

MockForge's middleware automatically tracks request success/failure:

```rust
// Automatic integration in chaos middleware
pub async fn chaos_middleware(...) -> Response {
    // Check circuit breaker
    if !circuit_breaker.allow_request().await {
        return service_unavailable();
    }

    // Acquire bulkhead slot
    let _guard = bulkhead.try_acquire().await?;

    // Process request
    let response = next.run(request).await;

    // Record result automatically
    if response.status().is_server_error() {
        circuit_breaker.record_failure().await;
    } else {
        circuit_breaker.record_success().await;
    }

    response
}
```

### Combined Scenarios

Create resilience test scenarios:

```yaml
# Scenario: Test circuit breaker under load
observability:
  chaos:
    # Inject 30% errors
    fault_injection:
      enabled: true
      http_errors: [503]
      http_error_probability: 0.3

    # Circuit breaker should open
    circuit_breaker:
      enabled: true
      failure_threshold: 5
      timeout_ms: 30000

    # Limit concurrent load
    bulkhead:
      enabled: true
      max_concurrent_requests: 50
```

## Examples

### Example 1: Protect Against Cascading Failures

```bash
# Start MockForge with circuit breaker
mockforge serve \
  --chaos \
  --chaos-http-errors "503" \
  --chaos-http-error-probability 0.5 \
  --circuit-breaker \
  --circuit-breaker-failure-threshold 3 \
  --circuit-breaker-timeout-ms 10000
```

Test script:
```bash
#!/bin/bash
# Send 100 requests
for i in {1..100}; do
  curl -w "%{http_code}\n" http://localhost:3000/api/test
  sleep 0.1
done

# Expected: First few fail, then circuit opens
# Response: 503, 503, 503, 503 (circuit open), 503 (unavailable)...
# After 10s: Circuit tries half-open
```

### Example 2: Resource Isolation with Bulkhead

```bash
# Start with bulkhead limiting concurrent requests
mockforge serve \
  --bulkhead \
  --bulkhead-max-concurrent 10 \
  --bulkhead-max-queue 5 \
  --bulkhead-queue-timeout-ms 2000
```

Load test:
```bash
# Generate 20 concurrent requests (exceeds limit)
ab -n 100 -c 20 http://localhost:3000/api/test

# Expected results:
# - 10 requests active
# - 5 requests queued
# - 5 requests rejected (overload)
```

### Example 3: Complete Resilience Stack

```bash
mockforge serve \
  --chaos \
  --chaos-latency-ms 200 \
  --chaos-http-errors "500,503" \
  --chaos-http-error-probability 0.2 \
  --circuit-breaker \
  --circuit-breaker-failure-threshold 5 \
  --circuit-breaker-success-threshold 3 \
  --bulkhead \
  --bulkhead-max-concurrent 100 \
  --bulkhead-max-queue 20 \
  --chaos-rate-limit 1000
```

This setup provides:
- ✅ Latency injection (200ms)
- ✅ Random errors (20%)
- ✅ Circuit breaker protection
- ✅ Bulkhead resource isolation
- ✅ Rate limiting (1000 req/s)

### Example 4: Monitor and Adapt

```javascript
// Monitor resilience patterns
async function monitorResilience() {
  // Get circuit breaker status
  const cbStatus = await fetch('/api/chaos/resilience/circuit-breaker/status')
    .then(r => r.json());

  // Get bulkhead status
  const bhStatus = await fetch('/api/chaos/resilience/bulkhead/status')
    .then(r => r.json());

  console.log('Circuit Breaker:', cbStatus.state);
  console.log('Bulkhead:', `${bhStatus.active_requests}/${bhStatus.max_concurrent}`);

  // Adapt based on state
  if (cbStatus.state === 'open') {
    // Increase timeout or alert
    await fetch('/api/chaos/config/circuit-breaker', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        ...cbStatus.config,
        timeout_ms: 120000  // Increase to 2 minutes
      })
    });
  }

  if (bhStatus.rejected_requests > 100) {
    // Scale up or alert
    console.warn('High rejection rate - consider scaling');
  }
}

setInterval(monitorResilience, 10000);  // Every 10 seconds
```

## Troubleshooting

### Circuit Breaker Issues

#### Issue: Circuit opens too frequently

**Cause**: Threshold too low for normal error rate

**Solution**:
```bash
# Increase failure threshold
curl -X PUT http://localhost:3000/api/chaos/config/circuit-breaker \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "failure_threshold": 10,
    "failure_rate_threshold": 60.0
  }'
```

#### Issue: Circuit doesn't close after recovery

**Cause**: Success threshold too high

**Solution**:
```bash
# Lower success threshold
curl -X PUT http://localhost:3000/api/chaos/config/circuit-breaker \
  -H "Content-Type: application/json" \
  -d '{
    "success_threshold": 1
  }'
```

### Bulkhead Issues

#### Issue: Too many rejected requests

**Cause**: max_concurrent_requests too low

**Solution**:
```bash
# Increase capacity
curl -X PUT http://localhost:3000/api/chaos/config/bulkhead \
  -H "Content-Type: application/json" \
  -d '{
    "max_concurrent_requests": 200,
    "max_queue_size": 50
  }'
```

#### Issue: Requests timing out in queue

**Cause**: Queue timeout too short or processing too slow

**Solution**:
```bash
# Increase timeout or disable queue
curl -X PUT http://localhost:3000/api/chaos/config/bulkhead \
  -H "Content-Type: application/json" \
  -d '{
    "queue_timeout_ms": 30000,
    "max_queue_size": 0  # Or disable queue
  }'
```

## Advanced Topics

### Custom Circuit Breaker Logic

For advanced use cases, you can use the circuit breaker programmatically:

```rust
use mockforge_chaos::{CircuitBreaker, CircuitBreakerConfig};

let config = CircuitBreakerConfig {
    enabled: true,
    failure_threshold: 5,
    ..Default::default()
};

let cb = CircuitBreaker::new(config);

// In your request handler
if !cb.allow_request().await {
    return Err("Service unavailable");
}

match make_request().await {
    Ok(response) => {
        cb.record_success().await;
        Ok(response)
    }
    Err(e) => {
        cb.record_failure().await;
        Err(e)
    }
}
```

### Per-Endpoint Bulkheads

Create separate bulkheads for different endpoints:

```rust
use mockforge_chaos::{Bulkhead, BulkheadConfig};

// Critical endpoint - strict limits
let critical_bulkhead = Bulkhead::new(BulkheadConfig {
    max_concurrent_requests: 10,
    max_queue_size: 0,
    ..Default::default()
});

// Normal endpoint - more relaxed
let normal_bulkhead = Bulkhead::new(BulkheadConfig {
    max_concurrent_requests: 100,
    max_queue_size: 20,
    ..Default::default()
});
```

## Advanced Features

MockForge provides advanced resilience features beyond basic circuit breakers and bulkheads:

### Per-Endpoint Circuit Breakers

Maintain separate circuit breaker state for each endpoint:

```rust
use mockforge_chaos::CircuitBreakerManager;

let cb_manager = CircuitBreakerManager::new(config, registry);

// Each endpoint gets its own circuit breaker
let users_breaker = cb_manager.get_breaker("/api/users").await;
let payments_breaker = cb_manager.get_breaker("/api/payments").await;
```

### Dynamic Threshold Adjustment

Automatically adjust thresholds based on traffic patterns:

```rust
// Record results with automatic threshold adjustment
cb_manager.record_with_adjustment(endpoint, success).await;
```

### Retry with Exponential Backoff

```rust
use mockforge_chaos::{RetryPolicy, ResilienceRetryConfig};

let retry_policy = RetryPolicy::new(ResilienceRetryConfig::default());

let result = retry_policy.execute(|| async {
    make_api_call().await
}).await?;
```

### Fallback Handlers

```rust
use mockforge_chaos::JsonFallbackHandler;
use serde_json::json;

let fallback = JsonFallbackHandler::new(json!({
    "error": "Service unavailable"
}));

if !breaker.allow_request().await {
    return Response::from(fallback.handle());
}
```

### Health Check Integration

```rust
use mockforge_chaos::HealthCheckIntegration;

let health_integration = HealthCheckIntegration::new(cb_manager);

// Start monitoring
health_integration.start_monitoring(
    "/api/users".to_string(),
    "http://api/health".to_string(),
    Duration::from_secs(30),
).await;
```

### Real-time Dashboard

Access the resilience dashboard at:
```
http://localhost:3000/api/resilience/dashboard/summary
```

Features:
- Real-time circuit breaker states
- Bulkhead utilization
- Success/failure rates
- Manual reset controls
- Auto-refresh

### Complete Documentation

For complete documentation on advanced features, see:
- **[ADVANCED_RESILIENCE.md](./ADVANCED_RESILIENCE.md)** - Complete advanced resilience guide

This includes:
- Detailed API documentation
- Configuration examples
- Usage patterns
- Best practices
- Troubleshooting

## See Also

- **[Advanced Resilience Guide](./ADVANCED_RESILIENCE.md)** - NEW! Complete advanced features
- [Chaos Engineering Guide](./CHAOS_ENGINEERING.md) - Base chaos capabilities
- [Protocol-Specific Chaos](./PROTOCOL_CHAOS.md) - gRPC, WebSocket, GraphQL chaos
- [Observability Guide](./OBSERVABILITY.md) - Metrics and monitoring
- [API Flight Recorder](./API_FLIGHT_RECORDER.md) - Request recording

## References

- [Release It! Design and Deploy Production-Ready Software](https://pragprog.com/titles/mnee2/release-it-second-edition/) by Michael T. Nygard
- [Martin Fowler - Circuit Breaker](https://martinfowler.com/bliki/CircuitBreaker.html)
- [Microsoft - Bulkhead Pattern](https://docs.microsoft.com/en-us/azure/architecture/patterns/bulkhead)
