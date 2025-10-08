# Advanced Resilience Patterns

This document describes the advanced resilience patterns available in MockForge, including circuit breakers, bulkheads, retry policies, and fallback handlers.

## Table of Contents

1. [Per-Endpoint Circuit Breakers](#per-endpoint-circuit-breakers)
2. [Per-Service Bulkheads](#per-service-bulkheads)
3. [Dynamic Threshold Adjustment](#dynamic-threshold-adjustment)
4. [Retry with Exponential Backoff](#retry-with-exponential-backoff)
5. [Fallback Handlers](#fallback-handlers)
6. [Prometheus Metrics](#prometheus-metrics)
7. [Health Check Integration](#health-check-integration)
8. [Circuit Breaker Dashboard](#circuit-breaker-dashboard)
9. [Configuration](#configuration)
10. [Usage Examples](#usage-examples)

## Per-Endpoint Circuit Breakers

Circuit breakers prevent cascading failures by stopping requests to failing services. MockForge provides per-endpoint circuit breakers that maintain separate state for each endpoint.

### Features

- **Three States**: Closed (normal), Open (blocked), Half-Open (testing)
- **Automatic State Transitions**: Based on failure thresholds and timeouts
- **Per-Endpoint Isolation**: Each endpoint has its own circuit breaker
- **Configurable Thresholds**: Customize failure rates and counts

### Circuit Breaker States

```
┌─────────┐
│ Closed  │ ──[failures > threshold]──> ┌──────┐
└─────────┘                              │ Open │
     ▲                                   └──────┘
     │                                      │
     │                                      │ [timeout elapsed]
     │                                      ▼
     │                                  ┌──────────┐
     └─────[successes > threshold]──── │HalfOpen  │
                                        └──────────┘
```

### Usage

```rust
use mockforge_chaos::{CircuitBreakerManager, CircuitBreakerConfig};
use prometheus::Registry;
use std::sync::Arc;

// Create circuit breaker manager
let registry = Arc::new(Registry::new());
let config = CircuitBreakerConfig {
    enabled: true,
    failure_threshold: 5,
    success_threshold: 2,
    timeout_ms: 60000,
    half_open_max_requests: 3,
    failure_rate_threshold: 50.0,
    min_requests_for_rate: 10,
    rolling_window_ms: 10000,
};

let cb_manager = CircuitBreakerManager::new(config, registry);

// Get circuit breaker for an endpoint
let breaker = cb_manager.get_breaker("/api/users").await;

// Check if request is allowed
if breaker.allow_request().await {
    // Make request
    match make_request().await {
        Ok(_) => breaker.record_success().await,
        Err(_) => breaker.record_failure().await,
    }
} else {
    // Circuit is open, handle rejection
    return fallback_response();
}
```

## Per-Service Bulkheads

Bulkheads isolate resources by limiting concurrent requests per service, preventing resource exhaustion.

### Features

- **Concurrent Request Limiting**: Set max concurrent requests per service
- **Request Queuing**: Queue excess requests with configurable timeout
- **Per-Service Isolation**: Each service has its own bulkhead
- **RAII Guards**: Automatic resource cleanup

### Usage

```rust
use mockforge_chaos::{BulkheadManager, BulkheadConfig};
use prometheus::Registry;
use std::sync::Arc;

// Create bulkhead manager
let registry = Arc::new(Registry::new());
let config = BulkheadConfig {
    enabled: true,
    max_concurrent_requests: 100,
    max_queue_size: 10,
    queue_timeout_ms: 5000,
};

let bh_manager = BulkheadManager::new(config, registry);

// Get bulkhead for a service
let bulkhead = bh_manager.get_bulkhead("payment-service").await;

// Acquire slot (blocks if full, queues if queue available)
match bulkhead.try_acquire().await {
    Ok(guard) => {
        // Process request
        // Guard automatically releases on drop
        process_request().await
    }
    Err(BulkheadError::Rejected) => {
        // Bulkhead full
        return error_response("Service busy");
    }
    Err(BulkheadError::Timeout) => {
        // Timeout in queue
        return error_response("Service timeout");
    }
}
```

## Dynamic Threshold Adjustment

Dynamic threshold adjustment automatically adapts circuit breaker thresholds based on traffic patterns and error rates.

### Features

- **Adaptive Thresholds**: Automatically adjust based on observed error rates
- **Traffic-Aware**: Considers request volume when adjusting
- **Configurable Bounds**: Set min/max threshold values
- **Target Error Rate**: Define acceptable error rate (e.g., 10%)

### How It Works

The dynamic threshold adjuster:
1. Monitors request success/failure rates over a sliding window
2. Compares actual error rate to target error rate
3. Adjusts failure threshold to maintain target error rate:
   - If error rate > target: Lower threshold (more sensitive)
   - If error rate < target/2: Raise threshold (less sensitive)

### Usage

```rust
// Dynamic adjustment is built into CircuitBreakerManager
// Adjustments happen automatically when recording results

// Record with automatic adjustment
cb_manager.record_with_adjustment("/api/users", success).await;
```

## Retry with Exponential Backoff

Automatic retry logic with exponential backoff for transient failures.

### Features

- **Configurable Attempts**: Set max retry attempts
- **Exponential Backoff**: Increasing delay between retries
- **Jitter**: Random jitter to prevent thundering herd
- **Configurable Multiplier**: Control backoff growth rate

### Usage

```rust
use mockforge_chaos::{RetryPolicy, ResilienceRetryConfig};

let config = ResilienceRetryConfig {
    max_attempts: 3,
    initial_backoff_ms: 100,
    max_backoff_ms: 30000,
    backoff_multiplier: 2.0,
    jitter_factor: 0.1,
};

let retry_policy = RetryPolicy::new(config);

// Execute with retry
let result = retry_policy.execute(|| async {
    make_api_call().await
}).await?;
```

### Backoff Calculation

```
attempt 1: 100ms + jitter
attempt 2: 200ms + jitter (100 * 2.0)
attempt 3: 400ms + jitter (200 * 2.0)
...
max: 30000ms + jitter
```

## Fallback Handlers

Provide custom fallback responses when circuit breakers open.

### Features

- **Custom Responses**: Define fallback behavior per endpoint
- **Type-Safe**: Trait-based design for type safety
- **JSON Support**: Built-in JSON fallback handler
- **Extensible**: Implement custom handlers

### Usage

```rust
use mockforge_chaos::{JsonFallbackHandler, FallbackHandler};
use serde_json::json;

// Create JSON fallback
let fallback = JsonFallbackHandler::new(json!({
    "error": "Service temporarily unavailable",
    "status": "circuit_open",
    "retry_after": 60
}));

// Use fallback when circuit is open
if !breaker.allow_request().await {
    return Response::builder()
        .status(503)
        .body(fallback.handle())
        .unwrap();
}
```

### Custom Fallback Handler

```rust
use mockforge_chaos::FallbackHandler;

struct CachedResponseFallback {
    cached_data: Vec<u8>,
}

impl FallbackHandler for CachedResponseFallback {
    fn handle(&self) -> Vec<u8> {
        self.cached_data.clone()
    }
}
```

## Prometheus Metrics

Export circuit breaker and bulkhead metrics to Prometheus for monitoring and alerting.

### Circuit Breaker Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `circuit_breaker_state` | Gauge | Current state (0=Closed, 1=Open, 2=HalfOpen) | endpoint |
| `circuit_breaker_requests_total` | Counter | Total requests | endpoint |
| `circuit_breaker_requests_successful` | Counter | Successful requests | endpoint |
| `circuit_breaker_requests_failed` | Counter | Failed requests | endpoint |
| `circuit_breaker_requests_rejected` | Counter | Rejected requests | endpoint |
| `circuit_breaker_state_transitions` | Counter | State transitions | endpoint |
| `circuit_breaker_request_duration_seconds` | Histogram | Request duration | endpoint |

### Bulkhead Metrics

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `bulkhead_active_requests` | Gauge | Active requests | service |
| `bulkhead_queued_requests` | Gauge | Queued requests | service |
| `bulkhead_requests_total` | Counter | Total requests | service |
| `bulkhead_requests_rejected` | Counter | Rejected requests | service |
| `bulkhead_requests_timeout` | Counter | Timeout requests | service |
| `bulkhead_queue_duration_seconds` | Histogram | Time in queue | service |

### Querying Metrics

```promql
# Circuit breaker open count by endpoint
sum by (endpoint) (circuit_breaker_state == 1)

# Circuit breaker success rate
rate(circuit_breaker_requests_successful[5m]) / rate(circuit_breaker_requests_total[5m])

# Bulkhead utilization
bulkhead_active_requests / bulkhead_max_concurrent

# Average queue time
rate(bulkhead_queue_duration_seconds_sum[5m]) / rate(bulkhead_queue_duration_seconds_count[5m])
```

## Health Check Integration

Automatically update circuit breaker state based on health check results.

### Features

- **Automatic Monitoring**: Periodic health checks
- **Circuit State Updates**: Update based on health
- **Configurable Intervals**: Set check frequency
- **HTTP Health Checks**: Built-in HTTP support

### Usage

```rust
use mockforge_chaos::HealthCheckIntegration;
use std::time::Duration;

// Create health check integration
let health_integration = HealthCheckIntegration::new(Arc::new(cb_manager));

// Manual health check update
health_integration.update_from_health("/api/users", healthy).await;

// Start periodic monitoring
health_integration.start_monitoring(
    "/api/users".to_string(),
    "http://api-server/health".to_string(),
    Duration::from_secs(30),
).await;
```

## Circuit Breaker Dashboard

Real-time web dashboard for monitoring circuit breaker and bulkhead states.

### Features

- **Real-Time Updates**: Auto-refresh every 3 seconds
- **Visual State Indicators**: Color-coded states
- **Detailed Statistics**: Success/failure rates, utilization
- **Reset Controls**: Manual reset of circuit breakers
- **Responsive Design**: Works on desktop and mobile

### API Endpoints

```
GET  /api/resilience/circuit-breakers          # List all circuit breakers
GET  /api/resilience/circuit-breakers/:endpoint # Get specific circuit breaker
POST /api/resilience/circuit-breakers/:endpoint/reset # Reset circuit breaker

GET  /api/resilience/bulkheads                 # List all bulkheads
GET  /api/resilience/bulkheads/:service        # Get specific bulkhead
POST /api/resilience/bulkheads/:service/reset  # Reset bulkhead stats

GET  /api/resilience/dashboard/summary         # Get dashboard summary
```

### Setting Up the Dashboard

```rust
use mockforge_chaos::{create_resilience_router, ResilienceApiState};
use axum::Router;

// Create API state
let state = ResilienceApiState {
    circuit_breaker_manager: Arc::new(cb_manager),
    bulkhead_manager: Arc::new(bh_manager),
};

// Create router
let router = create_resilience_router(state);

// Mount under /api/resilience
let app = Router::new()
    .nest("/api/resilience", router);
```

### Dashboard UI

Access the dashboard at:
```
http://localhost:3000/resilience
```

## Configuration

### Complete Circuit Breaker Configuration

```yaml
circuit_breaker:
  enabled: true
  # Open circuit after 5 consecutive failures
  failure_threshold: 5
  # Close circuit after 2 consecutive successes in half-open
  success_threshold: 2
  # Try half-open after 60 seconds
  timeout_ms: 60000
  # Allow 3 requests in half-open state
  half_open_max_requests: 3
  # Open circuit if failure rate exceeds 50%
  failure_rate_threshold: 50.0
  # Minimum 10 requests before calculating rate
  min_requests_for_rate: 10
  # Calculate rate over 10 second window
  rolling_window_ms: 10000
```

### Complete Bulkhead Configuration

```yaml
bulkhead:
  enabled: true
  # Maximum 100 concurrent requests
  max_concurrent_requests: 100
  # Queue up to 10 additional requests
  max_queue_size: 10
  # Queue timeout of 5 seconds
  queue_timeout_ms: 5000
```

## Usage Examples

### Complete Integration Example

```rust
use mockforge_chaos::{
    CircuitBreakerManager, BulkheadManager, RetryPolicy,
    CircuitBreakerConfig, BulkheadConfig, ResilienceRetryConfig,
    JsonFallbackHandler, HealthCheckIntegration,
};
use prometheus::Registry;
use std::sync::Arc;
use serde_json::json;

#[tokio::main]
async fn main() {
    // Setup
    let registry = Arc::new(Registry::new());

    // Circuit breaker
    let cb_config = CircuitBreakerConfig::default();
    let cb_manager = Arc::new(CircuitBreakerManager::new(cb_config, registry.clone()));

    // Bulkhead
    let bh_config = BulkheadConfig::default();
    let bh_manager = Arc::new(BulkheadManager::new(bh_config, registry.clone()));

    // Retry policy
    let retry_config = ResilienceRetryConfig::default();
    let retry_policy = RetryPolicy::new(retry_config);

    // Fallback
    let fallback = JsonFallbackHandler::new(json!({
        "error": "Service unavailable"
    }));

    // Health check integration
    let health_integration = HealthCheckIntegration::new(cb_manager.clone());

    // Make a resilient request
    let endpoint = "/api/users";

    // 1. Get circuit breaker
    let breaker = cb_manager.get_breaker(endpoint).await;

    // 2. Check if allowed
    if !breaker.allow_request().await {
        // Use fallback
        return Ok(fallback.handle());
    }

    // 3. Get bulkhead
    let bulkhead = bh_manager.get_bulkhead("user-service").await;
    let _guard = bulkhead.try_acquire().await?;

    // 4. Execute with retry
    let result = retry_policy.execute(|| async {
        make_api_request(endpoint).await
    }).await;

    // 5. Record result
    match result {
        Ok(response) => {
            breaker.record_success().await;
            cb_manager.record_with_adjustment(endpoint, true).await;
            Ok(response)
        }
        Err(err) => {
            breaker.record_failure().await;
            cb_manager.record_with_adjustment(endpoint, false).await;
            Err(err)
        }
    }
}
```

### Middleware Integration

```rust
use axum::{middleware, Router};
use mockforge_chaos::ResilienceApiState;

async fn resilience_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let endpoint = req.uri().path();

    // Get circuit breaker
    let breaker = state.circuit_breaker_manager.get_breaker(endpoint).await;

    // Check if allowed
    if !breaker.allow_request().await {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // Get bulkhead
    let bulkhead = state.bulkhead_manager.get_bulkhead("default").await;
    let _guard = match bulkhead.try_acquire().await {
        Ok(guard) => guard,
        Err(_) => return Err(StatusCode::TOO_MANY_REQUESTS),
    };

    // Process request
    let response = next.run(req).await;

    // Record result
    if response.status().is_success() {
        breaker.record_success().await;
    } else {
        breaker.record_failure().await;
    }

    Ok(response)
}

let app = Router::new()
    .route("/api/users", get(get_users))
    .layer(middleware::from_fn(resilience_middleware));
```

## Best Practices

1. **Threshold Tuning**: Start with conservative thresholds and adjust based on metrics
2. **Per-Service Bulkheads**: Use separate bulkheads for different services to prevent cross-service impact
3. **Fallback Strategies**: Always provide meaningful fallback responses
4. **Monitoring**: Set up Prometheus alerts for open circuits and high bulkhead utilization
5. **Health Checks**: Use health check integration for proactive circuit management
6. **Testing**: Test circuit breaker behavior under various failure scenarios
7. **Documentation**: Document circuit breaker configurations and expected behavior

## Troubleshooting

### Circuit Stays Open

- Check failure threshold configuration
- Verify timeout_ms allows enough time for recovery
- Review half_open_max_requests setting
- Check if health checks are failing

### Bulkhead Rejections

- Increase max_concurrent_requests if legitimate traffic
- Review queue_timeout_ms
- Check if requests are taking too long
- Monitor active request count

### False Positives

- Increase failure_threshold
- Adjust failure_rate_threshold
- Increase min_requests_for_rate
- Enable dynamic threshold adjustment

## Related Documentation

- [Chaos Engineering Guide](CHAOS_ENGINEERING.md)
- [Resilience Patterns](RESILIENCE_PATTERNS.md)
- [Observability](OBSERVABILITY.md)
- [Prometheus Metrics](OPENTELEMETRY.md)
