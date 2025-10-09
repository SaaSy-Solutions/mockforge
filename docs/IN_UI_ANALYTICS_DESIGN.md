# In-UI Analytics Page Design

## Overview

This document outlines the design for an in-UI analytics page that leverages the Prometheus metrics via a query API to provide real-time monitoring and historical analysis within the MockForge Admin UI.

## Architecture

```
┌─────────────────┐
│   Admin UI      │
│   (React/TS)    │
└────────┬────────┘
         │
         │ HTTP Requests
         ▼
┌─────────────────┐      ┌──────────────────┐
│  Admin Backend  │─────▶│  Prometheus API  │
│   (Axum/Rust)   │◀─────│  Query Endpoint  │
└─────────────────┘      └──────────────────┘
         │
         │ Queries
         ▼
┌─────────────────┐
│   Prometheus    │
│   (Port 9090)   │
└─────────────────┘
```

## UI Components

### 1. Overview Dashboard

**Location:** `/__mockforge/analytics`

**Layout:**
```
┌────────────────────────────────────────────────────────┐
│  MockForge Analytics                    [Last 1h ▼]   │
├────────────────────────────────────────────────────────┤
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │  Req/sec │  │ P95 Lat  │  │Error Rate│  │ Active ││
│  │  125.3   │  │  45ms    │  │   0.2%   │  │   42   ││
│  └──────────┘  └──────────┘  └──────────┘  └────────┘│
├────────────────────────────────────────────────────────┤
│  Request Rate Over Time                    [Protocol ▼│
│  ┌────────────────────────────────────────────────────┐│
│  │                     ╱╲                             ││
│  │                    ╱  ╲         ╱╲                 ││
│  │          ╱╲       ╱    ╲   ╱╲  ╱  ╲                ││
│  │    ─────╱──╲─────╱──────╲─╱──╲╱────╲───────        ││
│  └────────────────────────────────────────────────────┘│
├────────────────────────────────────────────────────────┤
│  Latency Percentiles                                   │
│  ┌────────────────────────────────────────────────────┐│
│  │  P50: ─────                                        ││
│  │  P95: ─────────                                    ││
│  │  P99: ──────────────                               ││
│  └────────────────────────────────────────────────────┘│
└────────────────────────────────────────────────────────┘
```

**Components:**

1. **Summary Cards**
   - Request Rate (requests/sec)
   - P95 Latency (ms)
   - Error Rate (%)
   - Active Connections

2. **Request Rate Chart**
   - Line graph showing requests over time
   - Filterable by protocol (HTTP, WS, gRPC, GraphQL, SMTP)
   - Time range selector (5m, 15m, 1h, 6h, 24h)

3. **Latency Percentiles Chart**
   - Multi-line graph (P50, P95, P99, P99.9)
   - Color-coded with thresholds (green < 100ms, yellow < 500ms, red > 500ms)

### 2. Endpoints View

**Location:** `/__mockforge/analytics/endpoints`

**Components:**

1. **Top Endpoints Table**
   - Columns: Path, Method, Requests, Avg Latency, P95, Errors, Error Rate
   - Sortable by any column
   - Click to drill down

2. **Endpoint Details Modal**
   - Request rate over time for specific endpoint
   - Latency distribution histogram
   - Error types and frequency
   - Recent errors (timestamp, status, duration)

**Example Table:**
```
Path              Method  Req/s  Avg Lat  P95    Errors  Error %
/api/users/:id    GET     45.2   25ms     45ms   0       0.0%
/api/orders       POST    12.3   156ms    280ms  2       0.5%
/api/products/:id GET     87.5   18ms     32ms   0       0.0%
```

### 3. Protocol-Specific Views

#### 3.1 WebSocket Analytics

**Components:**
- Active connections gauge
- Connection duration histogram
- Message rate (sent/received) over time
- Error rate chart
- Connection status distribution (normal, error, timeout)

#### 3.2 SMTP Analytics

**Components:**
- Active connections gauge
- Message receive rate
- Message storage rate
- Error breakdown by type
- Mailbox size trend

### 4. System Health

**Components:**
- Memory usage chart (MB over time)
- CPU usage chart (% over time)
- Thread count gauge
- Uptime display
- System alerts (if any thresholds exceeded)

## Backend API Endpoints

### `/admin/api/analytics/summary`

**Query Parameters:**
- `range`: Time range (5m, 15m, 1h, 6h, 24h)

**Response:**
```json
{
  "timestamp": "2025-10-09T15:30:00Z",
  "request_rate": 125.3,
  "p95_latency_ms": 45,
  "error_rate_percent": 0.2,
  "active_connections": 42
}
```

