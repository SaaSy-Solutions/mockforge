# MockForge In-UI Analytics

## Overview

The MockForge In-UI Analytics system provides real-time monitoring and performance insights directly within the admin interface, powered by Prometheus metrics. This eliminates the need for external tools for basic monitoring while still supporting advanced use cases with Grafana.

## Features

### ✅ Real-Time Metrics Dashboard
- **Auto-refresh**: Metrics update every 10 seconds automatically
- **Time Range Selection**: 5m, 15m, 1h, 6h, 24h intervals
- **Protocol-specific insights**: HTTP, WebSocket, SMTP, gRPC, GraphQL

### ✅ Comprehensive Metrics

#### Summary Cards
- **Request Rate**: Requests per second across all protocols
- **P95 Latency**: 95th percentile response time with color-coded thresholds
- **Error Rate**: Percentage of failed requests with visual indicators
- **Active Connections**: Current number of active connections

#### Request Metrics
- Line chart showing request rate by protocol over time
- Multi-protocol visualization (HTTP, WebSocket, gRPC, etc.)

#### Endpoint Performance
- Sortable table of top endpoints by:
  - Request rate
  - Average latency
  - P95 latency
  - Error rate
- Color-coded latency and error indicators
- Method-specific badges (GET, POST, PUT, DELETE)

#### WebSocket Metrics
- Active and total connections
- Message rate (sent/received)
- Error rate
- Average connection duration

#### System Health
- Memory usage (MB)
- CPU usage (%)
- Thread count
- Uptime

### ✅ Export Capabilities
- **Export All (JSON)**: Complete analytics snapshot with timestamp
- **Export Endpoints (CSV)**: Endpoint metrics in spreadsheet format

## Architecture

### Backend API

**Base URL**: `/__mockforge/analytics`

#### Endpoints

| Endpoint | Query Params | Response Type | Description |
|----------|-------------|---------------|-------------|
| `/summary` | `range=1h` | `SummaryMetrics` | Overall metrics summary |
| `/requests` | `range=1h` | `RequestMetrics` | Time-series request data |
| `/endpoints` | `limit=10` | `EndpointMetrics[]` | Top N endpoints |
| `/websocket` | - | `WebSocketMetrics` | WebSocket-specific metrics |
| `/smtp` | - | `SmtpMetrics` | SMTP-specific metrics |
| `/system` | - | `SystemMetrics` | System health metrics |

#### Response Format

All endpoints return data wrapped in `ApiResponse<T>`:

```json
{
  "success": true,
  "data": { /* metrics data */ }
}
```

### Frontend Components

```
src/
├── stores/
│   └── useAnalyticsStore.ts         # Zustand store for analytics state
├── components/analytics/
│   ├── SummaryCards.tsx             # Summary metric cards
│   ├── RequestRateChart.tsx         # Request rate line chart
│   ├── EndpointsTable.tsx           # Sortable endpoints table
│   ├── WebSocketMetricsCard.tsx     # WebSocket metrics display
│   └── SystemMetricsCard.tsx        # System health display
├── pages/
│   └── AnalyticsPage.tsx            # Main analytics page
└── utils/
    └── exportData.ts                # Export utilities (CSV/JSON)
```

### Data Flow

```
┌─────────────┐
│  Prometheus │ ← Metrics collection from MockForge
└──────┬──────┘
       │
       ▼
┌──────────────────┐
│ Backend API      │ ← Queries Prometheus, caches for 10s
│ (analytics.rs)   │
└──────┬───────────┘
       │
       ▼ HTTP/JSON
┌──────────────────┐
│ Analytics Store  │ ← Zustand state management
│ (Zustand)        │   Auto-refresh every 10s
└──────┬───────────┘
       │
       ▼
┌──────────────────┐
│ React Components │ ← Chart.js visualization
│ (UI)             │
└──────────────────┘
```

## Usage

### Accessing Analytics

1. Navigate to the MockForge Admin UI
2. Click on **Analytics** in the left sidebar
3. Select desired time range from dropdown (default: 1h)

### Time Range Options

- **5 minutes**: Detailed view with 15s intervals
- **15 minutes**: Recent data with 30s intervals
- **1 hour**: Standard view with 1m intervals
- **6 hours**: Extended view with 5m intervals
- **24 hours**: Daily overview with 15m intervals

### Exporting Data

#### Export All Metrics (JSON)
- Click "Export All" button in top-right
- Downloads `mockforge-analytics-YYYY-MM-DD.json`
- Includes all current metrics and metadata

#### Export Endpoints (CSV)
- Click "Export CSV" button in Endpoints table
- Downloads `mockforge-endpoints-YYYY-MM-DD.csv`
- Compatible with Excel, Google Sheets

## Configuration

### Environment Variables

```bash
# Set Prometheus URL (default: http://localhost:9090)
export PROMETHEUS_URL=http://prometheus:9090
```

### Backend Configuration

The Prometheus client includes a 10-second cache to reduce load:

```rust
// In prometheus_client.rs
pub fn new(prometheus_url: String) -> Self {
    Self {
        base_url: prometheus_url,
        cache_ttl: Duration::from_secs(10), // 10-second cache
        // ...
    }
}
```

### Frontend Auto-Refresh

Auto-refresh interval (10 seconds):

```typescript
// In useAnalyticsStore.ts
setInterval(() => {
  const store = useAnalyticsStore.getState();
  if (!store.isLoading && !store.error) {
    store.fetchAll();
  }
}, 10000); // 10 seconds
```

## Metrics Reference

