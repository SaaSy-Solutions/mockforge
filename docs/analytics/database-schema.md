# Analytics Database Schema Design

## Overview

This document describes the database schema for MockForge's Traffic Analytics & Metrics Dashboard. The schema is designed to efficiently store, query, and aggregate metrics data with configurable retention periods.

## Design Principles

1. **Time-Series Optimization** - Pre-aggregated data at multiple granularities (minute, hour, day)
2. **Efficient Querying** - Indexes on time ranges and common filter fields
3. **Storage Efficiency** - Automatic rollup and retention policies
4. **Multi-Tenant Support** - Workspace isolation for collaborative mode
5. **Protocol Agnostic** - Support for HTTP, gRPC, WebSocket, MQTT, SMTP, etc.

---

## Schema Tables

### 1. `metrics_aggregates_minute`

**Purpose:** Stores per-minute aggregated metrics for detailed recent analysis

**Retention:** 7 days (configurable)

```sql
CREATE TABLE metrics_aggregates_minute (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Time dimension
    timestamp INTEGER NOT NULL,  -- Unix timestamp (start of minute)

    -- Dimensions
    protocol TEXT NOT NULL,      -- HTTP, gRPC, WebSocket, MQTT, SMTP, etc.
    method TEXT,                 -- GET, POST, etc. (NULL for non-HTTP)
    endpoint TEXT,               -- Normalized path (/api/users/:id)
    status_code INTEGER,         -- HTTP status code or equivalent
    workspace_id TEXT,           -- NULL for non-collaborative mode
    environment TEXT,            -- dev, staging, prod, etc.

    -- Metrics
    request_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,

    -- Latency (milliseconds)
    latency_sum REAL NOT NULL DEFAULT 0.0,
    latency_min REAL,
    latency_max REAL,
    latency_p50 REAL,
    latency_p95 REAL,
    latency_p99 REAL,

    -- Traffic
    bytes_sent INTEGER NOT NULL DEFAULT 0,
    bytes_received INTEGER NOT NULL DEFAULT 0,

    -- Additional metrics
    active_connections INTEGER DEFAULT 0,  -- Snapshot at end of minute

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes for efficient querying
CREATE INDEX idx_metrics_minute_timestamp ON metrics_aggregates_minute(timestamp DESC);
CREATE INDEX idx_metrics_minute_protocol ON metrics_aggregates_minute(protocol);
CREATE INDEX idx_metrics_minute_endpoint ON metrics_aggregates_minute(endpoint);
CREATE INDEX idx_metrics_minute_workspace ON metrics_aggregates_minute(workspace_id);
CREATE INDEX idx_metrics_minute_composite ON metrics_aggregates_minute(timestamp, protocol, endpoint);
CREATE INDEX idx_metrics_minute_errors ON metrics_aggregates_minute(timestamp, error_count) WHERE error_count > 0;
```

---

### 2. `metrics_aggregates_hour`

**Purpose:** Stores per-hour aggregated metrics for weekly/monthly analysis

**Retention:** 30 days (configurable)

```sql
CREATE TABLE metrics_aggregates_hour (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Time dimension
    timestamp INTEGER NOT NULL,  -- Unix timestamp (start of hour)

    -- Dimensions (same as minute table)
    protocol TEXT NOT NULL,
    method TEXT,
    endpoint TEXT,
    status_code INTEGER,
    workspace_id TEXT,
    environment TEXT,

    -- Metrics (same structure as minute table)
    request_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,

    latency_sum REAL NOT NULL DEFAULT 0.0,
    latency_min REAL,
    latency_max REAL,
    latency_p50 REAL,
    latency_p95 REAL,
    latency_p99 REAL,

    bytes_sent INTEGER NOT NULL DEFAULT 0,
    bytes_received INTEGER NOT NULL DEFAULT 0,

    active_connections_avg REAL DEFAULT 0.0,
    active_connections_max INTEGER DEFAULT 0,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes
CREATE INDEX idx_metrics_hour_timestamp ON metrics_aggregates_hour(timestamp DESC);
CREATE INDEX idx_metrics_hour_protocol ON metrics_aggregates_hour(protocol);
CREATE INDEX idx_metrics_hour_endpoint ON metrics_aggregates_hour(endpoint);
CREATE INDEX idx_metrics_hour_workspace ON metrics_aggregates_hour(workspace_id);
CREATE INDEX idx_metrics_hour_composite ON metrics_aggregates_hour(timestamp, protocol, endpoint);
```

