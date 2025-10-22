# MockForge Analytics

Comprehensive traffic analytics and metrics dashboard for MockForge.

## Features

- **Time-Series Metrics** - Store and query metrics at minute/hour/day granularity
- **Endpoint Analytics** - Track performance, latency, and error rates per endpoint
- **Error Analysis** - Detailed error tracking and categorization
- **Client Analytics** - Analyze traffic by client IP and User-Agent
- **Traffic Patterns** - Heatmap visualization of requests by hour/day
- **Data Export** - Export to CSV or JSON for external analysis
- **Automatic Retention** - Configurable data retention and cleanup policies
- **Prometheus Integration** - Aggregates metrics from Prometheus

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
mockforge-analytics = "0.1"
```

### Basic Usage

```rust
use mockforge_analytics::{AnalyticsDatabase, AnalyticsConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize analytics
    let config = AnalyticsConfig {
        enabled: true,
        database_path: PathBuf::from("analytics.db"),
        ..Default::default()
    };

    let db = AnalyticsDatabase::new(&config.database_path).await?;
    db.run_migrations().await?;

    // Query overview metrics for the last hour
    let overview = db.get_overview_metrics(3600).await?;

    println!("Total Requests: {}", overview.total_requests);
    println!("Error Rate: {:.2}%", overview.error_rate);
    println!("P95 Latency: {:.2}ms", overview.p95_latency_ms);

    Ok(())
}
```

### With Aggregation Service

```rust
use mockforge_analytics::{MetricsAggregator, RetentionService};
use std::sync::Arc;

// Start the metrics aggregation service
let aggregator = Arc::new(MetricsAggregator::new(
    db.clone(),
    "http://localhost:9090",  // Prometheus URL
    config.clone(),
));
aggregator.start().await;

// Start the retention/cleanup service
let retention = Arc::new(RetentionService::new(
    db.clone(),
    config.retention,
));
retention.start().await;
```

## Configuration

### Default Configuration

```rust
let config = AnalyticsConfig::default();
// Aggregation interval: 60 seconds
// Retention: 7d (minute), 30d (hour), 365d (day)
// Cleanup interval: 24 hours
```

### Custom Configuration

```rust
use mockforge_analytics::{AnalyticsConfig, RetentionConfig};

let config = AnalyticsConfig {
    enabled: true,
    database_path: PathBuf::from("analytics.db"),
    aggregation_interval_seconds: 60,
    rollup_interval_hours: 1,
    retention: RetentionConfig {
        minute_aggregates_days: 7,
        hour_aggregates_days: 30,
        day_aggregates_days: 365,
        error_events_days: 14,
        client_analytics_days: 30,
        traffic_patterns_days: 90,
        snapshots_days: 90,
        cleanup_interval_hours: 24,
    },
    batch_size: 1000,
    max_query_results: 10000,
};
```

## API Examples

### Get Overview Metrics

```rust
// Last hour
let overview = db.get_overview_metrics(3600).await?;

// Last 24 hours
let overview = db.get_overview_metrics(86400).await?;

println!("Requests/sec: {:.2}", overview.requests_per_second);
println!("Active Connections: {}", overview.active_connections);
```

### Get Top Endpoints

```rust
let top_endpoints = db.get_top_endpoints(10, None).await?;

for ep in top_endpoints {
    println!("{} {} - {} requests ({:.2}% errors)",
        ep.protocol,
        ep.endpoint,
        ep.total_requests,
        ep.error_rate
    );
}
```

### Get Time Series Data

```rust
use mockforge_analytics::{AnalyticsFilter, Granularity};
use chrono::Utc;

let end_time = Utc::now().timestamp();
let start_time = end_time - 3600;  // Last hour

let filter = AnalyticsFilter {
    start_time: Some(start_time),
    end_time: Some(end_time),
    protocol: Some("HTTP".to_string()),
    ..Default::default()
};

let time_series = db.get_request_time_series(&filter, Granularity::Minute).await?;

for series in time_series {
    println!("Protocol: {}", series.label);
    for point in series.data {
        println!("  {} - {} requests", point.timestamp, point.value);
    }
}
```

### Get Latency Trends

```rust
let filter = AnalyticsFilter {
    start_time: Some(start_time),
    end_time: Some(end_time),
    endpoint: Some("/api/users".to_string()),
    ..Default::default()
};

let trends = db.get_latency_trends(&filter).await?;

for trend in trends {
    println!("Timestamp: {}", trend.timestamp);
    println!("  P50: {:.2}ms", trend.p50);
    println!("  P95: {:.2}ms", trend.p95);
    println!("  P99: {:.2}ms", trend.p99);
}
```

### Get Error Summary

```rust
let errors = db.get_error_summary(&filter, 10).await?;

for err in errors {
    println!("{} ({}) - {} occurrences",
        err.error_type,
        err.error_category,
        err.count
    );
    println!("  Affected endpoints: {:?}", err.endpoints);
}
```

## Data Export

### Export to CSV

```rust
use std::fs::File;

let filter = AnalyticsFilter {
    start_time: Some(start_time),
    end_time: Some(end_time),
    ..Default::default()
};

