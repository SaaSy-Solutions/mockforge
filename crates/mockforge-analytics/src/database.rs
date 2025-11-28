//! Database layer for analytics storage

use crate::error::{AnalyticsError, Result};
use crate::models::{
    AnalyticsFilter, AnalyticsSnapshot, DayMetricsAggregate, DriftPercentageMetrics,
    EndpointCoverage, EndpointStats, ErrorEvent, HourMetricsAggregate, MetricsAggregate,
    PersonaCIHit, RealityLevelStaleness, ScenarioUsageMetrics, TrafficPattern,
};
use futures::TryStreamExt;
use sqlx::{sqlite::SqlitePoolOptions, Executor, Pool, Sqlite, SqlitePool};
use std::path::Path;
use tracing::{debug, error, info};

/// Analytics database manager
#[derive(Clone)]
pub struct AnalyticsDatabase {
    pool: Pool<Sqlite>,
}

impl AnalyticsDatabase {
    /// Create a new analytics database connection
    ///
    /// # Arguments
    /// * `database_path` - Path to the `SQLite` database file (or ":memory:" for in-memory)
    pub async fn new(database_path: &Path) -> Result<Self> {
        let db_url = if database_path.to_str() == Some(":memory:") {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite://{}", database_path.display())
        };

        info!("Connecting to analytics database: {}", db_url);

        let pool =
            SqlitePoolOptions::new()
                .max_connections(10)
                .connect(&db_url)
                .await
                .map_err(|e| {
                    error!("Failed to connect to analytics database: {}", e);
                    AnalyticsError::Database(e)
                })?;

        // Enable WAL mode for better concurrency
        sqlx::query("PRAGMA journal_mode = WAL").execute(&pool).await?;

        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;

        Ok(Self { pool })
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running analytics database migrations");

        // Run initial schema migration
        let migration_sql = include_str!("../migrations/001_analytics_schema.sql");
        let mut conn = self.pool.acquire().await?;
        let mut stream = conn.execute_many(migration_sql);

        while let Some(_) = stream.try_next().await.map_err(|e| {
            error!("Migration error: {}", e);
            AnalyticsError::Migration(format!("Failed to execute migration: {e}"))
        })? {}

        // Run coverage metrics migration
        let coverage_migration_sql = include_str!("../migrations/002_coverage_metrics.sql");
        let mut conn = self.pool.acquire().await?;
        let mut stream = conn.execute_many(coverage_migration_sql);

        while let Some(_) = stream.try_next().await.map_err(|e| {
            error!("Coverage metrics migration error: {}", e);
            AnalyticsError::Migration(format!("Failed to execute coverage metrics migration: {e}"))
        })? {}

        info!("Analytics database migrations completed successfully");
        Ok(())
    }

    /// Get a reference to the database pool
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ========================================================================
    // Insert Operations
    // ========================================================================

