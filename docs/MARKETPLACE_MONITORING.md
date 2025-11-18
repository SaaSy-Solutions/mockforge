# Marketplace Monitoring & Dashboards

**Pillars:** [Cloud]

[Cloud] - Registry, orgs, governance, monetization, marketplace

This document describes the monitoring infrastructure for MockForge Cloud marketplace operations, including Prometheus metrics and Grafana dashboard setup.

## Overview

The marketplace monitoring system tracks key operational metrics for plugins, templates, and scenarios:
- **Publish rates**: How many items are published per hour/day
- **Download rates**: How many items are downloaded
- **Search performance**: Search latency and success rates
- **Error rates**: Categorized by error type
- **Item counts**: Total number of items in the marketplace

## Prometheus Metrics

All marketplace metrics are exposed via the `/metrics` endpoint on the registry server.

### Available Metrics

#### `mockforge_marketplace_publish_total`
**Type:** Counter
**Labels:** `type` (plugin, template, scenario), `status` (success, error)
**Description:** Total number of marketplace items published.

**Example:**
```
mockforge_marketplace_publish_total{type="plugin",status="success"} 1523
mockforge_marketplace_publish_total{type="plugin",status="error"} 12
mockforge_marketplace_publish_total{type="template",status="success"} 342
```

#### `mockforge_marketplace_publish_duration_seconds`
**Type:** Histogram
**Labels:** `type` (plugin, template, scenario)
**Buckets:** 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0
**Description:** Duration of publish operations in seconds.

**Example Query (p95 publish latency):**
```promql
histogram_quantile(0.95, rate(mockforge_marketplace_publish_duration_seconds_bucket[5m]))
```

#### `mockforge_marketplace_download_total`
**Type:** Counter
**Labels:** `type` (plugin, template, scenario), `status` (success, error)
**Description:** Total number of marketplace items downloaded/retrieved.

**Example:**
```
mockforge_marketplace_download_total{type="plugin",status="success"} 15234
mockforge_marketplace_download_total{type="scenario",status="success"} 5234
```

#### `mockforge_marketplace_download_duration_seconds`
**Type:** Histogram
**Labels:** `type` (plugin, template, scenario)
**Buckets:** 0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0
**Description:** Duration of download/get operations in seconds.

#### `mockforge_marketplace_search_total`
**Type:** Counter
**Labels:** `type` (plugin, template, scenario), `status` (success, error)
**Description:** Total number of marketplace searches performed.

**Example:**
```
mockforge_marketplace_search_total{type="plugin",status="success"} 52341
mockforge_marketplace_search_total{type="template",status="success"} 12341
```

#### `mockforge_marketplace_search_duration_seconds`
**Type:** Histogram
**Labels:** `type` (plugin, template, scenario)
**Buckets:** 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0
**Description:** Duration of search operations in seconds.

#### `mockforge_marketplace_errors_total`
**Type:** Counter
**Labels:** `type` (plugin, template, scenario), `error_code` (not_found, validation_failed, etc.)
**Description:** Total number of marketplace errors by type and error code.

**Example:**
```
mockforge_marketplace_errors_total{type="plugin",error_code="validation_failed"} 5
mockforge_marketplace_errors_total{type="template",error_code="not_found"} 12
```

#### `mockforge_marketplace_items_total`
**Type:** Gauge
**Labels:** `type` (plugin, template, scenario)
**Description:** Current total number of items in the marketplace.

**Example:**
```
mockforge_marketplace_items_total{type="plugin"} 1523
mockforge_marketplace_items_total{type="template"} 342
mockforge_marketplace_items_total{type="scenario"} 234
```

## Example Prometheus Queries

### Publish Rate (per hour)
```promql
rate(mockforge_marketplace_publish_total{status="success"}[1h]) * 3600
```

### Download Rate (per minute)
```promql
rate(mockforge_marketplace_download_total{status="success"}[1m]) * 60
```

### Search Rate (per second)
```promql
rate(mockforge_marketplace_search_total{status="success"}[5m])
```

### Error Rate Percentage
```promql
(rate(mockforge_marketplace_errors_total[5m]) /
 rate(mockforge_marketplace_publish_total[5m] + mockforge_marketplace_download_total[5m] + mockforge_marketplace_search_total[5m])) * 100
```

### 95th Percentile Search Latency
```promql
histogram_quantile(0.95,
  rate(mockforge_marketplace_search_duration_seconds_bucket[5m]))
```

### 99th Percentile Publish Latency
```promql
histogram_quantile(0.99,
  rate(mockforge_marketplace_publish_duration_seconds_bucket[5m]))
```

### Average Download Latency
```promql
rate(mockforge_marketplace_download_duration_seconds_sum[5m]) /
  rate(mockforge_marketplace_download_duration_seconds_count[5m])
```

