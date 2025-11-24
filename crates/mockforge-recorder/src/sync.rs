//! Automatic sync/polling for detecting upstream API changes
//!
//! This module provides functionality to periodically poll upstream APIs,
//! compare responses with recorded fixtures, and detect changes.

use crate::{
    database::RecorderDatabase,
    diff::{ComparisonResult, ResponseComparator},
    Result,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// GitOps configuration for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsConfig {
    /// Whether GitOps mode is enabled
    pub enabled: bool,
    /// PR provider (GitHub or GitLab)
    pub pr_provider: String, // "github" or "gitlab"
    /// Repository owner/org
    pub repo_owner: String,
    /// Repository name
    pub repo_name: String,
    /// Base branch (default: main)
    #[serde(default = "default_main_branch")]
    pub base_branch: String,
    /// Whether to update fixture files
    #[serde(default = "default_true")]
    pub update_fixtures: bool,
    /// Whether to regenerate SDKs
    #[serde(default)]
    pub regenerate_sdks: bool,
    /// Whether to update OpenAPI specs
    #[serde(default = "default_true")]
    pub update_docs: bool,
    /// Whether to auto-merge PRs
    #[serde(default)]
    pub auto_merge: bool,
    /// Authentication token (GitHub PAT or GitLab token)
    #[serde(skip_serializing)]
    pub token: Option<String>,
}

fn default_main_branch() -> String {
    "main".to_string()
}

/// Traffic-aware sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficAwareConfig {
    /// Whether traffic-aware sync is enabled
    pub enabled: bool,
    /// Minimum request count threshold (only sync if > N requests)
    pub min_requests_threshold: Option<usize>,
    /// Top percentage threshold (sync top X% of endpoints)
    pub top_percentage: Option<f64>,
    /// Lookback window in days for usage statistics
    #[serde(default = "default_lookback_days")]
    pub lookback_days: u64,
    /// Whether to sync endpoints with high reality ratio (mostly real)
    #[serde(default)]
    pub sync_real_endpoints: bool,
    /// Weight for request count in priority calculation
    #[serde(default = "default_count_weight")]
    pub weight_count: f64,
    /// Weight for recency in priority calculation
    #[serde(default = "default_recency_weight")]
    pub weight_recency: f64,
    /// Weight for reality ratio in priority calculation
    #[serde(default = "default_reality_weight")]
    pub weight_reality: f64,
}

fn default_lookback_days() -> u64 {
    7
}

fn default_count_weight() -> f64 {
    1.0
}

fn default_recency_weight() -> f64 {
    0.5
}

fn default_reality_weight() -> f64 {
    -0.3
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Whether sync is enabled
    pub enabled: bool,
    /// Upstream base URL to sync from
    pub upstream_url: Option<String>,
    /// Sync interval in seconds
    pub interval_seconds: u64,
    /// Whether to automatically update fixtures when changes detected
    pub auto_update: bool,
    /// Maximum number of requests to sync per interval
    pub max_requests_per_sync: usize,
    /// Timeout for sync requests in seconds
    pub request_timeout_seconds: u64,
    /// Headers to add to sync requests
    pub headers: HashMap<String, String>,
    /// Only sync GET requests (default: true)
    #[serde(default = "default_true")]
    pub sync_get_only: bool,
    /// GitOps configuration (optional)
    pub gitops_mode: Option<GitOpsConfig>,
    /// Traffic-aware sync configuration (optional)
    pub traffic_aware: Option<TrafficAwareConfig>,
}

fn default_true() -> bool {
    true
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            upstream_url: None,
            interval_seconds: 3600, // 1 hour default
            auto_update: false,
            max_requests_per_sync: 100,
            request_timeout_seconds: 30,
            headers: HashMap::new(),
            sync_get_only: true,
            gitops_mode: None,
            traffic_aware: None,
        }
    }
}

/// Sync status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// Whether sync is currently running
    pub is_running: bool,
    /// Last sync timestamp
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    /// Number of changes detected in last sync
    pub last_changes_detected: usize,
    /// Number of fixtures updated in last sync
    pub last_fixtures_updated: usize,
    /// Last sync error (if any)
    pub last_error: Option<String>,
    /// Total syncs performed
    pub total_syncs: u64,
}