// Export metrics
let mut file = File::create("metrics.csv")?;
db.export_to_csv(&mut file, &filter).await?;

// Export endpoints
let mut file = File::create("endpoints.csv")?;
db.export_endpoints_to_csv(&mut file, None, 100).await?;

// Export errors
let mut file = File::create("errors.csv")?;
db.export_errors_to_csv(&mut file, &filter, 1000).await?;
```

### Export to JSON

```rust
let json = db.export_to_json(&filter).await?;
std::fs::write("metrics.json", json)?;
```

## Database Schema

The analytics database consists of 8 tables:

1. **`metrics_aggregates_minute`** - Per-minute metrics (7-day retention)
2. **`metrics_aggregates_hour`** - Hourly rollups (30-day retention)
3. **`metrics_aggregates_day`** - Daily rollups (365-day retention)
4. **`endpoint_stats`** - Cumulative endpoint statistics
5. **`error_events`** - Individual error occurrences (7-day retention)
6. **`client_analytics`** - Client-level analytics (30-day retention)
7. **`traffic_patterns`** - Heatmap data (90-day retention)
8. **`analytics_snapshots`** - System snapshots (90-day retention)

**Total Indexes:** 40 optimized indexes for fast queries

See [database-schema.md](../../docs/analytics/database-schema.md) for detailed schema documentation.

## Architecture

```
┌─────────────────────┐
│   Prometheus        │
│   (Metrics Source)  │
└──────────┬──────────┘
           │
           ├─ HTTP API (query metrics)
           ▼
┌─────────────────────┐
│  MetricsAggregator  │  ← Runs every 1 minute
│  (Background Task)  │
└──────────┬──────────┘
           │
           ├─ Parse & aggregate
           ▼
┌─────────────────────┐
│  Analytics Database │
│  (SQLite)           │
│  - Minute aggregates│
│  - Hour rollups     │
│  - Day rollups      │
│  - Endpoint stats   │
│  - Error events     │
│  - Traffic patterns │
└──────────┬──────────┘
           │
           ├─ Query API
           ▼
┌─────────────────────┐
│  Dashboard / API    │
│  - Overview metrics │
│  - Time series      │
│  - Error analysis   │
│  - Export           │
└─────────────────────┘

┌─────────────────────┐
│  RetentionService   │  ← Runs daily
│  (Background Task)  │
└──────────┬──────────┘
           │
           ├─ Cleanup old data
           └─ Vacuum database
```

## Performance

### Storage Estimates

**High Traffic** (1000 req/sec):
- Minute-level (7 days): ~500 MB
- Hour-level (30 days): ~36 MB
- Day-level (365 days): ~18 MB
- Error events (7 days, 1% error rate): ~1.8 GB
- **Total: ~2.4 GB**

**Typical Usage** (100 req/sec):
- **Total: ~240 MB**

### Optimization Features

1. **Path Normalization** - Prevents cardinality explosion
2. **Pre-aggregation** - Reduces query load
3. **Strategic Indexes** - 40 indexes for common queries
4. **Batch Operations** - Minimizes database round-trips
5. **Automatic Cleanup** - Prevents unbounded growth

## Testing

Run the test suite:

```bash
cargo test -p mockforge-analytics
```

All tests:
- ✅ Database creation and migrations
- ✅ Metrics insertion and aggregation
- ✅ Endpoint statistics
- ✅ CSV export
- ✅ Retention service

## Integration

### With MockForge

The analytics system integrates with MockForge's observability infrastructure:

- **Prometheus Metrics** - Aggregates from Prometheus registry
- **Request Logger** - Complements in-memory logger with persistence
- **Recorder** - Links via `request_id` and `trace_id`
- **OpenTelemetry** - Stores `trace_id` and `span_id` for correlation

### With External Tools

- **Grafana** - Query via REST API for dashboard creation
- **Prometheus** - Continue using existing Prometheus metrics
- **Custom Analytics** - Export to CSV/JSON for external processing

## Roadmap

**Completed:**
- ✅ Core analytics database and schema
- ✅ Metrics aggregation service
- ✅ Query API
- ✅ Data export (CSV, JSON)
- ✅ Retention and cleanup
- ✅ Unit tests

**Pending:**
- ⏳ REST API endpoints for dashboard
- ⏳ WebSocket streaming for real-time updates
- ⏳ Dashboard UI components
- ⏳ Grafana dashboard templates
- ⏳ Integration tests
- ⏳ Benchmarks

## Documentation

- [Database Schema](../../docs/analytics/database-schema.md) - Comprehensive schema documentation
- [Implementation Summary](../../docs/analytics/implementation-summary.md) - Architecture and design decisions
- [API Documentation](https://docs.rs/mockforge-analytics) - Full API reference (coming soon)

## Contributing

Contributions are welcome! Please:

1. Write tests for new features
2. Update documentation
3. Follow existing code style
4. Add changelog entries

## License

Same as MockForge project - see root LICENSE file.

## Support

For questions or issues:
- File an issue on GitHub
- Check existing documentation
- Review test cases for usage examples
