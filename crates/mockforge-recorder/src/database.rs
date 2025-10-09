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
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_requests_timestamp ON requests(timestamp DESC)")
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

        let total_size: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM responses"
        )
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
        let protocol_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT protocol, COUNT(*) as count FROM requests GROUP BY protocol"
        )
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
            "SELECT AVG(duration_ms) FROM requests WHERE duration_ms IS NOT NULL"
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