---

### 3. `metrics_aggregates_day`

**Purpose:** Stores daily aggregated metrics for long-term trend analysis

**Retention:** 365 days (configurable)

```sql
CREATE TABLE metrics_aggregates_day (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Time dimension
    date TEXT NOT NULL,  -- ISO 8601 date (YYYY-MM-DD)
    timestamp INTEGER NOT NULL,  -- Unix timestamp (start of day UTC)

    -- Dimensions
    protocol TEXT NOT NULL,
    method TEXT,
    endpoint TEXT,
    status_code INTEGER,
    workspace_id TEXT,
    environment TEXT,

    -- Metrics
    request_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,

    latency_sum REAL NOT NULL DEFAULT 0.0,
    latency_min REAL,
    latency_max REAL,
    latency_p50 REAL,
    latency_p95 REAL,
    latency_p99 REAL,

    bytes_sent INTEGER NOT NULL DEFAULT 0,
    bytes_received INTEGER NOT NULL DEFAULT 0,

    active_connections_avg REAL DEFAULT 0.0,
    active_connections_max INTEGER DEFAULT 0,

    -- Daily specific metrics
    unique_clients INTEGER DEFAULT 0,  -- Distinct IP addresses
    peak_hour INTEGER,  -- Hour with most traffic (0-23)

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes
CREATE INDEX idx_metrics_day_date ON metrics_aggregates_day(date DESC);
CREATE INDEX idx_metrics_day_timestamp ON metrics_aggregates_day(timestamp DESC);
CREATE INDEX idx_metrics_day_protocol ON metrics_aggregates_day(protocol);
CREATE INDEX idx_metrics_day_endpoint ON metrics_aggregates_day(endpoint);
CREATE INDEX idx_metrics_day_workspace ON metrics_aggregates_day(workspace_id);
```

---

### 4. `endpoint_stats`

**Purpose:** Tracks cumulative statistics per endpoint for ranking and comparison

**Retention:** No automatic deletion, updated incrementally

```sql
CREATE TABLE endpoint_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    endpoint TEXT NOT NULL,
    protocol TEXT NOT NULL,
    method TEXT,
    workspace_id TEXT,
    environment TEXT,

    -- Cumulative metrics
    total_requests INTEGER NOT NULL DEFAULT 0,
    total_errors INTEGER NOT NULL DEFAULT 0,

    -- Latency stats
    avg_latency_ms REAL,
    min_latency_ms REAL,
    max_latency_ms REAL,
    p95_latency_ms REAL,

    -- Status code breakdown (JSON)
    status_codes TEXT,  -- {"200": 1000, "404": 50, "500": 10}

    -- Traffic
    total_bytes_sent INTEGER NOT NULL DEFAULT 0,
    total_bytes_received INTEGER NOT NULL DEFAULT 0,

    -- Time tracking
    first_seen INTEGER NOT NULL,  -- Unix timestamp
    last_seen INTEGER NOT NULL,   -- Unix timestamp

    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes
CREATE UNIQUE INDEX idx_endpoint_unique ON endpoint_stats(endpoint, protocol, method, workspace_id, environment);
CREATE INDEX idx_endpoint_requests ON endpoint_stats(total_requests DESC);
CREATE INDEX idx_endpoint_errors ON endpoint_stats(total_errors DESC);
CREATE INDEX idx_endpoint_latency ON endpoint_stats(avg_latency_ms DESC);
CREATE INDEX idx_endpoint_workspace ON endpoint_stats(workspace_id);
```

---

### 5. `error_events`

**Purpose:** Stores individual error occurrences for detailed analysis

**Retention:** 7 days (configurable)

