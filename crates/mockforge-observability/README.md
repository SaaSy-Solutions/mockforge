# MockForge Observability

Comprehensive observability features for MockForge including Prometheus metrics, OpenTelemetry tracing, structured logging, and system monitoring.

This crate provides enterprise-grade observability capabilities to monitor MockForge performance, track system health, and debug issues in production environments. Perfect for understanding how your mock servers behave under load and ensuring reliable testing infrastructure.

## Features

- **Prometheus Metrics**: Comprehensive metrics collection with automatic export
- **OpenTelemetry Tracing**: Distributed tracing with Jaeger and OTLP support
- **Structured Logging**: JSON-formatted logs with configurable levels and outputs
- **System Metrics**: CPU, memory, and thread monitoring
- **Flight Recorder**: Request/response recording for debugging
- **Multi-Protocol Support**: Metrics for HTTP, gRPC, WebSocket, and GraphQL
- **Performance Monitoring**: Response times, throughput, and error rates
- **Health Checks**: Built-in health endpoints and status monitoring

## Quick Start

### Basic Metrics Collection

```rust,no_run
use mockforge_observability::prometheus::MetricsRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize global metrics registry
    let registry = MetricsRegistry::new();

    // Record HTTP request metrics
    registry.record_http_request("GET", "/api/users", 200, 0.045);

    // Record gRPC call metrics
    registry.record_grpc_request("GetUser", 0, 0.032);

    // Export metrics in Prometheus format
    let metrics = registry.export_prometheus().await?;
    println!("{}", metrics);

    Ok(())
}
```

### Structured Logging

```rust,no_run
use mockforge_observability::{init_logging, LoggingConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    let logging_config = LoggingConfig {
        level: "info".to_string(),
        json_format: true,
        file_path: Some("./logs/mockforge.log".to_string()),
        max_file_size_mb: 10,
        max_files: 5,
    };

    init_logging(logging_config)?;

    // Logs will now be structured JSON
    tracing::info!("MockForge server started");
    tracing::error!("Failed to connect to database");

    Ok(())
}
```

### OpenTelemetry Tracing

```rust,no_run
use mockforge_observability::{init_with_otel, OtelTracingConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize OpenTelemetry tracing
    let tracing_config = OtelTracingConfig {
        service_name: "mockforge-server".to_string(),
        jaeger_endpoint: Some("http://localhost:14268/api/traces".to_string()),
        sampling_rate: 1.0,
    };

    init_with_otel(tracing_config).await?;

    // Create spans for request tracing
    let span = tracing::info_span!("http_request", method = "GET", path = "/api/users");
    let _enter = span.enter();

    // Your request handling code here...

    Ok(())
}
```

## Core Components

### Prometheus Metrics

Comprehensive metrics collection with automatic Prometheus export:

```rust,no_run
use mockforge_observability::prometheus::MetricsRegistry;

let registry = MetricsRegistry::new();

// HTTP metrics
registry.record_http_request("GET", "/api/users", 200, 0.045);
registry.record_http_response_size(2048); // bytes

// gRPC metrics
registry.record_grpc_request("GetUser", 0, 0.032); // method, status, duration

// WebSocket metrics
registry.record_websocket_connection();
registry.record_websocket_message(512); // message size

// GraphQL metrics
registry.record_graphql_request("GetUser", true, 0.028); // operation, success, duration

// Connection metrics
registry.record_active_connection();
registry.record_connection_closed();
```

### Available Metrics

#### HTTP Metrics
- `mockforge_http_requests_total{method, path, status}` - Total HTTP requests
- `mockforge_http_request_duration_seconds{method, path}` - Request duration histogram
- `mockforge_http_response_size_bytes` - Response size distribution
- `mockforge_http_active_connections` - Current active connections

#### gRPC Metrics
- `mockforge_grpc_requests_total{method, status}` - Total gRPC requests
- `mockforge_grpc_request_duration_seconds{method}` - gRPC request duration
- `mockforge_grpc_active_streams` - Active gRPC streams

#### WebSocket Metrics
- `mockforge_websocket_connections_total` - Total WebSocket connections
- `mockforge_websocket_active_connections` - Current active WebSocket connections
- `mockforge_websocket_messages_total{direction}` - WebSocket messages sent/received
- `mockforge_websocket_message_size_bytes` - WebSocket message size distribution