### Summary Metrics

```typescript
interface SummaryMetrics {
  timestamp: string;
  request_rate: number;           // Requests per second
  p95_latency_ms: number;         // 95th percentile latency
  error_rate_percent: number;     // Percentage of errors
  active_connections: number;     // Current active connections
}
```

### Endpoint Metrics

```typescript
interface EndpointMetrics {
  path: string;                   // Normalized endpoint path
  method: string;                 // HTTP method
  request_rate: number;           // Requests per second
  avg_latency_ms: number;         // Average latency
  p95_latency_ms: number;         // 95th percentile latency
  errors: number;                 // Error count
  error_rate_percent: number;     // Error percentage
}
```

### WebSocket Metrics

```typescript
interface WebSocketMetrics {
  active_connections: number;
  total_connections: number;
  message_rate_sent: number;
  message_rate_received: number;
  error_rate: number;
  avg_connection_duration_seconds: number;
}
```

## Performance Considerations

### Caching Strategy

1. **Backend Cache**: 10-second TTL on Prometheus query responses
2. **Recording Rules**: Pre-computed metrics for expensive queries (see `recording_rules.yml`)
3. **Data Decimation**: Appropriate step intervals based on time range

### Optimization Tips

1. **Use Recording Rules**: For frequently accessed metrics
2. **Limit Time Ranges**: Avoid querying 24h+ ranges frequently
3. **Endpoint Limit**: Default top 10 endpoints (adjustable)

## Troubleshooting

### No Data Displayed

**Problem**: All metrics show 0 or "No data available"

**Solutions**:
1. Verify Prometheus is running: `curl http://localhost:9090/-/healthy`
2. Check Prometheus URL env var: `echo $PROMETHEUS_URL`
3. Verify metrics are being collected: `curl http://localhost:9090/api/v1/query?query=mockforge_requests_total`
4. Check browser console for API errors

### High Latency

**Problem**: Analytics page loads slowly

**Solutions**:
1. Reduce time range (use 5m or 15m instead of 24h)
2. Enable recording rules in Prometheus
3. Increase backend cache TTL if acceptable
4. Check Prometheus query performance

### Export Fails

**Problem**: Export buttons don't work

**Solutions**:
1. Check browser console for errors
2. Verify data is loaded (summary should be visible)
3. Check browser download settings
4. Try different browser

## Advanced Usage

### Custom Time Ranges

To add custom time ranges, edit `src/pages/AnalyticsPage.tsx`:

```typescript
const timeRanges: { value: TimeRange; label: string }[] = [
  { value: '5m', label: 'Last 5 minutes' },
  { value: '15m', label: 'Last 15 minutes' },
  { value: '1h', label: 'Last hour' },
  { value: '6h', label: 'Last 6 hours' },
  { value: '24h', label: 'Last 24 hours' },
  // Add your custom ranges here
];
```

### Extending Metrics

To add new metrics:

1. **Backend**: Add handler in `src/handlers/analytics.rs`
2. **Store**: Add state/actions in `src/stores/useAnalyticsStore.ts`
3. **Component**: Create visualization in `src/components/analytics/`
4. **Page**: Add to `src/pages/AnalyticsPage.tsx`

### Integration with Grafana

The analytics system complements Grafana:

- **In-UI Analytics**: Quick overview, development/testing
- **Grafana**: Advanced dashboards, alerting, long-term storage

Use `examples/observability/grafana/dashboards/mockforge-comprehensive.json` for Grafana.

## API Examples

### Fetch Summary (cURL)

```bash
curl http://localhost:8080/__mockforge/analytics/summary?range=1h
```

Response:
```json
{
  "success": true,
  "data": {
    "timestamp": "2025-10-09T10:30:00Z",
    "request_rate": 125.3,
    "p95_latency_ms": 45.2,
    "error_rate_percent": 0.5,
    "active_connections": 42
  }
}
```

### Fetch Top Endpoints (cURL)

```bash
curl "http://localhost:8080/__mockforge/analytics/endpoints?limit=5"
```

### Fetch Request Metrics (JavaScript)

```typescript
const range = '1h';
const response = await fetch(`/__mockforge/analytics/requests?range=${range}`);
const { data } = await response.json();

// data.timestamps: [1696856400, 1696856460, ...]
// data.series: [{ name: 'http', values: [125.3, 132.5, ...] }]
```

## Best Practices

1. **Time Range Selection**
   - Use shorter ranges (5m, 15m) for debugging
   - Use 1h for general monitoring
   - Use 6h, 24h for trend analysis

2. **Performance**
   - Enable auto-refresh only when actively monitoring
   - Export data for offline analysis instead of long time ranges
   - Use recording rules for production deployments

3. **Alerting**
   - Use Prometheus alerting rules for critical issues
   - In-UI analytics for investigation and diagnosis
   - Export data for incident reports

## Related Documentation

- [Prometheus Metrics Guide](./PROMETHEUS_METRICS.md)
- [In-UI Analytics Design](./IN_UI_ANALYTICS_DESIGN.md)
- [Observability Stack Setup](../examples/observability/README.md)
- [Grafana Dashboards](../examples/observability/grafana/)

## Changelog

### v1.0.0 (2025-10-09)
- ✅ Initial release
- ✅ Summary metrics cards
- ✅ Request rate charts
- ✅ Endpoint performance table
- ✅ WebSocket metrics
- ✅ System health monitoring
- ✅ Export to CSV/JSON
- ✅ Auto-refresh (10s interval)
- ✅ Time range selection (5m-24h)
