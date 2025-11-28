# Analytics API Integration Guide

This guide shows how to integrate the MockForge Analytics system into your application.

## Overview

The analytics system consists of three components:

1. **Analytics Database** - SQLite storage for aggregated metrics (`mockforge-analytics` crate)
2. **REST API Endpoints** - HTTP endpoints for querying analytics (`mockforge-ui` handlers)
3. **WebSocket Streaming** - Real-time metrics updates (`mockforge-ui` WebSocket handler)

## Quick Start

### 1. Initialize Analytics Database

```rust
use mockforge_analytics::{AnalyticsConfig, AnalyticsDatabase};
use std::path::PathBuf;

// Create analytics config
let analytics_config = AnalyticsConfig {
    enabled: true,
    database_path: PathBuf::from("mockforge-analytics.db"),
    aggregation_interval_seconds: 60,
    ..Default::default()
};

// Initialize database
let analytics_db = AnalyticsDatabase::new(&analytics_config.database_path).await?;
analytics_db.run_migrations().await?;
```

### 2. Start Background Services

```rust
use mockforge_analytics::{MetricsAggregator, RetentionService};
use std::sync::Arc;

// Start metrics aggregation service (queries Prometheus)
let aggregator = Arc::new(MetricsAggregator::new(
    analytics_db.clone(),
    "http://localhost:9090",  // Prometheus URL
    analytics_config.clone(),
));
aggregator.start().await;

// Start retention/cleanup service
let retention = Arc::new(RetentionService::new(
    analytics_db.clone(),
    analytics_config.retention.clone(),
));
retention.start().await;
```

### 3. Add API Routes to Your Router

```rust
use axum::{Router, routing::get};
use mockforge_ui::handlers::analytics_v2::*;
use mockforge_ui::handlers::analytics_stream::*;

// Create analytics state
let analytics_state = AnalyticsV2State::new(analytics_db.clone());
let stream_state = AnalyticsStreamState::new(analytics_db.clone());

// Create analytics router
let analytics_v2_router = Router::new()
    // Overview endpoint
    .route("/api/v2/analytics/overview", get(get_overview))

    // Time-series data
    .route("/api/v2/analytics/requests", get(get_requests_timeseries))
    .route("/api/v2/analytics/latency", get(get_latency_trends))

    // Analysis endpoints
    .route("/api/v2/analytics/errors", get(get_error_summary))
    .route("/api/v2/analytics/endpoints", get(get_top_endpoints))
    .route("/api/v2/analytics/protocols", get(get_protocol_breakdown))
    .route("/api/v2/analytics/traffic-patterns", get(get_traffic_patterns))

    // Export endpoints
    .route("/api/v2/analytics/export/csv", get(export_csv))
    .route("/api/v2/analytics/export/json", get(export_json))

    .with_state(analytics_state);

// Add WebSocket streaming endpoint
let stream_router = Router::new()
    .route("/api/v2/analytics/stream", get(analytics_websocket_handler))
    .with_state(stream_state);

// Merge into main router
let app = Router::new()
    .merge(analytics_v2_router)
    .merge(stream_router);
```

### 4. Start Your Server

```rust
let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
axum::serve(listener, app).await?;
```

---

## API Endpoints Reference

### GET /api/v2/analytics/overview

**Description:** Get high-level dashboard metrics

**Query Parameters:**
- `duration` (optional, default: 3600) - Time window in seconds
- `workspace_id` (optional) - Filter by workspace
- `environment` (optional) - Filter by environment (dev, staging, prod)

