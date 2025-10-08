# MockForge OpenTelemetry Distributed Tracing

Complete guide to distributed tracing in MockForge using OpenTelemetry.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Exporters](#exporters)
- [Configuration](#configuration)
- [Trace Context Propagation](#trace-context-propagation)
- [Protocol-Specific Tracing](#protocol-specific-tracing)
- [Jaeger UI Guide](#jaeger-ui-guide)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

> **📚 Advanced Configuration**: See [OpenTelemetry Exporters Guide](./OPENTELEMETRY_EXPORTERS.md) for detailed exporter configuration, production deployment patterns, and cloud provider integration.

---

## Overview

MockForge implements OpenTelemetry distributed tracing to provide visibility into:

- **Request Flow**: Track requests across HTTP, gRPC, WebSocket, and GraphQL
- **Latency Analysis**: Identify slow operations and bottlenecks
- **Error Tracking**: Capture and analyze error spans with context
- **Service Dependencies**: Understand how different services interact
- **Context Propagation**: W3C Trace Context standard support

### Architecture

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │ traceparent: 00-trace-id-span-id-01
       ▼
┌─────────────┐
│  MockForge  │──▶ HTTP Span
│             │──▶ gRPC Span
│             │──▶ WebSocket Span
│             │──▶ GraphQL Span
└──────┬──────┘
       │ Trace data
       ▼
┌─────────────┐
│   Jaeger    │
│  Collector  │
└─────────────┘
```

---

## Quick Start

### 1. Enable Tracing

**Option A: Configuration File**

```yaml
# config.yaml
observability:
  opentelemetry:
    enabled: true
    service_name: "mockforge"
    environment: "development"
    jaeger_endpoint: "http://localhost:14268/api/traces"
    sampling_rate: 1.0
```

```bash
mockforge serve --config config.yaml
```

**Option B: CLI Flags**

```bash
mockforge serve \
  --tracing \
  --tracing-service-name "mockforge" \
  --tracing-environment "development" \
  --jaeger-endpoint "http://localhost:14268/api/traces" \
  --tracing-sampling-rate 1.0
```

### 2. Start Jaeger

```bash
cd examples/observability
docker-compose up -d jaeger
```

### 3. Generate Traces

```bash
# HTTP request
curl http://localhost:3000/api/users

# gRPC request (with grpcurl)
grpcurl -plaintext localhost:50051 user.UserService/GetUser

# WebSocket connection
wscat -c ws://localhost:3000/ws

# GraphQL query
curl -X POST http://localhost:3000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ hello }"}'
```

### 4. View Traces

Open Jaeger UI: http://localhost:16686

- Select service: `mockforge`
- Click "Find Traces"
- View trace details

---

## Exporters

MockForge supports multiple OpenTelemetry exporters:

### OTLP (Recommended for Production)
The OpenTelemetry Protocol (OTLP) exporter is vendor-neutral and works with:
- OpenTelemetry Collector
- Cloud providers (AWS X-Ray, Google Cloud Trace, Azure Monitor)
- Observability platforms (Grafana, DataDog, New Relic, Honeycomb)

```yaml
observability:
  opentelemetry:
    enabled: true
    exporter_type: otlp
    otlp_endpoint: "http://localhost:4317"
    service_name: "mockforge"
```

### Jaeger (Recommended for Development)
Direct export to Jaeger for local development and testing.

```yaml
observability:
  opentelemetry:
    enabled: true
    exporter_type: jaeger
    jaeger_endpoint: "http://localhost:14268/api/traces"
    service_name: "mockforge"
```

For detailed exporter configuration, see the [OpenTelemetry Exporters Guide](./OPENTELEMETRY_EXPORTERS.md).

---

## Configuration

### Full Configuration Options

```yaml
observability:
  opentelemetry:
    # Enable/disable tracing
    enabled: true

    # Service name (appears in Jaeger)
    service_name: "mockforge"

    # Deployment environment
    environment: "development"  # or "staging", "production"

    # Jaeger collector HTTP endpoint
    jaeger_endpoint: "http://localhost:14268/api/traces"

    # OTLP endpoint (alternative to Jaeger)
    otlp_endpoint: "http://localhost:4317"

    # Protocol for OTLP (grpc or http)
    protocol: "grpc"

    # Sampling rate (0.0 to 1.0)
    # 1.0 = 100% (all traces)
    # 0.1 = 10% (sample 1 in 10)
    sampling_rate: 1.0
```

### Environment-Specific Configurations

**Development**:
```yaml
opentelemetry:
  enabled: true
  service_name: "mockforge-dev"
  environment: "development"
  sampling_rate: 1.0  # Sample all requests
```

**Production**:
```yaml
opentelemetry:
  enabled: true
  service_name: "mockforge-prod"
  environment: "production"
  sampling_rate: 0.01  # Sample 1% of requests
```

### CLI Flags Reference

| Flag | Default | Description |
|------|---------|-------------|
| `--tracing` | false | Enable OpenTelemetry tracing |
| `--tracing-service-name` | "mockforge" | Service name for traces |
| `--tracing-environment` | "development" | Deployment environment |
| `--jaeger-endpoint` | "http://localhost:14268/api/traces" | Jaeger collector endpoint |
| `--tracing-sampling-rate` | 1.0 | Sampling rate (0.0 to 1.0) |

---

## Trace Context Propagation

MockForge supports W3C Trace Context standard for distributed tracing across services.

### W3C Trace Context Header

```
traceparent: 00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01
             |  |                                |                | |
             |  |                                |                | +-- Flags
             |  |                                |                +---- Span ID
             |  |                                +-------------------- Trace ID
             |  +---------------------------------------------------- Version
             +------------------------------------------------------- Format
```

### HTTP Context Propagation

**Incoming Request with Trace Context**:
```bash
curl -H "traceparent: 00-trace-id-span-id-01" \\
     http://localhost:3000/api/users
```

MockForge will:
1. Extract parent trace context from headers
2. Create child span for the request
3. Inject trace context into response headers

**Response Headers**:
```
traceparent: 00-{same-trace-id}-{new-span-id}-01
```

### gRPC Context Propagation

gRPC metadata automatically propagates trace context:

```go
// Client
md := metadata.Pairs("traceparent", "00-trace-id-span-id-01")
ctx := metadata.NewOutgoingContext(context.Background(), md)

// MockForge extracts and propagates automatically
```

### WebSocket Context Propagation

Trace context is extracted from WebSocket handshake headers:

```javascript
const ws = new WebSocket('ws://localhost:3000/ws', {
  headers: {
    'traceparent': '00-trace-id-span-id-01'
  }
});
```

### GraphQL Context Propagation

GraphQL requests propagate context via HTTP headers:

```bash
curl -X POST http://localhost:3000/graphql \\
  -H "traceparent: 00-trace-id-span-id-01" \\
  -H "Content-Type: application/json" \\
  -d '{"query": "{ users { id name } }"}'
```

---

## Protocol-Specific Tracing

### HTTP Tracing

**Span Attributes**:
- `http.method`: Request method (GET, POST, etc.)
- `http.route`: Matched route pattern
- `http.url`: Full URL path
- `http.status_code`: Response status code
- `http.duration_ms`: Request duration

**Example Trace**:
```
Span: GET /api/users/{id}
├─ Attribute: http.method = GET
├─ Attribute: http.route = /api/users/{id}
├─ Attribute: http.status_code = 200
└─ Attribute: http.duration_ms = 45
```

### gRPC Tracing

**Span Attributes**:
- `rpc.system`: grpc
- `rpc.service`: Service name (e.g., UserService)
- `rpc.method`: Method name (e.g., GetUser)
- `rpc.grpc.status_code`: Status code (0 = OK)
- `rpc.duration_ms`: Call duration
- `rpc.request.size`: Request size in bytes
- `rpc.response.size`: Response size in bytes

**Example Trace**:
```
Span: UserService::GetUser
├─ Attribute: rpc.system = grpc
├─ Attribute: rpc.service = UserService
├─ Attribute: rpc.method = GetUser
├─ Attribute: rpc.grpc.status_code = 0
├─ Attribute: rpc.duration_ms = 23
├─ Attribute: rpc.request.size = 128
└─ Attribute: rpc.response.size = 512
```

### WebSocket Tracing

**Connection Span Attributes**:
- `ws.path`: WebSocket path
- `network.protocol.name`: websocket
- `ws.duration_ms`: Connection duration
- `ws.messages.sent`: Total messages sent
- `ws.messages.received`: Total messages received

**Message Span Attributes**:
- `ws.direction`: inbound/outbound
- `ws.message.type`: text/binary
- `ws.message.size`: Message size in bytes
- `ws.processing_time_us`: Processing time in microseconds

**Example Trace**:
```
Span: WS Connect /ws/chat
├─ Span: WS Message inbound
│  ├─ Attribute: ws.direction = inbound
│  ├─ Attribute: ws.message.type = text
│  └─ Attribute: ws.message.size = 256
└─ Span: WS Message outbound
   ├─ Attribute: ws.direction = outbound
   └─ Attribute: ws.message.size = 512
```

### GraphQL Tracing

**Query Span Attributes**:
- `graphql.operation.type`: query/mutation/subscription
- `graphql.operation.name`: Operation name
- `graphql.document`: Full query document
- `graphql.duration_ms`: Execution duration
- `graphql.fields_resolved`: Number of fields resolved
- `graphql.resolver_calls`: Number of resolver calls

**Resolver Span Attributes**:
- `graphql.resolver.parent_type`: Parent type name
- `graphql.resolver.field_name`: Field being resolved
- `graphql.resolver.duration_us`: Resolver duration

**Example Trace**:
```
Span: GraphQL query GetUser
├─ Attribute: graphql.operation.type = query
├─ Attribute: graphql.operation.name = GetUser
├─ Span: Resolve User.id
├─ Span: Resolve User.name
└─ Span: Resolve User.email
```

---

## Jaeger UI Guide

### Service View

1. **Select Service**: Choose "mockforge" from dropdown
2. **Operation**: Select specific operation or "All"
3. **Lookback**: Choose time range (Last hour, Last day, etc.)
4. **Tags**: Filter by attributes (e.g., `http.status_code=500`)

### Trace Timeline

**Understanding the Timeline**:
```
Service: mockforge
├─ GET /api/users [150ms] ◀──── Total span duration
   ├─ Resolve users [45ms]
   ├─ DB Query [80ms]
   └─ Serialize response [25ms]
```

### Span Details

Click on a span to view:
- **Operation Name**: Span name
- **Duration**: How long the operation took
- **Tags**: All span attributes
- **Logs**: Event logs within the span
- **Process**: Service information

### Error Traces

Filter for errors:
```
Tags: error=true
Tags: http.status_code>=500
```

Red spans indicate errors in the timeline.

### Performance Analysis

**Find Slow Operations**:
1. Go to "Search" tab
2. Set Min Duration (e.g., "> 1s")
3. Click "Find Traces"
4. Sort by duration

**Compare Traces**:
- Select multiple traces
- Click "Compare"
- View side-by-side comparison

---

## Best Practices

### 1. Sampling Strategy

**Development**:
```yaml
sampling_rate: 1.0  # Sample everything
```

**Production**:
```yaml
sampling_rate: 0.01  # Sample 1% (adjust based on volume)
```

**High-Value Sampling**:
- Always sample errors (status >= 400)
- Always sample slow requests (> 1s)
- Sample normal requests based on volume

### 2. Span Naming

Good span names are:
- **Descriptive**: `GET /api/users/{id}` not `request`
- **Consistent**: Use same format across operations
- **Hierarchical**: Show parent-child relationships

### 3. Attribute Guidelines

Add attributes for:
- **Request identification**: user_id, session_id
- **Business context**: order_id, product_id
- **Technical context**: db_query, cache_hit

Avoid:
- **Sensitive data**: passwords, tokens, PII
- **High cardinality**: Don't use unique IDs as attribute keys
- **Large payloads**: Trim request/response bodies

### 4. Performance Considerations

- **Overhead**: ~100-500μs per span
- **Batch Export**: Spans are batched before export
- **Resource Limits**: Configure max queue size

### 5. Context Propagation

Always propagate trace context:
- Include `traceparent` header in HTTP requests
- Use gRPC metadata for RPC calls
- Propagate through message queues

---

## Troubleshooting

### Traces Not Appearing in Jaeger

**1. Check MockForge is sending traces**:
```bash
# Check logs for tracing initialization
mockforge serve --tracing | grep -i "tracing"
```

**2. Verify Jaeger is running**:
```bash
curl http://localhost:16686
docker ps | grep jaeger
```

**3. Test Jaeger collector**:
```bash
curl http://localhost:14268/api/traces
```

**4. Check network connectivity**:
```bash
# From MockForge to Jaeger
curl http://localhost:14268/api/traces -v
```

### Incomplete Traces

**Missing Child Spans**:
- Ensure context propagation is working
- Check `traceparent` header is being sent
- Verify sampling rate includes the operation

**Broken Trace Links**:
- Verify trace ID matches across spans
- Check for clock skew between services

### High Overhead

**Reduce Sampling**:
```yaml
sampling_rate: 0.1  # Sample 10% instead of 100%
```

**Batch Configuration**:
```yaml
# In tracer configuration
batch_timeout: 5s
max_batch_size: 512
```

### Jaeger UI Issues

**Traces Not Searchable**:
- Wait for Jaeger to index (usually < 1 minute)
- Check Jaeger storage backend health

**Missing Tags**:
- Verify attributes are set before span ends
- Check attribute key format (no special characters)

---

## Advanced Topics

### Custom Spans

For plugin development:

```rust
use mockforge_tracing::{create_request_span, record_success, Protocol};
use opentelemetry::KeyValue;

let mut span = create_request_span(
    Protocol::Http,
    "Custom Operation",
    vec![
        KeyValue::new("custom.attribute", "value"),
    ],
);

// ... perform operation ...

record_success(&mut span, vec![
    KeyValue::new("result.count", 42),
]);
```

### Span Events

Add events to track important moments:

```rust
span.add_event("cache_miss", vec![
    KeyValue::new("cache.key", key.to_string()),
]);
```

### Baggage Propagation

Use baggage for cross-process metadata:

```rust
use opentelemetry::baggage::BaggageExt;

let cx = Context::current()
    .with_baggage(vec![KeyValue::new("user_id", "12345")]);
```

---

## Reference

- [OpenTelemetry Specification](https://opentelemetry.io/docs/specs/otel/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [Jaeger Query API](https://www.jaegertracing.io/docs/latest/apis/#query-api)

---

## Support

For questions and issues:
- GitHub: https://github.com/SaaSy-Solutions/mockforge/issues
- Documentation: https://mockforge.dev/docs