### Top Error Codes
```promql
topk(5, rate(mockforge_marketplace_errors_total[1h]))
```

## Grafana Dashboard

### Recommended Panels

1. **Publish Rate**
   - Type: Graph
   - Query: `rate(mockforge_marketplace_publish_total{status="success"}[5m]) * 60`
   - Legend: `{{type}} publishes/min`

2. **Download Rate**
   - Type: Graph
   - Query: `rate(mockforge_marketplace_download_total{status="success"}[5m]) * 60`
   - Legend: `{{type}} downloads/min`

3. **Search Rate**
   - Type: Graph
   - Query: `rate(mockforge_marketplace_search_total{status="success"}[5m])`
   - Legend: `{{type}} searches/sec`

4. **Error Rate**
   - Type: Graph
   - Query: `rate(mockforge_marketplace_errors_total[5m])`
   - Legend: `{{type}} - {{error_code}}`

5. **Publish Latency (p95)**
   - Type: Graph
   - Query: `histogram_quantile(0.95, rate(mockforge_marketplace_publish_duration_seconds_bucket[5m]))`
   - Legend: `{{type}} p95 latency`

6. **Search Latency (p99)**
   - Type: Graph
   - Query: `histogram_quantile(0.99, rate(mockforge_marketplace_search_duration_seconds_bucket[5m]))`
   - Legend: `{{type}} p99 latency`

7. **Total Items**
   - Type: Stat
   - Query: `mockforge_marketplace_items_total`
   - Legend: `{{type}} items`

8. **Error Breakdown**
   - Type: Pie Chart
   - Query: `sum by (error_code) (rate(mockforge_marketplace_errors_total[1h]))`

### Dashboard JSON

A complete Grafana dashboard JSON is available at `examples/observability/grafana/dashboards/marketplace-overview.json`.

## Alerting Rules

### High Error Rate
```yaml
- alert: MarketplaceHighErrorRate
  expr: |
    (rate(mockforge_marketplace_errors_total[5m]) /
     (rate(mockforge_marketplace_publish_total[5m]) +
      rate(mockforge_marketplace_download_total[5m]) +
      rate(mockforge_marketplace_search_total[5m]))) * 100 > 5
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Marketplace error rate is above 5%"
    description: "Error rate is {{ $value }}%"
```

### High Publish Latency
```yaml
- alert: MarketplaceHighPublishLatency
  expr: |
    histogram_quantile(0.95,
      rate(mockforge_marketplace_publish_duration_seconds_bucket[5m])) > 5
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Marketplace publish latency (p95) is above 5 seconds"
    description: "P95 latency is {{ $value }}s"
```

### High Search Latency
```yaml
- alert: MarketplaceHighSearchLatency
  expr: |
    histogram_quantile(0.99,
      rate(mockforge_marketplace_search_duration_seconds_bucket[5m])) > 1
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Marketplace search latency (p99) is above 1 second"
    description: "P99 latency is {{ $value }}s"
```

## Setup Instructions

### 1. Enable Metrics Endpoint

The metrics endpoint is automatically enabled when the registry server starts. It's available at:
```
http://localhost:8080/metrics
```

### 2. Configure Prometheus

Add the registry server as a scrape target in `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'mockforge-registry'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 30s
```

### 3. Import Grafana Dashboard

1. Open Grafana
2. Go to Dashboards → Import
3. Upload `examples/observability/grafana/dashboards/marketplace-overview.json`
4. Select your Prometheus data source
5. Click Import

### 4. Configure Alerts

Add the alerting rules to your Prometheus configuration or use Grafana alerting.

## Best Practices

1. **Monitor Key Metrics**: Focus on publish rates, download rates, and error rates as primary health indicators.

2. **Set Appropriate Thresholds**: Adjust alert thresholds based on your expected load and performance requirements.

3. **Track Trends**: Use longer time windows (1h, 24h) to identify trends and capacity planning needs.

4. **Categorize Errors**: Use error codes to identify common failure patterns and prioritize fixes.

5. **Performance Budgets**: Set latency budgets (e.g., p95 < 500ms for searches) and alert when exceeded.

## Troubleshooting

### Metrics Not Appearing

1. Verify the metrics endpoint is accessible:
   ```bash
   curl http://localhost:8080/metrics | grep marketplace
   ```

2. Check Prometheus targets:
   - Navigate to `http://localhost:9090/targets`
   - Verify the registry server target is "UP"

3. Verify metric names:
   - All marketplace metrics start with `mockforge_marketplace_`

### High Cardinality

If you see too many unique metric combinations:
- Review error code labels - ensure they're not too granular
- Check that item types are limited to plugin, template, scenario

## Further Reading

- [Prometheus Metrics Guide](./PROMETHEUS_METRICS.md)
- [Observability Guide](./OBSERVABILITY.md)
- [Load Testing Guide](./LOAD_TESTING_GUIDE.md)