#### GraphQL Metrics
- `mockforge_graphql_requests_total{operation, success}` - Total GraphQL requests
- `mockforge_graphql_request_duration_seconds{operation}` - GraphQL request duration
- `mockforge_graphql_errors_total{type}` - GraphQL error count

#### System Metrics
- `mockforge_system_cpu_usage_percent` - CPU usage percentage
- `mockforge_system_memory_usage_bytes` - Memory usage in bytes
- `mockforge_system_threads_total` - Total thread count

### Structured Logging

JSON-formatted logging with configurable outputs:

```rust,no_run
use mockforge_observability::LoggingConfig;

let config = LoggingConfig {
    level: "debug".to_string(),        // error, warn, info, debug, trace
    json_format: true,                 // JSON or human-readable
    file_path: Some("./logs/app.log".to_string()),
    max_file_size_mb: 10,
    max_files: 5,                      // Log rotation
};

// Initialize logging
init_logging(config)?;

// Structured logs with context
tracing::info!(
    method = "GET",
    path = "/api/users",
    status = 200,
    duration_ms = 45,
    "HTTP request completed"
);
```

### OpenTelemetry Tracing

Distributed tracing with multiple backends:

```rust,no_run
use mockforge_observability::OtelTracingConfig;

// Jaeger tracing
let jaeger_config = OtelTracingConfig {
    service_name: "mockforge-api".to_string(),
    jaeger_endpoint: Some("http://localhost:14268/api/traces".to_string()),
    sampling_rate: 0.1, // 10% sampling
};

// OTLP tracing (generic OpenTelemetry protocol)
let otlp_config = OtelTracingConfig {
    service_name: "mockforge-api".to_string(),
    otlp_endpoint: Some("http://otel-collector:4317".to_string()),
    sampling_rate: 1.0,
};

init_with_otel(jaeger_config).await?;
```

### System Metrics Collection

Automatic system resource monitoring:

```rust,no_run
use mockforge_observability::system_metrics::{start_system_metrics_collector, SystemMetricsConfig};

let config = SystemMetricsConfig {
    collection_interval_seconds: 30,  // Collect every 30 seconds
    enabled: true,
};

start_system_metrics_collector(config).await?;
```

## Configuration

### Logging Configuration

```rust,no_run
use mockforge_observability::LoggingConfig;

let logging_config = LoggingConfig {
    level: "info".to_string(),
    json_format: true,
    file_path: Some("/var/log/mockforge.log".to_string()),
    max_file_size_mb: 100,
    max_files: 10,
};
```

### Tracing Configuration

```rust,no_run
use mockforge_observability::OtelTracingConfig;

let tracing_config = OtelTracingConfig {
    service_name: "mockforge-server".to_string(),
    environment: "production".to_string(),
    jaeger_endpoint: Some("http://jaeger:14268/api/traces".to_string()),
    otlp_endpoint: None,
    sampling_rate: 0.5, // 50% sampling
};
```

### Metrics Configuration

Metrics are automatically configured with sensible defaults. Customize via environment variables:

```bash
# Metrics collection
export MOCKFORGE_METRICS_ENABLED=true
export MOCKFORGE_METRICS_PATH=/metrics

# System metrics
export MOCKFORGE_SYSTEM_METRICS_ENABLED=true
export MOCKFORGE_SYSTEM_METRICS_INTERVAL=30
```

## Integration Examples

### HTTP Server with Full Observability

```rust,no_run
use axum::{routing::get, Router, extract::State};
use mockforge_observability::{
    prometheus::MetricsRegistry,
    init_logging,
    init_with_otel,
    LoggingConfig,
    OtelTracingConfig,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize observability
    init_logging(LoggingConfig {
        level: "info".to_string(),
        json_format: true,
        ..Default::default()
    })?;

    init_with_otel(OtelTracingConfig {
        service_name: "mockforge-http".to_string(),
        jaeger_endpoint: Some("http://localhost:14268/api/traces".to_string()),
        sampling_rate: 1.0,
    }).await?;

    // Create metrics registry
    let metrics = Arc::new(MetricsRegistry::new());

    // Build application with metrics middleware
    let app = Router::new()
        .route("/api/users", get(get_users))
        .route("/metrics", get(metrics_endpoint))
        .with_state(metrics);

    // Start server
    let addr = "0.0.0.0:3000".parse()?;
    println!("🚀 Server with full observability running at http://{}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}

async fn get_users(State(metrics): State<Arc<MetricsRegistry>>) -> &'static str {
    let start = std::time::Instant::now();

    // Your business logic here...
    let response = "{\"users\": [{\"id\": 1, \"name\": \"Alice\"}]}";

    // Record metrics
    let duration = start.elapsed().as_secs_f64();
    metrics.record_http_request("GET", "/api/users", 200, duration);

    response
}

async fn metrics_endpoint(State(metrics): State<Arc<MetricsRegistry>>) -> String {
    metrics.export_prometheus().await.unwrap_or_default()
}
```

