//! SQLite database for storing recorded requests and responses

use crate::{models::*, Result};
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use std::{collections::HashMap, path::Path};
use tracing::{debug, info};

/// SQLite database for recorder
#[derive(Clone)]
pub struct RecorderDatabase {
    pool: Pool<Sqlite>,
}

impl RecorderDatabase {
    /// Create a new database connection
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db_url = format!("sqlite:{}?mode=rwc", path.as_ref().display());
        let pool = SqlitePool::connect(&db_url).await?;

        let db = Self { pool };
        db.initialize_schema().await?;

        info!("Recorder database initialized at {:?}", path.as_ref());
        Ok(db)
    }

    /// Create an in-memory database (for testing)
    pub async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;

        let db = Self { pool };
        db.initialize_schema().await?;

        debug!("In-memory recorder database initialized");
        Ok(db)
    }

    /// Initialize database schema
    async fn initialize_schema(&self) -> Result<()> {
        // Create requests table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS requests (
                id TEXT PRIMARY KEY,
                protocol TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                method TEXT NOT NULL,
                path TEXT NOT NULL,
                query_params TEXT,
                headers TEXT NOT NULL,
                body TEXT,
                body_encoding TEXT NOT NULL DEFAULT 'utf8',
                client_ip TEXT,
                trace_id TEXT,
                span_id TEXT,
                duration_ms INTEGER,
                status_code INTEGER,
                tags TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create responses table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS responses (
                request_id TEXT PRIMARY KEY,
                status_code INTEGER NOT NULL,
                headers TEXT NOT NULL,
                body TEXT,
                body_encoding TEXT NOT NULL DEFAULT 'utf8',
                size_bytes INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (request_id) REFERENCES requests(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for common queries
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_requests_timestamp ON requests(timestamp DESC)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_requests_protocol ON requests(protocol)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_requests_method ON requests(method)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_requests_path ON requests(path)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_requests_trace_id ON requests(trace_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_requests_status_code ON requests(status_code)")
            .execute(&self.pool)
            .await?;

        // Create sync_snapshots table for Shadow Snapshot Mode
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sync_snapshots (
                id TEXT PRIMARY KEY,
                endpoint TEXT NOT NULL,
                method TEXT NOT NULL,
                sync_cycle_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                before_status_code INTEGER NOT NULL,
                after_status_code INTEGER NOT NULL,
                before_body TEXT NOT NULL,
                after_body TEXT NOT NULL,
                before_headers TEXT NOT NULL,
                after_headers TEXT NOT NULL,
                response_time_before_ms INTEGER,
                response_time_after_ms INTEGER,
                changes_summary TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for sync_snapshots
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_sync_snapshots_endpoint ON sync_snapshots(endpoint, method, timestamp DESC)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_sync_snapshots_cycle ON sync_snapshots(sync_cycle_id)",
        )
        .execute(&self.pool)
        .await?;

        debug!("Database schema initialized");
        Ok(())
    }

    /// Insert a new request
    pub async fn insert_request(&self, request: &RecordedRequest) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO requests (
                id, protocol, timestamp, method, path, query_params,
                headers, body, body_encoding, client_ip, trace_id, span_id,
                duration_ms, status_code, tags
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&request.id)
        .bind(request.protocol)
        .bind(request.timestamp)
        .bind(&request.method)
        .bind(&request.path)
        .bind(&request.query_params)
        .bind(&request.headers)
        .bind(&request.body)
        .bind(&request.body_encoding)
        .bind(&request.client_ip)
        .bind(&request.trace_id)
        .bind(&request.span_id)
        .bind(request.duration_ms)
        .bind(request.status_code)
        .bind(&request.tags)
        .execute(&self.pool)
        .await?;

        debug!("Recorded request: {} {} {}", request.protocol, request.method, request.path);
        Ok(())
    }

    /// Insert a response
    pub async fn insert_response(&self, response: &RecordedResponse) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO responses (
                request_id, status_code, headers, body, body_encoding,
                size_bytes, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&response.request_id)
        .bind(response.status_code)
        .bind(&response.headers)
        .bind(&response.body)
        .bind(&response.body_encoding)
        .bind(response.size_bytes)
        .bind(response.timestamp)
        .execute(&self.pool)
        .await?;

        debug!("Recorded response for request: {}", response.request_id);
        Ok(())
    }

    /// Get a request by ID
    pub async fn get_request(&self, id: &str) -> Result<Option<RecordedRequest>> {
        let request = sqlx::query_as::<_, RecordedRequest>(
            r#"
            SELECT id, protocol, timestamp, method, path, query_params,
                   headers, body, body_encoding, client_ip, trace_id, span_id,
                   duration_ms, status_code, tags
            FROM requests WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(request)
    }

    /// Get a response by request ID
    pub async fn get_response(&self, request_id: &str) -> Result<Option<RecordedResponse>> {
        let response = sqlx::query_as::<_, RecordedResponse>(
            r#"
            SELECT request_id, status_code, headers, body, body_encoding,
                   size_bytes, timestamp
            FROM responses WHERE request_id = ?
            "#,
        )
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(response)
    }

    /// Get an exchange (request + response) by request ID
    pub async fn get_exchange(&self, id: &str) -> Result<Option<RecordedExchange>> {
        let request = self.get_request(id).await?;
        if let Some(request) = request {
            let response = self.get_response(id).await?;
            Ok(Some(RecordedExchange { request, response }))
        } else {
            Ok(None)
        }
    }

    /// Update an existing response
    pub async fn update_response(
        &self,
        request_id: &str,
        status_code: i32,
        headers: &str,
        body: &str,
        size_bytes: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE responses
            SET status_code = ?,
                headers = ?,
                body = ?,
                body_encoding = 'base64',
                size_bytes = ?,
                timestamp = datetime('now')
            WHERE request_id = ?
            "#,
        )
        .bind(status_code)
        .bind(headers)
        .bind(body)
        .bind(size_bytes)
        .bind(request_id)
        .execute(&self.pool)
        .await?;

        debug!("Updated response for request {}", request_id);
        Ok(())
    }

    /// List recent requests
    pub async fn list_recent(&self, limit: i32) -> Result<Vec<RecordedRequest>> {
        let requests = sqlx::query_as::<_, RecordedRequest>(
            r#"
            SELECT id, protocol, timestamp, method, path, query_params,
                   headers, body, body_encoding, client_ip, trace_id, span_id,
                   duration_ms, status_code, tags
            FROM requests
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(requests)
    }

    /// Delete old requests
    pub async fn delete_older_than(&self, days: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM requests
            WHERE timestamp < datetime('now', ? || ' days')
            "#,
        )
        .bind(format!("-{}", days))
        .execute(&self.pool)
        .await?;

        info!("Deleted {} old requests", result.rows_affected());
        Ok(result.rows_affected())
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let total_requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM requests")
            .fetch_one(&self.pool)
            .await?;

        let total_responses: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM responses")
            .fetch_one(&self.pool)
            .await?;

        let total_size: i64 =
            sqlx::query_scalar("SELECT COALESCE(SUM(size_bytes), 0) FROM responses")
                .fetch_one(&self.pool)
                .await?;

        Ok(DatabaseStats {
            total_requests,
            total_responses,
            total_size_bytes: total_size,
        })
    }

    /// Get detailed statistics for API
    pub async fn get_statistics(&self) -> Result<DetailedStats> {
        let total_requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM requests")
            .fetch_one(&self.pool)
            .await?;

        // Get count by protocol
        let protocol_rows: Vec<(String, i64)> =
            sqlx::query_as("SELECT protocol, COUNT(*) as count FROM requests GROUP BY protocol")
                .fetch_all(&self.pool)
                .await?;

        let by_protocol: HashMap<String, i64> = protocol_rows.into_iter().collect();

        // Get count by status code
        let status_rows: Vec<(i32, i64)> = sqlx::query_as(
            "SELECT status_code, COUNT(*) as count FROM requests WHERE status_code IS NOT NULL GROUP BY status_code"
        )
        .fetch_all(&self.pool)
        .await?;

        let by_status_code: HashMap<i32, i64> = status_rows.into_iter().collect();

        // Get average duration
        let avg_duration: Option<f64> = sqlx::query_scalar(
            "SELECT AVG(duration_ms) FROM requests WHERE duration_ms IS NOT NULL",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(DetailedStats {
            total_requests,
            by_protocol,
            by_status_code,
            avg_duration_ms: avg_duration,
        })
    }

    /// Clear all recordings
    pub async fn clear_all(&self) -> Result<()> {
        sqlx::query("DELETE FROM responses").execute(&self.pool).await?;
        sqlx::query("DELETE FROM requests").execute(&self.pool).await?;
        info!("Cleared all recordings");
        Ok(())
    }

    /// Close the database connection
    pub async fn close(self) {
        self.pool.close().await;
        debug!("Recorder database connection closed");
    }

    /// Insert a sync snapshot
    pub async fn insert_sync_snapshot(&self, snapshot: &crate::sync_snapshots::SyncSnapshot) -> Result<()> {
        let before_headers_json = serde_json::to_string(&snapshot.before.headers)?;
        let after_headers_json = serde_json::to_string(&snapshot.after.headers)?;
        let before_body_encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &snapshot.before.body,
        );
        let after_body_encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &snapshot.after.body,
        );
        let changes_summary_json = serde_json::to_string(&snapshot.changes)?;

        sqlx::query(
            r#"
            INSERT INTO sync_snapshots (
                id, endpoint, method, sync_cycle_id, timestamp,
                before_status_code, after_status_code,
                before_body, after_body,
                before_headers, after_headers,
                response_time_before_ms, response_time_after_ms,
                changes_summary
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&snapshot.id)
        .bind(&snapshot.endpoint)
        .bind(&snapshot.method)
        .bind(&snapshot.sync_cycle_id)
        .bind(snapshot.timestamp.to_rfc3339())
        .bind(snapshot.before.status_code as i32)
        .bind(snapshot.after.status_code as i32)
        .bind(&before_body_encoded)
        .bind(&after_body_encoded)
        .bind(&before_headers_json)
        .bind(&after_headers_json)
        .bind(snapshot.response_time_before.map(|v| v as i64))
        .bind(snapshot.response_time_after.map(|v| v as i64))
        .bind(&changes_summary_json)
        .execute(&self.pool)
        .await?;

        debug!("Inserted sync snapshot: {} for {} {}", snapshot.id, snapshot.method, snapshot.endpoint);
        Ok(())
    }

    /// Get snapshots for an endpoint
    pub async fn get_snapshots_for_endpoint(
        &self,
        endpoint: &str,
        method: Option<&str>,
        limit: Option<i32>,
    ) -> Result<Vec<crate::sync_snapshots::SyncSnapshot>> {
        let limit = limit.unwrap_or(100);

        // If endpoint is empty, get all snapshots
        let query = if endpoint.is_empty() {
            sqlx::query_as::<_, SyncSnapshotRow>(
                r#"
                SELECT id, endpoint, method, sync_cycle_id, timestamp,
                       before_status_code, after_status_code,
                       before_body, after_body,
                       before_headers, after_headers,
                       response_time_before_ms, response_time_after_ms,
                       changes_summary
                FROM sync_snapshots
                ORDER BY timestamp DESC
                LIMIT ?
                "#,
            )
            .bind(limit)
        } else if let Some(method) = method {
            sqlx::query_as::<_, SyncSnapshotRow>(
                r#"
                SELECT id, endpoint, method, sync_cycle_id, timestamp,
                       before_status_code, after_status_code,
                       before_body, after_body,
                       before_headers, after_headers,
                       response_time_before_ms, response_time_after_ms,
                       changes_summary
                FROM sync_snapshots
                WHERE endpoint = ? AND method = ?
                ORDER BY timestamp DESC
                LIMIT ?
                "#,
            )
            .bind(endpoint)
            .bind(method)
            .bind(limit)
        } else {
            sqlx::query_as::<_, SyncSnapshotRow>(
                r#"
                SELECT id, endpoint, method, sync_cycle_id, timestamp,
                       before_status_code, after_status_code,
                       before_body, after_body,
                       before_headers, after_headers,
                       response_time_before_ms, response_time_after_ms,
                       changes_summary
                FROM sync_snapshots
                WHERE endpoint = ?
                ORDER BY timestamp DESC
                LIMIT ?
                "#,
            )
            .bind(endpoint)
            .bind(limit)
        };

        let rows = query.fetch_all(&self.pool).await?;

        let mut snapshots = Vec::new();
        for row in rows {
            snapshots.push(row.to_snapshot()?);
        }

        Ok(snapshots)
    }

    /// Get snapshots by sync cycle ID
    pub async fn get_snapshots_by_cycle(
        &self,
        sync_cycle_id: &str,
    ) -> Result<Vec<crate::sync_snapshots::SyncSnapshot>> {
        let rows = sqlx::query_as::<_, SyncSnapshotRow>(
            r#"
            SELECT id, endpoint, method, sync_cycle_id, timestamp,
                   before_status_code, after_status_code,
                   before_body, after_body,
                   before_headers, after_headers,
                   response_time_before_ms, response_time_after_ms,
                   changes_summary
            FROM sync_snapshots
            WHERE sync_cycle_id = ?
            ORDER BY timestamp DESC
            "#,
        )
        .bind(sync_cycle_id)
        .fetch_all(&self.pool)
        .await?;

        let mut snapshots = Vec::new();
        for row in rows {
            snapshots.push(row.to_snapshot()?);
        }

        Ok(snapshots)
    }

    /// Delete old snapshots (retention policy)
    pub async fn delete_old_snapshots(&self, keep_per_endpoint: i32) -> Result<u64> {
        // This is a simplified retention policy - keep the most recent N snapshots per endpoint+method
        // SQLite doesn't support window functions well, so we'll use a subquery approach
        let result = sqlx::query(
            r#"
            DELETE FROM sync_snapshots
            WHERE id NOT IN (
                SELECT id FROM sync_snapshots
                ORDER BY timestamp DESC
                LIMIT (
                    SELECT COUNT(*) FROM (
                        SELECT DISTINCT endpoint || '|' || method FROM sync_snapshots
                    )
                ) * ?
            )
            "#,
        )
        .bind(keep_per_endpoint)
        .execute(&self.pool)
        .await?;

        info!("Deleted {} old snapshots (kept {} per endpoint)", result.rows_affected(), keep_per_endpoint);
        Ok(result.rows_affected())
    }
}