**Response:**
```json
{
  "success": true,
  "data": {
    "total_requests": 150000,
    "total_errors": 1250,
    "error_rate": 0.83,
    "avg_latency_ms": 45.2,
    "p95_latency_ms": 125.8,
    "p99_latency_ms": 250.3,
    "active_connections": 42,
    "total_bytes_sent": 5242880,
    "total_bytes_received": 2621440,
    "requests_per_second": 41.67,
    "top_protocols": [
      {
        "protocol": "HTTP",
        "request_count": 120000,
        "error_count": 1000,
        "avg_latency_ms": 42.5
      }
    ],
    "top_endpoints": [
      {
        "endpoint": "/api/users",
        "protocol": "HTTP",
        "method": "GET",
        "request_count": 50000,
        "error_count": 100,
        "error_rate": 0.2,
        "avg_latency_ms": 35.2,
        "p95_latency_ms": 85.3
      }
    ]
  }
}
```

### GET /api/v2/analytics/requests

**Description:** Get request count time-series data

**Query Parameters:**
- `start_time` (optional) - Unix timestamp
- `end_time` (optional) - Unix timestamp
- `duration` (optional, default: 3600) - Time window in seconds
- `granularity` (optional, default: "minute") - minute, hour, or day
- `protocol` (optional) - Filter by protocol
- `workspace_id` (optional) - Filter by workspace

**Response:**
```json
{
  "success": true,
  "data": {
    "series": [
      {
        "label": "HTTP",
        "data": [
          {"timestamp": 1729600800, "value": 1234.5},
          {"timestamp": 1729600860, "value": 1189.2},
          {"timestamp": 1729600920, "value": 1267.8}
        ]
      },
      {
        "label": "gRPC",
        "data": [
          {"timestamp": 1729600800, "value": 345.2},
          {"timestamp": 1729600860, "value": 378.1}
        ]
      }
    ]
  }
}
```

### GET /api/v2/analytics/latency

**Description:** Get latency percentiles over time

**Query Parameters:** Same as `/requests`

**Response:**
```json
{
  "success": true,
  "data": {
    "trends": [
      {
        "timestamp": 1729600800,
        "p50": 25.3,
        "p95": 95.8,
        "p99": 185.2,
        "avg": 35.4,
        "min": 5.2,
        "max": 450.1
      }
    ]
  }
}
```

### GET /api/v2/analytics/errors

**Description:** Get error summary grouped by type and category

**Query Parameters:**
- `start_time`, `end_time`, `duration` - Time filtering
- `limit` (optional, default: 100) - Max number of error types
- `endpoint` (optional) - Filter by endpoint
- `workspace_id` (optional) - Filter by workspace

**Response:**
```json
{
  "success": true,
  "data": {
    "errors": [
      {
        "error_type": "timeout",
        "error_category": "network_error",
        "count": 145,
        "endpoints": ["/api/slow-endpoint", "/api/another"],
        "last_occurrence": "2025-10-22T14:30:00Z"
      },
      {
        "error_type": "not_found",
        "error_category": "client_error",
        "count": 89,
        "endpoints": ["/api/missing"],
        "last_occurrence": "2025-10-22T14:28:15Z"
      }
    ]
  }
}
```

### GET /api/v2/analytics/endpoints

**Description:** Get top endpoints by traffic

**Query Parameters:**
- `limit` (optional, default: 100) - Max number of endpoints
- `workspace_id` (optional) - Filter by workspace

**Response:**
```json
{
  "success": true,
  "data": {
    "endpoints": [
      {
        "endpoint": "/api/users",
        "protocol": "HTTP",
        "method": "GET",
        "total_requests": 50000,
        "total_errors": 250,
        "error_rate": 0.5,
        "avg_latency_ms": 42.3,
        "p95_latency_ms": 105.8,
        "bytes_sent": 52428800,
        "bytes_received": 10485760
      }
    ]
  }
}
```

### GET /api/v2/analytics/protocols

**Description:** Get traffic breakdown by protocol

**Response:**
```json
{
  "success": true,
  "data": {
    "protocols": [
      {
        "protocol": "HTTP",
        "request_count": 120000,
        "error_count": 1200,
        "avg_latency_ms": 45.2
      },
      {
        "protocol": "gRPC",
        "request_count": 25000,
        "error_count": 50,
        "avg_latency_ms": 32.1
      }
    ]
  }
}
```

