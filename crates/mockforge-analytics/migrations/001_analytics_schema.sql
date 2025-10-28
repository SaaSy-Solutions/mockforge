-- MockForge Analytics Database Schema
-- Migration 001: Initial analytics schema

-- ============================================================================
-- 1. Minute-Level Aggregates (7-day retention)
-- ============================================================================
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

-- ============================================================================
-- 2. Hour-Level Aggregates (30-day retention)
-- ============================================================================
CREATE TABLE metrics_aggregates_hour (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Time dimension
    timestamp INTEGER NOT NULL,  -- Unix timestamp (start of hour)

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

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes
CREATE INDEX idx_metrics_hour_timestamp ON metrics_aggregates_hour(timestamp DESC);
CREATE INDEX idx_metrics_hour_protocol ON metrics_aggregates_hour(protocol);
CREATE INDEX idx_metrics_hour_endpoint ON metrics_aggregates_hour(endpoint);
CREATE INDEX idx_metrics_hour_workspace ON metrics_aggregates_hour(workspace_id);
CREATE INDEX idx_metrics_hour_composite ON metrics_aggregates_hour(timestamp, protocol, endpoint);

-- ============================================================================
-- 3. Day-Level Aggregates (365-day retention)
-- ============================================================================
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
    unique_clients INTEGER DEFAULT 0,
    peak_hour INTEGER,  -- Hour with most traffic (0-23)

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes
CREATE INDEX idx_metrics_day_date ON metrics_aggregates_day(date DESC);
CREATE INDEX idx_metrics_day_timestamp ON metrics_aggregates_day(timestamp DESC);
CREATE INDEX idx_metrics_day_protocol ON metrics_aggregates_day(protocol);
CREATE INDEX idx_metrics_day_endpoint ON metrics_aggregates_day(endpoint);
CREATE INDEX idx_metrics_day_workspace ON metrics_aggregates_day(workspace_id);

-- ============================================================================
-- 4. Endpoint Statistics (Cumulative)
-- ============================================================================
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
    first_seen INTEGER NOT NULL,
    last_seen INTEGER NOT NULL,

    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes
CREATE UNIQUE INDEX idx_endpoint_unique ON endpoint_stats(endpoint, protocol, COALESCE(method, ''), COALESCE(workspace_id, ''), COALESCE(environment, ''));
CREATE INDEX idx_endpoint_requests ON endpoint_stats(total_requests DESC);
CREATE INDEX idx_endpoint_errors ON endpoint_stats(total_errors DESC);
CREATE INDEX idx_endpoint_latency ON endpoint_stats(avg_latency_ms DESC);
CREATE INDEX idx_endpoint_workspace ON endpoint_stats(workspace_id);

-- ============================================================================
-- 5. Error Events (7-day retention)
-- ============================================================================
CREATE TABLE error_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    timestamp INTEGER NOT NULL,

    -- Request context
    protocol TEXT NOT NULL,
    method TEXT,
    endpoint TEXT,
    status_code INTEGER,

    -- Error details
    error_type TEXT,
    error_message TEXT,
    error_category TEXT,  -- client_error (4xx), server_error (5xx), network_error

    -- Request metadata
    request_id TEXT,
    trace_id TEXT,
    span_id TEXT,

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

-- ============================================================================
-- 6. Client Analytics (30-day retention)
-- ============================================================================
CREATE TABLE client_analytics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Time dimension (hourly aggregation)
    timestamp INTEGER NOT NULL,

    -- Client identification
    client_ip TEXT NOT NULL,
    user_agent TEXT,
    user_agent_family TEXT,
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
    top_endpoints TEXT,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes
CREATE INDEX idx_client_timestamp ON client_analytics(timestamp DESC);
CREATE INDEX idx_client_ip ON client_analytics(client_ip);
CREATE INDEX idx_client_agent ON client_analytics(user_agent_family);
CREATE INDEX idx_client_workspace ON client_analytics(workspace_id);
CREATE INDEX idx_client_requests ON client_analytics(request_count DESC);

-- ============================================================================
-- 7. Traffic Patterns (90-day retention)
-- ============================================================================
CREATE TABLE traffic_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Time dimensions
    date TEXT NOT NULL,
    hour INTEGER NOT NULL,
    day_of_week INTEGER NOT NULL,

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
CREATE UNIQUE INDEX idx_pattern_unique ON traffic_patterns(date, hour, protocol, COALESCE(workspace_id, ''), COALESCE(environment, ''));

-- ============================================================================
-- 8. Analytics Snapshots (90-day retention)
-- ============================================================================
CREATE TABLE analytics_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    timestamp INTEGER NOT NULL,

    -- Snapshot metadata
    snapshot_type TEXT NOT NULL,

    -- Global metrics
    total_requests INTEGER NOT NULL DEFAULT 0,
    total_errors INTEGER NOT NULL DEFAULT 0,
    avg_latency_ms REAL,
    active_connections INTEGER DEFAULT 0,

    -- Protocol breakdown (JSON)
    protocol_stats TEXT,

    -- Top endpoints (JSON array)
    top_endpoints TEXT,

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
