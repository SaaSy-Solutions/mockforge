# OpenTelemetry Exporters Guide

MockForge provides comprehensive OpenTelemetry integration with support for multiple exporter backends. This allows you to send distributed traces to your preferred observability platform.

## Supported Exporters

### 1. OTLP (OpenTelemetry Protocol)
The OTLP exporter is the modern, vendor-neutral way to export traces. It works with:
- OpenTelemetry Collector
- Cloud providers (AWS X-Ray, Google Cloud Trace, Azure Monitor)
- Observability platforms (Grafana, DataDog, New Relic, Honeycomb, etc.)

### 2. Jaeger
Direct export to Jaeger for local development and testing.

## Configuration

### OTLP Exporter (Recommended)

```rust
use mockforge_tracing::{TracingConfig, init_tracer, shutdown_tracer};

// Create OTLP configuration
let config = TracingConfig::with_otlp(
    "mockforge".to_string(),
    "http://localhost:4317".to_string(), // OpenTelemetry Collector endpoint
)
.with_sampling_rate(1.0)
.with_environment("production".to_string())
.with_service_version("1.0.0".to_string());

// Initialize the tracer
let tracer = init_tracer(config).expect("Failed to initialize tracer");

// ... your application code ...

// Shutdown when done
shutdown_tracer();
```

### Jaeger Exporter

```rust
use mockforge_tracing::{TracingConfig, init_tracer, shutdown_tracer};

// Create Jaeger configuration
let config = TracingConfig::with_jaeger(
    "mockforge".to_string(),
    "http://localhost:14268/api/traces".to_string(),
)
.with_sampling_rate(1.0)
.with_environment("development".to_string());

// Initialize the tracer
let tracer = init_tracer(config).expect("Failed to initialize tracer");

// ... your application code ...

// Shutdown when done
shutdown_tracer();
```

## Deployment Patterns

### OpenTelemetry Collector (Production)

For production deployments, we recommend using the OpenTelemetry Collector as a central aggregation point:

```yaml
# docker-compose.yml
services:
  otel-collector:
    image: otel/opentelemetry-collector-contrib:latest
    command: ["--config=/etc/otel-collector-config.yml"]
    volumes:
      - ./otel-collector-config.yml:/etc/otel-collector-config.yml
    ports:
      - "4317:4317"  # OTLP gRPC receiver
      - "4318:4318"  # OTLP HTTP receiver
      - "8888:8888"  # Prometheus metrics
      - "13133:13133" # Health check

  mockforge:
    image: mockforge:latest
    environment:
      - OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
      - OTEL_SERVICE_NAME=mockforge
      - OTEL_SERVICE_VERSION=1.0.0
```

Example OpenTelemetry Collector configuration:

```yaml
# otel-collector-config.yml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    timeout: 10s
    send_batch_size: 1024

  memory_limiter:
    check_interval: 1s
    limit_mib: 512

exporters:
  # Export to Jaeger
  jaeger:
    endpoint: jaeger:14250
    tls:
      insecure: true

  # Export to Prometheus
  prometheus:
    endpoint: "0.0.0.0:8889"

  # Export to any OTLP-compatible backend
  otlp:
    endpoint: your-backend:4317
    tls:
      insecure: false

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [memory_limiter, batch]
      exporters: [jaeger, otlp]

    metrics:
      receivers: [otlp]
      processors: [memory_limiter, batch]
      exporters: [prometheus]
```

### Local Development with Jaeger

For local development, you can run Jaeger directly:

```bash
docker run -d --name jaeger \
  -p 16686:16686 \
  -p 14268:14268 \
  jaegertracing/all-in-one:latest
```

Then configure MockForge to use Jaeger:

```rust
let config = TracingConfig::with_jaeger(
    "mockforge".to_string(),
    "http://localhost:14268/api/traces".to_string(),
);
```

Access Jaeger UI at http://localhost:16686

### Cloud Provider Integration

#### AWS X-Ray via OTLP

```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317

exporters:
  awsxray:
    region: us-west-2

service:
  pipelines:
    traces:
      receivers: [otlp]
      exporters: [awsxray]
```

#### Google Cloud Trace

```yaml
exporters:
  googlecloud:
    project: your-gcp-project

service:
  pipelines:
    traces:
      receivers: [otlp]
      exporters: [googlecloud]
```

#### Grafana Cloud

```yaml
exporters:
  otlp:
    endpoint: tempo-us-central1.grafana.net:443
    headers:
      authorization: Basic <base64-encoded-credentials>

service:
  pipelines:
    traces:
      receivers: [otlp]
      exporters: [otlp]
```