**Prometheus Queries:**
```rust
// Request rate
"sum(rate(mockforge_requests_total[5m]))"

// P95 latency
"histogram_quantile(0.95, sum(rate(mockforge_request_duration_seconds_bucket[5m])) by (le)) * 1000"

// Error rate
"(sum(rate(mockforge_errors_total[5m])) / sum(rate(mockforge_requests_total[5m]))) * 100"

// Active connections
"sum(mockforge_requests_in_flight)"
```

### `/admin/api/analytics/requests`

**Query Parameters:**
- `range`: Time range
- `protocol`: Filter by protocol (optional)
- `step`: Data point interval

**Response:**
```json
{
  "timestamps": [1696856400, 1696856460, ...],
  "series": [
    {
      "name": "http",
      "values": [125.3, 132.5, 128.1, ...]
    },
    {
      "name": "websocket",
      "values": [12.5, 15.2, 13.1, ...]
    }
  ]
}
```

### `/admin/api/analytics/endpoints`

**Query Parameters:**
- `range`: Time range
- `limit`: Number of results (default: 10)
- `sort_by`: Field to sort by (requests, latency, errors)

**Response:**
```json
{
  "endpoints": [
    {
      "path": "/api/users/:id",
      "method": "GET",
      "request_rate": 45.2,
      "avg_latency_ms": 25,
      "p95_latency_ms": 45,
      "errors": 0,
      "error_rate_percent": 0.0
    }
  ]
}
```

### `/admin/api/analytics/websocket`

**Response:**
```json
{
  "active_connections": 42,
  "total_connections": 1523,
  "message_rate_sent": 125.5,
  "message_rate_received": 118.3,
  "error_rate": 0.1,
  "avg_connection_duration_seconds": 342.5
}
```

### `/admin/api/analytics/system`

**Response:**
```json
{
  "memory_usage_mb": 456.7,
  "cpu_usage_percent": 23.5,
  "thread_count": 48,
  "uptime_seconds": 86400
}
```

## Implementation Plan

### Phase 1: Backend API Layer

1. **Create Prometheus Client** (`crates/mockforge-ui/src/prometheus_client.rs`)
   ```rust
   pub struct PrometheusClient {
       base_url: String,
       client: reqwest::Client,
   }

   impl PrometheusClient {
       pub async fn query(&self, query: &str) -> Result<Value>;
       pub async fn query_range(&self, query: &str, start: i64, end: i64, step: &str) -> Result<Value>;
   }
   ```

2. **Create Analytics Handlers** (`crates/mockforge-ui/src/handlers/analytics.rs`)
   - Implement all API endpoints listed above
   - Use PrometheusClient to fetch data
   - Transform Prometheus responses to UI-friendly format

3. **Add Routes** (`crates/mockforge-ui/src/routes.rs`)
   ```rust
   router
       .route("/__mockforge/analytics/summary", get(analytics::get_summary))
       .route("/__mockforge/analytics/requests", get(analytics::get_requests))
       .route("/__mockforge/analytics/endpoints", get(analytics::get_endpoints))
       .route("/__mockforge/analytics/websocket", get(analytics::get_websocket))
       .route("/__mockforge/analytics/system", get(analytics::get_system))
   ```

### Phase 2: Frontend Components

1. **Analytics Store** (`ui/src/stores/useAnalyticsStore.ts`)
   ```typescript
   interface AnalyticsStore {
       summary: SummaryMetrics | null;
       requests: RequestMetrics | null;
       endpoints: EndpointMetrics[];
       loading: boolean;
       error: string | null;

       fetchSummary: (range: TimeRange) => Promise<void>;
       fetchRequests: (range: TimeRange) => Promise<void>;
       fetchEndpoints: (range: TimeRange) => Promise<void>;
   }
   ```

2. **Chart Components** (`ui/src/components/analytics/`)
   - `RequestRateChart.tsx` - Line chart for request rate
   - `LatencyChart.tsx` - Multi-line latency percentiles
   - `EndpointsTable.tsx` - Sortable table of endpoints
   - `WebSocketMetrics.tsx` - WS-specific dashboard
   - `SystemHealth.tsx` - System metrics display

3. **Analytics Page** (`ui/src/pages/AnalyticsPage.tsx`)
   - Combine all components
   - Handle time range selection
   - Auto-refresh every 10 seconds
   - Export to CSV functionality

### Phase 3: Charting Library

**Recommendation:** Use **Recharts** or **Chart.js**

**Why Recharts:**
- React-friendly declarative API
- Good TypeScript support
- Responsive and customizable
- Built-in animations