```sql
CREATE TABLE error_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    timestamp INTEGER NOT NULL,

    -- Request context
    protocol TEXT NOT NULL,
    method TEXT,
    endpoint TEXT,
    status_code INTEGER,

    -- Error details
    error_type TEXT,  -- timeout, connection_refused, internal_error, etc.
    error_message TEXT,
    error_category TEXT,  -- client_error (4xx), server_error (5xx), network_error, etc.

    -- Request metadata
    request_id TEXT,  -- Link to recorder if available
    trace_id TEXT,    -- OpenTelemetry trace ID
    span_id TEXT,     -- OpenTelemetry span ID

    client_ip TEXT,
    user_agent TEXT,

    workspace_id TEXT,
    environment TEXT,

    -- Additional context (JSON)
    metadata TEXT,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes
CREATE INDEX idx_error_timestamp ON error_events(timestamp DESC);
CREATE INDEX idx_error_type ON error_events(error_type);
CREATE INDEX idx_error_endpoint ON error_events(endpoint);
CREATE INDEX idx_error_category ON error_events(error_category);
CREATE INDEX idx_error_trace ON error_events(trace_id) WHERE trace_id IS NOT NULL;
CREATE INDEX idx_error_workspace ON error_events(workspace_id);
```

---

### 6. `client_analytics`

**Purpose:** Tracks metrics by client (IP, User-Agent) for client analysis

**Retention:** 30 days (configurable)

```sql
CREATE TABLE client_analytics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Time dimension (hourly aggregation)
    timestamp INTEGER NOT NULL,

    -- Client identification
    client_ip TEXT NOT NULL,
    user_agent TEXT,
    user_agent_family TEXT,  -- Chrome, Firefox, curl, etc.
    user_agent_version TEXT,

    -- Dimensions
    protocol TEXT NOT NULL,
    workspace_id TEXT,
    environment TEXT,

    -- Metrics
    request_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,

    avg_latency_ms REAL,

    bytes_sent INTEGER NOT NULL DEFAULT 0,
    bytes_received INTEGER NOT NULL DEFAULT 0,

    -- Top endpoints called (JSON array)
    top_endpoints TEXT,  -- ["GET /api/users", "POST /api/orders"]

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes
CREATE INDEX idx_client_timestamp ON client_analytics(timestamp DESC);
CREATE INDEX idx_client_ip ON client_analytics(client_ip);
CREATE INDEX idx_client_agent ON client_analytics(user_agent_family);
CREATE INDEX idx_client_workspace ON client_analytics(workspace_id);
CREATE INDEX idx_client_requests ON client_analytics(request_count DESC);
```

---

### 7. `traffic_patterns`

**Purpose:** Stores aggregated traffic patterns for heatmap visualization

**Retention:** 90 days (configurable)

```sql
CREATE TABLE traffic_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Time dimensions
    date TEXT NOT NULL,  -- YYYY-MM-DD
    hour INTEGER NOT NULL,  -- 0-23
    day_of_week INTEGER NOT NULL,  -- 0=Sunday, 6=Saturday

    -- Dimensions
    protocol TEXT NOT NULL,
    workspace_id TEXT,
    environment TEXT,

    -- Metrics
    request_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,
    avg_latency_ms REAL,
    unique_clients INTEGER DEFAULT 0,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes
CREATE INDEX idx_pattern_date ON traffic_patterns(date DESC);
CREATE INDEX idx_pattern_hour ON traffic_patterns(hour);
CREATE INDEX idx_pattern_dow ON traffic_patterns(day_of_week);
CREATE INDEX idx_pattern_workspace ON traffic_patterns(workspace_id);
CREATE UNIQUE INDEX idx_pattern_unique ON traffic_patterns(date, hour, protocol, workspace_id, environment);
```

---

### 8. `analytics_snapshots`

**Purpose:** Stores periodic snapshots of current system state for comparison

**Retention:** 90 days (configurable)

```sql
CREATE TABLE analytics_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    timestamp INTEGER NOT NULL,

    -- Snapshot metadata
    snapshot_type TEXT NOT NULL,  -- hourly, daily, weekly

    -- Global metrics
    total_requests INTEGER NOT NULL DEFAULT 0,
    total_errors INTEGER NOT NULL DEFAULT 0,
    avg_latency_ms REAL,
    active_connections INTEGER DEFAULT 0,

    -- Protocol breakdown (JSON)
    protocol_stats TEXT,  -- {"http": {"requests": 1000, "errors": 10}, "grpc": {...}}

    -- Top endpoints (JSON array)
    top_endpoints TEXT,  -- [{"endpoint": "/api/users", "requests": 500}, ...]

    -- System metrics
    memory_usage_bytes INTEGER,
    cpu_usage_percent REAL,
    thread_count INTEGER,
    uptime_seconds INTEGER,

    workspace_id TEXT,
    environment TEXT,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes
CREATE INDEX idx_snapshot_timestamp ON analytics_snapshots(timestamp DESC);
CREATE INDEX idx_snapshot_type ON analytics_snapshots(snapshot_type);
CREATE INDEX idx_snapshot_workspace ON analytics_snapshots(workspace_id);
```