### gRPC Server with Tracing

```rust,no_run
use mockforge_observability::{init_with_otel, OtelTracingConfig};
use tonic::{transport::Server, Request, Response, Status};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    init_with_otel(OtelTracingConfig {
        service_name: "mockforge-grpc".to_string(),
        jaeger_endpoint: Some("http://localhost:14268/api/traces".to_string()),
        sampling_rate: 1.0,
    }).await?;

    // Create gRPC server with tracing
    let addr = "0.0.0.0:50051".parse()?;
    let user_service = UserService::default();

    println!("🚀 gRPC server with tracing running at http://{}", addr);

    Server::builder()
        .add_service(UserServiceServer::new(user_service))
        .serve(addr)
        .await?;

    Ok(())
}
```

## Performance Considerations

- **Metrics Overhead**: Minimal performance impact with efficient metric collection
- **Logging Performance**: JSON formatting adds small overhead, file I/O can be async
- **Tracing Sampling**: Use sampling rates to control tracing volume in production
- **System Metrics**: Collection interval can be adjusted based on monitoring needs
- **Memory Usage**: Metrics registries use bounded memory with cleanup mechanisms

## Troubleshooting

### Common Issues

**Metrics not appearing:**
- Check Prometheus scrape configuration
- Verify metrics endpoint is accessible
- Ensure metrics are being recorded before scraping

**Logs not structured:**
- Verify JSON format is enabled in LoggingConfig
- Check log level settings
- Ensure tracing subscriber is properly initialized

**Tracing not working:**
- Verify Jaeger/OTLP endpoint is accessible
- Check service name configuration
- Ensure sampling rate allows traces through

**High memory usage:**
- Adjust log file rotation settings
- Reduce system metrics collection frequency
- Check for metric registry leaks

## Development

### Testing Observability Features

```rust,no_run
#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_observability::prometheus::MetricsRegistry;

    #[tokio::test]
    async fn test_metrics_collection() {
        let registry = MetricsRegistry::new();

        // Record some metrics
        registry.record_http_request("GET", "/test", 200, 0.1);
        registry.record_http_request("POST", "/test", 201, 0.05);

        // Export and verify
        let metrics = registry.export_prometheus().await.unwrap();
        assert!(metrics.contains("mockforge_http_requests_total"));
        assert!(metrics.contains("mockforge_http_request_duration_seconds"));
    }
}
```

### Custom Metrics

```rust,no_run
use prometheus::{register_counter, register_histogram, Counter, Histogram};

// Register custom metrics
lazy_static::lazy_static! {
    static ref CUSTOM_COUNTER: Counter = register_counter!(
        "mockforge_custom_operations_total",
        "Total number of custom operations"
    ).unwrap();

    static ref CUSTOM_HISTOGRAM: Histogram = register_histogram!(
        "mockforge_custom_operation_duration_seconds",
        "Duration of custom operations"
    ).unwrap();
}

// Use custom metrics
CUSTOM_COUNTER.inc();
let _timer = CUSTOM_HISTOGRAM.start_timer(); // Measures until dropped
```

## Examples

See the [examples directory](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples) for complete working examples including:

- Full observability stack setup
- Custom metrics implementation
- Distributed tracing configuration
- Log aggregation patterns
- Performance monitoring dashboards

## Related Crates

- [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
- [`prometheus`](https://docs.rs/prometheus): Metrics collection library
- [`tracing`](https://docs.rs/tracing): Logging and tracing framework
- [`opentelemetry`](https://docs.rs/opentelemetry): Observability standards

## License

Licensed under MIT OR Apache-2.0