/// Detected change in an API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedChange {
    /// Request ID from database
    pub request_id: String,
    /// Request method
    pub method: String,
    /// Request path
    pub path: String,
    /// Comparison result
    pub comparison: ComparisonResult,
    /// Whether fixture was updated
    pub updated: bool,
}

/// Sync service for polling upstream APIs and detecting changes
pub struct SyncService {
    config: Arc<RwLock<SyncConfig>>,
    database: Arc<RecorderDatabase>,
    status: Arc<RwLock<SyncStatus>>,
    http_client: Client,
}

impl SyncService {
    /// Create a new sync service
    pub fn new(config: SyncConfig, database: Arc<RecorderDatabase>) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config: Arc::new(RwLock::new(config)),
            database,
            status: Arc::new(RwLock::new(SyncStatus {
                is_running: false,
                last_sync: None,
                last_changes_detected: 0,
                last_fixtures_updated: 0,
                last_error: None,
                total_syncs: 0,
            })),
            http_client,
        }
    }

    /// Start the sync service (runs in background)
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let config = Arc::clone(&self.config);
        let database = Arc::clone(&self.database);
        let status = Arc::clone(&self.status);
        let http_client = self.http_client.clone();

        tokio::spawn(async move {
            let mut interval_timer =
                interval(Duration::from_secs(config.read().await.interval_seconds));
            interval_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                interval_timer.tick().await;

                let config_guard = config.read().await;
                if !config_guard.enabled {
                    continue;
                }

                let upstream_url = match &config_guard.upstream_url {
                    Some(url) => url.clone(),
                    None => {
                        warn!("Sync enabled but no upstream_url configured");
                        continue;
                    }
                };

                let auto_update = config_guard.auto_update;
                let max_requests = config_guard.max_requests_per_sync;
                let sync_get_only = config_guard.sync_get_only;
                let headers = config_guard.headers.clone();
                drop(config_guard);

                // Update status
                {
                    let mut status_guard = status.write().await;
                    status_guard.is_running = true;
                }

                info!("Starting automatic sync from upstream: {}", upstream_url);

                let config_guard = config.read().await;
                let traffic_analyzer = config_guard
                    .traffic_aware
                    .as_ref()
                    .map(|ta_config| crate::sync_traffic::TrafficAnalyzer::new(ta_config.clone()));
                drop(config_guard);

                match Self::sync_once(
                    &http_client,
                    &database,
                    &upstream_url,
                    auto_update,
                    max_requests,
                    sync_get_only,
                    &headers,
                    traffic_analyzer.as_ref(),
                    None, // Continuum engine not available in background sync yet
                )
                .await
                {
                    Ok((changes, updated)) => {
                        let mut status_guard = status.write().await;
                        status_guard.is_running = false;
                        status_guard.last_sync = Some(chrono::Utc::now());
                        status_guard.last_changes_detected = changes.len();
                        status_guard.last_fixtures_updated = updated;
                        status_guard.last_error = None;
                        status_guard.total_syncs += 1;

                        if !changes.is_empty() {
                            info!(
                                "Sync complete: {} changes detected, {} fixtures updated",
                                changes.len(),
                                updated
                            );
                        } else {
                            debug!("Sync complete: No changes detected");
                        }
                    }
                    Err(e) => {
                        let mut status_guard = status.write().await;
                        status_guard.is_running = false;
                        status_guard.last_error = Some(e.to_string());
                        warn!("Sync failed: {}", e);
                    }
                }
            }
        })
    }

    /// Perform a single sync operation
    async fn sync_once(
        http_client: &Client,
        database: &RecorderDatabase,
        upstream_url: &str,
        auto_update: bool,
        max_requests: usize,
        sync_get_only: bool,
        headers: &HashMap<String, String>,
        traffic_analyzer: Option<&crate::sync_traffic::TrafficAnalyzer>,
        continuum_engine: Option<
            &mockforge_core::reality_continuum::engine::RealityContinuumEngine,
        >,
    ) -> Result<(Vec<DetectedChange>, usize)> {
        // Generate sync cycle ID for grouping snapshots from this sync operation
        let sync_cycle_id = format!("sync_{}", Uuid::new_v4());

        // Get recent recorded requests
        let mut recorded_requests = database.list_recent(max_requests as i32).await?;

        // Apply traffic-aware filtering if enabled
        if let Some(analyzer) = traffic_analyzer {
            // Aggregate usage stats from database
            let usage_stats = analyzer.aggregate_usage_stats_from_db(database).await;

            // Get endpoint list for reality ratio lookup
            let endpoints: Vec<(&str, &str)> =
                recorded_requests.iter().map(|r| (r.method.as_str(), r.path.as_str())).collect();

            // Get reality ratios
            let reality_ratios = analyzer.get_reality_ratios(&endpoints, continuum_engine).await;

            // Calculate priorities
            let priorities = analyzer.calculate_priorities(&usage_stats, &reality_ratios);

            // Filter requests based on priorities
            let prioritized_endpoints: std::collections::HashSet<String> =
                priorities.iter().map(|p| format!("{} {}", p.method, p.endpoint)).collect();

            recorded_requests.retain(|req| {
                let key = format!("{} {}", req.method, req.path);
                prioritized_endpoints.contains(&key)
            });

            debug!(
                "Traffic-aware filtering: {} requests after filtering (from {} total)",
                recorded_requests.len(),
                max_requests
            );
        }

        let mut changes = Vec::new();
        let mut updated_count = 0;

        for request in recorded_requests {
            // Skip non-GET requests if configured
            if sync_get_only && request.method.to_uppercase() != "GET" {
                continue;
            }

            // Build full URL
            let full_url =
                if request.path.starts_with("http://") || request.path.starts_with("https://") {
                    request.path.clone()
                } else {
                    format!("{}{}", upstream_url.trim_end_matches('/'), request.path)
                };

            // Replay the request to upstream
            match Self::replay_to_upstream(
                http_client,
                &full_url,
                &request.method,
                &request.headers,
                headers,
            )
            .await
            {
                Ok((status, response_headers, response_body)) => {
                    // Get original exchange
                    if let Ok(Some(exchange)) = database.get_exchange(&request.id).await {
                        if let Some(original_response) = exchange.response {
                            let original_headers = original_response.headers_map();
                            let original_body =
                                original_response.decoded_body().unwrap_or_default();

                            // Compare responses
                            let comparison = ResponseComparator::compare(
                                original_response.status_code,
                                &original_headers,
                                &original_body,
                                status as i32,
                                &response_headers,
                                &response_body,
                            );

                            if !comparison.matches {
                                debug!(
                                    "Change detected for {} {}: {} differences",
                                    request.method,
                                    request.path,
                                    comparison.differences.len()
                                );

                                // Create snapshot before updating fixture (Shadow Snapshot Mode)
                                let snapshot_before = crate::sync_snapshots::SnapshotData {
                                    status_code: original_response.status_code as u16,
                                    headers: original_headers.clone(),
                                    body: original_body.clone(),
                                    body_json: serde_json::from_slice(&original_body).ok(),
                                };

                                let snapshot_after = crate::sync_snapshots::SnapshotData {
                                    status_code: status,
                                    headers: response_headers.clone(),
                                    body: response_body.clone(),
                                    body_json: serde_json::from_slice(&response_body).ok(),
                                };

                                let snapshot = crate::sync_snapshots::SyncSnapshot::new(
                                    request.path.clone(),
                                    request.method.clone(),
                                    sync_cycle_id.clone(),
                                    snapshot_before,
                                    snapshot_after,
                                    comparison.clone(),
                                    request.duration_ms.map(|d| d as u64),
                                    None, // Response time after sync not available yet
                                );

                                // Store snapshot in database
                                if let Err(e) = database.insert_sync_snapshot(&snapshot).await {
                                    warn!(
                                        "Failed to store snapshot for {} {}: {}",
                                        request.method, request.path, e
                                    );
                                }

                                let mut updated = false;
                                if auto_update {
                                    // Update the fixture with new response
                                    match Self::update_fixture(
                                        database,
                                        &request.id,
                                        status,
                                        &response_headers,
                                        &response_body,
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            updated = true;
                                            updated_count += 1;
                                            info!(
                                                "Updated fixture for {} {}",
                                                request.method, request.path
                                            );
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Failed to update fixture for {} {}: {}",
                                                request.method, request.path, e
                                            );
                                        }
                                    }
                                }

                                changes.push(DetectedChange {
                                    request_id: request.id.clone(),
                                    method: request.method.clone(),
                                    path: request.path.clone(),
                                    comparison,
                                    updated,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!(
                        "Failed to replay {} {} to upstream: {}",
                        request.method, request.path, e
                    );
                    // Continue with other requests
                }
            }
        }

        Ok((changes, updated_count))
    }

    /// Perform sync with GitOps integration
    pub async fn sync_with_gitops(
        &self,
        gitops_handler: Option<&crate::sync_gitops::GitOpsSyncHandler>,
    ) -> Result<(Vec<DetectedChange>, usize, Option<mockforge_core::pr_generation::PRResult>)> {
        self.sync_with_gitops_and_drift(
            gitops_handler,
            None, // drift_evaluator
        )
        .await
    }

    /// Perform sync with GitOps and drift budget evaluation
    pub async fn sync_with_gitops_and_drift(
        &self,
        gitops_handler: Option<&crate::sync_gitops::GitOpsSyncHandler>,
        drift_evaluator: Option<&crate::sync_drift::SyncDriftEvaluator>,
    ) -> Result<(Vec<DetectedChange>, usize, Option<mockforge_core::pr_generation::PRResult>)> {
        let config = self.config.read().await.clone();
        let upstream_url = config.upstream_url.ok_or_else(|| {
            crate::RecorderError::InvalidFilter("No upstream_url configured".to_string())
        })?;

        {
            let mut status = self.status.write().await;
            status.is_running = true;
        }

        // Generate sync cycle ID
        let sync_cycle_id = format!("sync_{}", Uuid::new_v4());

        let traffic_analyzer = config
            .traffic_aware
            .as_ref()
            .map(|ta_config| crate::sync_traffic::TrafficAnalyzer::new(ta_config.clone()));

        let result = Self::sync_once(
            &self.http_client,
            &self.database,
            &upstream_url,
            false, // Don't auto-update when GitOps is enabled
            config.max_requests_per_sync,
            config.sync_get_only,
            &config.headers,
            traffic_analyzer.as_ref(),
            None, // Continuum engine not available in sync_with_gitops yet
        )
        .await;

        let (changes, _updated_count) = match &result {
            Ok((c, u)) => (c.clone(), *u),
            Err(_) => (Vec::new(), 0),
        };

        // Process changes with GitOps if enabled
        let pr_result = if let Some(handler) = gitops_handler {
            handler
                .process_sync_changes(&self.database, &changes, &sync_cycle_id)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        // Evaluate drift budgets and create incidents if enabled
        if let Some(evaluator) = drift_evaluator {
            if let Err(e) = evaluator
                .evaluate_sync_changes(&changes, &sync_cycle_id, None, None, None)
                .await
            {
                warn!("Failed to evaluate drift budgets for sync changes: {}", e);
            }
        }

        {
            let mut status = self.status.write().await;
            status.is_running = false;
            match &result {
                Ok((changes, updated)) => {
                    status.last_sync = Some(chrono::Utc::now());
                    status.last_changes_detected = changes.len();
                    status.last_fixtures_updated = *updated;
                    status.last_error = None;
                    status.total_syncs += 1;
                }
                Err(e) => {
                    status.last_error = Some(e.to_string());
                }
            }
        }

        match result {
            Ok((changes, updated)) => Ok((changes, updated, pr_result)),
            Err(e) => Err(e),
        }
    }

    /// Replay a request to the upstream URL
    async fn replay_to_upstream(
        http_client: &Client,
        url: &str,
        method: &str,
        original_headers: &str,
        additional_headers: &HashMap<String, String>,
    ) -> Result<(u16, HashMap<String, String>, Vec<u8>)> {
        // Parse original headers
        let mut headers_map = HashMap::new();
        if let Ok(json) = serde_json::from_str::<HashMap<String, String>>(original_headers) {
            headers_map = json;
        }

        // Add additional headers (merge)
        for (key, value) in additional_headers {
            headers_map.insert(key.clone(), value.clone());
        }

        // Build request
        let reqwest_method = match method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            "HEAD" => reqwest::Method::HEAD,
            "OPTIONS" => reqwest::Method::OPTIONS,
            _ => {
                return Err(crate::RecorderError::InvalidFilter(format!(
                    "Unsupported method: {}",
                    method
                )))
            }
        };

        let mut request_builder = http_client.request(reqwest_method, url);

        // Add headers
        for (key, value) in &headers_map {
            if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
                if let Ok(header_value) = reqwest::header::HeaderValue::from_str(value) {
                    request_builder = request_builder.header(header_name, header_value);
                }
            }
        }

        // Execute request
        let response = request_builder
            .send()
            .await
            .map_err(|e| crate::RecorderError::InvalidFilter(format!("Request failed: {}", e)))?;

        let status = response.status().as_u16();
        let mut response_headers = HashMap::new();

        for (key, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                response_headers.insert(key.to_string(), value_str.to_string());
            }
        }

        let response_body = response
            .bytes()
            .await
            .map_err(|e| {
                crate::RecorderError::InvalidFilter(format!("Failed to read response body: {}", e))
            })?
            .to_vec();

        Ok((status, response_headers, response_body))
    }

    /// Update a fixture with new response data
    async fn update_fixture(
        database: &RecorderDatabase,
        request_id: &str,
        status_code: u16,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<()> {
        // Update the response in the database
        let headers_json = serde_json::to_string(headers)?;
        let body_encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, body);
        let body_size = body.len() as i64;

        database
            .update_response(
                request_id,
                status_code as i32,
                &headers_json,
                &body_encoded,
                body_size,
            )
            .await?;

        Ok(())
    }

    /// Get current sync status
    pub async fn get_status(&self) -> SyncStatus {
        self.status.read().await.clone()
    }

    /// Get sync configuration
    pub async fn get_config(&self) -> SyncConfig {
        self.config.read().await.clone()
    }

    /// Update sync configuration
    pub async fn update_config(&self, new_config: SyncConfig) {
        *self.config.write().await = new_config;
    }

    /// Manually trigger a sync
    pub async fn sync_now(&self) -> Result<(Vec<DetectedChange>, usize)> {
        let config = self.config.read().await.clone();
        let upstream_url = config.upstream_url.ok_or_else(|| {
            crate::RecorderError::InvalidFilter("No upstream_url configured".to_string())
        })?;

        {
            let mut status = self.status.write().await;
            status.is_running = true;
        }

        let traffic_analyzer = config
            .traffic_aware
            .as_ref()
            .map(|ta_config| crate::sync_traffic::TrafficAnalyzer::new(ta_config.clone()));

        let result = Self::sync_once(
            &self.http_client,
            &self.database,
            &upstream_url,
            config.auto_update,
            config.max_requests_per_sync,
            config.sync_get_only,
            &config.headers,
            traffic_analyzer.as_ref(),
            None, // Continuum engine not available in sync_now yet
        )
        .await;

        {
            let mut status = self.status.write().await;
            status.is_running = false;
            match &result {
                Ok((changes, updated)) => {
                    status.last_sync = Some(chrono::Utc::now());
                    status.last_changes_detected = changes.len();
                    status.last_fixtures_updated = *updated;
                    status.last_error = None;
                    status.total_syncs += 1;
                }
                Err(e) => {
                    status.last_error = Some(e.to_string());
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.interval_seconds, 3600);
        assert!(!config.auto_update);
        assert_eq!(config.max_requests_per_sync, 100);
    }

    #[test]
    fn test_sync_status_creation() {
        let status = SyncStatus {
            is_running: false,
            last_sync: None,
            last_changes_detected: 0,
            last_fixtures_updated: 0,
            last_error: None,
            total_syncs: 0,
        };

        assert!(!status.is_running);
        assert_eq!(status.total_syncs, 0);
    }
}