/// Internal row representation for sync snapshots
#[derive(Debug)]
struct SyncSnapshotRow {
    id: String,
    endpoint: String,
    method: String,
    sync_cycle_id: String,
    timestamp: String,
    before_status_code: i32,
    after_status_code: i32,
    before_body: String,
    after_body: String,
    before_headers: String,
    after_headers: String,
    response_time_before_ms: Option<i64>,
    response_time_after_ms: Option<i64>,
    changes_summary: String,
}

impl SyncSnapshotRow {
    fn to_snapshot(&self) -> Result<crate::sync_snapshots::SyncSnapshot> {
        use crate::sync_snapshots::{SnapshotData, SyncSnapshot};
        use std::collections::HashMap;

        let timestamp = chrono::DateTime::parse_from_rfc3339(&self.timestamp)
            .map_err(|e| crate::RecorderError::InvalidFilter(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&chrono::Utc);

        let before_headers: HashMap<String, String> = serde_json::from_str(&self.before_headers)?;
        let after_headers: HashMap<String, String> = serde_json::from_str(&self.after_headers)?;
        let changes: crate::diff::ComparisonResult = serde_json::from_str(&self.changes_summary)?;

        let before_body = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &self.before_body,
        )
        .map_err(|e| crate::RecorderError::InvalidFilter(format!("Invalid base64: {}", e)))?;

        let after_body = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &self.after_body,
        )
        .map_err(|e| crate::RecorderError::InvalidFilter(format!("Invalid base64: {}", e)))?;