    /// Insert a minute-level metrics aggregate
    pub async fn insert_minute_aggregate(&self, agg: &MetricsAggregate) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO metrics_aggregates_minute (
                timestamp, protocol, method, endpoint, status_code, workspace_id, environment,
                request_count, error_count, latency_sum, latency_min, latency_max,
                latency_p50, latency_p95, latency_p99, bytes_sent, bytes_received, active_connections
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(agg.timestamp)
        .bind(&agg.protocol)
        .bind(&agg.method)
        .bind(&agg.endpoint)
        .bind(agg.status_code)
        .bind(&agg.workspace_id)
        .bind(&agg.environment)
        .bind(agg.request_count)
        .bind(agg.error_count)
        .bind(agg.latency_sum)
        .bind(agg.latency_min)
        .bind(agg.latency_max)
        .bind(agg.latency_p50)
        .bind(agg.latency_p95)
        .bind(agg.latency_p99)
        .bind(agg.bytes_sent)
        .bind(agg.bytes_received)
        .bind(agg.active_connections)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Insert multiple minute-level aggregates in a batch
    pub async fn insert_minute_aggregates_batch(
        &self,
        aggregates: &[MetricsAggregate],
    ) -> Result<()> {
        if aggregates.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        for agg in aggregates {
            sqlx::query(
                r"
                INSERT INTO metrics_aggregates_minute (
                    timestamp, protocol, method, endpoint, status_code, workspace_id, environment,
                    request_count, error_count, latency_sum, latency_min, latency_max,
                    latency_p50, latency_p95, latency_p99, bytes_sent, bytes_received, active_connections
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(agg.timestamp)
            .bind(&agg.protocol)
            .bind(&agg.method)
            .bind(&agg.endpoint)
            .bind(agg.status_code)
            .bind(&agg.workspace_id)
            .bind(&agg.environment)
            .bind(agg.request_count)
            .bind(agg.error_count)
            .bind(agg.latency_sum)
            .bind(agg.latency_min)
            .bind(agg.latency_max)
            .bind(agg.latency_p50)
            .bind(agg.latency_p95)
            .bind(agg.latency_p99)
            .bind(agg.bytes_sent)
            .bind(agg.bytes_received)
            .bind(agg.active_connections)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        debug!("Inserted {} minute aggregates", aggregates.len());
        Ok(())
    }

    /// Insert an hour-level metrics aggregate
    pub async fn insert_hour_aggregate(&self, agg: &HourMetricsAggregate) -> Result<i64> {
        let result = sqlx::query(
            r"
            INSERT INTO metrics_aggregates_hour (
                timestamp, protocol, method, endpoint, status_code, workspace_id, environment,
                request_count, error_count, latency_sum, latency_min, latency_max,
                latency_p50, latency_p95, latency_p99, bytes_sent, bytes_received,
                active_connections_avg, active_connections_max
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(agg.timestamp)
        .bind(&agg.protocol)
        .bind(&agg.method)
        .bind(&agg.endpoint)
        .bind(agg.status_code)
        .bind(&agg.workspace_id)
        .bind(&agg.environment)
        .bind(agg.request_count)
        .bind(agg.error_count)
        .bind(agg.latency_sum)
        .bind(agg.latency_min)
        .bind(agg.latency_max)
        .bind(agg.latency_p50)
        .bind(agg.latency_p95)
        .bind(agg.latency_p99)
        .bind(agg.bytes_sent)
        .bind(agg.bytes_received)
        .bind(agg.active_connections_avg)
        .bind(agg.active_connections_max)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Insert a day-level metrics aggregate
    pub async fn insert_day_aggregate(&self, agg: &DayMetricsAggregate) -> Result<i64> {
        let result = sqlx::query(
            r"
            INSERT INTO metrics_aggregates_day (
                date, timestamp, protocol, method, endpoint, status_code, workspace_id, environment,
                request_count, error_count, latency_sum, latency_min, latency_max,
                latency_p50, latency_p95, latency_p99, bytes_sent, bytes_received,
                active_connections_avg, active_connections_max, unique_clients, peak_hour
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&agg.date)
        .bind(agg.timestamp)
        .bind(&agg.protocol)
        .bind(&agg.method)
        .bind(&agg.endpoint)
        .bind(agg.status_code)
        .bind(&agg.workspace_id)
        .bind(&agg.environment)
        .bind(agg.request_count)
        .bind(agg.error_count)
        .bind(agg.latency_sum)
        .bind(agg.latency_min)
        .bind(agg.latency_max)
        .bind(agg.latency_p50)
        .bind(agg.latency_p95)
        .bind(agg.latency_p99)
        .bind(agg.bytes_sent)
        .bind(agg.bytes_received)
        .bind(agg.active_connections_avg)
        .bind(agg.active_connections_max)
        .bind(agg.unique_clients)
        .bind(agg.peak_hour)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Insert or update endpoint statistics
    pub async fn upsert_endpoint_stats(&self, stats: &EndpointStats) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO endpoint_stats (
                endpoint, protocol, method, workspace_id, environment,
                total_requests, total_errors, avg_latency_ms, min_latency_ms, max_latency_ms,
                p95_latency_ms, status_codes, total_bytes_sent, total_bytes_received,
                first_seen, last_seen
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (endpoint, protocol, COALESCE(method, ''), COALESCE(workspace_id, ''), COALESCE(environment, ''))
            DO UPDATE SET
                total_requests = total_requests + excluded.total_requests,
                total_errors = total_errors + excluded.total_errors,
                avg_latency_ms = excluded.avg_latency_ms,
                min_latency_ms = MIN(min_latency_ms, excluded.min_latency_ms),
                max_latency_ms = MAX(max_latency_ms, excluded.max_latency_ms),
                p95_latency_ms = excluded.p95_latency_ms,
                status_codes = excluded.status_codes,
                total_bytes_sent = total_bytes_sent + excluded.total_bytes_sent,
                total_bytes_received = total_bytes_received + excluded.total_bytes_received,
                last_seen = excluded.last_seen,
                updated_at = strftime('%s', 'now')
            ",
        )
        .bind(&stats.endpoint)
        .bind(&stats.protocol)
        .bind(&stats.method)
        .bind(&stats.workspace_id)
        .bind(&stats.environment)
        .bind(stats.total_requests)
        .bind(stats.total_errors)
        .bind(stats.avg_latency_ms)
        .bind(stats.min_latency_ms)
        .bind(stats.max_latency_ms)
        .bind(stats.p95_latency_ms)
        .bind(&stats.status_codes)
        .bind(stats.total_bytes_sent)
        .bind(stats.total_bytes_received)
        .bind(stats.first_seen)
        .bind(stats.last_seen)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert an error event
    pub async fn insert_error_event(&self, error: &ErrorEvent) -> Result<i64> {
        let result = sqlx::query(
            r"
            INSERT INTO error_events (
                timestamp, protocol, method, endpoint, status_code,
                error_type, error_message, error_category,
                request_id, trace_id, span_id,
                client_ip, user_agent, workspace_id, environment, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(error.timestamp)
        .bind(&error.protocol)
        .bind(&error.method)
        .bind(&error.endpoint)
        .bind(error.status_code)
        .bind(&error.error_type)
        .bind(&error.error_message)
        .bind(&error.error_category)
        .bind(&error.request_id)
        .bind(&error.trace_id)
        .bind(&error.span_id)
        .bind(&error.client_ip)
        .bind(&error.user_agent)
        .bind(&error.workspace_id)
        .bind(&error.environment)
        .bind(&error.metadata)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Insert a traffic pattern
    pub async fn insert_traffic_pattern(&self, pattern: &TrafficPattern) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO traffic_patterns (
                date, hour, day_of_week, protocol, workspace_id, environment,
                request_count, error_count, avg_latency_ms, unique_clients
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (date, hour, protocol, COALESCE(workspace_id, ''), COALESCE(environment, ''))
            DO UPDATE SET
                request_count = request_count + excluded.request_count,
                error_count = error_count + excluded.error_count,
                avg_latency_ms = excluded.avg_latency_ms,
                unique_clients = excluded.unique_clients
            ",
        )
        .bind(&pattern.date)
        .bind(pattern.hour)
        .bind(pattern.day_of_week)
        .bind(&pattern.protocol)
        .bind(&pattern.workspace_id)
        .bind(&pattern.environment)
        .bind(pattern.request_count)
        .bind(pattern.error_count)
        .bind(pattern.avg_latency_ms)
        .bind(pattern.unique_clients)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert an analytics snapshot
    pub async fn insert_snapshot(&self, snapshot: &AnalyticsSnapshot) -> Result<i64> {
        let result = sqlx::query(
            r"
            INSERT INTO analytics_snapshots (
                timestamp, snapshot_type, total_requests, total_errors, avg_latency_ms,
                active_connections, protocol_stats, top_endpoints,
                memory_usage_bytes, cpu_usage_percent, thread_count, uptime_seconds,
                workspace_id, environment
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(snapshot.timestamp)
        .bind(&snapshot.snapshot_type)
        .bind(snapshot.total_requests)
        .bind(snapshot.total_errors)
        .bind(snapshot.avg_latency_ms)
        .bind(snapshot.active_connections)
        .bind(&snapshot.protocol_stats)
        .bind(&snapshot.top_endpoints)
        .bind(snapshot.memory_usage_bytes)
        .bind(snapshot.cpu_usage_percent)
        .bind(snapshot.thread_count)
        .bind(snapshot.uptime_seconds)
        .bind(&snapshot.workspace_id)
        .bind(&snapshot.environment)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// Get minute aggregates for a time range
    pub async fn get_minute_aggregates(
        &self,
        filter: &AnalyticsFilter,
    ) -> Result<Vec<MetricsAggregate>> {
        let mut query = String::from("SELECT * FROM metrics_aggregates_minute WHERE 1=1");

        if filter.start_time.is_some() {
            query.push_str(" AND timestamp >= ?");
        }
        if filter.end_time.is_some() {
            query.push_str(" AND timestamp <= ?");
        }
        if filter.protocol.is_some() {
            query.push_str(" AND protocol = ?");
        }
        if filter.endpoint.is_some() {
            query.push_str(" AND endpoint = ?");
        }
        if filter.method.is_some() {
            query.push_str(" AND method = ?");
        }
        if filter.status_code.is_some() {
            query.push_str(" AND status_code = ?");
        }
        if filter.workspace_id.is_some() {
            query.push_str(" AND workspace_id = ?");
        }
        if filter.environment.is_some() {
            query.push_str(" AND environment = ?");
        }

        query.push_str(" ORDER BY timestamp DESC");

        if filter.limit.is_some() {
            query.push_str(" LIMIT ?");
        }

        // Build the query with bound parameters
        let mut sql_query = sqlx::query_as::<_, MetricsAggregate>(&query);

        if let Some(start) = filter.start_time {
            sql_query = sql_query.bind(start);
        }
        if let Some(end) = filter.end_time {
            sql_query = sql_query.bind(end);
        }
        if let Some(ref protocol) = filter.protocol {
            sql_query = sql_query.bind(protocol);
        }
        if let Some(ref endpoint) = filter.endpoint {
            sql_query = sql_query.bind(endpoint);
        }
        if let Some(ref method) = filter.method {
            sql_query = sql_query.bind(method);
        }
        if let Some(status) = filter.status_code {
            sql_query = sql_query.bind(status);
        }
        if let Some(ref workspace) = filter.workspace_id {
            sql_query = sql_query.bind(workspace);
        }
        if let Some(ref env) = filter.environment {
            sql_query = sql_query.bind(env);
        }
        if let Some(limit) = filter.limit {
            sql_query = sql_query.bind(limit);
        }

        let results = sql_query.fetch_all(&self.pool).await?;

        Ok(results)
    }

    /// Get hour-level aggregates
    pub async fn get_hour_aggregates(
        &self,
        filter: &AnalyticsFilter,
    ) -> Result<Vec<HourMetricsAggregate>> {
        let mut query = String::from("SELECT * FROM metrics_aggregates_hour WHERE 1=1");

        if filter.start_time.is_some() {
            query.push_str(" AND timestamp >= ?");
        }
        if filter.end_time.is_some() {
            query.push_str(" AND timestamp <= ?");
        }
        if filter.protocol.is_some() {
            query.push_str(" AND protocol = ?");
        }
        if filter.endpoint.is_some() {
            query.push_str(" AND endpoint = ?");
        }
        if filter.method.is_some() {
            query.push_str(" AND method = ?");
        }
        if filter.status_code.is_some() {
            query.push_str(" AND status_code = ?");
        }
        if filter.workspace_id.is_some() {
            query.push_str(" AND workspace_id = ?");
        }
        if filter.environment.is_some() {
            query.push_str(" AND environment = ?");
        }

        query.push_str(" ORDER BY timestamp DESC");

        if filter.limit.is_some() {
            query.push_str(" LIMIT ?");
        }

        // Build the query with bound parameters
        let mut sql_query = sqlx::query_as::<_, HourMetricsAggregate>(&query);

        if let Some(start) = filter.start_time {
            sql_query = sql_query.bind(start);
        }
        if let Some(end) = filter.end_time {
            sql_query = sql_query.bind(end);
        }
        if let Some(ref protocol) = filter.protocol {
            sql_query = sql_query.bind(protocol);
        }
        if let Some(ref endpoint) = filter.endpoint {
            sql_query = sql_query.bind(endpoint);
        }
        if let Some(ref method) = filter.method {
            sql_query = sql_query.bind(method);
        }
        if let Some(status) = filter.status_code {
            sql_query = sql_query.bind(status);
        }
        if let Some(ref workspace) = filter.workspace_id {
            sql_query = sql_query.bind(workspace);
        }
        if let Some(ref env) = filter.environment {
            sql_query = sql_query.bind(env);
        }
        if let Some(limit) = filter.limit {
            sql_query = sql_query.bind(limit);
        }

        let results = sql_query.fetch_all(&self.pool).await?;

        Ok(results)
    }

    /// Get top endpoints by request count
    pub async fn get_top_endpoints(
        &self,
        limit: i64,
        workspace_id: Option<&str>,
    ) -> Result<Vec<EndpointStats>> {
        let mut query = String::from("SELECT * FROM endpoint_stats WHERE 1=1");

        if workspace_id.is_some() {
            query.push_str(" AND workspace_id = ?");
        }

        query.push_str(" ORDER BY total_requests DESC LIMIT ?");

        let mut sql_query = sqlx::query_as::<_, EndpointStats>(&query);

        if let Some(workspace) = workspace_id {
            sql_query = sql_query.bind(workspace);
        }

        sql_query = sql_query.bind(limit);

        let results = sql_query.fetch_all(&self.pool).await?;

        Ok(results)
    }

    /// Get recent error events
    pub async fn get_recent_errors(
        &self,
        limit: i64,
        filter: &AnalyticsFilter,
    ) -> Result<Vec<ErrorEvent>> {
        let mut query = String::from("SELECT * FROM error_events WHERE 1=1");

        if filter.start_time.is_some() {
            query.push_str(" AND timestamp >= ?");
        }
        if filter.end_time.is_some() {
            query.push_str(" AND timestamp <= ?");
        }
        if filter.endpoint.is_some() {
            query.push_str(" AND endpoint = ?");
        }
        if filter.workspace_id.is_some() {
            query.push_str(" AND workspace_id = ?");
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT ?");

        let mut sql_query = sqlx::query_as::<_, ErrorEvent>(&query);

        if let Some(start) = filter.start_time {
            sql_query = sql_query.bind(start);
        }
        if let Some(end) = filter.end_time {
            sql_query = sql_query.bind(end);
        }
        if let Some(ref endpoint) = filter.endpoint {
            sql_query = sql_query.bind(endpoint);
        }
        if let Some(ref workspace) = filter.workspace_id {
            sql_query = sql_query.bind(workspace);
        }

        sql_query = sql_query.bind(limit);

        let results = sql_query.fetch_all(&self.pool).await?;

        Ok(results)
    }

    /// Get traffic patterns for heatmap
    pub async fn get_traffic_patterns(
        &self,
        days: i64,
        workspace_id: Option<&str>,
    ) -> Result<Vec<TrafficPattern>> {
        let start_date = chrono::Utc::now() - chrono::Duration::days(days);
        let start_date_str = start_date.format("%Y-%m-%d").to_string();

        let mut query = String::from("SELECT * FROM traffic_patterns WHERE date >= ?");

        if let Some(_workspace) = workspace_id {
            query.push_str(" AND workspace_id = ?");
        }

        query.push_str(" ORDER BY date ASC, hour ASC");

        let mut query_builder = sqlx::query_as::<_, TrafficPattern>(&query).bind(start_date_str);

        if let Some(workspace) = workspace_id {
            query_builder = query_builder.bind(workspace);
        }

        let results = query_builder.fetch_all(&self.pool).await?;

        Ok(results)
    }

    // ========================================================================
    // Cleanup Operations
    // ========================================================================

    /// Delete old minute aggregates
    pub async fn cleanup_minute_aggregates(&self, days: u32) -> Result<u64> {
        let cutoff = chrono::Utc::now().timestamp() - (i64::from(days) * 86400);

        let result = sqlx::query("DELETE FROM metrics_aggregates_minute WHERE timestamp < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        info!(
            "Cleaned up {} minute aggregates older than {} days",
            result.rows_affected(),
            days
        );
        Ok(result.rows_affected())
    }

    /// Delete old hour aggregates
    pub async fn cleanup_hour_aggregates(&self, days: u32) -> Result<u64> {
        let cutoff = chrono::Utc::now().timestamp() - (i64::from(days) * 86400);

        let result = sqlx::query("DELETE FROM metrics_aggregates_hour WHERE timestamp < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        info!("Cleaned up {} hour aggregates older than {} days", result.rows_affected(), days);
        Ok(result.rows_affected())
    }

    /// Delete old error events
    pub async fn cleanup_error_events(&self, days: u32) -> Result<u64> {
        let cutoff = chrono::Utc::now().timestamp() - (i64::from(days) * 86400);

        let result = sqlx::query("DELETE FROM error_events WHERE timestamp < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        info!("Cleaned up {} error events older than {} days", result.rows_affected(), days);
        Ok(result.rows_affected())
    }

    /// Vacuum the database to reclaim space
    pub async fn vacuum(&self) -> Result<()> {
        info!("Running VACUUM on analytics database");
        sqlx::query("VACUUM").execute(&self.pool).await?;
        info!("VACUUM completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_creation() {
        let db = AnalyticsDatabase::new(Path::new(":memory:")).await.unwrap();
        db.run_migrations().await.unwrap();
    }

    #[tokio::test]
    async fn test_insert_minute_aggregate() {
        let db = AnalyticsDatabase::new(Path::new(":memory:")).await.unwrap();
        db.run_migrations().await.unwrap();

        let agg = MetricsAggregate {
            id: None,
            timestamp: chrono::Utc::now().timestamp(),
            protocol: "HTTP".to_string(),
            method: Some("GET".to_string()),
            endpoint: Some("/api/test".to_string()),
            status_code: Some(200),
            workspace_id: None,
            environment: None,
            request_count: 100,
            error_count: 5,
            latency_sum: 500.0,
            latency_min: Some(10.0),
            latency_max: Some(100.0),
            latency_p50: Some(45.0),
            latency_p95: Some(95.0),
            latency_p99: Some(99.0),
            bytes_sent: 10000,
            bytes_received: 5000,
            active_connections: Some(10),
            created_at: None,
        };

        let id = db.insert_minute_aggregate(&agg).await.unwrap();
        assert!(id > 0);
    }
}

// ============================================================================
// Coverage Metrics Operations (MockOps)
// ============================================================================

impl AnalyticsDatabase {
    /// Record scenario usage
    pub async fn record_scenario_usage(
        &self,
        scenario_id: &str,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        // SQLite doesn't support ON CONFLICT with multiple columns easily, so use INSERT OR REPLACE
        // First try to update existing record
        let rows_affected = sqlx::query(
            "UPDATE scenario_usage_metrics
             SET usage_count = usage_count + 1,
                 last_used_at = ?,
                 updated_at = ?
             WHERE scenario_id = ? AND (workspace_id = ? OR (workspace_id IS NULL AND ? IS NULL))
               AND (org_id = ? OR (org_id IS NULL AND ? IS NULL))",
        )
        .bind(now)
        .bind(now)
        .bind(scenario_id)
        .bind(workspace_id)
        .bind(workspace_id)
        .bind(org_id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;

        // If no rows were updated, insert a new record
        if rows_affected.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO scenario_usage_metrics (scenario_id, workspace_id, org_id, usage_count, last_used_at, created_at, updated_at)
                 VALUES (?, ?, ?, 1, ?, ?, ?)"
            )
            .bind(scenario_id)
            .bind(workspace_id)
            .bind(org_id)
            .bind(now)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Record persona CI hit
    pub async fn record_persona_ci_hit(
        &self,
        persona_id: &str,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
        ci_run_id: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO persona_ci_hits (persona_id, workspace_id, org_id, ci_run_id, hit_count, hit_at)
             VALUES (?, ?, ?, ?, 1, ?)"
        )
        .bind(persona_id)
        .bind(workspace_id)
        .bind(org_id)
        .bind(ci_run_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record endpoint test coverage
    pub async fn record_endpoint_coverage(
        &self,
        endpoint: &str,
        method: Option<&str>,
        protocol: &str,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
        coverage_percentage: Option<f64>,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        // Try to update existing record
        let rows_affected = sqlx::query(
            "UPDATE endpoint_coverage
             SET test_count = test_count + 1,
                 last_tested_at = ?,
                 coverage_percentage = COALESCE(?, coverage_percentage),
                 updated_at = ?
             WHERE endpoint = ? AND (method = ? OR (method IS NULL AND ? IS NULL))
               AND protocol = ? AND (workspace_id = ? OR (workspace_id IS NULL AND ? IS NULL))
               AND (org_id = ? OR (org_id IS NULL AND ? IS NULL))",
        )
        .bind(now)
        .bind(coverage_percentage)
        .bind(now)
        .bind(endpoint)
        .bind(method)
        .bind(method)
        .bind(protocol)
        .bind(workspace_id)
        .bind(workspace_id)
        .bind(org_id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;

        // If no rows were updated, insert a new record
        if rows_affected.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO endpoint_coverage (endpoint, method, protocol, workspace_id, org_id, test_count, last_tested_at, coverage_percentage, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?, ?)"
            )
            .bind(endpoint)
            .bind(method)
            .bind(protocol)
            .bind(workspace_id)
            .bind(org_id)
            .bind(now)
            .bind(coverage_percentage)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Record reality level staleness
    pub async fn record_reality_level_staleness(
        &self,
        workspace_id: &str,
        org_id: Option<&str>,
        endpoint: Option<&str>,
        method: Option<&str>,
        protocol: Option<&str>,
        current_reality_level: Option<&str>,
        staleness_days: Option<i32>,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let last_updated = if let Some(days) = staleness_days {
            Some(now - (i64::from(days) * 86400))
        } else {
            Some(now)
        };

        sqlx::query(
            "INSERT INTO reality_level_staleness (workspace_id, org_id, endpoint, method, protocol, current_reality_level, last_updated_at, staleness_days, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT DO NOTHING"
        )
        .bind(workspace_id)
        .bind(org_id)
        .bind(endpoint)
        .bind(method)
        .bind(protocol)
        .bind(current_reality_level)
        .bind(last_updated)
        .bind(staleness_days)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record drift percentage metrics
    pub async fn record_drift_percentage(
        &self,
        workspace_id: &str,
        org_id: Option<&str>,
        total_mocks: i64,
        drifting_mocks: i64,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let drift_percentage = if total_mocks > 0 {
            (drifting_mocks as f64 / total_mocks as f64) * 100.0
        } else {
            0.0
        };

        sqlx::query(
            "INSERT INTO drift_percentage_metrics (workspace_id, org_id, total_mocks, drifting_mocks, drift_percentage, measured_at)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(workspace_id)
        .bind(org_id)
        .bind(total_mocks)
        .bind(drifting_mocks)
        .bind(drift_percentage)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get scenario usage metrics
    pub async fn get_scenario_usage(
        &self,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<ScenarioUsageMetrics>> {
        let limit = limit.unwrap_or(100);
        let mut query = String::from(
            "SELECT id, scenario_id, workspace_id, org_id, usage_count, last_used_at, usage_pattern, created_at, updated_at
             FROM scenario_usage_metrics
             WHERE 1=1"
        );

        if workspace_id.is_some() {
            query.push_str(" AND workspace_id = ?");
        }
        if org_id.is_some() {
            query.push_str(" AND org_id = ?");
        }
        query.push_str(" ORDER BY usage_count DESC LIMIT ?");

        let mut q = sqlx::query_as::<_, ScenarioUsageMetrics>(&query);
        if let Some(ws_id) = workspace_id {
            q = q.bind(ws_id);
        }
        if let Some(o_id) = org_id {
            q = q.bind(o_id);
        }
        q = q.bind(limit);

        let results = q.fetch_all(&self.pool).await?;
        Ok(results)
    }

    /// Get persona CI hits
    pub async fn get_persona_ci_hits(
        &self,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<PersonaCIHit>> {
        let limit = limit.unwrap_or(100);
        let mut query = String::from(
            "SELECT id, persona_id, workspace_id, org_id, ci_run_id, hit_count, hit_at, created_at
             FROM persona_ci_hits
             WHERE 1=1",
        );

        if workspace_id.is_some() {
            query.push_str(" AND workspace_id = ?");
        }
        if org_id.is_some() {
            query.push_str(" AND org_id = ?");
        }
        query.push_str(" ORDER BY hit_at DESC LIMIT ?");

        let mut q = sqlx::query_as::<_, PersonaCIHit>(&query);
        if let Some(ws_id) = workspace_id {
            q = q.bind(ws_id);
        }
        if let Some(o_id) = org_id {
            q = q.bind(o_id);
        }
        q = q.bind(limit);

        let results = q.fetch_all(&self.pool).await?;
        Ok(results)
    }

    /// Get endpoint coverage
    pub async fn get_endpoint_coverage(
        &self,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
        min_coverage: Option<f64>,
    ) -> Result<Vec<EndpointCoverage>> {
        let mut query = String::from(
            "SELECT id, endpoint, method, protocol, workspace_id, org_id, test_count, last_tested_at, coverage_percentage, created_at, updated_at
             FROM endpoint_coverage
             WHERE 1=1"
        );

        if workspace_id.is_some() {
            query.push_str(" AND workspace_id = ?");
        }
        if org_id.is_some() {
            query.push_str(" AND org_id = ?");
        }
        if min_coverage.is_some() {
            query.push_str(" AND (coverage_percentage IS NULL OR coverage_percentage < ?)");
        }
        query.push_str(" ORDER BY coverage_percentage ASC NULLS LAST, test_count DESC");

        let mut q = sqlx::query_as::<_, EndpointCoverage>(&query);
        if let Some(ws_id) = workspace_id {
            q = q.bind(ws_id);
        }
        if let Some(o_id) = org_id {
            q = q.bind(o_id);
        }
        if let Some(min_cov) = min_coverage {
            q = q.bind(min_cov);
        }

        let results = q.fetch_all(&self.pool).await?;
        Ok(results)
    }

    /// Get reality level staleness
    pub async fn get_reality_level_staleness(
        &self,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
        max_staleness_days: Option<i32>,
    ) -> Result<Vec<RealityLevelStaleness>> {
        let mut query = String::from(
            "SELECT id, workspace_id, org_id, endpoint, method, protocol, current_reality_level, last_updated_at, staleness_days, created_at, updated_at
             FROM reality_level_staleness
             WHERE 1=1"
        );

        if workspace_id.is_some() {
            query.push_str(" AND workspace_id = ?");
        }
        if org_id.is_some() {
            query.push_str(" AND org_id = ?");
        }
        if max_staleness_days.is_some() {
            query.push_str(" AND (staleness_days IS NULL OR staleness_days > ?)");
        }
        query.push_str(" ORDER BY staleness_days DESC NULLS LAST");

        let mut q = sqlx::query_as::<_, RealityLevelStaleness>(&query);
        if let Some(ws_id) = workspace_id {
            q = q.bind(ws_id);
        }
        if let Some(o_id) = org_id {
            q = q.bind(o_id);
        }
        if let Some(max_days) = max_staleness_days {
            q = q.bind(max_days);
        }

        let results = q.fetch_all(&self.pool).await?;
        Ok(results)
    }

    /// Get drift percentage metrics
    pub async fn get_drift_percentage(
        &self,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<DriftPercentageMetrics>> {
        let limit = limit.unwrap_or(100);
        let mut query = String::from(
            "SELECT id, workspace_id, org_id, total_mocks, drifting_mocks, drift_percentage, measured_at, created_at
             FROM drift_percentage_metrics
             WHERE 1=1"
        );

        if workspace_id.is_some() {
            query.push_str(" AND workspace_id = ?");
        }
        if org_id.is_some() {
            query.push_str(" AND org_id = ?");
        }
        query.push_str(" ORDER BY measured_at DESC LIMIT ?");

        let mut q = sqlx::query_as::<_, DriftPercentageMetrics>(&query);
        if let Some(ws_id) = workspace_id {
            q = q.bind(ws_id);
        }
        if let Some(o_id) = org_id {
            q = q.bind(o_id);
        }
        q = q.bind(limit);

        let results = q.fetch_all(&self.pool).await?;
        Ok(results)
    }
}