## Advanced Configuration

### Custom Resource Attributes

Resource attributes help identify and filter traces:

```rust
use mockforge_tracing::{TracingConfig, ExporterType};

let config = TracingConfig::with_otlp(
    "mockforge".to_string(),
    "http://localhost:4317".to_string(),
)
.with_service_version("1.2.3".to_string())
.with_environment("production".to_string())
.with_sampling_rate(0.1); // Sample 10% of traces
```

### Sampling Strategies

Configure sampling to control trace volume:

- **Always sample** (development): `with_sampling_rate(1.0)`
- **Production** (10%): `with_sampling_rate(0.1)`
- **High-traffic** (1%): `with_sampling_rate(0.01)`

### OTLP Configuration Options

The OTLP exporter supports:
- **Protocol**: gRPC (port 4317) or HTTP/Protobuf (port 4318)
- **Authentication**: Custom headers for API keys or tokens
- **Compression**: gzip compression for reduced bandwidth
- **Timeouts**: Configurable export timeouts

Example with all options:

```rust
use mockforge_tracing::{OtlpExporter, OtlpProtocol, OtlpCompression};

let exporter = OtlpExporter::new("https://api.honeycomb.io:443".to_string())
    .with_protocol(OtlpProtocol::Grpc)
    .with_header("x-honeycomb-team".to_string(), "your-api-key".to_string())
    .with_timeout(Duration::from_secs(30))
    .with_compression(OtlpCompression::Gzip);
```

## Troubleshooting

### Traces not appearing

1. **Check exporter connectivity**:
   ```bash
   # Test OTLP endpoint
   telnet localhost 4317

   # Test Jaeger endpoint
   curl http://localhost:14268/api/traces
   ```

2. **Verify sampling rate**: Ensure sampling_rate > 0.0

3. **Check logs**: Enable debug logging to see trace exports:
   ```bash
   RUST_LOG=mockforge_tracing=debug cargo run
   ```

### Performance considerations

1. **Use batching**: The OTLP exporter batches spans by default
2. **Adjust sampling**: Lower sampling rates in high-traffic scenarios
3. **Use OpenTelemetry Collector**: Offload export processing from your application

## Integration with MockForge

MockForge automatically instruments:
- HTTP requests (method, path, status, duration)
- gRPC calls (service, method, status)
- WebSocket connections (events, messages)
- GraphQL operations (queries, mutations, subscriptions)

All protocols support:
- W3C Trace Context propagation
- Parent-child span relationships
- Error recording with stack traces
- Custom attributes and tags

## Examples

### Complete HTTP Server with OTLP

```rust
use mockforge_tracing::{TracingConfig, init_tracer, shutdown_tracer};
use mockforge_http::http_tracing_middleware;
use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    // Initialize OTLP tracing
    let config = TracingConfig::with_otlp(
        "mockforge-http".to_string(),
        "http://localhost:4317".to_string(),
    );
    let _tracer = init_tracer(config).expect("Failed to initialize tracer");

    // Create HTTP server with tracing middleware
    let app = Router::new()
        .route("/api/users", get(get_users))
        .layer(middleware::from_fn(http_tracing_middleware));

    // Run server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();

    // Cleanup
    shutdown_tracer();
}

async fn get_users() -> &'static str {
    "Users list"
}
```

### Multi-Service Trace Propagation

When MockForge calls downstream services, traces are automatically propagated:

```rust
// Service A (MockForge)
async fn call_service_b(trace_context: TraceContext) {
    let client = reqwest::Client::new();

    // Trace context is automatically injected into headers
    let response = client
        .get("http://service-b:8080/api/data")
        .header("traceparent", trace_context.to_traceparent())
        .send()
        .await;
}
```

## Metrics and Logs Correlation

Correlate traces with metrics and logs using trace IDs:

```rust
use tracing::{info, span};

// Logs automatically include trace context
let span = span!(tracing::Level::INFO, "process_request");
let _enter = span.enter();

info!("Processing user request"); // Includes trace_id and span_id
```

## Best Practices

1. **Use OTLP for production**: More flexible and vendor-neutral
2. **Deploy OpenTelemetry Collector**: Centralize configuration and routing
3. **Set appropriate sampling**: Balance observability with cost
4. **Add custom attributes**: Include business-relevant metadata
5. **Monitor exporter health**: Track export failures and latency
6. **Use semantic conventions**: Follow OpenTelemetry naming standards

## Resources

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [OTLP Specification](https://opentelemetry.io/docs/specs/otlp/)
- [OpenTelemetry Collector](https://opentelemetry.io/docs/collector/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
