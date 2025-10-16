# MockForge Chaos

Chaos engineering features for MockForge - fault injection, resilience testing, and advanced orchestration capabilities.

This crate provides comprehensive chaos engineering tools including traffic shaping, fault injection, circuit breakers, bulkheads, and intelligent scenario orchestration. It's designed for testing system resilience, implementing chaos engineering practices, and ensuring application reliability under adverse conditions.

## Features

- **Fault Injection**: Simulate network failures, service outages, and system errors
- **Traffic Shaping**: Control bandwidth, latency, and packet loss
- **Circuit Breakers**: Automatic failure detection and recovery
- **Bulkheads**: Resource isolation and failure containment
- **Scenario Orchestration**: Complex chaos experiment orchestration
- **A/B Testing**: Statistical comparison of system behaviors
- **Machine Learning**: Anomaly detection and predictive remediation
- **Multi-Cluster**: Distributed chaos testing across clusters
- **Observability**: Comprehensive metrics and monitoring
- **GitOps Integration**: Version control for chaos experiments

## Quick Start

### Basic Fault Injection

```rust,no_run
use mockforge_chaos::{FaultInjector, FaultType, ChaosConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create fault injector
    let config = ChaosConfig {
        fault_injection: Some(FaultInjectionConfig {
            global_error_rate: 0.1, // 10% error rate
            default_status_codes: vec![500, 502, 503],
            ..Default::default()
        }),
        ..Default::default()
    };

    let injector = FaultInjector::new(config);

    // Inject faults into your service
    let result = injector.inject_fault().await?;
    match result {
        FaultType::HttpError { status_code, .. } => {
            println!("Injected HTTP error: {}", status_code);
        }
        FaultType::NetworkTimeout { .. } => {
            println!("Injected network timeout");
        }
        _ => println!("Other fault injected"),
    }

    Ok(())
}
```

### Circuit Breaker Pattern

```rust,no_run
use mockforge_chaos::resilience::{CircuitBreaker, CircuitBreakerManager};

let circuit_breaker = CircuitBreaker::new(
    "api_service",
    5,  // failure threshold
    std::time::Duration::from_secs(60), // recovery timeout
);

// Use in your service calls
match circuit_breaker.call(|| async {
    // Your service call here
    reqwest::get("http://api.example.com").await
}).await {
    Ok(response) => println!("Success: {:?}", response),
    Err(e) => println!("Circuit breaker error: {:?}", e),
}
```

### Traffic Shaping

```rust,no_run
use mockforge_chaos::{TrafficShaper, TrafficShapingConfig};

let config = TrafficShapingConfig {
    bandwidth_limit: Some(1024 * 1024), // 1MB/s
    latency_ms: Some(100),              // 100ms delay
    packet_loss_rate: Some(0.05),       // 5% packet loss
    ..Default::default()
};

let shaper = TrafficShaper::new(config);

// Apply traffic shaping to network interface
shaper.apply_to_interface("eth0").await?;
```

## Core Components

### Fault Injection

Simulate various types of failures:

```rust,no_run
use mockforge_chaos::fault::{FaultInjector, FaultType};

let injector = FaultInjector::new(ChaosConfig::default());

// Different fault types
let faults = vec![
    FaultType::HttpError { status_code: 500, message: "Internal Server Error".to_string() },
    FaultType::NetworkTimeout { duration_ms: 5000 },
    FaultType::ConnectionReset,
    FaultType::DataCorruption { corruption_rate: 0.1 },
];

// Inject random faults
for _ in 0..10 {
    if let Some(fault) = injector.should_inject_fault() {
        println!("Injecting fault: {:?}", fault);
        injector.inject_fault_type(fault).await?;
    }
}
```

### Resilience Patterns

#### Circuit Breaker

```rust,no_run
use mockforge_chaos::resilience::CircuitBreaker;

let cb = CircuitBreaker::new("service", 3, std::time::Duration::from_secs(30));

let result = cb.call(|| async {
    // Potentially failing operation
    fallible_operation().await
}).await;

match result {
    Ok(value) => println!("Success: {}", value),
    Err(CircuitBreakerError::Open) => println!("Circuit breaker is open"),
    Err(CircuitBreakerError::Rejected) => println!("Request rejected"),
}
```