---

## Data Flow & Aggregation Strategy

### 1. Real-Time Collection

```
Incoming Request
  ↓
Prometheus Metrics Registry (in-memory)
  ↓
Request Logger (ring buffer, 1000 entries)
  ↓
Recorder SQLite (optional, full request/response)
```

### 2. Periodic Aggregation

```
Background Aggregation Service (every 1 minute)
  ↓
Query Prometheus for last minute's data
  ↓
INSERT INTO metrics_aggregates_minute
  ↓
UPDATE endpoint_stats
  ↓
INSERT INTO error_events (if errors)
  ↓
INSERT INTO client_analytics (hourly)
  ↓
INSERT INTO traffic_patterns (hourly)
```

### 3. Rollup Aggregation

```
Background Rollup Service (every 1 hour)
  ↓
SELECT FROM metrics_aggregates_minute WHERE timestamp >= last_hour
  ↓
Aggregate to hour level
  ↓
INSERT INTO metrics_aggregates_hour
```

```
Background Rollup Service (every 24 hours)
  ↓
SELECT FROM metrics_aggregates_hour WHERE timestamp >= last_day
  ↓
Aggregate to day level
  ↓
INSERT INTO metrics_aggregates_day
  ↓
INSERT INTO analytics_snapshots (daily snapshot)
```

### 4. Data Retention Cleanup

```
Background Cleanup Service (daily)
  ↓
DELETE FROM metrics_aggregates_minute WHERE timestamp < 7 days ago
DELETE FROM error_events WHERE timestamp < 7 days ago
DELETE FROM metrics_aggregates_hour WHERE timestamp < 30 days ago
DELETE FROM client_analytics WHERE timestamp < 30 days ago
DELETE FROM metrics_aggregates_day WHERE timestamp < 365 days ago
DELETE FROM traffic_patterns WHERE date < 90 days ago
DELETE FROM analytics_snapshots WHERE timestamp < 90 days ago
  ↓
VACUUM  -- Reclaim space
```

---

## Query Examples

### Get Request Count Over Last 24 Hours (5-minute granularity)

```sql
SELECT
    timestamp,
    SUM(request_count) as total_requests,
    SUM(error_count) as total_errors
FROM metrics_aggregates_minute
WHERE timestamp >= strftime('%s', 'now', '-24 hours')
GROUP BY timestamp / 300  -- 5-minute buckets (300 seconds)
ORDER BY timestamp ASC;
```

### Top 10 Endpoints by Traffic

```sql
SELECT
    endpoint,
    protocol,
    method,
    total_requests,
    total_errors,
    avg_latency_ms,
    (total_errors * 100.0 / total_requests) as error_rate
FROM endpoint_stats
WHERE last_seen >= strftime('%s', 'now', '-7 days')
ORDER BY total_requests DESC
LIMIT 10;
```

### Error Rate by Hour (Last Week)

```sql
SELECT
    timestamp,
    SUM(request_count) as requests,
    SUM(error_count) as errors,
    (SUM(error_count) * 100.0 / SUM(request_count)) as error_rate_pct
FROM metrics_aggregates_hour
WHERE timestamp >= strftime('%s', 'now', '-7 days')
GROUP BY timestamp
ORDER BY timestamp ASC;
```

### Traffic Heatmap (Requests by Hour and Day of Week)

```sql
SELECT
    day_of_week,
    hour,
    SUM(request_count) as total_requests
FROM traffic_patterns
WHERE date >= date('now', '-30 days')
GROUP BY day_of_week, hour
ORDER BY day_of_week, hour;
```

### Top Clients by Request Count

```sql
SELECT
    client_ip,
    user_agent_family,
    SUM(request_count) as total_requests,
    SUM(error_count) as total_errors,
    AVG(avg_latency_ms) as avg_latency
FROM client_analytics
WHERE timestamp >= strftime('%s', 'now', '-24 hours')
GROUP BY client_ip, user_agent_family
ORDER BY total_requests DESC
LIMIT 20;
```

### P95 Latency Trend (Last 7 Days)

