# Advanced Resilience Features

This document describes the advanced resilience patterns implemented in MockForge, including persistent state, distributed circuit breakers, advanced retry strategies, custom health checks, WebSocket notifications, alert integration, SLO tracking, and per-user bulkheads.

## Table of Contents

1. [Persistent State](#1-persistent-state)
2. [Distributed Circuit Breakers](#2-distributed-circuit-breakers)
3. [Advanced Retry Strategies](#3-advanced-retry-strategies)
4. [Custom Health Check Protocols](#4-custom-health-check-protocols)
5. [WebSocket Updates](#5-websocket-updates)
6. [Alert Integration](#6-alert-integration)
7. [SLO Integration](#7-slo-integration)
8. [Per-User Bulkheads](#8-per-user-bulkheads)

---

## 1. Persistent State

Circuit breaker state can be persisted to disk or distributed storage, allowing recovery after restarts.

### File-based Persistence

```rust
use mockforge_chaos::resilience::{CircuitBreaker, CircuitBreakerConfig};
use std::path::PathBuf;

let config = CircuitBreakerConfig::default();
let breaker = CircuitBreaker::new(config)
    .with_endpoint("api-service")
    .with_persistence(PathBuf::from("/var/lib/mockforge/circuit-breaker-state.bin"));

// State is automatically saved on transitions
// Load state on startup
breaker.load_state().await?;
```

### Features

- **Automatic Persistence**: State is saved automatically on state transitions (Open, Half-Open, Closed)
- **Crash Recovery**: Circuit breaker state survives restarts
- **Binary Format**: Uses efficient `bincode` serialization
- **Async I/O**: Non-blocking file operations

### Stored State

The persisted state includes:
- Current circuit state (Closed, Open, Half-Open)
- Consecutive failures and successes
- Total, successful, failed, and rejected request counts
- Last state change timestamp

---

## 2. Distributed Circuit Breakers

Share circuit breaker state across multiple instances using Redis.

### Setup (requires `distributed` feature)

```toml
[dependencies]
mockforge-chaos = { version = "0.1", features = ["distributed"] }
```

### Usage

```rust
use mockforge_chaos::resilience::CircuitBreaker;

let config = CircuitBreakerConfig::default();
let breaker = CircuitBreaker::new(config)
    .with_endpoint("api-service")
    .with_distributed_state("redis://localhost:6379")
    .await?;

// State is automatically synchronized across all instances
breaker.record_failure().await;
```

### Features

- **Redis Backend**: Uses Redis for distributed state storage
- **Automatic Sync**: State changes are propagated to Redis
- **TTL Support**: Circuit breaker state expires after 1 hour
- **Fallback**: Falls back to file-based persistence if Redis is unavailable
- **Multi-Instance**: Multiple application instances share the same circuit state

### Benefits

- **Consistent State**: All instances see the same circuit breaker state
- **Coordinated Protection**: Circuit opens across all instances when threshold is reached
- **High Availability**: Works with Redis clusters

---

## 3. Advanced Retry Strategies

Circuit breaker-aware retry policy that respects circuit state.

### Basic Usage

```rust
use mockforge_chaos::resilience::{
    CircuitBreakerAwareRetry, RetryConfig, CircuitBreaker
};
use std::sync::Arc;

let retry_config = RetryConfig {
    max_attempts: 3,
    initial_backoff_ms: 100,
    max_backoff_ms: 5000,
    backoff_multiplier: 2.0,
    jitter_factor: 0.1,
};

let breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
let retry = CircuitBreakerAwareRetry::new(retry_config)
    .with_circuit_breaker(breaker);

// Execute with circuit-breaker-aware retry
let result = retry.execute(|| async {
    // Your operation here
    make_api_call().await
}).await?;
```

### Features

- **Circuit State Check**: Checks circuit state before each retry attempt
- **Early Abort**: Stops retrying if circuit opens during execution
- **Exponential Backoff**: Configurable backoff with jitter
- **Automatic Recording**: Records successes and failures in circuit breaker

### Configuration

```rust
RetryConfig {
    max_attempts: 3,           // Maximum retry attempts
    initial_backoff_ms: 100,   // Initial backoff duration
    max_backoff_ms: 5000,      // Maximum backoff duration
    backoff_multiplier: 2.0,   // Backoff multiplier (exponential)
    jitter_factor: 0.1,        // Jitter to prevent thundering herd (0.0-1.0)
}
```

---

## 4. Custom Health Check Protocols

Support for multiple protocols beyond HTTP.

### Supported Protocols

```rust
use mockforge_chaos::resilience::{
    HealthCheckProtocol, HealthCheckIntegration
};

// HTTP/HTTPS
let http = HealthCheckProtocol::Http {
    url: "http://api.example.com/health".to_string()
};

// TCP Connection
let tcp = HealthCheckProtocol::Tcp {
    host: "api.example.com".to_string(),
    port: 8080,
};

// gRPC Health Check
let grpc = HealthCheckProtocol::Grpc {
    endpoint: "http://api.example.com:9090".to_string()
};

// WebSocket Connection
let ws = HealthCheckProtocol::WebSocket {
    url: "ws://api.example.com/ws".to_string()
};

// Custom Health Checker
let custom = HealthCheckProtocol::Custom {
    checker: Arc::new(MyCustomHealthChecker),
};
```

### Custom Health Checker

Implement your own health check logic:

```rust
use mockforge_chaos::resilience::CustomHealthChecker;

struct DatabaseHealthChecker {
    connection_string: String,
}

impl CustomHealthChecker for DatabaseHealthChecker {
    fn check(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
        Box::pin(async move {
            // Custom health check logic
            match check_database_connection(&self.connection_string).await {
                Ok(_) => true,
                Err(_) => false,
            }
        })
    }
}
```

### Monitoring

```rust
let integration = HealthCheckIntegration::new(circuit_manager);

// Start monitoring with custom protocol
integration.start_monitoring(
    "api-service".to_string(),
    HealthCheckProtocol::Grpc {
        endpoint: "http://api.example.com:9090".to_string()
    },
    Duration::from_secs(30), // Check interval
).await;
```

---

## 5. WebSocket Updates

Real-time circuit breaker state updates via WebSocket.

### Setup

```rust
use mockforge_chaos::resilience::{
    ResilienceWebSocketNotifier, CircuitBreaker
};
use std::sync::Arc;

let notifier = ResilienceWebSocketNotifier::new();
let breaker = Arc::new(CircuitBreaker::new(config));

// Start monitoring
notifier.monitor_circuit_breaker(breaker.clone()).await;
```

### Client Connection

```rust
// Register a WebSocket connection
let mut rx = notifier.register().await;

// Receive state change notifications
tokio::spawn(async move {
    while let Ok(message) = rx.recv().await {
        println!("State change: {}", message);
        // Message is JSON serialized CircuitStateChange
    }
});
```

### State Change Event Format

```json
{
  "endpoint": "api-service",
  "old_state": "Closed",
  "new_state": "Open",
  "timestamp": "2025-10-08T12:00:00Z",
  "reason": "Failure threshold exceeded"
}
```

### Integration with Web Dashboard

```rust
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(notifier): State<Arc<ResilienceWebSocketNotifier>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, notifier))
}

async fn handle_socket(socket: WebSocket, notifier: Arc<ResilienceWebSocketNotifier>) {
    let mut rx = notifier.register().await;

    while let Ok(message) = rx.recv().await {
        if socket.send(Message::Text(message)).await.is_err() {
            break;
        }
    }
}
```

---

## 6. Alert Integration

Automatic alerts on circuit breaker state changes.

### Setup

```rust
use mockforge_chaos::{
    resilience::CircuitBreakerAlertHandler,
    alerts::AlertManager,
};
use std::sync::Arc;

let alert_manager = Arc::new(AlertManager::new());
let alert_handler = CircuitBreakerAlertHandler::new(alert_manager.clone());

// Monitor circuit breaker
alert_handler.monitor(breaker.clone()).await;
```

### Alert Behavior

- **Critical Alert**: Fired when circuit transitions to Open state
- **Auto-Resolution**: Information logged when circuit closes after being open
- **Rich Metadata**: Includes endpoint, reason, and timestamp

### Alert Example

```rust
Alert {
    id: "uuid",
    timestamp: "2025-10-08T12:00:00Z",
    severity: Critical,
    alert_type: Custom {
        message: "Circuit breaker opened for api-service",
        metadata: {
            "endpoint": "api-service",
            "reason": "Failure threshold exceeded",
            "timestamp": "2025-10-08T12:00:00Z"
        }
    },
    message: "Circuit breaker for endpoint 'api-service' has opened: Failure threshold exceeded",
    resolved: false,
    resolved_at: None,
}
```

### Custom Alert Handlers

```rust
use mockforge_chaos::alerts::{AlertHandler, Alert};

struct SlackAlertHandler {
    webhook_url: String,
}

impl AlertHandler for SlackAlertHandler {
    fn handle(&self, alert: &Alert) {
        // Send alert to Slack
        tokio::spawn(send_to_slack(self.webhook_url.clone(), alert.clone()));
    }
}

// Register custom handler
alert_manager.add_handler(Box::new(SlackAlertHandler {
    webhook_url: "https://hooks.slack.com/...".to_string()
}));
```

---

## 7. SLO Integration

Circuit breaker triggers based on SLO violations.

### SLO Configuration

```rust
use mockforge_chaos::resilience::{SLOConfig, SLOCircuitBreakerIntegration};
use std::time::Duration;

let slo_config = SLOConfig {
    target_success_rate: 0.99,      // 99% success rate
    window_duration: Duration::from_secs(300),  // 5-minute window
    error_budget_percent: 1.0,      // 1% error budget
};
```

### Usage

```rust
let integration = SLOCircuitBreakerIntegration::new(circuit_manager);

// Record requests with SLO tracking
integration.record_request(
    "api-service",
    true,  // success = true
    slo_config.clone()
).await;

// Automatically triggers circuit breaker if SLO is violated
```

### SLO Metrics

```rust
// Get SLO status
if let Some((success_rate, budget_remaining, violated)) =
    integration.get_slo_status("api-service").await {

    println!("Success Rate: {:.2}%", success_rate * 100.0);
    println!("Error Budget Remaining: {:.2}%", budget_remaining);
    println!("SLO Violated: {}", violated);
}
```

### SLO Tracker

```rust
use mockforge_chaos::resilience::{SLOTracker, SLOConfig};

let tracker = SLOTracker::new(slo_config);

// Record results
tracker.record(true).await;  // Success
tracker.record(false).await; // Failure

// Check metrics
let success_rate = tracker.success_rate().await;
let violated = tracker.is_violated().await;
let budget_remaining = tracker.error_budget_remaining().await;
```

### Benefits

- **Proactive Protection**: Circuit breaker opens before complete service failure
- **SLO Compliance**: Ensures service stays within defined SLO targets
- **Error Budget Tracking**: Monitor error budget consumption
- **Rolling Window**: Time-based window for SLO calculation

---

## 8. Per-User Bulkheads

Resource isolation at the user level.

### Setup

```rust
use mockforge_chaos::resilience::{PerUserBulkhead, BulkheadConfig};
use prometheus::Registry;
use std::sync::Arc;

let config = BulkheadConfig {
    enabled: true,
    max_concurrent_requests: 10,
    max_queue_size: 5,
    queue_timeout_ms: 5000,
};

let registry = Arc::new(Registry::new());
let per_user_bulkhead = PerUserBulkhead::new(config, registry);
```

### Usage

```rust
// Try to acquire a slot for a user
match per_user_bulkhead.try_acquire("user123").await {
    Ok(guard) => {
        // Process request
        process_user_request().await?;
        // Guard is automatically released when dropped
    }
    Err(BulkheadError::Rejected) => {
        // Too many concurrent requests for this user
        return Err("Rate limited");
    }
    Err(BulkheadError::Timeout) => {
        // Request timed out in queue
        return Err("Queue timeout");
    }
}
```

### Statistics

```rust
// Get stats for specific user
if let Some(stats) = per_user_bulkhead.get_user_stats("user123").await {
    println!("Active: {}", stats.active_requests);
    println!("Queued: {}", stats.queued_requests);
    println!("Total: {}", stats.total_requests);
    println!("Rejected: {}", stats.rejected_requests);
    println!("Timeouts: {}", stats.timeout_requests);
}

// Get all user stats
let all_stats = per_user_bulkhead.get_all_stats().await;
for (user_id, stats) in all_stats {
    println!("{}: {} active requests", user_id, stats.active_requests);
}
```

### Cleanup

```rust
// Remove bulkhead for inactive users
per_user_bulkhead.remove_user("user123").await;
```

### Benefits

- **Fair Resource Allocation**: Each user gets their own resource pool
- **Noisy Neighbor Prevention**: One user can't monopolize resources
- **Per-User Rate Limiting**: Fine-grained control over user access
- **Automatic Isolation**: Users are automatically isolated from each other

---

## Complete Example

Here's a complete example combining all features:

```rust
use mockforge_chaos::{
    resilience::*,
    alerts::AlertManager,
};
use std::sync::Arc;
use std::time::Duration;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup circuit breaker with persistence and distributed state
    let cb_config = CircuitBreakerConfig::default();
    let breaker = Arc::new(
        CircuitBreaker::new(cb_config)
            .with_endpoint("api-service")
            .with_persistence(PathBuf::from("/var/lib/mockforge/circuit.bin"))
            .with_distributed_state("redis://localhost:6379")
            .await?
    );

    // Load previous state
    breaker.load_state().await?;

    // 2. Setup circuit breaker manager
    let registry = Arc::new(prometheus::Registry::new());
    let circuit_manager = Arc::new(CircuitBreakerManager::new(
        CircuitBreakerConfig::default(),
        registry.clone(),
    ));

    // 3. Setup health checks with custom protocol
    let health_integration = HealthCheckIntegration::new(circuit_manager.clone());
    health_integration.start_monitoring(
        "api-service".to_string(),
        HealthCheckProtocol::Grpc {
            endpoint: "http://api.example.com:9090".to_string()
        },
        Duration::from_secs(30),
    ).await;

    // 4. Setup WebSocket notifications
    let ws_notifier = Arc::new(ResilienceWebSocketNotifier::new());
    ws_notifier.monitor_circuit_breaker(breaker.clone()).await;

    // 5. Setup alert integration
    let alert_manager = Arc::new(AlertManager::new());
    let alert_handler = CircuitBreakerAlertHandler::new(alert_manager.clone());
    alert_handler.monitor(breaker.clone()).await;

    // 6. Setup SLO tracking
    let slo_integration = SLOCircuitBreakerIntegration::new(circuit_manager.clone());
    let slo_config = SLOConfig::default();

    // 7. Setup per-user bulkheads
    let per_user_bulkhead = Arc::new(PerUserBulkhead::new(
        BulkheadConfig::default(),
        registry.clone(),
    ));

    // 8. Setup circuit-breaker-aware retry
    let retry = CircuitBreakerAwareRetry::new(RetryConfig::default())
        .with_circuit_breaker(breaker.clone());

    // Example request handling
    let user_id = "user123";

    // Acquire user bulkhead slot
    let _guard = per_user_bulkhead.try_acquire(user_id).await?;

    // Execute with retry and circuit breaker
    let result = retry.execute(|| async {
        // Make API call
        make_api_call().await
    }).await;

    // Record in SLO tracker
    let success = result.is_ok();
    slo_integration.record_request("api-service", success, slo_config).await;

    Ok(())
}
```

---

## Configuration Reference

### Circuit Breaker Configuration

```rust
CircuitBreakerConfig {
    enabled: true,
    failure_threshold: 5,           // Failures before opening
    success_threshold: 2,           // Successes to close from half-open
    timeout_ms: 60000,              // Timeout before half-open
    half_open_max_requests: 3,      // Max requests in half-open
    failure_rate_threshold: 50.0,   // Failure rate % to open
    min_requests_for_rate: 10,      // Min requests for rate calculation
    rolling_window_ms: 10000,       // Rolling window duration
}
```

### Bulkhead Configuration

```rust
BulkheadConfig {
    enabled: true,
    max_concurrent_requests: 100,   // Max concurrent requests
    max_queue_size: 10,             // Max queue size
    queue_timeout_ms: 5000,         // Queue timeout
}
```

### Retry Configuration

```rust
RetryConfig {
    max_attempts: 3,                // Max retry attempts
    initial_backoff_ms: 100,        // Initial backoff
    max_backoff_ms: 30000,          // Max backoff
    backoff_multiplier: 2.0,        // Backoff multiplier
    jitter_factor: 0.1,             // Jitter factor
}
```

### SLO Configuration

```rust
SLOConfig {
    target_success_rate: 0.99,                      // 99% target
    window_duration: Duration::from_secs(300),      // 5-minute window
    error_budget_percent: 1.0,                      // 1% error budget
}
```

---

## Best Practices

1. **Persistence**: Always enable persistence for production environments
2. **Distributed State**: Use Redis for multi-instance deployments
3. **Health Checks**: Implement appropriate protocol-specific health checks
4. **Monitoring**: Use WebSocket notifications for real-time dashboards
5. **Alerts**: Configure alerts for critical state changes
6. **SLO Tracking**: Monitor SLO compliance to prevent violations
7. **User Isolation**: Use per-user bulkheads for multi-tenant systems
8. **Retry Strategy**: Always use circuit-breaker-aware retry
9. **Metrics**: Export Prometheus metrics for observability
10. **Testing**: Test circuit breaker behavior in staging environments

---

## Troubleshooting

### Circuit Breaker Not Opening

- Check `failure_threshold` configuration
- Verify failures are being recorded with `record_failure()`
- Check if enough requests meet `min_requests_for_rate`

### State Not Persisting

- Verify file permissions for persistence path
- Check Redis connectivity for distributed state
- Look for errors in logs during `save_state()`

### SLO Violations Not Triggering

- Ensure requests are being recorded with `record_request()`
- Check `target_success_rate` is reasonable
- Verify `window_duration` is appropriate for your use case

### Per-User Bulkhead Not Isolating

- Verify `user_id` is correctly extracted
- Check `max_concurrent_requests` configuration
- Ensure guards are properly dropped after request completion

---

## Metrics

All components export Prometheus metrics:

### Circuit Breaker Metrics
- `circuit_breaker_state` - Current state (0=Closed, 1=Open, 2=Half-Open)
- `circuit_breaker_requests_total` - Total requests
- `circuit_breaker_requests_successful` - Successful requests
- `circuit_breaker_requests_failed` - Failed requests
- `circuit_breaker_requests_rejected` - Rejected requests
- `circuit_breaker_state_transitions` - State transitions
- `circuit_breaker_request_duration_seconds` - Request duration

### Bulkhead Metrics
- `bulkhead_active_requests` - Active requests
- `bulkhead_queued_requests` - Queued requests
- `bulkhead_requests_total` - Total requests
- `bulkhead_requests_rejected` - Rejected requests
- `bulkhead_requests_timeout` - Timeout requests
- `bulkhead_queue_duration_seconds` - Time in queue

---

## License

Copyright © 2025 MockForge. All rights reserved.