#### Bulkhead

```rust,no_run
use mockforge_chaos::resilience::{Bulkhead, BulkheadConfig};

let config = BulkheadConfig {
    max_concurrent_calls: 10,
    max_wait_duration: std::time::Duration::from_secs(5),
};

let bulkhead = Bulkhead::new("resource_pool", config);

let result = bulkhead.execute(|| async {
    // Resource-intensive operation
    heavy_computation().await
}).await;

match result {
    Ok(value) => println!("Completed: {}", value),
    Err(BulkheadError::Rejected) => println!("Bulkhead rejected request"),
}
```

### Scenario Orchestration

Create complex chaos experiments:

```rust,no_run
use mockforge_chaos::scenario_orchestrator::{ScenarioOrchestrator, OrchestratedScenario, ScenarioStep};

let orchestrator = ScenarioOrchestrator::new();

let scenario = OrchestratedScenario {
    name: "api_failure_simulation".to_string(),
    steps: vec![
        ScenarioStep::Delay { duration_ms: 1000 },
        ScenarioStep::InjectFault { fault_type: FaultType::HttpError { status_code: 503, message: "Service Unavailable".to_string() } },
        ScenarioStep::Delay { duration_ms: 2000 },
        ScenarioStep::InjectFault { fault_type: FaultType::NetworkTimeout { duration_ms: 10000 } },
        ScenarioStep::Assert { condition: "response_time > 5000".to_string() },
    ],
};

orchestrator.execute_scenario(scenario).await?;
```

### A/B Testing

Statistical comparison of system behaviors:

```rust,no_run
use mockforge_chaos::ab_testing::{ABTestingEngine, ABTestConfig, TestVariant};

let engine = ABTestingEngine::new();

let config = ABTestConfig {
    name: "latency_optimization".to_string(),
    variants: vec![
        TestVariant {
            name: "baseline".to_string(),
            weight: 50,
            config: ChaosConfig::default(),
        },
        TestVariant {
            name: "optimized".to_string(),
            weight: 50,
            config: ChaosConfig {
                latency: Some(LatencyConfig {
                    fixed_delay_ms: Some(50),
                    ..Default::default()
                }),
                ..Default::default()
            },
        },
    ],
    duration_minutes: 60,
    success_criteria: vec![/* ... */],
};

let test_id = engine.start_test(config).await?;
println!("Started A/B test: {}", test_id);
```

### Machine Learning Features

#### Anomaly Detection

```rust,no_run
use mockforge_chaos::ml_anomaly_detector::{AnomalyDetector, AnomalyDetectorConfig};

let config = AnomalyDetectorConfig {
    sensitivity: 0.8,
    training_window_hours: 24,
    ..Default::default()
};

let detector = AnomalyDetector::new(config);

// Feed metrics data
detector.add_data_point("response_time", 150.0, chrono::Utc::now()).await?;

// Check for anomalies
if let Some(anomaly) = detector.detect_anomaly("response_time").await? {
    println!("Anomaly detected: {:?}", anomaly);
}
```

#### Predictive Remediation

```rust,no_run
use mockforge_chaos::predictive_remediation::PredictiveRemediationEngine;

let engine = PredictiveRemediationEngine::new();

// Analyze system metrics
let prediction = engine.predict_failure("api_service", 30).await?; // 30-minute prediction

if prediction.probability > 0.8 {
    println!("High failure probability detected!");
    // Trigger remediation
    engine.trigger_remediation(prediction).await?;
}
```

## Advanced Features

### Multi-Cluster Orchestration

```rust,no_run
use mockforge_chaos::multi_cluster::{MultiClusterOrchestrator, ClusterTarget};

let orchestrator = MultiClusterOrchestrator::new();

let targets = vec![
    ClusterTarget {
        name: "production-us-east".to_string(),
        endpoint: "https://k8s-us-east.example.com".to_string(),
        credentials: /* ... */,
    },
    ClusterTarget {
        name: "production-us-west".to_string(),
        endpoint: "https://k8s-us-west.example.com".to_string(),
        credentials: /* ... */,
    },
];

// Execute chaos scenario across clusters
orchestrator.execute_multi_cluster(scenario, targets).await?;
```

