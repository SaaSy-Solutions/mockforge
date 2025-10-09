# Prometheus Metrics Guide

MockForge provides comprehensive Prometheus metrics that can be used for monitoring, alerting, and performance analysis.

## Table of Contents

- [Configuration](#configuration)
- [Available Metrics](#available-metrics)
  - [HTTP Metrics](#http-metrics)
  - [WebSocket Metrics](#websocket-metrics)
  - [gRPC Metrics](#grpc-metrics)
  - [GraphQL Metrics](#graphql-metrics)
  - [SMTP Metrics](#smtp-metrics)
  - [Plugin Metrics](#plugin-metrics)
  - [System Metrics](#system-metrics)
  - [Chaos Engineering Metrics](#chaos-engineering-metrics)
- [Example Queries](#example-prometheus-queries)
- [Grafana Dashboard](#grafana-dashboard)
- [Best Practices](#best-practices)

## Configuration

Enable Prometheus metrics in your `config.yaml`:

```yaml
observability:
  prometheus:
    enabled: true
    port: 9090
    host: "0.0.0.0"
    path: "/metrics"
```

The metrics endpoint will be available at `http://localhost:9090/metrics`.

## Available Metrics

### HTTP Metrics

#### `mockforge_requests_total`
**Type:** Counter
**Labels:** `protocol`, `method`, `status`
**Description:** Total number of HTTP requests processed by MockForge.

**Example:**
```
mockforge_requests_total{protocol="http",method="GET",status="200"} 1523
mockforge_requests_total{protocol="http",method="POST",status="201"} 342
mockforge_requests_total{protocol="http",method="GET",status="404"} 12
```

#### `mockforge_requests_by_path_total`
**Type:** Counter
**Labels:** `path`, `method`, `status`
**Description:** Total number of requests by specific path. Paths are normalized to prevent cardinality explosion (e.g., `/api/users/123` becomes `/api/users/:id`).

**Example:**
```
mockforge_requests_by_path_total{path="/api/users/:id",method="GET",status="200"} 856
mockforge_requests_by_path_total{path="/api/posts",method="POST",status="201"} 234
```

#### `mockforge_request_duration_seconds`
**Type:** Histogram
**Labels:** `protocol`, `method`
**Buckets:** 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0
**Description:** Request duration in seconds. Provides percentiles (p50, p95, p99) and average latency.

**Example Percentile Query (p95):**
```promql
histogram_quantile(0.95, rate(mockforge_request_duration_seconds_bucket[5m]))
```

#### `mockforge_request_duration_by_path_seconds`
**Type:** Histogram
**Labels:** `path`, `method`
**Buckets:** 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0
**Description:** Request duration by specific path in seconds.

**Example Query (p99 latency for /api/users/:id):**
```promql
histogram_quantile(0.99,
  rate(mockforge_request_duration_by_path_seconds_bucket{path="/api/users/:id"}[5m]))
```

#### `mockforge_average_latency_by_path_seconds`
**Type:** Gauge
**Labels:** `path`, `method`
**Description:** Exponentially weighted moving average of request latency by path.

**Example:**
```
mockforge_average_latency_by_path_seconds{path="/api/users/:id",method="GET"} 0.045
mockforge_average_latency_by_path_seconds{path="/api/posts",method="POST"} 0.123
```

#### `mockforge_requests_in_flight`
**Type:** Gauge
**Labels:** `protocol`
**Description:** Number of requests currently being processed.

**Example:**
```
mockforge_requests_in_flight{protocol="http"} 15
mockforge_requests_in_flight{protocol="grpc"} 3
```

#### `mockforge_errors_total`
**Type:** Counter
**Labels:** `protocol`, `error_type`
**Description:** Total number of errors by protocol and error type.

**Example:**
```
mockforge_errors_total{protocol="http",error_type="client_error"} 45
mockforge_errors_total{protocol="http",error_type="server_error"} 3
```

#### `mockforge_error_rate`
**Type:** Gauge
**Labels:** `protocol`
**Description:** Error rate by protocol (0.0 to 1.0).

### WebSocket Metrics

#### `mockforge_ws_connections_active`
**Type:** Gauge
**Description:** Number of active WebSocket connections.

**Example:**
```
mockforge_ws_connections_active 42
```

#### `mockforge_ws_connections_total`
**Type:** Counter
**Description:** Total number of WebSocket connections established.

**Example:**
```
mockforge_ws_connections_total 1523
```

#### `mockforge_ws_connection_duration_seconds`
**Type:** Histogram
**Labels:** `status`
**Buckets:** 1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0, 1800.0, 3600.0
**Description:** WebSocket connection duration in seconds.

**Example Query (Average connection duration):**
```promql
rate(mockforge_ws_connection_duration_seconds_sum[5m]) /
  rate(mockforge_ws_connection_duration_seconds_count[5m])
```

#### `mockforge_ws_messages_sent_total`
**Type:** Counter
**Description:** Total number of WebSocket messages sent.

#### `mockforge_ws_messages_received_total`
**Type:** Counter
**Description:** Total number of WebSocket messages received.

#### `mockforge_ws_errors_total`
**Type:** Counter
**Description:** Total number of WebSocket errors.

### gRPC Metrics

#### `mockforge_requests_total{protocol="grpc"}`
Uses the same `mockforge_requests_total` metric with `protocol="grpc"`.

**Labels:** `protocol`, `method`, `status`

### GraphQL Metrics

#### `mockforge_requests_total{protocol="graphql"}`
Uses the same `mockforge_requests_total` metric with `protocol="graphql"`.

**Labels:** `protocol`, `method` (operation type: query, mutation, subscription), `status`

### SMTP Metrics

#### `mockforge_smtp_connections_active`
**Type:** Gauge
**Description:** Number of active SMTP connections.

#### `mockforge_smtp_connections_total`
**Type:** Counter
**Description:** Total number of SMTP connections established.

#### `mockforge_smtp_messages_received_total`
**Type:** Counter
**Description:** Total number of SMTP messages received.

#### `mockforge_smtp_messages_stored_total`
**Type:** Counter
**Description:** Total number of SMTP messages stored in the mailbox.

#### `mockforge_smtp_errors_total`
**Type:** Counter
**Labels:** `error_type`
**Description:** Total number of SMTP errors by type.

### Plugin Metrics

#### `mockforge_plugin_executions_total`
**Type:** Counter
**Labels:** `plugin_name`, `status`
**Description:** Total number of plugin executions.

**Example:**
```
mockforge_plugin_executions_total{plugin_name="auth-plugin",status="success"} 1234
mockforge_plugin_executions_total{plugin_name="auth-plugin",status="failure"} 5
```

#### `mockforge_plugin_execution_duration_seconds`
**Type:** Histogram
**Labels:** `plugin_name`
**Buckets:** 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0
**Description:** Plugin execution duration in seconds.

#### `mockforge_plugin_errors_total`
**Type:** Counter
**Labels:** `plugin_name`, `error_type`
**Description:** Total number of plugin errors.

### System Metrics

#### `mockforge_memory_usage_bytes`
**Type:** Gauge
**Description:** Memory usage in bytes.

**Example Query (Memory in MB):**
```promql
mockforge_memory_usage_bytes / 1024 / 1024
```

#### `mockforge_cpu_usage_percent`
**Type:** Gauge
**Description:** CPU usage percentage (0-100).

#### `mockforge_thread_count`
**Type:** Gauge
**Description:** Number of active threads.

#### `mockforge_uptime_seconds`
**Type:** Gauge
**Description:** Server uptime in seconds.

**Example Query (Uptime in hours):**
```promql
mockforge_uptime_seconds / 3600
```

### Chaos Engineering Metrics

#### `mockforge_active_scenario_mode`
**Type:** Gauge
**Description:** Active chaos scenario mode.
- 0 = healthy
- 1 = degraded
- 2 = error
- 3 = chaos

#### `mockforge_chaos_triggers_total`
**Type:** Counter
**Description:** Total number of chaos mode triggers.

## Example Prometheus Queries

### Request Rate (requests per second)
```promql
rate(mockforge_requests_total[5m])
```

### Request Rate by Path
```promql
rate(mockforge_requests_by_path_total[5m])
```

### Error Rate Percentage
```promql
(rate(mockforge_errors_total[5m]) / rate(mockforge_requests_total[5m])) * 100
```

### 95th Percentile Latency
```promql
histogram_quantile(0.95,
  rate(mockforge_request_duration_seconds_bucket[5m]))
```

### 99th Percentile Latency by Path
```promql
histogram_quantile(0.99,
  sum(rate(mockforge_request_duration_by_path_seconds_bucket[5m])) by (path, le))
```

### Average Request Duration
```promql
rate(mockforge_request_duration_seconds_sum[5m]) /
  rate(mockforge_request_duration_seconds_count[5m])
```

### Top 5 Slowest Endpoints
```promql
topk(5, mockforge_average_latency_by_path_seconds)
```

### Memory Usage in MB
```promql
mockforge_memory_usage_bytes / 1024 / 1024
```

### Active Connections (All Protocols)
```promql
sum(mockforge_requests_in_flight)
```

### WebSocket Message Rate
```promql
rate(mockforge_ws_messages_sent_total[5m]) +
  rate(mockforge_ws_messages_received_total[5m])
```

### SMTP Message Rate
```promql
rate(mockforge_smtp_messages_received_total[5m])
```

## Grafana Dashboard

An example Grafana dashboard configuration is available at `examples/observability/grafana/dashboards/mockforge-overview.json`.

### Key Panels

1. **Request Rate** - Line graph showing requests/second over time
2. **Error Rate** - Percentage of failed requests
3. **Latency Percentiles** - p50, p95, p99 latency
4. **Top Endpoints** - Table showing most-used endpoints
5. **Active Connections** - Gauge showing current connection count
6. **System Resources** - CPU and memory usage
7. **WebSocket Activity** - Connection count and message rate
8. **SMTP Activity** - Message rate and storage

## Best Practices

### 1. Use Appropriate Aggregation Windows

For dashboards, use 5m or 15m windows:
```promql
rate(mockforge_requests_total[5m])
```

For alerts, use shorter windows (1m or 2m):
```promql
rate(mockforge_requests_total[1m])
```

### 2. Alert on Key Metrics

**High Error Rate:**
```promql
(rate(mockforge_errors_total[5m]) / rate(mockforge_requests_total[5m])) * 100 > 5
```

**High Latency (p99 > 1s):**
```promql
histogram_quantile(0.99, rate(mockforge_request_duration_seconds_bucket[5m])) > 1
```

**High Memory Usage (> 80%):**
```promql
mockforge_memory_usage_bytes / (total_system_memory) > 0.8
```

### 3. Path Normalization

MockForge automatically normalizes paths to prevent cardinality explosion:
- `/api/users/123` → `/api/users/:id`
- `/api/users/550e8400-e29b-41d4-a716-446655440000` → `/api/users/:id`
- `/api/posts/abc123def456` → `/api/posts/:id`

### 4. Cardinality Considerations

Be mindful of high-cardinality labels:
- ✅ Good: `path="/api/users/:id"`, `method="GET"`, `status="200"`
- ❌ Bad: Using actual user IDs or timestamps as labels

### 5. Recording Rules

For frequently-used queries, consider creating Prometheus recording rules:

```yaml
groups:
  - name: mockforge
    interval: 30s
    rules:
      - record: mockforge:request_rate:5m
        expr: rate(mockforge_requests_total[5m])

      - record: mockforge:error_rate:5m
        expr: rate(mockforge_errors_total[5m]) / rate(mockforge_requests_total[5m])

      - record: mockforge:latency_p95:5m
        expr: histogram_quantile(0.95, rate(mockforge_request_duration_seconds_bucket[5m]))
```

## Integration with CI/CD

### Docker Compose Setup

See `examples/observability/docker-compose.yml` for a complete setup with Prometheus and Grafana.

```bash
cd examples/observability
docker-compose up -d
```

Access:
- MockForge: http://localhost:3000
- Prometheus: http://localhost:9091
- Grafana: http://localhost:3001 (admin/admin)

### Kubernetes Setup

See `helm/mockforge/templates/servicemonitor.yaml` for Prometheus Operator integration.

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: mockforge-metrics
spec:
  selector:
    matchLabels:
      app: mockforge
  endpoints:
    - port: metrics
      interval: 30s
```

## Troubleshooting

### Metrics Not Appearing

1. Verify Prometheus is enabled:
```yaml
observability:
  prometheus:
    enabled: true
```

2. Check the metrics endpoint:
```bash
curl http://localhost:9090/metrics
```

3. Verify Prometheus can scrape MockForge:
- Check Prometheus targets: http://localhost:9091/targets
- Ensure firewall rules allow access to port 9090

### High Cardinality Issues

If you see too many unique metric combinations:
1. Check for dynamic path segments that aren't being normalized
2. Review custom labels added by plugins
3. Consider adjusting the path normalization logic in `mockforge-observability/src/prometheus/metrics.rs`

## Further Reading

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Dashboards](https://grafana.com/docs/grafana/latest/dashboards/)
- [MockForge Observability Guide](./OBSERVABILITY.md)
- [Advanced Monitoring](./ADVANCED_OBSERVABILITY.md)
