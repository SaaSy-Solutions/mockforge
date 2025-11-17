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

        // Create behavioral_sequences table for Behavioral Cloning
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS behavioral_sequences (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                steps TEXT NOT NULL,
                frequency REAL NOT NULL,
                confidence REAL NOT NULL,
                learned_from TEXT,
                description TEXT,
                tags TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create endpoint_probabilities table for Behavioral Cloning
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS endpoint_probabilities (
                endpoint TEXT NOT NULL,
                method TEXT NOT NULL,
                status_code_distribution TEXT NOT NULL,
                latency_distribution TEXT NOT NULL,
                error_patterns TEXT,
                payload_variations TEXT,
                sample_count INTEGER NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (endpoint, method)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create edge_case_patterns table for Behavioral Cloning
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS edge_case_patterns (
                id TEXT PRIMARY KEY,
                endpoint TEXT NOT NULL,
                method TEXT NOT NULL,
                pattern_type TEXT NOT NULL,
                original_probability REAL NOT NULL,
                amplified_probability REAL,
                conditions TEXT,
                sample_responses TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for behavioral cloning tables
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_behavioral_sequences_name ON behavioral_sequences(name)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_endpoint_probabilities_endpoint ON endpoint_probabilities(endpoint, method)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_edge_case_patterns_endpoint ON edge_case_patterns(endpoint, method)",
        )
        .execute(&self.pool)
        .await?;

        // Create flows table for behavioral cloning v1
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS flows (
                id TEXT PRIMARY KEY,
                name TEXT,
                description TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                tags TEXT,  -- JSON array
                metadata TEXT  -- JSON object
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create flow_steps table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS flow_steps (
                flow_id TEXT NOT NULL,
                step_index INTEGER NOT NULL,
                request_id TEXT NOT NULL,
                step_label TEXT,  -- e.g., "login", "list", "checkout"
                timing_ms INTEGER,  -- delay from previous step
                FOREIGN KEY (flow_id) REFERENCES flows(id) ON DELETE CASCADE,
                FOREIGN KEY (request_id) REFERENCES requests(id) ON DELETE CASCADE,
                PRIMARY KEY (flow_id, step_index)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create scenarios table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS scenarios (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                description TEXT,
                scenario_data TEXT NOT NULL,  -- JSON serialized BehavioralScenario
                metadata TEXT,  -- JSON object
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                tags TEXT,  -- JSON array
                UNIQUE(name, version)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for flows and scenarios
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_flows_created_at ON flows(created_at DESC)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_flow_steps_flow_id ON flow_steps(flow_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_scenarios_name ON scenarios(name, version)")
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

    /// Insert a behavioral sequence
    pub async fn insert_behavioral_sequence(
        &self,
        sequence: &mockforge_core::behavioral_cloning::BehavioralSequence,
    ) -> Result<()> {
        let steps_json = serde_json::to_string(&sequence.steps)?;
        let learned_from_json = serde_json::to_string(&sequence.learned_from)?;
        let tags_json = serde_json::to_string(&sequence.tags)?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO behavioral_sequences (
                id, name, steps, frequency, confidence, learned_from, description, tags
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&sequence.id)
        .bind(&sequence.name)
        .bind(&steps_json)
        .bind(sequence.frequency)
        .bind(sequence.confidence)
        .bind(&learned_from_json)
        .bind(&sequence.description)
        .bind(&tags_json)
        .execute(&self.pool)
        .await?;

        debug!("Inserted behavioral sequence: {}", sequence.id);
        Ok(())
    }

    /// Get all behavioral sequences
    pub async fn get_behavioral_sequences(&self) -> Result<Vec<mockforge_core::behavioral_cloning::BehavioralSequence>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, steps, frequency, confidence, learned_from, description, tags
            FROM behavioral_sequences
            ORDER BY frequency DESC, confidence DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut sequences = Vec::new();
        for row in rows {
            use sqlx::Row;
            let steps_json: String = row.try_get("steps")?;
            let learned_from_json: String = row.try_get("learned_from")?;
            let tags_json: String = row.try_get("tags")?;

            sequences.push(mockforge_core::behavioral_cloning::BehavioralSequence {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                steps: serde_json::from_str(&steps_json)?,
                frequency: row.try_get("frequency")?,
                confidence: row.try_get("confidence")?,
                learned_from: serde_json::from_str(&learned_from_json).unwrap_or_default(),
                description: row.try_get("description")?,
                tags: serde_json::from_str(&tags_json).unwrap_or_default(),
            });
        }

        Ok(sequences)
    }

    /// Insert or update endpoint probability model
    pub async fn insert_endpoint_probability_model(
        &self,
        model: &mockforge_core::behavioral_cloning::EndpointProbabilityModel,
    ) -> Result<()> {
        let status_code_dist_json = serde_json::to_string(&model.status_code_distribution)?;
        let latency_dist_json = serde_json::to_string(&model.latency_distribution)?;
        let error_patterns_json = serde_json::to_string(&model.error_patterns)?;
        let payload_variations_json = serde_json::to_string(&model.payload_variations)?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO endpoint_probabilities (
                endpoint, method, status_code_distribution, latency_distribution,
                error_patterns, payload_variations, sample_count, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&model.endpoint)
        .bind(&model.method)
        .bind(&status_code_dist_json)
        .bind(&latency_dist_json)
        .bind(&error_patterns_json)
        .bind(&payload_variations_json)
        .bind(model.sample_count as i64)
        .bind(model.updated_at)
        .execute(&self.pool)
        .await?;

        debug!("Inserted probability model: {} {}", model.method, model.endpoint);
        Ok(())
    }

    /// Get endpoint probability model
    pub async fn get_endpoint_probability_model(
        &self,
        endpoint: &str,
        method: &str,
    ) -> Result<Option<mockforge_core::behavioral_cloning::EndpointProbabilityModel>> {
        let row = sqlx::query(
            r#"
            SELECT endpoint, method, status_code_distribution, latency_distribution,
                   error_patterns, payload_variations, sample_count, updated_at
            FROM endpoint_probabilities
            WHERE endpoint = ? AND method = ?
            "#,
        )
        .bind(endpoint)
        .bind(method)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let status_code_dist_json: String = row.try_get("status_code_distribution")?;
            let latency_dist_json: String = row.try_get("latency_distribution")?;
            let error_patterns_json: String = row.try_get("error_patterns")?;
            let payload_variations_json: String = row.try_get("payload_variations")?;

            Ok(Some(mockforge_core::behavioral_cloning::EndpointProbabilityModel {
                endpoint: row.try_get("endpoint")?,
                method: row.try_get("method")?,
                status_code_distribution: serde_json::from_str(&status_code_dist_json)?,
                latency_distribution: serde_json::from_str(&latency_dist_json)?,
                error_patterns: serde_json::from_str(&error_patterns_json).unwrap_or_default(),
                payload_variations: serde_json::from_str(&payload_variations_json).unwrap_or_default(),
                sample_count: row.try_get::<i64, _>("sample_count")? as u64,
                updated_at: row.try_get("updated_at")?,
                original_error_probabilities: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all endpoint probability models
    pub async fn get_all_endpoint_probability_models(
        &self,
    ) -> Result<Vec<mockforge_core::behavioral_cloning::EndpointProbabilityModel>> {
        let rows = sqlx::query(
            r#"
            SELECT endpoint, method, status_code_distribution, latency_distribution,
                   error_patterns, payload_variations, sample_count, updated_at
            FROM endpoint_probabilities
            ORDER BY sample_count DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut models = Vec::new();
        for row in rows {
            use sqlx::Row;
            let status_code_dist_json: String = row.try_get("status_code_distribution")?;
            let latency_dist_json: String = row.try_get("latency_distribution")?;
            let error_patterns_json: String = row.try_get("error_patterns")?;
            let payload_variations_json: String = row.try_get("payload_variations")?;

            models.push(mockforge_core::behavioral_cloning::EndpointProbabilityModel {
                endpoint: row.try_get("endpoint")?,
                method: row.try_get("method")?,
                status_code_distribution: serde_json::from_str(&status_code_dist_json)?,
                latency_distribution: serde_json::from_str(&latency_dist_json)?,
                error_patterns: serde_json::from_str(&error_patterns_json).unwrap_or_default(),
                payload_variations: serde_json::from_str(&payload_variations_json).unwrap_or_default(),
                sample_count: row.try_get::<i64, _>("sample_count")? as u64,
                original_error_probabilities: None,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(models)
    }

    /// Get requests grouped by trace_id for sequence learning
    pub async fn get_requests_by_trace(
        &self,
        min_requests_per_trace: Option<i32>,
    ) -> Result<Vec<(String, Vec<RecordedRequest>)>> {
        // Get all requests with trace_id, ordered by trace_id and timestamp
        let requests = sqlx::query_as::<_, RecordedRequest>(
            r#"
            SELECT id, protocol, timestamp, method, path, query_params,
                   headers, body, body_encoding, client_ip, trace_id, span_id,
                   duration_ms, status_code, tags
            FROM requests
            WHERE trace_id IS NOT NULL AND trace_id != ''
            ORDER BY trace_id, timestamp ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        // Group by trace_id
        let mut grouped: std::collections::HashMap<String, Vec<RecordedRequest>> = std::collections::HashMap::new();
        for request in requests {
            if let Some(trace_id) = &request.trace_id {
                grouped.entry(trace_id.clone()).or_insert_with(Vec::new).push(request);
            }
        }

        // Filter by minimum requests per trace if specified
        let mut result: Vec<(String, Vec<RecordedRequest>)> = grouped
            .into_iter()
            .filter(|(_, requests)| {
                min_requests_per_trace.map_or(true, |min| requests.len() >= min as usize)
            })
            .collect();

        // Sort by trace_id for consistency
        result.sort_by_key(|(trace_id, _)| trace_id.clone());

        Ok(result)
    }

    /// Get requests and responses for a specific endpoint and method
    ///
    /// Returns a list of (request, response) pairs for building probability models.
    pub async fn get_exchanges_for_endpoint(
        &self,
        endpoint: &str,
        method: &str,
        limit: Option<i32>,
    ) -> Result<Vec<(RecordedRequest, Option<RecordedResponse>)>> {
        let limit = limit.unwrap_or(10000);
        let requests = sqlx::query_as::<_, RecordedRequest>(
            r#"
            SELECT id, protocol, timestamp, method, path, query_params,
                   headers, body, body_encoding, client_ip, trace_id, span_id,
                   duration_ms, status_code, tags
            FROM requests
            WHERE path = ? AND method = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(endpoint)
        .bind(method)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut exchanges = Vec::new();
        for request in requests {
            let response = self.get_response(&request.id).await?;
            exchanges.push((request, response));
        }

        Ok(exchanges)
    }

    /// Create a new flow
    pub async fn create_flow(
        &self,
        flow_id: &str,
        name: Option<&str>,
        description: Option<&str>,
        tags: &[String],
    ) -> Result<()> {
        let tags_json = serde_json::to_string(tags)?;
        let metadata_json = serde_json::to_string(&std::collections::HashMap::<String, serde_json::Value>::new())?;

        sqlx::query(
            r#"
            INSERT INTO flows (id, name, description, created_at, tags, metadata)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(flow_id)
        .bind(name)
        .bind(description)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&tags_json)
        .bind(&metadata_json)
        .execute(&self.pool)
        .await?;

        debug!("Created flow: {}", flow_id);
        Ok(())
    }

    /// Add a step to a flow
    pub async fn add_flow_step(
        &self,
        flow_id: &str,
        request_id: &str,
        step_index: usize,
        step_label: Option<&str>,
        timing_ms: Option<u64>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO flow_steps (flow_id, step_index, request_id, step_label, timing_ms)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(flow_id)
        .bind(step_index as i64)
        .bind(request_id)
        .bind(step_label)
        .bind(timing_ms.map(|t| t as i64))
        .execute(&self.pool)
        .await?;

        debug!("Added step {} to flow {}", step_index, flow_id);
        Ok(())
    }

    /// Get the number of steps in a flow
    pub async fn get_flow_step_count(&self, flow_id: &str) -> Result<usize> {
        let count: Option<i64> = sqlx::query_scalar(
            "SELECT COUNT(*) FROM flow_steps WHERE flow_id = ?",
        )
        .bind(flow_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(count.unwrap_or(0) as usize)
    }

    /// Get flow steps
    pub async fn get_flow_steps(&self, flow_id: &str) -> Result<Vec<FlowStepRow>> {
        let rows = sqlx::query_as::<_, FlowStepRow>(
            r#"
            SELECT request_id, step_index, step_label, timing_ms
            FROM flow_steps
            WHERE flow_id = ?
            ORDER BY step_index ASC
            "#,
        )
        .bind(flow_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get flow metadata
    pub async fn get_flow_metadata(&self, flow_id: &str) -> Result<Option<FlowMetadataRow>> {
        let row = sqlx::query_as::<_, FlowMetadataRow>(
            r#"
            SELECT id, name, description, created_at, tags, metadata
            FROM flows
            WHERE id = ?
            "#,
        )
        .bind(flow_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// List flows
    pub async fn list_flows(&self, limit: Option<i64>) -> Result<Vec<FlowMetadataRow>> {
        let limit = limit.unwrap_or(100);
        let rows = sqlx::query_as::<_, FlowMetadataRow>(
            r#"
            SELECT id, name, description, created_at, tags, metadata
            FROM flows
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Update flow metadata
    pub async fn update_flow_metadata(
        &self,
        flow_id: &str,
        name: Option<&str>,
        description: Option<&str>,
        tags: Option<&[String]>,
    ) -> Result<()> {
        if let Some(tags) = tags {
            let tags_json = serde_json::to_string(tags)?;
            sqlx::query(
                r#"
                UPDATE flows
                SET name = COALESCE(?, name),
                    description = COALESCE(?, description),
                    tags = ?
                WHERE id = ?
                "#,
            )
            .bind(name)
            .bind(description)
            .bind(&tags_json)
            .bind(flow_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE flows
                SET name = COALESCE(?, name),
                    description = COALESCE(?, description)
                WHERE id = ?
                "#,
            )
            .bind(name)
            .bind(description)
            .bind(flow_id)
            .execute(&self.pool)
            .await?;
        }

        info!("Updated flow metadata: {}", flow_id);
        Ok(())
    }

    /// Delete a flow
    pub async fn delete_flow(&self, flow_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM flows WHERE id = ?")
            .bind(flow_id)
            .execute(&self.pool)
            .await?;

        info!("Deleted flow: {}", flow_id);
        Ok(())
    }

    /// Store a behavioral scenario
    pub async fn store_scenario(
        &self,
        scenario: &crate::behavioral_cloning::BehavioralScenario,
        version: &str,
    ) -> Result<()> {
        let scenario_json = serde_json::to_string(scenario)?;
        let metadata_json = serde_json::to_string(&scenario.metadata)?;
        let tags_json = serde_json::to_string(&scenario.tags)?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO scenarios (
                id, name, version, description, scenario_data, metadata, updated_at, tags
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&scenario.id)
        .bind(&scenario.name)
        .bind(version)
        .bind(&scenario.description)
        .bind(&scenario_json)
        .bind(&metadata_json)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&tags_json)
        .execute(&self.pool)
        .await?;

        debug!("Stored scenario: {} v{}", scenario.id, version);
        Ok(())
    }

    /// Get a scenario by ID
    pub async fn get_scenario(
        &self,
        scenario_id: &str,
    ) -> Result<Option<crate::behavioral_cloning::BehavioralScenario>> {
        let row = sqlx::query(
            r#"
            SELECT scenario_data
            FROM scenarios
            WHERE id = ?
            ORDER BY version DESC
            LIMIT 1
            "#,
        )
        .bind(scenario_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let scenario_json: String = row.try_get("scenario_data")?;
            let scenario: crate::behavioral_cloning::BehavioralScenario =
                serde_json::from_str(&scenario_json)?;
            Ok(Some(scenario))
        } else {
            Ok(None)
        }
    }

    /// Get a scenario by name and version
    pub async fn get_scenario_by_name_version(
        &self,
        name: &str,
        version: &str,
    ) -> Result<Option<crate::behavioral_cloning::BehavioralScenario>> {
        let row = sqlx::query(
            r#"
            SELECT scenario_data
            FROM scenarios
            WHERE name = ? AND version = ?
            "#,
        )
        .bind(name)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let scenario_json: String = row.try_get("scenario_data")?;
            let scenario: crate::behavioral_cloning::BehavioralScenario =
                serde_json::from_str(&scenario_json)?;
            Ok(Some(scenario))
        } else {
            Ok(None)
        }
    }

    /// List all scenarios
    pub async fn list_scenarios(
        &self,
        limit: Option<i64>,
    ) -> Result<Vec<ScenarioMetadataRow>> {
        let limit = limit.unwrap_or(100);
        let rows = sqlx::query_as::<_, ScenarioMetadataRow>(
            r#"
            SELECT id, name, version, description, created_at, updated_at, tags
            FROM scenarios
            ORDER BY name, version DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Delete a scenario
    pub async fn delete_scenario(&self, scenario_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM scenarios WHERE id = ?")
            .bind(scenario_id)
            .execute(&self.pool)
            .await?;

        info!("Deleted scenario: {}", scenario_id);
        Ok(())
    }
}

/// Flow step row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FlowStepRow {
    pub request_id: String,
    #[sqlx(rename = "step_index")]
    pub step_index: i64,
    pub step_label: Option<String>,
    pub timing_ms: Option<i64>,
}

/// Flow metadata row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FlowMetadataRow {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    #[sqlx(rename = "created_at")]
    pub created_at: String,
    pub tags: String,
    pub metadata: String,
}

/// Scenario metadata row from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScenarioMetadataRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    #[sqlx(rename = "created_at")]
    pub created_at: String,
    #[sqlx(rename = "updated_at")]
    pub updated_at: String,
    pub tags: String,
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