**Example:**
```typescript
import { LineChart, Line, XAxis, YAxis, Tooltip, Legend } from 'recharts';

<LineChart data={requestData} width={600} height={300}>
  <XAxis dataKey="timestamp" />
  <YAxis />
  <Tooltip />
  <Legend />
  <Line type="monotone" dataKey="http" stroke="#8884d8" />
  <Line type="monotone" dataKey="websocket" stroke="#82ca9d" />
</LineChart>
```

### Phase 4: Advanced Features

1. **Real-Time Updates**
   - WebSocket connection to stream metrics
   - Or Server-Sent Events (SSE)
   - Or polling every 5-10 seconds

2. **Alerting Dashboard**
   - Show active alerts from Prometheus
   - Color-code by severity
   - Click to see alert details

3. **Custom Dashboards**
   - Allow users to create custom metric views
   - Save/load dashboard configurations
   - Share dashboard URLs

4. **Export Capabilities**
   - Export charts as PNG/SVG
   - Export data as CSV
   - Generate PDF reports

## Configuration

Add to `config.yaml`:
```yaml
observability:
  prometheus:
    enabled: true
    port: 9090
    host: "0.0.0.0"

  analytics:
    enabled: true
    prometheus_url: "http://localhost:9090"
    refresh_interval_seconds: 10
    default_time_range: "1h"
```

## Security Considerations

1. **Authentication**
   - Analytics endpoints should require authentication
   - Use same auth mechanism as other admin endpoints

2. **Rate Limiting**
   - Limit Prometheus query rate to prevent DoS
   - Cache query results for 5-10 seconds

3. **Query Validation**
   - Validate time ranges
   - Sanitize query parameters
   - Prevent arbitrary Prometheus query injection

## Performance Optimization

1. **Caching**
   - Cache Prometheus responses for 10 seconds
   - Use Redis or in-memory cache

2. **Query Optimization**
   - Use recording rules for expensive queries
   - Limit time ranges for queries
   - Use appropriate step intervals

3. **Data Aggregation**
   - Pre-aggregate data on backend
   - Send only necessary data to frontend
   - Use data decimation for long time ranges

## Testing

1. **Backend Tests**
   ```rust
   #[tokio::test]
   async fn test_analytics_summary() {
       let app = create_test_app();
       let response = app
           .oneshot(Request::get("/__mockforge/analytics/summary?range=1h"))
           .await
           .unwrap();

       assert_eq!(response.status(), StatusCode::OK);
   }
   ```

2. **Frontend Tests**
   ```typescript
   describe('AnalyticsPage', () => {
       it('renders summary metrics', async () => {
           render(<AnalyticsPage />);
           await waitFor(() => {
               expect(screen.getByText('Request Rate')).toBeInTheDocument();
           });
       });
   });
   ```

## Future Enhancements

1. **Predictive Analytics**
   - Trend analysis
   - Anomaly detection
   - Capacity planning

2. **Comparison Views**
   - Compare current vs. previous time period
   - Compare different environments
   - A/B test result visualization

3. **Custom Metrics**
   - Allow users to define custom metrics via plugins
   - Visualize plugin-specific metrics

4. **Alerting Integration**
   - Configure alerts from UI
   - View alert history
   - Test alert conditions

## Wireframes

### Analytics Overview
```
┌──────────────────────────────────────────────────────────────┐
│ MockForge Analytics                          [Refresh] [⚙️]  │
├──────────────────────────────────────────────────────────────┤
│ Time Range: [5m] [15m] [1h] [6h] [24h]  Auto-refresh: [On▼] │
├──────────────────────────────────────────────────────────────┤
│ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌──────────┐│
│ │ Requests/s  │ │ P95 Latency │ │ Error Rate  │ │  Active  ││
│ │   125.3     │ │    45ms     │ │    0.2%     │ │    42    ││
│ │  ↑ +5.2%    │ │  ↓ -12ms    │ │  ↓ -0.1%    │ │  ↑ +3    ││
│ └─────────────┘ └─────────────┘ └─────────────┘ └──────────┘│
├──────────────────────────────────────────────────────────────┤
│ Request Rate by Protocol        [Line] [Bar] [Area]    [📊] │
│ ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓│
│ ┃                                                          ┃│
│ ┃   HTTP ━━━━━━━    WebSocket ━━━━    gRPC ━━━━          ┃│
│ ┃                                                          ┃│
│ ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛│
└──────────────────────────────────────────────────────────────┘
```

## Conclusion

The in-UI analytics page provides a powerful, real-time monitoring solution that leverages the comprehensive Prometheus metrics without requiring external tools. This makes MockForge a complete, self-contained solution for API mocking with built-in observability.

**Benefits:**
- ✅ No external tools required for basic monitoring
- ✅ Real-time updates
- ✅ Developer-friendly UI
- ✅ Integrated with existing admin interface
- ✅ Can still export to Grafana for advanced use cases
