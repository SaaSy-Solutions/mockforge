# MockForge Observability Stack

Complete observability setup for MockForge with Prometheus metrics, Grafana dashboards, and Jaeger distributed tracing.

## Quick Start

### 1. Start MockForge with Metrics and Tracing

```bash
# Option 1: Using configuration file
mockforge serve --config ../config-with-tracing.yaml

# Option 2: Using CLI flags
mockforge serve \
  --metrics --metrics-port 9090 \
  --tracing --jaeger-endpoint "http://localhost:14268/api/traces"
```

### 2. Start Observability Stack

```bash
cd examples/observability
docker-compose up -d
```

This will start:
- **Prometheus** on http://localhost:9091 - Metrics collection
- **Grafana** on http://localhost:3050 - Dashboards (admin/admin)
- **Jaeger** on http://localhost:16686 - Distributed tracing UI

### 3. Access Dashboards

**Prometheus:**
- URL: http://localhost:9091
- Targets: http://localhost:9091/targets
- Query: http://localhost:9091/graph

**Grafana:**
- URL: http://localhost:3050
- Username: `admin`
- Password: `admin`

**Jaeger:**
- URL: http://localhost:16686
- Search traces by service, operation, or tags
- View trace timeline and span details

**MockForge Direct Endpoints:**
- Metrics: http://localhost:9090/metrics
- Health: http://localhost:9090/health

---

## Configuration

### MockForge Configuration

Edit `../config-with-tracing.yaml`:

```yaml
observability:
  # Prometheus metrics
  prometheus:
    enabled: true
    port: 9090
    host: "0.0.0.0"
    path: "/metrics"

  # OpenTelemetry distributed tracing
  opentelemetry:
    enabled: true
    service_name: "mockforge"
    environment: "development"
    jaeger_endpoint: "http://localhost:14268/api/traces"
    sampling_rate: 1.0  # 100% sampling
```

### Prometheus Configuration

Edit `prometheus.yml` to change scrape intervals or add targets.

---

## Available Metrics

### Request Metrics

```promql
# Total requests by protocol
mockforge_requests_total

# Request duration histogram
mockforge_request_duration_seconds

# In-flight requests
mockforge_requests_in_flight

# Error rate
rate(mockforge_errors_total[5m]) / rate(mockforge_requests_total[5m])
```

### Latency Metrics

```promql
# P50 latency by protocol
histogram_quantile(0.50, rate(mockforge_request_duration_seconds_bucket[5m]))

# P95 latency by protocol
histogram_quantile(0.95, rate(mockforge_request_duration_seconds_bucket[5m]))

# P99 latency by protocol
histogram_quantile(0.99, rate(mockforge_request_duration_seconds_bucket[5m]))
```

### Protocol-Specific Metrics

```promql
# HTTP requests per second
rate(mockforge_requests_total{protocol="http"}[1m])

# gRPC requests per second
rate(mockforge_requests_total{protocol="grpc"}[1m])

# WebSocket active connections
mockforge_ws_connections_active

# GraphQL requests per second
rate(mockforge_requests_total{protocol="graphql"}[1m])
```

### Plugin Metrics

```promql
# Plugin execution rate
rate(mockforge_plugin_executions_total[5m])

# Plugin execution duration
mockforge_plugin_execution_duration_seconds

# Plugin error rate
rate(mockforge_plugin_errors_total[5m])
```

---

## Example Queries

### Request Rate by Protocol

```promql
sum by (protocol) (rate(mockforge_requests_total[5m]))
```

### Average Latency by Method

```promql
sum by (method) (rate(mockforge_request_duration_seconds_sum{protocol="http"}[5m]))
/
sum by (method) (rate(mockforge_request_duration_seconds_count{protocol="http"}[5m]))
```

### Error Rate Percentage

```promql
(
  sum(rate(mockforge_errors_total[5m]))
  /
  sum(rate(mockforge_requests_total[5m]))
) * 100
```

### Top 5 Slowest Endpoints

```promql
topk(5,
  sum by (method) (
    rate(mockforge_request_duration_seconds_sum{protocol="http"}[5m])
  )
  /
  sum by (method) (
    rate(mockforge_request_duration_seconds_count{protocol="http"}[5m])
  )
)
```

---

## Grafana Dashboard

### Import Dashboard

1. Open Grafana at http://localhost:3050
2. Login with admin/admin
3. Go to **Dashboards** → **Import**
4. Upload `grafana/dashboards/mockforge-overview.json`

### Panels Included

- **Request Rate** - Total requests per second by protocol
- **Latency Percentiles** - P50, P95, P99 latency over time
- **Error Rate** - Error percentage by protocol
- **Active Connections** - WebSocket connections
- **Plugin Metrics** - Plugin execution stats
- **Protocol Distribution** - Pie chart of requests by protocol

---

## Alerts (Optional)

### High Error Rate Alert

Add to Prometheus `prometheus.yml`:

```yaml
rule_files:
  - 'alerts.yml'
```

Create `alerts.yml`:

```yaml
groups:
  - name: mockforge_alerts
    rules:
      - alert: HighErrorRate
        expr: |
          (
            sum(rate(mockforge_errors_total[5m]))
            /
            sum(rate(mockforge_requests_total[5m]))
          ) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value | humanizePercentage }}"

      - alert: HighLatency
        expr: |
          histogram_quantile(0.95,
            rate(mockforge_request_duration_seconds_bucket[5m])
          ) > 1.0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High P95 latency detected"
          description: "P95 latency is {{ $value }}s"
```

---

## Troubleshooting

### Metrics Not Showing in Prometheus

1. Check MockForge is running with metrics enabled:
   ```bash
   curl http://localhost:9090/metrics
   ```

2. Check Prometheus targets:
   ```bash
   curl http://localhost:9091/targets
   ```

3. Verify Docker networking:
   ```bash
   docker exec mockforge-prometheus ping host.docker.internal
   ```

### Grafana Not Connecting to Prometheus

1. Check Grafana data source configuration
2. Verify Prometheus URL: `http://prometheus:9090`
3. Check Docker network: `mockforge-network`

---

## Cleanup

```bash
# Stop and remove containers
docker-compose down

# Remove volumes (WARNING: deletes all data)
docker-compose down -v
```

---

## Next Steps

- ✅ **Phase 1:** Prometheus metrics and Grafana dashboards
- ✅ **Phase 2:** OpenTelemetry distributed tracing with Jaeger
- **Phase 3:** API Flight Recorder for request/response analysis
- **Phase 4:** Scenario Control and Chaos Engineering metrics
- **Phase 5:** Admin UI integration for observability features

---

## Reference

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [PromQL Query Examples](https://prometheus.io/docs/prometheus/latest/querying/examples/)