        let before_body_json = serde_json::from_slice(&before_body).ok();
        let after_body_json = serde_json::from_slice(&after_body).ok();

        Ok(SyncSnapshot {
            id: self.id.clone(),
            endpoint: self.endpoint.clone(),
            method: self.method.clone(),
            sync_cycle_id: self.sync_cycle_id.clone(),
            timestamp,
            before: SnapshotData {
                status_code: self.before_status_code as u16,
                headers: before_headers,
                body: before_body,
                body_json: before_body_json,
            },
            after: SnapshotData {
                status_code: self.after_status_code as u16,
                headers: after_headers,
                body: after_body,
                body_json: after_body_json,
            },
            changes,
            response_time_before: self.response_time_before_ms.map(|v| v as u64),
            response_time_after: self.response_time_after_ms.map(|v| v as u64),
        })
    }
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for SyncSnapshotRow {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        Ok(SyncSnapshotRow {
            id: row.try_get("id")?,
            endpoint: row.try_get("endpoint")?,
            method: row.try_get("method")?,
            sync_cycle_id: row.try_get("sync_cycle_id")?,
            timestamp: row.try_get("timestamp")?,
            before_status_code: row.try_get("before_status_code")?,
            after_status_code: row.try_get("after_status_code")?,
            before_body: row.try_get("before_body")?,
            after_body: row.try_get("after_body")?,
            before_headers: row.try_get("before_headers")?,
            after_headers: row.try_get("after_headers")?,
            response_time_before_ms: row.try_get("response_time_before_ms")?,
            response_time_after_ms: row.try_get("response_time_after_ms")?,
            changes_summary: row.try_get("changes_summary")?,
        })
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_requests: i64,
    pub total_responses: i64,
    pub total_size_bytes: i64,
}

