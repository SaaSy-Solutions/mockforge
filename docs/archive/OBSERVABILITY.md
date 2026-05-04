# MockForge Observability Guide

Complete guide to monitoring and observing MockForge with Prometheus metrics and Grafana dashboards.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Available Metrics](#available-metrics)
- [Prometheus Setup](#prometheus-setup)
- [Grafana Dashboards](#grafana-dashboards)
- [Common Queries](#common-queries)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

---

## Overview

MockForge provides comprehensive observability through:

- **Prometheus Metrics** - Request rates, latencies, errors across all protocols
- **Multi-Protocol Support** - HTTP, gRPC, WebSocket, GraphQL metrics
- **Plugin Metrics** - Track plugin execution and performance
- **System Metrics** - Memory, CPU, active connections
- **Real-time Monitoring** - Live dashboards with Grafana

---

## Quick Start

### 1. Enable Metrics

**Option A: CLI Flags**
```bash
mockforge serve --metrics --metrics-port 9090
```

**Option B: Configuration File**
```yaml
# config.yaml
observability:
  prometheus:
    enabled: true
    port: 9090
```

```bash
mockforge serve --config config.yaml
```

### 2. Verify Metrics Endpoint

```bash
curl http://localhost:9090/metrics
```

You should see Prometheus-formatted metrics.

### 3. Start Observability Stack (Optional)

```bash
cd examples/observability
docker-compose up -d
```

Access:
- **Prometheus**: http://localhost:9091
- **Grafana**: http://localhost:3050 (admin/admin)

---

## Configuration

### Full Configuration Example

```yaml
observability:
  # Prometheus metrics
  prometheus:
    enabled: true
    port: 9090
    host: "0.0.0.0"
    path: "/metrics"

  # OpenTelemetry (Phase 2 - Coming Soon)
  opentelemetry:
    enabled: false
    endpoint: "http://localhost:4317"
    protocol: "grpc"
    sampling_rate: 1.0
```

### CLI Options

```bash
mockforge serve \
  --metrics \              # Enable metrics endpoint
  --metrics-port 9090      # Metrics server port
```

### Environment Variables

```bash
# Override configuration with environment variables
export MOCKFORGE_METRICS_ENABLED=true
export MOCKFORGE_METRICS_PORT=9090
```

---

## Available Metrics

### Request Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `mockforge_requests_total` | Counter | protocol, method, status | Total request count |
| `mockforge_request_duration_seconds` | Histogram | protocol, method | Request latency distribution |
| `mockforge_requests_in_flight` | Gauge | protocol | Active requests being processed |

### Error Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `mockforge_errors_total` | Counter | protocol, error_type | Total error count |
| `mockforge_error_rate` | Gauge | protocol | Error rate (0.0 to 1.0) |

### Plugin Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `mockforge_plugin_executions_total` | Counter | plugin_name, status | Plugin execution count |
| `mockforge_plugin_execution_duration_seconds` | Histogram | plugin_name | Plugin execution time |
| `mockforge_plugin_errors_total` | Counter | plugin_name, error_type | Plugin error count |

### WebSocket Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `mockforge_ws_connections_active` | Gauge | - | Active WebSocket connections |
| `mockforge_ws_messages_sent_total` | Counter | - | Total messages sent |
| `mockforge_ws_messages_received_total` | Counter | - | Total messages received |

### System Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `mockforge_memory_usage_bytes` | Gauge | - | Memory usage in bytes |
| `mockforge_cpu_usage_percent` | Gauge | - | CPU usage percentage |

### Scenario Metrics (Phase 4)

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `mockforge_active_scenario_mode` | Gauge | - | Current scenario mode (0-3) |
| `mockforge_chaos_triggers_total` | Counter | - | Chaos mode trigger count |

---

## Prometheus Setup

### 1. Install Prometheus

**Docker:**
```bash
docker run -d \
  -p 9091:9090 \
  -v $(pwd)/prometheus.yml:/etc/prometheus/prometheus.yml \
  prom/prometheus
```

**Kubernetes:**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: prometheus
spec:
  replicas: 1
  selector:
    matchLabels:
      app: prometheus
  template:
    metadata:
      labels:
        app: prometheus
    spec:
      containers:
      - name: prometheus
        image: prom/prometheus:latest
        ports:
        - containerPort: 9090
```

### 2. Configure Prometheus

**prometheus.yml:**
```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'mockforge'
    static_configs:
      - targets: ['localhost:9090']
```

### 3. Verify Scraping

1. Open Prometheus UI: http://localhost:9091
2. Go to **Status** → **Targets**
3. Verify MockForge target is UP

---

## Grafana Dashboards

### 1. Install Grafana

**Docker:**
```bash
docker run -d \
  -p 3050:3000 \
  -e GF_SECURITY_ADMIN_PASSWORD=admin \
  grafana/grafana
```

### 2. Add Prometheus Data Source

1. Open Grafana: http://localhost:3050
2. Login: admin/admin
3. Go to **Configuration** → **Data Sources**
4. Add Prometheus: http://prometheus:9090

### 3. Import Dashboard

1. Go to **Dashboards** → **Import**
2. Upload `examples/observability/grafana/dashboards/mockforge-overview.json`
3. Select Prometheus datasource
4. Click **Import**

### Dashboard Panels

The MockForge Overview dashboard includes:

- **Request Rate by Protocol** - Time series of requests/second
- **Latency Percentiles** - P50, P95, P99 latency over time
- **Overall Error Rate** - Gauge showing current error percentage
- **Active WebSocket Connections** - Real-time connection count
- **Requests by Protocol** - Pie chart distribution
- **Plugin Executions** - Table of plugin execution counts

---

## Common Queries

### Request Rate

```promql
# Total requests per second
sum(rate(mockforge_requests_total[5m]))

# By protocol
sum by (protocol) (rate(mockforge_requests_total[5m]))

# By method (HTTP)
sum by (method) (rate(mockforge_requests_total{protocol="http"}[5m]))
```

### Latency Analysis

```promql
# P50 latency (median)
histogram_quantile(0.50, rate(mockforge_request_duration_seconds_bucket[5m]))

# P95 latency
histogram_quantile(0.95, rate(mockforge_request_duration_seconds_bucket[5m]))

# P99 latency
histogram_quantile(0.99, rate(mockforge_request_duration_seconds_bucket[5m]))

# Average latency by protocol
sum by (protocol) (rate(mockforge_request_duration_seconds_sum[5m]))
/
sum by (protocol) (rate(mockforge_request_duration_seconds_count[5m]))
```

### Error Analysis

```promql
# Overall error rate (percentage)
(
  sum(rate(mockforge_errors_total[5m]))
  /
  sum(rate(mockforge_requests_total[5m]))
) * 100

# Error rate by protocol
(
  sum by (protocol) (rate(mockforge_errors_total[5m]))
  /
  sum by (protocol) (rate(mockforge_requests_total[5m]))
) * 100

# Errors per minute by type
sum by (error_type) (rate(mockforge_errors_total[1m])) * 60
```

### Protocol-Specific

```promql
# HTTP 5xx errors
sum(rate(mockforge_requests_total{protocol="http", status=~"5.."}[5m]))

# gRPC success rate
sum(rate(mockforge_requests_total{protocol="grpc", status="ok"}[5m]))
/
sum(rate(mockforge_requests_total{protocol="grpc"}[5m]))

# WebSocket message rate
rate(mockforge_ws_messages_sent_total[5m])

# GraphQL query rate
rate(mockforge_requests_total{protocol="graphql"}[5m])
```

### Plugin Performance

```promql
# Top 5 slowest plugins
topk(5,
  sum by (plugin_name) (rate(mockforge_plugin_execution_duration_seconds_sum[5m]))
  /
  sum by (plugin_name) (rate(mockforge_plugin_execution_duration_seconds_count[5m]))
)

# Plugin success rate
sum by (plugin_name) (rate(mockforge_plugin_executions_total{status="success"}[5m]))
/
sum by (plugin_name) (rate(mockforge_plugin_executions_total[5m]))
```

---

## Troubleshooting

### Metrics Endpoint Not Responding

**Check if metrics are enabled:**
```bash
# Verify endpoint
curl http://localhost:9090/metrics

# Check process
ps aux | grep mockforge
```

**Verify configuration:**
```yaml
observability:
  prometheus:
    enabled: true  # Must be true
    port: 9090     # Check port
```

**Check logs:**
```bash
# Look for metrics server startup message
mockforge serve --metrics | grep "Metrics"
```

### No Data in Prometheus

**Verify Prometheus scrape config:**
```yaml
scrape_configs:
  - job_name: 'mockforge'
    static_configs:
      - targets: ['localhost:9090']  # Correct host:port
```

**Check Prometheus targets:**
```bash
curl http://localhost:9091/targets
```

Status should be "UP" for MockForge.

**Network issues:**
```bash
# Test connectivity
curl http://localhost:9090/metrics

# Check firewall
sudo iptables -L | grep 9090
```

### Grafana Dashboard Shows No Data

**Verify data source:**
1. Go to Grafana → Configuration → Data Sources
2. Test Prometheus connection
3. Verify URL: `http://prometheus:9090`

**Check time range:**
- Default: Last 1 hour
- Ensure MockForge has been running long enough

**Inspect queries:**
1. Click panel title → Edit
2. Check Query tab
3. Verify PromQL query returns data

---

## Best Practices

### 1. Retention and Storage

**Prometheus retention:**
```bash
# Set retention period (default 15 days)
prometheus --storage.tsdb.retention.time=30d
```

**Disk space monitoring:**
```bash
# Check Prometheus data size
du -sh /prometheus

# Monitor disk usage
df -h /prometheus
```

### 2. Query Optimization

**Use appropriate time ranges:**
```promql
# Good: 5-minute rate for real-time
rate(mockforge_requests_total[5m])

# Good: 1-hour increase for totals
increase(mockforge_requests_total[1h])

# Avoid: Very short ranges (may miss data)
rate(mockforge_requests_total[10s])
```

**Use recording rules for expensive queries:**
```yaml
# prometheus-rules.yml
groups:
  - name: mockforge_aggregates
    interval: 30s
    rules:
      - record: job:mockforge_request_rate:5m
        expr: sum(rate(mockforge_requests_total[5m]))
```

### 3. Alerting

**Define critical alerts:**
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
          severity: critical
        annotations:
          summary: "MockForge error rate above 5%"

      - alert: HighLatency
        expr: |
          histogram_quantile(0.95,
            rate(mockforge_request_duration_seconds_bucket[5m])
          ) > 1.0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "MockForge P95 latency above 1s"
```

### 4. Security

**Secure metrics endpoint:**
```yaml
# Add authentication (future feature)
observability:
  prometheus:
    auth:
      username: "metrics"
      password: "secret"
```

**Network restrictions:**
```bash
# Bind to localhost only
observability:
  prometheus:
    host: "127.0.0.1"

# Or use firewall rules
sudo iptables -A INPUT -p tcp --dport 9090 -s 10.0.0.0/8 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 9090 -j DROP
```

### 5. Performance Impact

**Metrics collection overhead:**
- CPU: < 1% additional usage
- Memory: ~100KB for registry
- Latency: ~1-5μs per request

**Optimize scrape interval:**
```yaml
# Balance between freshness and overhead
scrape_interval: 15s  # Good for production
scrape_interval: 5s   # Good for development
```

---

## Next Steps

- **Phase 2:** [OpenTelemetry Integration](./OPENTELEMETRY.md) - Distributed tracing
- **Phase 3:** [API Flight Recorder](./FLIGHT_RECORDER.md) - Request/response recording
- **Phase 4:** [Scenario Control](./SCENARIO_CONTROL.md) - Chaos engineering

---

## Resources

- [Prometheus Documentation](https://prometheus.io/docs/)
- [PromQL Query Language](https://prometheus.io/docs/prometheus/latest/querying/basics/)
- [Grafana Documentation](https://grafana.com/docs/)
- [MockForge Examples](../examples/observability/)

---

## Support

For questions and issues:
- GitHub: https://github.com/SaaSy-Solutions/mockforge/issues
- Documentation: https://mockforge.dev/docs