### GET /api/v2/analytics/traffic-patterns

**Description:** Get traffic patterns for heatmap (by hour and day of week)

**Query Parameters:**
- `days` (optional, default: 30) - Number of days to include
- `workspace_id` (optional) - Filter by workspace

**Response:**
```json
{
  "success": true,
  "data": {
    "patterns": [
      {
        "date": "2025-10-22",
        "hour": 14,
        "day_of_week": 2,
        "request_count": 5432,
        "error_count": 54,
        "avg_latency_ms": 48.3
      }
    ]
  }
}
```

### GET /api/v2/analytics/export/csv

**Description:** Export metrics to CSV format

**Query Parameters:** Same as `/requests`

**Response:** CSV file with headers:
```csv
timestamp,protocol,method,endpoint,status_code,request_count,error_count,avg_latency_ms,p95_latency_ms,bytes_sent,bytes_received
```

### GET /api/v2/analytics/export/json

**Description:** Export metrics to JSON format

**Query Parameters:** Same as `/requests`

**Response:** JSON array of metric aggregates

---

## WebSocket Streaming

### WS /api/v2/analytics/stream

**Description:** Real-time analytics updates via WebSocket

**Connection:**
```javascript
const ws = new WebSocket('ws://localhost:8080/api/v2/analytics/stream');

ws.onopen = () => {
  console.log('Connected to analytics stream');

  // Send configuration (optional)
  ws.send(JSON.stringify({
    interval_seconds: 5,      // Update every 5 seconds
    duration_seconds: 3600,   // Last hour of data
    protocol: "HTTP",         // Optional filter
    workspace_id: "workspace-123"  // Optional filter
  }));
};

ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  console.log('Metrics update:', update);

  // Update dashboard UI
  updateDashboard(update);
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('Disconnected from analytics stream');
};
```

**Update Message Format:**
```json
{
  "timestamp": 1729614123,
  "total_requests": 150234,
  "total_errors": 1267,
  "error_rate": 0.84,
  "avg_latency_ms": 45.8,
  "p95_latency_ms": 126.3,
  "p99_latency_ms": 251.7,
  "active_connections": 38,
  "requests_per_second": 41.73
}
```

**Configuration Updates:**

Send a new JSON configuration message at any time to update the stream parameters:

```javascript
ws.send(JSON.stringify({
  interval_seconds: 10,     // Change to 10-second updates
  duration_seconds: 7200,   // Last 2 hours
  endpoint: "/api/users"    // Add endpoint filter
}));
```

---

## Error Handling

All API endpoints follow this error response format:

```json
{
  "success": false,
  "error": "Error message here"
}
```

**HTTP Status Codes:**
- `200 OK` - Success
- `400 Bad Request` - Invalid query parameters
- `500 Internal Server Error` - Server-side error (database, etc.)

---

## Example: Complete Integration

Here's a complete example showing how to integrate analytics into a MockForge server:

```rust
use axum::{Router, routing::get};
use mockforge_analytics::{
    AnalyticsConfig, AnalyticsDatabase, MetricsAggregator, RetentionService,
};
use mockforge_ui::handlers::{analytics_v2::*, analytics_stream::*};
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize analytics database
    let analytics_config = AnalyticsConfig {
        enabled: true,
        database_path: PathBuf::from("analytics.db"),
        ..Default::default()
    };

    let analytics_db = AnalyticsDatabase::new(&analytics_config.database_path).await?;
    analytics_db.run_migrations().await?;

    // 2. Start background services
    let aggregator = Arc::new(MetricsAggregator::new(
        analytics_db.clone(),
        "http://localhost:9090",
        analytics_config.clone(),
    ));
    aggregator.start().await;

    let retention = Arc::new(RetentionService::new(
        analytics_db.clone(),
        analytics_config.retention,
    ));
    retention.start().await;

    // 3. Create API states
    let analytics_state = AnalyticsV2State::new(analytics_db.clone());
    let stream_state = AnalyticsStreamState::new(analytics_db);

    // 4. Build router
    let app = Router::new()
        // Analytics endpoints
        .route("/api/v2/analytics/overview", get(get_overview))
        .route("/api/v2/analytics/requests", get(get_requests_timeseries))
        .route("/api/v2/analytics/latency", get(get_latency_trends))
        .route("/api/v2/analytics/errors", get(get_error_summary))
        .route("/api/v2/analytics/endpoints", get(get_top_endpoints))
        .route("/api/v2/analytics/protocols", get(get_protocol_breakdown))
        .route("/api/v2/analytics/traffic-patterns", get(get_traffic_patterns))
        .route("/api/v2/analytics/export/csv", get(export_csv))
        .route("/api/v2/analytics/export/json", get(export_json))
        .with_state(analytics_state)
        // WebSocket streaming
        .route("/api/v2/analytics/stream", get(analytics_websocket_handler))
        .with_state(stream_state);

    // 5. Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("Analytics API listening on http://127.0.0.1:8080");
    axum::serve(listener, app).await?;

    Ok(())
}
```

---

## Testing the API

### Using cURL

```bash
# Get overview metrics
curl "http://localhost:8080/api/v2/analytics/overview?duration=3600"

# Get time-series data
curl "http://localhost:8080/api/v2/analytics/requests?granularity=minute&duration=1800"

# Get latency trends
curl "http://localhost:8080/api/v2/analytics/latency?duration=3600"

# Get error summary
curl "http://localhost:8080/api/v2/analytics/errors?limit=20"

# Get top endpoints
curl "http://localhost:8080/api/v2/analytics/endpoints?limit=10"

# Export to CSV
curl "http://localhost:8080/api/v2/analytics/export/csv?duration=86400" > metrics.csv
```

### Using wscat (WebSocket)

```bash
# Install wscat
npm install -g wscat

# Connect to stream
wscat -c ws://localhost:8080/api/v2/analytics/stream

# Send configuration
> {"interval_seconds": 5, "duration_seconds": 3600}

# Receive updates
< {"timestamp": 1729614123, "total_requests": 150234, ...}
```

---

## Performance Considerations

1. **Query Duration** - Longer durations require more database queries. Recommended limits:
   - Minute granularity: Max 24 hours
   - Hour granularity: Max 30 days
   - Day granularity: Max 365 days

2. **WebSocket Connections** - Each connection creates a background task. Monitor connection count and consider rate limiting.

3. **Export Endpoints** - Large exports can consume significant memory. Consider streaming for very large datasets.

4. **Database Size** - Monitor analytics database size and adjust retention policies as needed.

---

## Troubleshooting

### No Data in Analytics

1. **Check aggregation service** - Ensure `MetricsAggregator` is running
2. **Verify Prometheus** - Check that Prometheus is accessible and has metrics
3. **Check logs** - Look for errors in aggregation service logs

### Slow Queries

1. **Check indexes** - Ensure all 40 indexes are created (run migrations)
2. **Vacuum database** - Run `VACUUM` to optimize database
3. **Reduce query duration** - Use shorter time ranges or coarser granularity

### WebSocket Disconnects

1. **Check network** - Firewalls may block WebSocket connections
2. **Increase interval** - Reduce update frequency to decrease load
3. **Monitor resources** - Check memory and CPU usage

---

## Next Steps

- **Dashboard UI** - Build React/Vue components using these endpoints
- **Grafana Integration** - Import Prometheus metrics into Grafana
- **Alerting** - Add alert rules based on error rates, latency thresholds
- **Custom Reports** - Generate scheduled reports via export endpoints

For more information, see:
- [Database Schema Documentation](database-schema.md)
- [Implementation Summary](implementation-summary.md)
- [MockForge Analytics README](../../crates/mockforge-analytics/README.md)