```sql
SELECT
    date,
    endpoint,
    latency_p95
FROM metrics_aggregates_day
WHERE date >= date('now', '-7 days')
  AND endpoint IN (
    SELECT endpoint FROM endpoint_stats
    ORDER BY total_requests DESC
    LIMIT 10
  )
ORDER BY date ASC, endpoint;
```

---

## Storage Estimates

### Assumptions:
- 1000 req/sec average
- 50 unique endpoints
- 5 protocols (HTTP, gRPC, WebSocket, etc.)
- 10 status codes

### Minute-Level Data (7 days retention):
- Rows per minute: ~250 (50 endpoints × 5 protocols)
- Rows per day: ~360,000
- 7 days: ~2.5M rows
- Row size: ~200 bytes
- **Total: ~500 MB**

### Hour-Level Data (30 days retention):
- Rows per hour: ~250
- Rows per day: ~6,000
- 30 days: ~180,000 rows
- **Total: ~36 MB**

### Day-Level Data (365 days retention):
- Rows per day: ~250
- 365 days: ~91,250 rows
- **Total: ~18 MB**

### Error Events (7 days, 1% error rate):
- 10 req/sec errors × 60 × 60 × 24 × 7 = ~6M events
- Row size: ~300 bytes
- **Total: ~1.8 GB**

### Total Estimated Storage: **~2.4 GB** (for high-traffic scenario)

---

## Configuration Schema

```rust
pub struct AnalyticsConfig {
    pub enabled: bool,
    pub database_path: PathBuf,

    // Aggregation settings
    pub aggregation_interval_seconds: u64,  // Default: 60
    pub rollup_interval_hours: u64,         // Default: 1

    // Retention policies
    pub retention: RetentionConfig,

    // Performance settings
    pub batch_size: usize,                  // Default: 1000
    pub max_query_results: usize,           // Default: 10000
}

pub struct RetentionConfig {
    pub minute_aggregates_days: u32,        // Default: 7
    pub hour_aggregates_days: u32,          // Default: 30
    pub day_aggregates_days: u32,           // Default: 365
    pub error_events_days: u32,             // Default: 7
    pub client_analytics_days: u32,         // Default: 30
    pub traffic_patterns_days: u32,         // Default: 90
    pub snapshots_days: u32,                // Default: 90
    pub cleanup_interval_hours: u32,        // Default: 24
}
```

---

## Migration Strategy

### Phase 1: Create Tables
- Run SQL migration to create all tables and indexes
- Verify schema integrity

### Phase 2: Backfill Historical Data (Optional)
- If Prometheus has historical data (TSDB retention)
- Query Prometheus API for past metrics
- Populate aggregates tables

### Phase 3: Start Aggregation Service
- Begin periodic aggregation from Prometheus
- Monitor for errors and performance

### Phase 4: Enable UI
- Connect analytics API to database
- Display metrics in dashboard

---

## Indexes Summary

| Table | Indexes | Purpose |
|-------|---------|---------|
| metrics_aggregates_minute | 6 | Time range, protocol, endpoint, workspace, composite, errors |
| metrics_aggregates_hour | 5 | Time range, protocol, endpoint, workspace, composite |
| metrics_aggregates_day | 5 | Date, timestamp, protocol, endpoint, workspace |
| endpoint_stats | 5 | Unique endpoint, requests, errors, latency, workspace |
| error_events | 6 | Timestamp, type, endpoint, category, trace, workspace |
| client_analytics | 5 | Timestamp, IP, agent, workspace, requests |
| traffic_patterns | 5 | Date, hour, day of week, workspace, unique pattern |
| analytics_snapshots | 3 | Timestamp, type, workspace |

**Total Indexes: 40**

---

## Notes

1. **SQLite vs PostgreSQL**: Schema designed for SQLite but can be easily ported to PostgreSQL for higher scale
2. **Workspace Isolation**: All tables include optional `workspace_id` for multi-tenant support
3. **Environment Filtering**: Support for dev/staging/prod filtering
4. **Prometheus Integration**: Schema complements (not replaces) Prometheus metrics
5. **OpenTelemetry**: Trace/span IDs stored for distributed tracing correlation
6. **JSON Fields**: Used for flexible metadata and complex aggregations
7. **Normalization**: Endpoints normalized (UUIDs → :id) to prevent cardinality explosion