### GitOps Integration

```rust,no_run
use mockforge_chaos::gitops::{GitOpsManager, GitOpsConfig};

let config = GitOpsConfig {
    repository_url: "https://github.com/org/chaos-experiments".to_string(),
    branch: "main".to_string(),
    auth: GitOpsAuth::Token { token: "ghp_...".to_string() },
};

let gitops = GitOpsManager::new(config);

// Sync chaos configurations from Git
gitops.sync_from_git().await?;

// Push experiment results back to Git
gitops.push_results(experiment_results).await?;
```

### Observability API

```rust,no_run
use mockforge_chaos::observability_api::create_observability_router;
use axum::Router;

let app = Router::new()
    .merge(create_observability_router())
    .layer(ChaosMiddleware::new(ChaosConfig::default()));

// Routes available:
// GET /metrics - Prometheus metrics
// GET /health - Health check
// GET /chaos/status - Chaos experiment status
// POST /chaos/scenarios - Execute scenarios
```

## Configuration

### ChaosConfig

```rust,no_run
use mockforge_chaos::config::ChaosConfig;

let config = ChaosConfig {
    fault_injection: Some(FaultInjectionConfig {
        global_error_rate: 0.05,
        default_status_codes: vec![500, 502, 503],
        network_failure_rate: 0.01,
        timeout_rate: 0.02,
    }),
    latency: Some(LatencyConfig {
        fixed_delay_ms: Some(100),
        jitter_ms: Some(50),
        distribution: LatencyDistribution::Normal,
    }),
    traffic_shaping: Some(TrafficShapingConfig {
        bandwidth_limit: Some(1024 * 1024), // 1MB/s
        packet_loss_rate: Some(0.001),      // 0.1%
    }),
    rate_limiting: Some(RateLimitConfig {
        requests_per_second: 100,
        burst_size: 20,
    }),
    circuit_breaker: Some(CircuitBreakerConfig {
        failure_threshold: 5,
        recovery_timeout_secs: 60,
        success_threshold: 3,
    }),
    bulkhead: Some(BulkheadConfig {
        max_concurrent_calls: 10,
        max_wait_duration: std::time::Duration::from_secs(5),
    }),
};
```

## Integration Examples

### With HTTP Services

```rust,no_run
use mockforge_chaos::middleware::chaos_middleware;
use axum::Router;

let app = Router::new()
    .route("/api/*path", axum::routing::get(handler))
    .layer(chaos_middleware(ChaosConfig::default()));
```

### With gRPC Services

```rust,no_run
use mockforge_chaos::protocols::grpc::GrpcChaos;

let grpc_chaos = GrpcChaos::new(ChaosConfig::default());

// Apply chaos to gRPC calls
let result = grpc_chaos.intercept_call(request).await?;
```

### With WebSocket Connections

```rust,no_run
use mockforge_chaos::protocols::websocket::WebSocketChaos;

let ws_chaos = WebSocketChaos::new(ChaosConfig::default());

// Inject chaos into WebSocket messages
ws_chaos.intercept_message(&mut message).await?;
```

## Performance Considerations

- **Resource Usage**: Chaos features add overhead - monitor system resources
- **Distributed Mode**: Use Redis backend for multi-instance coordination
- **Metrics Collection**: Enable metrics for observability without impacting performance
- **Gradual Rollout**: Start with low fault injection rates and increase gradually

## Safety Features

- **Approval Workflows**: Require approval for destructive chaos experiments
- **Automatic Rollback**: Built-in remediation for failed experiments
- **Safety Checks**: Pre-flight validation before executing chaos
- **Rate Limiting**: Prevent overwhelming systems with too much chaos

## Examples

See the [examples directory](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples) for complete working examples including:

- Basic fault injection scenarios
- Circuit breaker implementations
- Multi-cluster chaos orchestration
- A/B testing setups
- Machine learning integration

## Related Crates

- [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
- [`mockforge-observability`](https://docs.rs/mockforge-observability): Metrics and monitoring

## License

Licensed under MIT OR Apache-2.0