/// Detailed statistics for API
#[derive(Debug, Clone)]
pub struct DetailedStats {
    pub total_requests: i64,
    pub by_protocol: HashMap<String, i64>,
    pub by_status_code: HashMap<i32, i64>,
    pub avg_duration_ms: Option<f64>,
}

// Implement FromRow for RecordedRequest
impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for RecordedRequest {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        Ok(RecordedRequest {
            id: row.try_get("id")?,
            protocol: row.try_get("protocol")?,
            timestamp: row.try_get("timestamp")?,
            method: row.try_get("method")?,
            path: row.try_get("path")?,
            query_params: row.try_get("query_params")?,
            headers: row.try_get("headers")?,
            body: row.try_get("body")?,
            body_encoding: row.try_get("body_encoding")?,
            client_ip: row.try_get("client_ip")?,
            trace_id: row.try_get("trace_id")?,
            span_id: row.try_get("span_id")?,
            duration_ms: row.try_get("duration_ms")?,
            status_code: row.try_get("status_code")?,
            tags: row.try_get("tags")?,
        })
    }
}

// Implement FromRow for RecordedResponse
impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for RecordedResponse {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        Ok(RecordedResponse {
            request_id: row.try_get("request_id")?,
            status_code: row.try_get("status_code")?,
            headers: row.try_get("headers")?,
            body: row.try_get("body")?,
            body_encoding: row.try_get("body_encoding")?,
            size_bytes: row.try_get("size_bytes")?,
            timestamp: row.try_get("timestamp")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_database_creation() {
        let db = RecorderDatabase::new_in_memory().await.unwrap();
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.total_requests, 0);
    }

    #[tokio::test]
    async fn test_insert_and_get_request() {
        let db = RecorderDatabase::new_in_memory().await.unwrap();

        let request = RecordedRequest {
            id: "test-123".to_string(),
            protocol: Protocol::Http,
            timestamp: Utc::now(),
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            query_params: None,
            headers: "{}".to_string(),
            body: None,
            body_encoding: "utf8".to_string(),
            client_ip: Some("127.0.0.1".to_string()),
            trace_id: None,
            span_id: None,
            duration_ms: Some(42),
            status_code: Some(200),
            tags: None,
        };

        db.insert_request(&request).await.unwrap();

        let retrieved = db.get_request("test-123").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().path, "/api/test");
    }
}
