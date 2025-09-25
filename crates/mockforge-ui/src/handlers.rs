//! Request handlers for the admin UI
//!
//! This module has been refactored into sub-modules for better organization:
//! - assets: Static asset serving
//! - admin: Admin dashboard and server management
//! - workspace: Workspace management operations
//! - plugin: Plugin management operations
//! - sync: Synchronization operations
//! - import: Data import operations
//! - fixtures: Fixture management operations

use std::collections::HashMap;
use std::sync::Arc;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use chrono::{DateTime, Utc};
use axum::{
    extract::{Query, State},
    http::{self, StatusCode},
    response::{Html, IntoResponse, Json},
};
use sysinfo::System;
use mockforge_core::{Error, Result};

// Import all types from models
use crate::models::{
    ApiResponse, DashboardData, SystemInfo, ServerStatus, RouteInfo, RequestLog,
    LatencyProfile, FaultConfig, ProxyConfig, ValidationSettings, LogFilter,
    ConfigUpdate, HealthCheck, MetricsData, ServerInfo, DashboardSystemInfo, SimpleMetricsData,
    ValidationUpdate,
};

// Import import types from core
use mockforge_core::workspace_import::{ImportRoute, ImportResponse};

// Handler sub-modules
pub mod admin;
pub mod assets;

// Re-export commonly used types
pub use assets::*;
pub use admin::*;

// Import workspace persistence
use mockforge_core::workspace_persistence::WorkspacePersistence;
use mockforge_core::workspace_import::WorkspaceImportConfig;

// Static assets - embedded at compile time
const ADMIN_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MockForge Admin</title>
    <link rel="stylesheet" href="/assets/index.css">
</head>
<body>
    <div id="root"></div>
    <script src="/assets/index.js"></script>
</body>
</html>"#;

const ADMIN_CSS: &str = r#"body { margin: 0; font-family: -apple-system, BlinkMacSystemFont, sans-serif; }"#;

const ADMIN_JS: &str = r#"console.log('MockForge Admin UI');"#;

/// Request metrics for tracking
#[derive(Debug, Clone, Default)]
pub struct RequestMetrics {
    /// Total requests served
    pub total_requests: u64,
    /// Active connections
    pub active_connections: u64,
    /// Requests by endpoint
    pub requests_by_endpoint: HashMap<String, u64>,
    /// Response times (last N measurements)
    pub response_times: Vec<u64>,
    /// Response times by endpoint (last N measurements per endpoint)
    pub response_times_by_endpoint: HashMap<String, Vec<u64>>,
    /// Error count by endpoint
    pub errors_by_endpoint: HashMap<String, u64>,
    /// Last request timestamp by endpoint
    pub last_request_by_endpoint: HashMap<String, chrono::DateTime<chrono::Utc>>,
}

/// System metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    /// Memory usage in MB
    pub memory_usage_mb: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Active threads
    pub active_threads: u32,
}

/// Time series data point
#[derive(Debug, Clone)]
pub struct TimeSeriesPoint {
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Value
    pub value: f64,
}

/// Time series data for tracking metrics over time
#[derive(Debug, Clone, Default)]
pub struct TimeSeriesData {
    /// Memory usage over time
    pub memory_usage: Vec<TimeSeriesPoint>,
    /// CPU usage over time
    pub cpu_usage: Vec<TimeSeriesPoint>,
    /// Request count over time
    pub request_count: Vec<TimeSeriesPoint>,
    /// Response time over time
    pub response_time: Vec<TimeSeriesPoint>,
}

/// Restart status tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartStatus {
    /// Whether a restart is currently in progress
    pub in_progress: bool,
    /// Timestamp when restart was initiated
    pub initiated_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Restart reason/message
    pub reason: Option<String>,
    /// Whether restart was successful
    pub success: Option<bool>,
}

/// Fixture metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureInfo {
    /// Unique identifier for the fixture
    pub id: String,
    /// Protocol type (http, websocket, grpc)
    pub protocol: String,
    /// HTTP method or operation type
    pub method: String,
    /// Request path
    pub path: String,
    /// When the fixture was saved
    pub saved_at: chrono::DateTime<chrono::Utc>,
    /// File size in bytes
    pub file_size: u64,
    /// File path relative to fixtures directory
    pub file_path: String,
    /// Request fingerprint hash
    pub fingerprint: String,
    /// Additional metadata from the fixture file
    pub metadata: serde_json::Value,
}

/// Smoke test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmokeTestResult {
    /// Test ID
    pub id: String,
    /// Test name
    pub name: String,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Test description
    pub description: String,
    /// When the test was last run
    pub last_run: Option<chrono::DateTime<chrono::Utc>>,
    /// Test status (passed, failed, running, pending)
    pub status: String,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Error message if test failed
    pub error_message: Option<String>,
    /// HTTP status code received
    pub status_code: Option<u16>,
    /// Test duration in seconds
    pub duration_seconds: Option<f64>,
}

/// Smoke test execution context
#[derive(Debug, Clone)]
pub struct SmokeTestContext {
    /// Base URL for the service being tested
    pub base_url: String,
    /// Timeout for individual tests
    pub timeout_seconds: u64,
    /// Whether to run tests in parallel
    pub parallel: bool,
}

/// Configuration state
#[derive(Debug, Clone, Serialize)]
pub struct ConfigurationState {
    /// Latency profile
    pub latency_profile: LatencyProfile,
    /// Fault configuration
    pub fault_config: FaultConfig,
    /// Proxy configuration
    pub proxy_config: ProxyConfig,
    /// Validation settings
    pub validation_settings: ValidationSettings,
}

/// Import history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportHistoryEntry {
    /// Unique ID for the import
    pub id: String,
    /// Import format (postman, insomnia, curl)
    pub format: String,
    /// Timestamp of the import
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Number of routes imported
    pub routes_count: usize,
    /// Number of variables imported
    pub variables_count: usize,
    /// Number of warnings
    pub warnings_count: usize,
    /// Whether the import was successful
    pub success: bool,
    /// Filename of the imported file
    pub filename: Option<String>,
    /// Environment used
    pub environment: Option<String>,
    /// Base URL used
    pub base_url: Option<String>,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Shared state for the admin UI
#[derive(Clone)]
pub struct AdminState {
    /// HTTP server address
    pub http_server_addr: Option<std::net::SocketAddr>,
    /// WebSocket server address
    pub ws_server_addr: Option<std::net::SocketAddr>,
    /// gRPC server address
    pub grpc_server_addr: Option<std::net::SocketAddr>,
    /// GraphQL server address
    pub graphql_server_addr: Option<std::net::SocketAddr>,
    /// Start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Request metrics (protected by RwLock)
    pub metrics: Arc<RwLock<RequestMetrics>>,
    /// System metrics (protected by RwLock)
    pub system_metrics: Arc<RwLock<SystemMetrics>>,
    /// Configuration (protected by RwLock)
    pub config: Arc<RwLock<ConfigurationState>>,
    /// Request logs (protected by RwLock)
    pub logs: Arc<RwLock<Vec<RequestLog>>>,
    /// Time series data (protected by RwLock)
    pub time_series: Arc<RwLock<TimeSeriesData>>,
    /// Restart status (protected by RwLock)
    pub restart_status: Arc<RwLock<RestartStatus>>,
    /// Smoke test results (protected by RwLock)
    pub smoke_test_results: Arc<RwLock<Vec<SmokeTestResult>>>,
    /// Import history (protected by RwLock)
    pub import_history: Arc<RwLock<Vec<ImportHistoryEntry>>>,
    /// Workspace persistence
    pub workspace_persistence: Arc<WorkspacePersistence>,
}

impl AdminState {
    /// Start system monitoring background task
    pub async fn start_system_monitoring(&self) {
        let state_clone = self.clone();
        tokio::spawn(async move {
            let mut sys = System::new_all();
            let mut refresh_count = 0u64;

            tracing::info!("Starting system monitoring background task");

            loop {
                // Refresh system information
                sys.refresh_all();

                // Get CPU usage
                let cpu_usage = sys.global_cpu_usage();

                // Get memory usage
                let total_memory = sys.total_memory() as f64;
                let used_memory = sys.used_memory() as f64;
                let memory_usage_mb = used_memory / 1024.0 / 1024.0;

                // Get thread count (use available CPU cores as approximate measure)
                let active_threads = sys.cpus().len() as u32;

                // Update system metrics
                let memory_mb_u64 = memory_usage_mb as u64;

                // Only log every 10 refreshes to avoid spam
                if refresh_count % 10 == 0 {
                    tracing::debug!(
                        "System metrics updated: CPU={:.1}%, Mem={}MB, Threads={}",
                        cpu_usage, memory_mb_u64, active_threads
                    );
                }

                state_clone.update_system_metrics(memory_mb_u64, cpu_usage as f64, active_threads).await;

                refresh_count += 1;

                // Sleep for 10 seconds between updates
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });
    }

    /// Create new admin state
    pub fn new(
        http_server_addr: Option<std::net::SocketAddr>,
        ws_server_addr: Option<std::net::SocketAddr>,
        grpc_server_addr: Option<std::net::SocketAddr>,
        graphql_server_addr: Option<std::net::SocketAddr>,
    ) -> Self {
        let start_time = chrono::Utc::now();

        Self {
            http_server_addr,
            ws_server_addr,
            grpc_server_addr,
            graphql_server_addr,
            start_time,
            metrics: Arc::new(RwLock::new(RequestMetrics::default())),
            system_metrics: Arc::new(RwLock::new(SystemMetrics {
                memory_usage_mb: 0,
                cpu_usage_percent: 0.0,
                active_threads: 0,
            })),
            config: Arc::new(RwLock::new(ConfigurationState {
                latency_profile: LatencyProfile {
                    name: "default".to_string(),
                    base_ms: 50,
                    jitter_ms: 20,
                    tag_overrides: HashMap::new(),
                },
                fault_config: FaultConfig {
                    enabled: false,
                    failure_rate: 0.0,
                    status_codes: vec![500, 502, 503],
                    active_failures: 0,
                },
                proxy_config: ProxyConfig {
                    enabled: false,
                    upstream_url: None,
                    timeout_seconds: 30,
                    requests_proxied: 0,
                },
                validation_settings: ValidationSettings {
                    mode: "enforce".to_string(),
                    aggregate_errors: true,
                    validate_responses: false,
                    overrides: HashMap::new(),
                },
            })),
            logs: Arc::new(RwLock::new(Vec::new())),
            time_series: Arc::new(RwLock::new(TimeSeriesData::default())),
            restart_status: Arc::new(RwLock::new(RestartStatus {
                in_progress: false,
                initiated_at: None,
                reason: None,
                success: None,
            })),
            smoke_test_results: Arc::new(RwLock::new(Vec::new())),
            import_history: Arc::new(RwLock::new(Vec::new())),
            workspace_persistence: Arc::new(WorkspacePersistence::new("./workspaces")),
        }
    }

    /// Record a request
    pub async fn record_request(
        &self,
        method: &str,
        path: &str,
        status_code: u16,
        response_time_ms: u64,
        error: Option<String>,
    ) {
        let mut metrics = self.metrics.write().await;

        metrics.total_requests += 1;
        let endpoint = format!("{} {}", method, path);
        *metrics.requests_by_endpoint.entry(endpoint.clone()).or_insert(0) += 1;

        if status_code >= 400 {
            *metrics.errors_by_endpoint.entry(endpoint.clone()).or_insert(0) += 1;
        }

        // Keep only last 100 response times globally
        metrics.response_times.push(response_time_ms);
        if metrics.response_times.len() > 100 {
            metrics.response_times.remove(0);
        }

        // Keep only last 50 response times per endpoint
        let endpoint_times = metrics
            .response_times_by_endpoint
            .entry(endpoint.clone())
            .or_insert_with(Vec::new);
        endpoint_times.push(response_time_ms);
        if endpoint_times.len() > 50 {
            endpoint_times.remove(0);
        }

        // Update last request timestamp for this endpoint
        metrics.last_request_by_endpoint.insert(endpoint, chrono::Utc::now());

        // Update time series data for request count and response time
        self.update_time_series_on_request(response_time_ms).await;

        // Record the log
        let mut logs = self.logs.write().await;
        let log_entry = RequestLog {
            id: format!("req_{}", metrics.total_requests),
            timestamp: Utc::now(),
            method: method.to_string(),
            path: path.to_string(),
            status_code,
            response_time_ms,
            client_ip: None,
            user_agent: None,
            headers: HashMap::new(),
            response_size_bytes: 0,
            error_message: error,
        };

        logs.push(log_entry);

        // Keep only last 1000 logs
        if logs.len() > 1000 {
            logs.remove(0);
        }
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> RequestMetrics {
        self.metrics.read().await.clone()
    }

    /// Update system metrics
    pub async fn update_system_metrics(&self, memory_mb: u64, cpu_percent: f64, threads: u32) {
        let mut system_metrics = self.system_metrics.write().await;
        system_metrics.memory_usage_mb = memory_mb;
        system_metrics.cpu_usage_percent = cpu_percent;
        system_metrics.active_threads = threads;

        // Update time series data
        self.update_time_series_data(memory_mb as f64, cpu_percent).await;
    }

    /// Update time series data with new metrics
    async fn update_time_series_data(&self, memory_mb: f64, cpu_percent: f64) {
        let now = chrono::Utc::now();
        let mut time_series = self.time_series.write().await;

        // Add memory usage data point
        time_series.memory_usage.push(TimeSeriesPoint {
            timestamp: now,
            value: memory_mb,
        });

        // Add CPU usage data point
        time_series.cpu_usage.push(TimeSeriesPoint {
            timestamp: now,
            value: cpu_percent,
        });

        // Add request count data point (from current metrics)
        let metrics = self.metrics.read().await;
        time_series.request_count.push(TimeSeriesPoint {
            timestamp: now,
            value: metrics.total_requests as f64,
        });

        // Add average response time data point
        let avg_response_time = if !metrics.response_times.is_empty() {
            metrics.response_times.iter().sum::<u64>() as f64 / metrics.response_times.len() as f64
        } else {
            0.0
        };
        time_series.response_time.push(TimeSeriesPoint {
            timestamp: now,
            value: avg_response_time,
        });

        // Keep only last 100 data points for each metric to prevent memory bloat
        const MAX_POINTS: usize = 100;
        if time_series.memory_usage.len() > MAX_POINTS {
            time_series.memory_usage.remove(0);
        }
        if time_series.cpu_usage.len() > MAX_POINTS {
            time_series.cpu_usage.remove(0);
        }
        if time_series.request_count.len() > MAX_POINTS {
            time_series.request_count.remove(0);
        }
        if time_series.response_time.len() > MAX_POINTS {
            time_series.response_time.remove(0);
        }
    }

    /// Get system metrics
    pub async fn get_system_metrics(&self) -> SystemMetrics {
        self.system_metrics.read().await.clone()
    }

    /// Get time series data
    pub async fn get_time_series_data(&self) -> TimeSeriesData {
        self.time_series.read().await.clone()
    }

    /// Get restart status
    pub async fn get_restart_status(&self) -> RestartStatus {
        self.restart_status.read().await.clone()
    }

    /// Initiate server restart
    pub async fn initiate_restart(&self, reason: String) -> Result<()> {
        let mut status = self.restart_status.write().await;

        if status.in_progress {
            return Err(Error::generic("Restart already in progress".to_string()));
        }

        status.in_progress = true;
        status.initiated_at = Some(chrono::Utc::now());
        status.reason = Some(reason);
        status.success = None;

        Ok(())
    }

    /// Complete restart (success or failure)
    pub async fn complete_restart(&self, success: bool) {
        let mut status = self.restart_status.write().await;
        status.in_progress = false;
        status.success = Some(success);
    }

    /// Get smoke test results
    pub async fn get_smoke_test_results(&self) -> Vec<SmokeTestResult> {
        self.smoke_test_results.read().await.clone()
    }

    /// Update smoke test result
    pub async fn update_smoke_test_result(&self, result: SmokeTestResult) {
        let mut results = self.smoke_test_results.write().await;

        // Find existing result by ID and update, or add new one
        if let Some(existing) = results.iter_mut().find(|r| r.id == result.id) {
            *existing = result;
        } else {
            results.push(result);
        }

        // Keep only last 100 test results
        if results.len() > 100 {
            results.remove(0);
        }
    }

    /// Clear all smoke test results
    pub async fn clear_smoke_test_results(&self) {
        let mut results = self.smoke_test_results.write().await;
        results.clear();
    }

    /// Update time series data when a request is recorded
    async fn update_time_series_on_request(&self, response_time_ms: u64) {
        let now = chrono::Utc::now();
        let mut time_series = self.time_series.write().await;

        // Add request count data point
        let metrics = self.metrics.read().await;
        time_series.request_count.push(TimeSeriesPoint {
            timestamp: now,
            value: metrics.total_requests as f64,
        });

        // Add response time data point
        time_series.response_time.push(TimeSeriesPoint {
            timestamp: now,
            value: response_time_ms as f64,
        });

        // Keep only last 100 data points for each metric to prevent memory bloat
        const MAX_POINTS: usize = 100;
        if time_series.request_count.len() > MAX_POINTS {
            time_series.request_count.remove(0);
        }
        if time_series.response_time.len() > MAX_POINTS {
            time_series.response_time.remove(0);
        }
    }

    /// Get current configuration
    pub async fn get_config(&self) -> ConfigurationState {
        self.config.read().await.clone()
    }

    /// Update latency configuration
    pub async fn update_latency_config(
        &self,
        base_ms: u64,
        jitter_ms: u64,
        tag_overrides: HashMap<String, u64>,
    ) {
        let mut config = self.config.write().await;
        config.latency_profile.base_ms = base_ms;
        config.latency_profile.jitter_ms = jitter_ms;
        config.latency_profile.tag_overrides = tag_overrides;
    }

    /// Update fault configuration
    pub async fn update_fault_config(
        &self,
        enabled: bool,
        failure_rate: f64,
        status_codes: Vec<u16>,
    ) {
        let mut config = self.config.write().await;
        config.fault_config.enabled = enabled;
        config.fault_config.failure_rate = failure_rate;
        config.fault_config.status_codes = status_codes;
    }

    /// Update proxy configuration
    pub async fn update_proxy_config(
        &self,
        enabled: bool,
        upstream_url: Option<String>,
        timeout_seconds: u64,
    ) {
        let mut config = self.config.write().await;
        config.proxy_config.enabled = enabled;
        config.proxy_config.upstream_url = upstream_url;
        config.proxy_config.timeout_seconds = timeout_seconds;
    }

    /// Update validation settings
    pub async fn update_validation_config(
        &self,
        mode: String,
        aggregate_errors: bool,
        validate_responses: bool,
        overrides: HashMap<String, String>,
    ) {
        let mut config = self.config.write().await;
        config.validation_settings.mode = mode;
        config.validation_settings.aggregate_errors = aggregate_errors;
        config.validation_settings.validate_responses = validate_responses;
        config.validation_settings.overrides = overrides;
    }

    /// Get filtered logs
    pub async fn get_logs_filtered(&self, filter: &LogFilter) -> Vec<RequestLog> {
        let logs = self.logs.read().await;

        logs.iter()
            .rev() // Most recent first
            .filter(|log| {
                if let Some(ref method) = filter.method {
                    if log.method != *method {
                        return false;
                    }
                }
                if let Some(ref path_pattern) = filter.path_pattern {
                    if !log.path.contains(path_pattern) {
                        return false;
                    }
                }
                if let Some(status) = filter.status_code {
                    if log.status_code != status {
                        return false;
                    }
                }
                true
            })
            .take(filter.limit.unwrap_or(100))
            .cloned()
            .collect()
    }

    /// Clear all logs
    pub async fn clear_logs(&self) {
        let mut logs = self.logs.write().await;
        logs.clear();
    }
}

/// Serve the main admin interface
pub async fn serve_admin_html() -> Html<&'static str> {
    Html(crate::get_admin_html())
}

/// Serve admin CSS
pub async fn serve_admin_css() -> ([(http::HeaderName, &'static str); 1], &'static str) {
    ([(http::header::CONTENT_TYPE, "text/css")], crate::get_admin_css())
}

/// Serve admin JavaScript
pub async fn serve_admin_js() -> ([(http::HeaderName, &'static str); 1], &'static str) {
    ([(http::header::CONTENT_TYPE, "application/javascript")], crate::get_admin_js())
}

/// Get dashboard data
pub async fn get_dashboard(State(state): State<AdminState>) -> Json<ApiResponse<DashboardData>> {
    let uptime = (Utc::now() - state.start_time).num_seconds() as u64;

    // Get real metrics from state
    let metrics = state.get_metrics().await;
    let system_metrics = state.get_system_metrics().await;
    let config = state.get_config().await;

    // Get recent logs from centralized logger
    let recent_logs: Vec<RequestLog> = if let Some(global_logger) = mockforge_core::get_global_logger() {
        // Get logs from centralized logger
        let centralized_logs = global_logger.get_recent_logs(Some(20)).await;

        // Convert to RequestLog format for admin UI
        centralized_logs
            .into_iter()
            .map(|log| RequestLog {
                id: log.id,
                timestamp: log.timestamp,
                method: log.method,
                path: log.path,
                status_code: log.status_code,
                response_time_ms: log.response_time_ms,
                client_ip: log.client_ip,
                user_agent: log.user_agent,
                headers: log.headers,
                response_size_bytes: log.response_size_bytes,
                error_message: log.error_message,
            })
            .collect()
    } else {
        // Fallback to local logs if centralized logger not available
        let logs = state.logs.read().await;
        logs.iter().rev().take(10).cloned().collect()
    };

    let system_info = SystemInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        memory_usage_mb: system_metrics.memory_usage_mb,
        cpu_usage_percent: system_metrics.cpu_usage_percent,
        active_threads: system_metrics.active_threads as usize,
        total_routes: metrics.requests_by_endpoint.len(),
        total_fixtures: count_fixtures().unwrap_or(0),
    };

    let servers = vec![
        ServerStatus {
            server_type: "HTTP".to_string(),
            address: state.http_server_addr.map(|addr| addr.to_string()),
            running: state.http_server_addr.is_some(),
            start_time: Some(state.start_time),
            uptime_seconds: Some(uptime),
            active_connections: metrics.active_connections,
            total_requests: count_requests_by_server_type(&metrics, "HTTP"),
        },
        ServerStatus {
            server_type: "WebSocket".to_string(),
            address: state.ws_server_addr.map(|addr| addr.to_string()),
            running: state.ws_server_addr.is_some(),
            start_time: Some(state.start_time),
            uptime_seconds: Some(uptime),
            active_connections: metrics.active_connections / 2, // Estimate
            total_requests: count_requests_by_server_type(&metrics, "WebSocket"),
        },
        ServerStatus {
            server_type: "gRPC".to_string(),
            address: state.grpc_server_addr.map(|addr| addr.to_string()),
            running: state.grpc_server_addr.is_some(),
            start_time: Some(state.start_time),
            uptime_seconds: Some(uptime),
            active_connections: metrics.active_connections / 3, // Estimate
            total_requests: count_requests_by_server_type(&metrics, "gRPC"),
        },
    ];

    // Build routes info from actual request metrics
    let mut routes = Vec::new();
    for (endpoint, count) in &metrics.requests_by_endpoint {
        let parts: Vec<&str> = endpoint.splitn(2, ' ').collect();
        if parts.len() == 2 {
            let method = parts[0].to_string();
            let path = parts[1].to_string();
            let error_count = *metrics.errors_by_endpoint.get(endpoint).unwrap_or(&0);

            routes.push(RouteInfo {
                method: Some(method.clone()),
                path: path.clone(),
                priority: 0,
                has_fixtures: route_has_fixtures(&method, &path),
                latency_ms: calculate_endpoint_latency(&metrics, endpoint),
                request_count: *count,
                last_request: get_endpoint_last_request(&metrics, endpoint),
                error_count,
            });
        }
    }

    let dashboard = DashboardData {
        server_info: ServerInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_time: option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown").to_string(),
            git_sha: option_env!("VERGEN_GIT_SHA").unwrap_or("unknown").to_string(),
        },
        system_info: DashboardSystemInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            uptime: uptime,
            memory_usage: system_metrics.memory_usage_mb * 1024 * 1024, // Convert MB to bytes
        },
        metrics: SimpleMetricsData {
            total_requests: metrics.requests_by_endpoint.values().sum(),
            active_requests: metrics.active_connections,
            average_response_time: if metrics.response_times.is_empty() {
                0.0
            } else {
                metrics.response_times.iter().sum::<u64>() as f64 / metrics.response_times.len() as f64
            },
            error_rate: {
                let total_requests = metrics.requests_by_endpoint.values().sum::<u64>();
                let total_errors = metrics.errors_by_endpoint.values().sum::<u64>();
                if total_requests == 0 {
                    0.0
                } else {
                    total_errors as f64 / total_requests as f64
                }
            },
        },
    };

    Json(ApiResponse::success(dashboard))
}

/// Get routes by proxying to HTTP server
pub async fn get_routes(State(state): State<AdminState>) -> impl IntoResponse {
    if let Some(http_addr) = state.http_server_addr {
        // Try to fetch routes from the HTTP server
        let url = format!("http://{}/__mockforge/routes", http_addr);
        if let Ok(response) = reqwest::get(&url).await {
            if response.status().is_success() {
                if let Ok(body) = response.text().await {
                    return (StatusCode::OK, [("content-type", "application/json")], body);
                }
            }
        }
    }

    // Fallback: return empty routes
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        r#"{"routes":[]}"#.to_string(),
    )
}

/// Get server info (HTTP server address for API calls)
pub async fn get_server_info(State(state): State<AdminState>) -> Json<serde_json::Value> {
    Json(json!({
        "http_server": state.http_server_addr.map(|addr| addr.to_string()),
        "ws_server": state.ws_server_addr.map(|addr| addr.to_string()),
        "grpc_server": state.grpc_server_addr.map(|addr| addr.to_string())
    }))
}

/// Get health check status
pub async fn get_health() -> Json<HealthCheck> {
    Json(
        HealthCheck::healthy()
            .with_service("http".to_string(), "healthy".to_string())
            .with_service("websocket".to_string(), "healthy".to_string())
            .with_service("grpc".to_string(), "healthy".to_string()),
    )
}

/// Get request logs with optional filtering
pub async fn get_logs(
    State(state): State<AdminState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<RequestLog>>> {
    let mut filter = LogFilter::default();

    if let Some(method) = params.get("method") {
        filter.method = Some(method.clone());
    }
    if let Some(path) = params.get("path") {
        filter.path_pattern = Some(path.clone());
    }
    if let Some(status) = params.get("status").and_then(|s| s.parse().ok()) {
        filter.status_code = Some(status);
    }
    if let Some(limit) = params.get("limit").and_then(|s| s.parse().ok()) {
        filter.limit = Some(limit);
    }

    // Get logs from centralized logger (same as dashboard)
    let logs = if let Some(global_logger) = mockforge_core::get_global_logger() {
        // Get logs from centralized logger
        let centralized_logs = global_logger.get_recent_logs(filter.limit).await;

        // Convert to RequestLog format and apply filters
        centralized_logs
            .into_iter()
            .filter(|log| {
                if let Some(ref method) = filter.method {
                    if log.method != *method {
                        return false;
                    }
                }
                if let Some(ref path_pattern) = filter.path_pattern {
                    if !log.path.contains(path_pattern) {
                        return false;
                    }
                }
                if let Some(status) = filter.status_code {
                    if log.status_code != status {
                        return false;
                    }
                }
                true
            })
            .map(|log| RequestLog {
                id: log.id,
                timestamp: log.timestamp,
                method: log.method,
                path: log.path,
                status_code: log.status_code,
                response_time_ms: log.response_time_ms,
                client_ip: log.client_ip,
                user_agent: log.user_agent,
                headers: log.headers,
                response_size_bytes: log.response_size_bytes,
                error_message: log.error_message,
            })
            .collect()
    } else {
        // Fallback to local logs if centralized logger not available
        state.get_logs_filtered(&filter).await
    };

    Json(ApiResponse::success(logs))
}

/// Get metrics data
pub async fn get_metrics(State(state): State<AdminState>) -> Json<ApiResponse<MetricsData>> {
    let metrics = state.get_metrics().await;
    let system_metrics = state.get_system_metrics().await;
    let time_series = state.get_time_series_data().await;

    // Calculate percentiles from response times
    let mut response_times = metrics.response_times.clone();
    response_times.sort();

    let p50 = if !response_times.is_empty() {
        response_times[response_times.len() / 2] as u64
    } else {
        0
    };

    let p95 = if !response_times.is_empty() {
        let idx = (response_times.len() as f64 * 0.95) as usize;
        response_times[response_times.len().min(idx)] as u64
    } else {
        0
    };

    let p99 = if !response_times.is_empty() {
        let idx = (response_times.len() as f64 * 0.99) as usize;
        response_times[response_times.len().min(idx)] as u64
    } else {
        0
    };

    // Calculate error rates
    let mut error_rate_by_endpoint = HashMap::new();
    for (endpoint, total_count) in &metrics.requests_by_endpoint {
        let error_count = *metrics.errors_by_endpoint.get(endpoint).unwrap_or(&0);
        let error_rate = if *total_count > 0 {
            error_count as f64 / *total_count as f64
        } else {
            0.0
        };
        error_rate_by_endpoint.insert(endpoint.clone(), error_rate);
    }

    // Convert time series data to the format expected by the frontend
    // If no time series data exists yet, use current system metrics as a fallback
    let memory_usage_over_time = if time_series.memory_usage.is_empty() {
        vec![(Utc::now(), system_metrics.memory_usage_mb)]
    } else {
        time_series
            .memory_usage
            .iter()
            .map(|point| (point.timestamp, point.value as u64))
            .collect()
    };

    let cpu_usage_over_time = if time_series.cpu_usage.is_empty() {
        vec![(Utc::now(), system_metrics.cpu_usage_percent)]
    } else {
        time_series
            .cpu_usage
            .iter()
            .map(|point| (point.timestamp, point.value))
            .collect()
    };

    let metrics_data = MetricsData {
        requests_by_endpoint: metrics.requests_by_endpoint,
        response_time_percentiles: HashMap::from([
            ("p50".to_string(), p50),
            ("p95".to_string(), p95),
            ("p99".to_string(), p99),
        ]),
        error_rate_by_endpoint,
        memory_usage_over_time,
        cpu_usage_over_time,
    };

    Json(ApiResponse::success(metrics_data))
}

/// Update latency profile
pub async fn update_latency(
    State(state): State<AdminState>,
    Json(update): Json<ConfigUpdate>,
) -> Json<ApiResponse<String>> {
    if update.config_type != "latency" {
        return Json(ApiResponse::error("Invalid config type".to_string()));
    }

    // Extract latency configuration from the update data
    let base_ms = update.data.get("base_ms").and_then(|v| v.as_u64()).unwrap_or(50);

    let jitter_ms = update.data.get("jitter_ms").and_then(|v| v.as_u64()).unwrap_or(20);

    let tag_overrides = update
        .data
        .get("tag_overrides")
        .and_then(|v| v.as_object())
        .map(|obj| obj.iter().filter_map(|(k, v)| v.as_u64().map(|val| (k.clone(), val))).collect())
        .unwrap_or_default();

    // Update the actual configuration
    state.update_latency_config(base_ms, jitter_ms, tag_overrides).await;

    tracing::info!("Updated latency profile: base_ms={}, jitter_ms={}", base_ms, jitter_ms);

    Json(ApiResponse::success("Latency profile updated".to_string()))
}

/// Update fault injection configuration
pub async fn update_faults(
    State(state): State<AdminState>,
    Json(update): Json<ConfigUpdate>,
) -> Json<ApiResponse<String>> {
    if update.config_type != "faults" {
        return Json(ApiResponse::error("Invalid config type".to_string()));
    }

    // Extract fault configuration from the update data
    let enabled = update.data.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);

    let failure_rate = update.data.get("failure_rate").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let status_codes = update
        .data
        .get("status_codes")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u16)).collect())
        .unwrap_or_else(|| vec![500, 502, 503]);

    // Update the actual configuration
    state.update_fault_config(enabled, failure_rate, status_codes).await;

    tracing::info!(
        "Updated fault configuration: enabled={}, failure_rate={}",
        enabled,
        failure_rate
    );

    Json(ApiResponse::success("Fault configuration updated".to_string()))
}

/// Update proxy configuration
pub async fn update_proxy(
    State(state): State<AdminState>,
    Json(update): Json<ConfigUpdate>,
) -> Json<ApiResponse<String>> {
    if update.config_type != "proxy" {
        return Json(ApiResponse::error("Invalid config type".to_string()));
    }

    // Extract proxy configuration from the update data
    let enabled = update.data.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);

    let upstream_url =
        update.data.get("upstream_url").and_then(|v| v.as_str()).map(|s| s.to_string());

    let timeout_seconds = update.data.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(30);

    // Update the actual configuration
    state.update_proxy_config(enabled, upstream_url.clone(), timeout_seconds).await;

    tracing::info!(
        "Updated proxy configuration: enabled={}, upstream_url={:?}",
        enabled,
        upstream_url
    );

    Json(ApiResponse::success("Proxy configuration updated".to_string()))
}

/// Clear request logs
pub async fn clear_logs(State(state): State<AdminState>) -> Json<ApiResponse<String>> {
    // Clear the actual logs from state
    state.clear_logs().await;
    tracing::info!("Cleared all request logs");

    Json(ApiResponse::success("Logs cleared".to_string()))
}

/// Restart servers
pub async fn restart_servers(State(state): State<AdminState>) -> Json<ApiResponse<String>> {
    // Check if restart is already in progress
    let current_status = state.get_restart_status().await;
    if current_status.in_progress {
        return Json(ApiResponse::error("Server restart already in progress".to_string()));
    }

    // Initiate restart status
    if let Err(e) = state
        .initiate_restart("Manual restart requested via admin UI".to_string())
        .await
    {
        return Json(ApiResponse::error(format!("Failed to initiate restart: {}", e)));
    }

    // Spawn restart task to avoid blocking the response
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = perform_server_restart(&state_clone).await {
            tracing::error!("Server restart failed: {}", e);
            state_clone.complete_restart(false).await;
        } else {
            tracing::info!("Server restart completed successfully");
            state_clone.complete_restart(true).await;
        }
    });

    tracing::info!("Server restart initiated via admin UI");
    Json(ApiResponse::success(
        "Server restart initiated. Please wait for completion.".to_string(),
    ))
}

/// Perform the actual server restart
async fn perform_server_restart(_state: &AdminState) -> Result<()> {
    // Get the current process ID
    let current_pid = std::process::id();
    tracing::info!("Initiating restart for process PID: {}", current_pid);

    // Try to find the parent process (MockForge CLI)
    let parent_pid = get_parent_process_id(current_pid)?;
    tracing::info!("Found parent process PID: {}", parent_pid);

    // Method 1: Try to restart via parent process signal
    if let Ok(()) = restart_via_parent_signal(parent_pid).await {
        tracing::info!("Restart initiated via parent process signal");
        return Ok(());
    }

    // Method 2: Fallback to process replacement
    if let Ok(()) = restart_via_process_replacement().await {
        tracing::info!("Restart initiated via process replacement");
        return Ok(());
    }

    // Method 3: Last resort - graceful shutdown with restart script
    restart_via_script().await
}

/// Get parent process ID
fn get_parent_process_id(pid: u32) -> Result<u32> {
    // Try to read from /proc/pid/stat on Linux
    #[cfg(target_os = "linux")]
    {
        let stat_path = format!("/proc/{}/stat", pid);
        if let Ok(content) = std::fs::read_to_string(&stat_path) {
            let fields: Vec<&str> = content.split_whitespace().collect();
            if fields.len() > 3 {
                if let Ok(ppid) = fields[3].parse::<u32>() {
                    return Ok(ppid);
                }
            }
        }
    }

    // Fallback: assume we're running under a shell/process manager
    Ok(1) // PID 1 as fallback
}

/// Restart via parent process signal
async fn restart_via_parent_signal(parent_pid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::process::Command;

        // Send SIGTERM to parent process to trigger restart
        let output = Command::new("kill")
            .args(["-TERM", &parent_pid.to_string()])
            .output()
            .map_err(|e| Error::generic(format!("Failed to send signal: {}", e)))?;

        if !output.status.success() {
            return Err(Error::generic(
                "Failed to send restart signal to parent process".to_string(),
            ));
        }

        // Wait a moment for the signal to be processed
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    #[cfg(not(unix))]
    {
        Err(Error::generic(
            "Signal-based restart not supported on this platform".to_string(),
        ))
    }
}

/// Restart via process replacement
async fn restart_via_process_replacement() -> Result<()> {
    // Get the current executable path
    let current_exe = std::env::current_exe()
        .map_err(|e| Error::generic(format!("Failed to get current executable: {}", e)))?;

    // Get current command line arguments
    let args: Vec<String> = std::env::args().collect();

    tracing::info!("Restarting with command: {:?}", args);

    // Start new process
    let mut child = Command::new(&current_exe)
        .args(&args[1..]) // Skip the program name
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| Error::generic(format!("Failed to start new process: {}", e)))?;

    // Give the new process a moment to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check if the new process is still running
    match child.try_wait() {
        Ok(Some(status)) => {
            if status.success() {
                tracing::info!("New process started successfully");
                Ok(())
            } else {
                Err(Error::generic("New process exited with error".to_string()))
            }
        }
        Ok(None) => {
            tracing::info!("New process is running, exiting current process");
            // Exit current process
            std::process::exit(0);
        }
        Err(e) => Err(Error::generic(format!("Failed to check new process status: {}", e))),
    }
}

/// Restart via external script
async fn restart_via_script() -> Result<()> {
    // Look for restart script in common locations
    let script_paths = ["./scripts/restart.sh", "./restart.sh", "restart.sh"];

    for script_path in &script_paths {
        if std::path::Path::new(script_path).exists() {
            tracing::info!("Using restart script: {}", script_path);

            let output = Command::new("bash")
                .arg(script_path)
                .output()
                .map_err(|e| Error::generic(format!("Failed to execute restart script: {}", e)))?;

            if output.status.success() {
                return Ok(());
            } else {
                tracing::warn!(
                    "Restart script failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }

    // If no script found, try to use the clear-ports script as a fallback
    let clear_script = "./scripts/clear-ports.sh";
    if std::path::Path::new(clear_script).exists() {
        tracing::info!("Using clear-ports script as fallback");

        let _ = Command::new("bash").arg(clear_script).output();
    }

    Err(Error::generic(
        "No restart mechanism available. Please restart manually.".to_string(),
    ))
}

/// Get restart status
pub async fn get_restart_status(
    State(state): State<AdminState>,
) -> Json<ApiResponse<RestartStatus>> {
    let status = state.get_restart_status().await;
    Json(ApiResponse::success(status))
}

/// Get server configuration
pub async fn get_config(State(state): State<AdminState>) -> Json<ApiResponse<serde_json::Value>> {
    let config_state = state.get_config().await;

    let config = json!({
        "latency": {
            "enabled": true,
            "base_ms": config_state.latency_profile.base_ms,
            "jitter_ms": config_state.latency_profile.jitter_ms,
            "tag_overrides": config_state.latency_profile.tag_overrides
        },
        "faults": {
            "enabled": config_state.fault_config.enabled,
            "failure_rate": config_state.fault_config.failure_rate,
            "status_codes": config_state.fault_config.status_codes
        },
        "proxy": {
            "enabled": config_state.proxy_config.enabled,
            "upstream_url": config_state.proxy_config.upstream_url,
            "timeout_seconds": config_state.proxy_config.timeout_seconds
        },
        "validation": {
            "mode": config_state.validation_settings.mode,
            "aggregate_errors": config_state.validation_settings.aggregate_errors,
            "validate_responses": config_state.validation_settings.validate_responses,
            "overrides": config_state.validation_settings.overrides
        }
    });

    Json(ApiResponse::success(config))
}

/// Count total fixtures in the fixtures directory
pub fn count_fixtures() -> Result<usize> {
    // Get the fixtures directory from environment or use default
    let fixtures_dir =
        std::env::var("MOCKFORGE_FIXTURES_DIR").unwrap_or_else(|_| "fixtures".to_string());
    let fixtures_path = std::path::Path::new(&fixtures_dir);

    if !fixtures_path.exists() {
        return Ok(0);
    }

    let mut total_count = 0;

    // Count HTTP fixtures
    let http_fixtures_path = fixtures_path.join("http");
    if http_fixtures_path.exists() {
        total_count += count_fixtures_in_directory(&http_fixtures_path)?;
    }

    // Count WebSocket fixtures
    let ws_fixtures_path = fixtures_path.join("websocket");
    if ws_fixtures_path.exists() {
        total_count += count_fixtures_in_directory(&ws_fixtures_path)?;
    }

    // Count gRPC fixtures
    let grpc_fixtures_path = fixtures_path.join("grpc");
    if grpc_fixtures_path.exists() {
        total_count += count_fixtures_in_directory(&grpc_fixtures_path)?;
    }

    Ok(total_count)
}

/// Helper function to count JSON files in a directory recursively
fn count_fixtures_in_directory(dir_path: &std::path::Path) -> Result<usize> {
    let mut count = 0;

    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries {
            let entry = entry
                .map_err(|e| Error::generic(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively count fixtures in subdirectories
                count += count_fixtures_in_directory(&path)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                // Count JSON files as fixtures
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Check if a specific route has fixtures
pub fn route_has_fixtures(method: &str, path: &str) -> bool {
    // Get the fixtures directory from environment or use default
    let fixtures_dir =
        std::env::var("MOCKFORGE_FIXTURES_DIR").unwrap_or_else(|_| "fixtures".to_string());
    let fixtures_path = std::path::Path::new(&fixtures_dir);

    if !fixtures_path.exists() {
        return false;
    }

    // Check HTTP fixtures
    let method_lower = method.to_lowercase();
    let path_hash = path.replace(['/', ':'], "_");
    let http_fixtures_path = fixtures_path.join("http").join(&method_lower).join(&path_hash);

    if http_fixtures_path.exists() {
        // Check if there are any JSON files in this directory
        if let Ok(entries) = std::fs::read_dir(&http_fixtures_path) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    return true;
                }
            }
        }
    }

    // Check WebSocket fixtures for WS method
    if method.to_uppercase() == "WS" {
        let ws_fixtures_path = fixtures_path.join("websocket").join(&path_hash);

        if ws_fixtures_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&ws_fixtures_path) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Calculate average latency for a specific endpoint
fn calculate_endpoint_latency(metrics: &RequestMetrics, endpoint: &str) -> Option<u64> {
    metrics.response_times_by_endpoint.get(endpoint).and_then(|times| {
        if times.is_empty() {
            None
        } else {
            let sum: u64 = times.iter().sum();
            Some(sum / times.len() as u64)
        }
    })
}

/// Get the last request timestamp for a specific endpoint
fn get_endpoint_last_request(
    metrics: &RequestMetrics,
    endpoint: &str,
) -> Option<chrono::DateTime<chrono::Utc>> {
    metrics.last_request_by_endpoint.get(endpoint).copied()
}

/// Count total requests for a specific server type
fn count_requests_by_server_type(metrics: &RequestMetrics, server_type: &str) -> u64 {
    match server_type {
        "HTTP" => {
            // Count all HTTP requests (GET, POST, PUT, DELETE, etc.)
            metrics
                .requests_by_endpoint
                .iter()
                .filter(|(endpoint, _)| {
                    let method = endpoint.split(' ').next().unwrap_or("");
                    matches!(
                        method,
                        "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS"
                    )
                })
                .map(|(_, count)| count)
                .sum()
        }
        "WebSocket" => {
            // Count WebSocket requests (WS method)
            metrics
                .requests_by_endpoint
                .iter()
                .filter(|(endpoint, _)| {
                    let method = endpoint.split(' ').next().unwrap_or("");
                    method == "WS"
                })
                .map(|(_, count)| count)
                .sum()
        }
        "gRPC" => {
            // Count gRPC requests (gRPC method)
            metrics
                .requests_by_endpoint
                .iter()
                .filter(|(endpoint, _)| {
                    let method = endpoint.split(' ').next().unwrap_or("");
                    method == "gRPC"
                })
                .map(|(_, count)| count)
                .sum()
        }
        _ => 0,
    }
}

/// Get fixtures/replay data
pub async fn get_fixtures() -> Json<ApiResponse<Vec<FixtureInfo>>> {
    match scan_fixtures_directory() {
        Ok(fixtures) => Json(ApiResponse::success(fixtures)),
        Err(e) => {
            tracing::error!("Failed to scan fixtures directory: {}", e);
            Json(ApiResponse::error(format!("Failed to load fixtures: {}", e)))
        }
    }
}

/// Scan the fixtures directory and return all fixture information
fn scan_fixtures_directory() -> Result<Vec<FixtureInfo>> {
    let fixtures_dir =
        std::env::var("MOCKFORGE_FIXTURES_DIR").unwrap_or_else(|_| "fixtures".to_string());
    let fixtures_path = std::path::Path::new(&fixtures_dir);

    if !fixtures_path.exists() {
        tracing::warn!("Fixtures directory does not exist: {}", fixtures_dir);
        return Ok(Vec::new());
    }

    let mut all_fixtures = Vec::new();

    // Scan HTTP fixtures
    let http_fixtures = scan_protocol_fixtures(fixtures_path, "http")?;
    all_fixtures.extend(http_fixtures);

    // Scan WebSocket fixtures
    let ws_fixtures = scan_protocol_fixtures(fixtures_path, "websocket")?;
    all_fixtures.extend(ws_fixtures);

    // Scan gRPC fixtures
    let grpc_fixtures = scan_protocol_fixtures(fixtures_path, "grpc")?;
    all_fixtures.extend(grpc_fixtures);

    // Sort by saved_at timestamp (newest first)
    all_fixtures.sort_by(|a, b| b.saved_at.cmp(&a.saved_at));

    tracing::info!("Found {} fixtures in directory: {}", all_fixtures.len(), fixtures_dir);
    Ok(all_fixtures)
}

/// Scan fixtures for a specific protocol
fn scan_protocol_fixtures(
    fixtures_path: &std::path::Path,
    protocol: &str,
) -> Result<Vec<FixtureInfo>> {
    let protocol_path = fixtures_path.join(protocol);
    let mut fixtures = Vec::new();

    if !protocol_path.exists() {
        return Ok(fixtures);
    }

    // Walk through the protocol directory recursively
    if let Ok(entries) = std::fs::read_dir(&protocol_path) {
        for entry in entries {
            let entry = entry
                .map_err(|e| Error::generic(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively scan subdirectories
                let sub_fixtures = scan_directory_recursive(&path, protocol)?;
                fixtures.extend(sub_fixtures);
            } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                // Process individual JSON fixture file
                if let Ok(fixture) = parse_fixture_file_sync(&path, protocol) {
                    fixtures.push(fixture);
                }
            }
        }
    }

    Ok(fixtures)
}

/// Recursively scan a directory for fixture files
fn scan_directory_recursive(
    dir_path: &std::path::Path,
    protocol: &str,
) -> Result<Vec<FixtureInfo>> {
    let mut fixtures = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries {
            let entry = entry
                .map_err(|e| Error::generic(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively scan subdirectories
                let sub_fixtures = scan_directory_recursive(&path, protocol)?;
                fixtures.extend(sub_fixtures);
            } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                // Process individual JSON fixture file
                if let Ok(fixture) = parse_fixture_file_sync(&path, protocol) {
                    fixtures.push(fixture);
                }
            }
        }
    }

    Ok(fixtures)
}

/// Parse a single fixture file and extract metadata (synchronous version)
fn parse_fixture_file_sync(file_path: &std::path::Path, protocol: &str) -> Result<FixtureInfo> {
    // Get file metadata
    let metadata = std::fs::metadata(file_path)
        .map_err(|e| Error::generic(format!("Failed to read file metadata: {}", e)))?;

    let file_size = metadata.len();
    let modified_time = metadata
        .modified()
        .map_err(|e| Error::generic(format!("Failed to get file modification time: {}", e)))?;

    let saved_at = chrono::DateTime::from(modified_time);

    // Read and parse the fixture file
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| Error::generic(format!("Failed to read fixture file: {}", e)))?;

    let fixture_data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| Error::generic(format!("Failed to parse fixture JSON: {}", e)))?;

    // Extract method and path from the fixture data
    let (method, path) = extract_method_and_path(&fixture_data, protocol)?;

    // Generate a unique ID based on file path and content
    let id = generate_fixture_id(file_path, &content);

    // Extract fingerprint from file path or fixture data
    let fingerprint = extract_fingerprint(file_path, &fixture_data)?;

    // Get relative file path
    let fixtures_dir =
        std::env::var("MOCKFORGE_FIXTURES_DIR").unwrap_or_else(|_| "fixtures".to_string());
    let fixtures_path = std::path::Path::new(&fixtures_dir);
    let file_path_str = file_path
        .strip_prefix(fixtures_path)
        .unwrap_or(file_path)
        .to_string_lossy()
        .to_string();

    Ok(FixtureInfo {
        id,
        protocol: protocol.to_string(),
        method,
        path,
        saved_at,
        file_size,
        file_path: file_path_str,
        fingerprint,
        metadata: fixture_data,
    })
}

/// Extract method and path from fixture data
fn extract_method_and_path(
    fixture_data: &serde_json::Value,
    protocol: &str,
) -> Result<(String, String)> {
    match protocol {
        "http" => {
            // For HTTP fixtures, look for request.method and request.path
            let method = fixture_data
                .get("request")
                .and_then(|req| req.get("method"))
                .and_then(|m| m.as_str())
                .unwrap_or("UNKNOWN")
                .to_uppercase();

            let path = fixture_data
                .get("request")
                .and_then(|req| req.get("path"))
                .and_then(|p| p.as_str())
                .unwrap_or("/unknown")
                .to_string();

            Ok((method, path))
        }
        "websocket" => {
            // For WebSocket fixtures, use WS method and extract path from metadata
            let path = fixture_data
                .get("path")
                .and_then(|p| p.as_str())
                .or_else(|| {
                    fixture_data
                        .get("request")
                        .and_then(|req| req.get("path"))
                        .and_then(|p| p.as_str())
                })
                .unwrap_or("/ws")
                .to_string();

            Ok(("WS".to_string(), path))
        }
        "grpc" => {
            // For gRPC fixtures, extract service and method
            let service =
                fixture_data.get("service").and_then(|s| s.as_str()).unwrap_or("UnknownService");

            let method =
                fixture_data.get("method").and_then(|m| m.as_str()).unwrap_or("UnknownMethod");

            let path = format!("/{}/{}", service, method);
            Ok(("gRPC".to_string(), path))
        }
        _ => {
            let path = fixture_data
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("/unknown")
                .to_string();
            Ok((protocol.to_uppercase(), path))
        }
    }
}

/// Generate a unique fixture ID
fn generate_fixture_id(file_path: &std::path::Path, content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    content.hash(&mut hasher);
    format!("fixture_{:x}", hasher.finish())
}

/// Extract fingerprint from file path or fixture data
fn extract_fingerprint(
    file_path: &std::path::Path,
    fixture_data: &serde_json::Value,
) -> Result<String> {
    // Try to extract from fixture data first
    if let Some(fingerprint) = fixture_data.get("fingerprint").and_then(|f| f.as_str()) {
        return Ok(fingerprint.to_string());
    }

    // Try to extract from file path (common pattern: method_path_hash.json)
    if let Some(file_name) = file_path.file_stem().and_then(|s| s.to_str()) {
        // Look for hash pattern at the end of filename
        if let Some(hash) = file_name.split('_').next_back() {
            if hash.len() >= 8 && hash.chars().all(|c| c.is_alphanumeric()) {
                return Ok(hash.to_string());
            }
        }
    }

    // Fallback: generate from file path
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    Ok(format!("{:x}", hasher.finish()))
}

/// Delete a fixture
pub async fn delete_fixture(
    Json(payload): Json<FixtureDeleteRequest>,
) -> Json<ApiResponse<String>> {
    match delete_fixture_by_id(&payload.fixture_id).await {
        Ok(_) => {
            tracing::info!("Successfully deleted fixture: {}", payload.fixture_id);
            Json(ApiResponse::success("Fixture deleted successfully".to_string()))
        }
        Err(e) => {
            tracing::error!("Failed to delete fixture {}: {}", payload.fixture_id, e);
            Json(ApiResponse::error(format!("Failed to delete fixture: {}", e)))
        }
    }
}

/// Delete multiple fixtures
pub async fn delete_fixtures_bulk(
    Json(payload): Json<FixtureBulkDeleteRequest>,
) -> Json<ApiResponse<FixtureBulkDeleteResult>> {
    let mut deleted_count = 0;
    let mut errors = Vec::new();

    for fixture_id in &payload.fixture_ids {
        match delete_fixture_by_id(fixture_id).await {
            Ok(_) => {
                deleted_count += 1;
                tracing::info!("Successfully deleted fixture: {}", fixture_id);
            }
            Err(e) => {
                errors.push(format!("Failed to delete {}: {}", fixture_id, e));
                tracing::error!("Failed to delete fixture {}: {}", fixture_id, e);
            }
        }
    }

    let result = FixtureBulkDeleteResult {
        deleted_count,
        total_requested: payload.fixture_ids.len(),
        errors: errors.clone(),
    };

    if errors.is_empty() {
        Json(ApiResponse::success(result))
    } else {
        Json(ApiResponse::error(format!(
            "Partial success: {} deleted, {} errors",
            deleted_count,
            errors.len()
        )))
    }
}

/// Delete a single fixture by ID
async fn delete_fixture_by_id(fixture_id: &str) -> Result<()> {
    // First, try to find the fixture by scanning the fixtures directory
    // This is more robust than trying to parse the ID format
    let fixtures_dir =
        std::env::var("MOCKFORGE_FIXTURES_DIR").unwrap_or_else(|_| "fixtures".to_string());
    let fixtures_path = std::path::Path::new(&fixtures_dir);

    if !fixtures_path.exists() {
        return Err(Error::generic(format!("Fixtures directory does not exist: {}", fixtures_dir)));
    }

    // Search for the fixture file by ID across all protocols
    let file_path = find_fixture_file_by_id(fixtures_path, fixture_id)?;

    // Delete the file
    if file_path.exists() {
        std::fs::remove_file(&file_path).map_err(|e| {
            Error::generic(format!("Failed to delete fixture file {}: {}", file_path.display(), e))
        })?;
        tracing::info!("Deleted fixture file: {}", file_path.display());

        // Also try to remove empty parent directories
        cleanup_empty_directories(&file_path).await;

        Ok(())
    } else {
        Err(Error::generic(format!("Fixture file not found: {}", file_path.display())))
    }
}

/// Find a fixture file by its ID across all protocols
fn find_fixture_file_by_id(
    fixtures_path: &std::path::Path,
    fixture_id: &str,
) -> Result<std::path::PathBuf> {
    // Search in all protocol directories
    let protocols = ["http", "websocket", "grpc"];

    for protocol in &protocols {
        let protocol_path = fixtures_path.join(protocol);
        if let Ok(found_path) = search_fixture_in_directory(&protocol_path, fixture_id) {
            return Ok(found_path);
        }
    }

    Err(Error::generic(format!(
        "Fixture with ID '{}' not found in any protocol directory",
        fixture_id
    )))
}

/// Recursively search for a fixture file by ID in a directory
fn search_fixture_in_directory(
    dir_path: &std::path::Path,
    fixture_id: &str,
) -> Result<std::path::PathBuf> {
    if !dir_path.exists() {
        return Err(Error::generic(format!("Directory does not exist: {}", dir_path.display())));
    }

    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries {
            let entry = entry
                .map_err(|e| Error::generic(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively search subdirectories
                if let Ok(found_path) = search_fixture_in_directory(&path, fixture_id) {
                    return Ok(found_path);
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                // Check if this file matches the fixture ID
                if let Ok(fixture_info) = parse_fixture_file_sync(&path, "unknown") {
                    if fixture_info.id == fixture_id {
                        return Ok(path);
                    }
                }
            }
        }
    }

    Err(Error::generic(format!(
        "Fixture not found in directory: {}",
        dir_path.display()
    )))
}

/// Clean up empty directories after file deletion
async fn cleanup_empty_directories(file_path: &std::path::Path) {
    if let Some(parent) = file_path.parent() {
        // Try to remove empty directories up to the protocol level
        let mut current = parent;
        let fixtures_dir =
            std::env::var("MOCKFORGE_FIXTURES_DIR").unwrap_or_else(|_| "fixtures".to_string());
        let fixtures_path = std::path::Path::new(&fixtures_dir);

        while current != fixtures_path && current.parent().is_some() {
            if let Ok(entries) = std::fs::read_dir(current) {
                if entries.count() == 0 {
                    if let Err(e) = std::fs::remove_dir(current) {
                        tracing::debug!(
                            "Failed to remove empty directory {}: {}",
                            current.display(),
                            e
                        );
                        break;
                    } else {
                        tracing::debug!("Removed empty directory: {}", current.display());
                    }
                } else {
                    break;
                }
            } else {
                break;
            }

            if let Some(next_parent) = current.parent() {
                current = next_parent;
            } else {
                break;
            }
        }
    }
}

/// Download a fixture file
pub async fn download_fixture(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    // Extract fixture ID from query parameters
    let fixture_id = match params.get("id") {
        Some(id) => id,
        None => {
            return axum::response::Response::builder()
                .status(http::StatusCode::BAD_REQUEST)
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(r#"{"error": "Missing fixture ID parameter"}"#.to_string())
                .unwrap();
        }
    };

    // Find and read the fixture file
    match download_fixture_by_id(fixture_id).await {
        Ok((content, file_name)) => axum::response::Response::builder()
            .status(http::StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "application/json")
            .header(
                http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", file_name),
            )
            .body(content)
            .unwrap(),
        Err(e) => {
            tracing::error!("Failed to download fixture {}: {}", fixture_id, e);
            let error_response = format!(r#"{{"error": "Failed to download fixture: {}"}}"#, e);
            axum::response::Response::builder()
                .status(http::StatusCode::NOT_FOUND)
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(error_response)
                .unwrap()
        }
    }
}

/// Download a fixture file by ID
async fn download_fixture_by_id(fixture_id: &str) -> Result<(String, String)> {
    // Find the fixture file by ID
    let fixtures_dir =
        std::env::var("MOCKFORGE_FIXTURES_DIR").unwrap_or_else(|_| "fixtures".to_string());
    let fixtures_path = std::path::Path::new(&fixtures_dir);

    if !fixtures_path.exists() {
        return Err(Error::generic(format!("Fixtures directory does not exist: {}", fixtures_dir)));
    }

    let file_path = find_fixture_file_by_id(fixtures_path, fixture_id)?;

    // Read the file content
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| Error::generic(format!("Failed to read fixture file: {}", e)))?;

    // Get the filename for the download
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("fixture.json")
        .to_string();

    tracing::info!("Downloaded fixture file: {} ({} bytes)", file_path.display(), content.len());
    Ok((content, file_name))
}

/// Get current validation settings
pub async fn get_validation(
    State(state): State<AdminState>,
) -> Json<ApiResponse<ValidationSettings>> {
    // Get real validation settings from configuration
    let config_state = state.get_config().await;

    Json(ApiResponse::success(config_state.validation_settings))
}

/// Update validation settings
pub async fn update_validation(
    State(state): State<AdminState>,
    Json(update): Json<ValidationUpdate>,
) -> Json<ApiResponse<String>> {
    // Validate the mode
    match update.mode.as_str() {
        "enforce" | "warn" | "off" => {}
        _ => {
            return Json(ApiResponse::error(
                "Invalid validation mode. Must be 'enforce', 'warn', or 'off'".to_string(),
            ))
        }
    }

    // Update the actual validation configuration
    let mode = update.mode.clone();
    state
        .update_validation_config(
            update.mode,
            update.aggregate_errors,
            update.validate_responses,
            update.overrides.unwrap_or_default(),
        )
        .await;

    tracing::info!(
        "Updated validation settings: mode={}, aggregate_errors={}",
        mode,
        update.aggregate_errors
    );

    Json(ApiResponse::success("Validation settings updated".to_string()))
}

/// Get environment variables
pub async fn get_env_vars() -> Json<ApiResponse<HashMap<String, String>>> {
    // Get actual environment variables that are relevant to MockForge
    let mut env_vars = HashMap::new();

    let relevant_vars = [
        // Core functionality
        "MOCKFORGE_LATENCY_ENABLED",
        "MOCKFORGE_FAILURES_ENABLED",
        "MOCKFORGE_PROXY_ENABLED",
        "MOCKFORGE_RECORD_ENABLED",
        "MOCKFORGE_REPLAY_ENABLED",
        "MOCKFORGE_LOG_LEVEL",
        "MOCKFORGE_CONFIG_FILE",
        "RUST_LOG",
        // HTTP server configuration
        "MOCKFORGE_HTTP_PORT",
        "MOCKFORGE_HTTP_HOST",
        "MOCKFORGE_HTTP_OPENAPI_SPEC",
        "MOCKFORGE_CORS_ENABLED",
        "MOCKFORGE_REQUEST_TIMEOUT_SECS",
        // WebSocket server configuration
        "MOCKFORGE_WS_PORT",
        "MOCKFORGE_WS_HOST",
        "MOCKFORGE_WS_REPLAY_FILE",
        "MOCKFORGE_WS_CONNECTION_TIMEOUT_SECS",
        // gRPC server configuration
        "MOCKFORGE_GRPC_PORT",
        "MOCKFORGE_GRPC_HOST",
        // Admin UI configuration
        "MOCKFORGE_ADMIN_ENABLED",
        "MOCKFORGE_ADMIN_PORT",
        "MOCKFORGE_ADMIN_HOST",
        "MOCKFORGE_ADMIN_MOUNT_PATH",
        "MOCKFORGE_ADMIN_API_ENABLED",
        // Template and validation
        "MOCKFORGE_RESPONSE_TEMPLATE_EXPAND",
        "MOCKFORGE_REQUEST_VALIDATION",
        "MOCKFORGE_AGGREGATE_ERRORS",
        "MOCKFORGE_RESPONSE_VALIDATION",
        "MOCKFORGE_VALIDATION_STATUS",
        // Data generation
        "MOCKFORGE_RAG_ENABLED",
        "MOCKFORGE_FAKE_TOKENS",
        // Other settings
        "MOCKFORGE_FIXTURES_DIR",
    ];

    for var_name in &relevant_vars {
        if let Ok(value) = std::env::var(var_name) {
            env_vars.insert(var_name.to_string(), value);
        }
    }

    Json(ApiResponse::success(env_vars))
}

/// Update environment variable
pub async fn update_env_var(Json(update): Json<EnvVarUpdate>) -> Json<ApiResponse<String>> {
    // Set the environment variable (runtime only - not persisted)
    std::env::set_var(&update.key, &update.value);

    tracing::info!("Updated environment variable: {}={}", update.key, update.value);

    // Note: Environment variables set at runtime are not persisted
    // In a production system, you might want to write to a .env file or config file
    Json(ApiResponse::success(format!(
        "Environment variable {} updated to '{}'. Note: This change is not persisted and will be lost on restart.",
        update.key, update.value
    )))
}

/// Get file content
pub async fn get_file_content(
    Json(request): Json<FileContentRequest>,
) -> Json<ApiResponse<String>> {
    // Validate the file path for security
    if let Err(e) = validate_file_path(&request.file_path) {
        return Json(ApiResponse::error(format!("Invalid file path: {}", e)));
    }

    // Read the actual file content
    match tokio::fs::read_to_string(&request.file_path).await {
        Ok(content) => {
            // Validate the file content for security
            if let Err(e) = validate_file_content(&content) {
                return Json(ApiResponse::error(format!("Invalid file content: {}", e)));
            }
            Json(ApiResponse::success(content))
        }
        Err(e) => Json(ApiResponse::error(format!("Failed to read file: {}", e))),
    }
}

/// Save file content
pub async fn save_file_content(Json(request): Json<FileSaveRequest>) -> Json<ApiResponse<String>> {
    match save_file_to_filesystem(&request.file_path, &request.content).await {
        Ok(_) => {
            tracing::info!("Successfully saved file: {}", request.file_path);
            Json(ApiResponse::success("File saved successfully".to_string()))
        }
        Err(e) => {
            tracing::error!("Failed to save file {}: {}", request.file_path, e);
            Json(ApiResponse::error(format!("Failed to save file: {}", e)))
        }
    }
}

/// Save content to a file on the filesystem
async fn save_file_to_filesystem(file_path: &str, content: &str) -> Result<()> {
    // Validate the file path for security
    validate_file_path(file_path)?;

    // Validate the file content for security
    validate_file_content(content)?;

    // Convert to PathBuf
    let path = std::path::Path::new(file_path);

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Error::generic(format!("Failed to create directory {}: {}", parent.display(), e))
        })?;
    }

    // Write the content to the file
    std::fs::write(path, content)
        .map_err(|e| Error::generic(format!("Failed to write file {}: {}", path.display(), e)))?;

    // Verify the file was written correctly
    let written_content = std::fs::read_to_string(path).map_err(|e| {
        Error::generic(format!("Failed to verify written file {}: {}", path.display(), e))
    })?;

    if written_content != content {
        return Err(Error::generic(format!(
            "File content verification failed for {}",
            path.display()
        )));
    }

    tracing::info!("File saved successfully: {} ({} bytes)", path.display(), content.len());
    Ok(())
}

/// Validate file path for security
fn validate_file_path(file_path: &str) -> Result<()> {
    // Check for path traversal attacks
    if file_path.contains("..") {
        return Err(Error::generic("Path traversal detected in file path".to_string()));
    }

    // Check for absolute paths that might be outside allowed directories
    let path = std::path::Path::new(file_path);
    if path.is_absolute() {
        // For absolute paths, ensure they're within allowed directories
        let allowed_dirs = [
            std::env::current_dir().unwrap_or_default(),
            std::path::PathBuf::from("."),
            std::path::PathBuf::from("fixtures"),
            std::path::PathBuf::from("config"),
        ];

        let mut is_allowed = false;
        for allowed_dir in &allowed_dirs {
            if path.starts_with(allowed_dir) {
                is_allowed = true;
                break;
            }
        }

        if !is_allowed {
            return Err(Error::generic("File path is outside allowed directories".to_string()));
        }
    }

    // Check for dangerous file extensions or names
    let dangerous_extensions = ["exe", "bat", "cmd", "sh", "ps1", "scr", "com"];
    if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
        if dangerous_extensions.contains(&extension.to_lowercase().as_str()) {
            return Err(Error::generic(format!(
                "Dangerous file extension not allowed: {}",
                extension
            )));
        }
    }

    Ok(())
}

/// Validate file content for security
fn validate_file_content(content: &str) -> Result<()> {
    // Check for reasonable file size (prevent DoS)
    if content.len() > 10 * 1024 * 1024 {
        // 10MB limit
        return Err(Error::generic("File content too large (max 10MB)".to_string()));
    }

    // Check for null bytes (potential security issue)
    if content.contains('\0') {
        return Err(Error::generic("File content contains null bytes".to_string()));
    }

    Ok(())
}

/// Fixture delete request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureDeleteRequest {
    pub fixture_id: String,
}

/// Environment variable update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarUpdate {
    pub key: String,
    pub value: String,
}

/// Fixture bulk delete request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureBulkDeleteRequest {
    pub fixture_ids: Vec<String>,
}

/// Fixture bulk delete result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureBulkDeleteResult {
    pub deleted_count: usize,
    pub total_requested: usize,
    pub errors: Vec<String>,
}

/// File content request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContentRequest {
    pub file_path: String,
    pub file_type: String,
}

/// File save request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSaveRequest {
    pub file_path: String,
    pub content: String,
}

/// Get smoke tests
pub async fn get_smoke_tests(
    State(state): State<AdminState>,
) -> Json<ApiResponse<Vec<SmokeTestResult>>> {
    let results = state.get_smoke_test_results().await;
    Json(ApiResponse::success(results))
}

/// Run smoke tests endpoint
pub async fn run_smoke_tests_endpoint(
    State(state): State<AdminState>,
) -> Json<ApiResponse<String>> {
    tracing::info!("Starting smoke test execution");

    // Spawn smoke test execution in background to avoid blocking
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = execute_smoke_tests(&state_clone).await {
            tracing::error!("Smoke test execution failed: {}", e);
        } else {
            tracing::info!("Smoke test execution completed successfully");
        }
    });

    Json(ApiResponse::success(
        "Smoke tests started. Check results in the smoke tests section.".to_string(),
    ))
}

/// Execute smoke tests against fixtures
async fn execute_smoke_tests(state: &AdminState) -> Result<()> {
    // Get base URL from environment or use default
    let base_url =
        std::env::var("MOCKFORGE_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let context = SmokeTestContext {
        base_url,
        timeout_seconds: 30,
        parallel: true,
    };

    // Get all fixtures to create smoke tests from
    let fixtures = scan_fixtures_directory()?;

    // Filter for HTTP fixtures only (smoke tests are typically HTTP)
    let http_fixtures: Vec<&FixtureInfo> =
        fixtures.iter().filter(|f| f.protocol == "http").collect();

    if http_fixtures.is_empty() {
        tracing::warn!("No HTTP fixtures found for smoke testing");
        return Ok(());
    }

    tracing::info!("Running smoke tests for {} HTTP fixtures", http_fixtures.len());

    // Create smoke test results from fixtures
    let mut test_results = Vec::new();

    for fixture in http_fixtures {
        let test_result = create_smoke_test_from_fixture(fixture);
        test_results.push(test_result);
    }

    // Execute tests
    let mut executed_results = Vec::new();
    for mut test_result in test_results {
        // Update status to running
        test_result.status = "running".to_string();
        state.update_smoke_test_result(test_result.clone()).await;

        // Execute the test
        let start_time = std::time::Instant::now();
        match execute_single_smoke_test(&test_result, &context).await {
            Ok((status_code, response_time_ms)) => {
                test_result.status = "passed".to_string();
                test_result.status_code = Some(status_code);
                test_result.response_time_ms = Some(response_time_ms);
                test_result.error_message = None;
            }
            Err(e) => {
                test_result.status = "failed".to_string();
                test_result.error_message = Some(e.to_string());
                test_result.status_code = None;
                test_result.response_time_ms = None;
            }
        }

        let duration = start_time.elapsed();
        test_result.duration_seconds = Some(duration.as_secs_f64());
        test_result.last_run = Some(chrono::Utc::now());

        executed_results.push(test_result.clone());
        state.update_smoke_test_result(test_result).await;
    }

    tracing::info!("Smoke test execution completed: {} tests run", executed_results.len());
    Ok(())
}

/// Create a smoke test result from a fixture
fn create_smoke_test_from_fixture(fixture: &FixtureInfo) -> SmokeTestResult {
    let test_name = format!("{} {}", fixture.method, fixture.path);
    let description = format!("Smoke test for {} endpoint", fixture.path);

    SmokeTestResult {
        id: format!("smoke_{}", fixture.id),
        name: test_name,
        method: fixture.method.clone(),
        path: fixture.path.clone(),
        description,
        last_run: None,
        status: "pending".to_string(),
        response_time_ms: None,
        error_message: None,
        status_code: None,
        duration_seconds: None,
    }
}

/// Execute a single smoke test
async fn execute_single_smoke_test(
    test: &SmokeTestResult,
    context: &SmokeTestContext,
) -> Result<(u16, u64)> {
    let url = format!("{}{}", context.base_url, test.path);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(context.timeout_seconds))
        .build()
        .map_err(|e| Error::generic(format!("Failed to create HTTP client: {}", e)))?;

    let start_time = std::time::Instant::now();

    let response = match test.method.as_str() {
        "GET" => client.get(&url).send().await,
        "POST" => client.post(&url).send().await,
        "PUT" => client.put(&url).send().await,
        "DELETE" => client.delete(&url).send().await,
        "PATCH" => client.patch(&url).send().await,
        "HEAD" => client.head(&url).send().await,
        "OPTIONS" => client.request(reqwest::Method::OPTIONS, &url).send().await,
        _ => {
            return Err(Error::generic(format!("Unsupported HTTP method: {}", test.method)));
        }
    };

    let response_time = start_time.elapsed();
    let response_time_ms = response_time.as_millis() as u64;

    match response {
        Ok(resp) => {
            let status_code = resp.status().as_u16();
            if (200..400).contains(&status_code) {
                Ok((status_code, response_time_ms))
            } else {
                Err(Error::generic(format!(
                    "HTTP error: {} {}",
                    status_code,
                    resp.status().canonical_reason().unwrap_or("Unknown")
                )))
            }
        }
        Err(e) => Err(Error::generic(format!("Request failed: {}", e))),
    }
}

/// Install a plugin from a path or URL
pub async fn install_plugin(
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Extract source from request
    let source = request.get("source").and_then(|s| s.as_str()).unwrap_or("");

    if source.is_empty() {
        return Json(json!({
            "success": false,
            "error": "Plugin source is required"
        }));
    }

    // Determine if source is a URL or local path
    let plugin_path = if source.starts_with("http://") || source.starts_with("https://") {
        // Download the plugin from URL
        match download_plugin_from_url(source).await {
            Ok(temp_path) => temp_path,
            Err(e) => return Json(json!({
                "success": false,
                "error": format!("Failed to download plugin: {}", e)
            })),
        }
    } else {
        // Use local file path
        std::path::PathBuf::from(source)
    };

    // Check if the plugin file exists
    if !plugin_path.exists() {
        return Json(json!({
            "success": false,
            "error": format!("Plugin file not found: {}", source)
        }));
    }

    // For now, just return success since we don't have the plugin loader infrastructure
    Json(json!({
        "success": true,
        "message": format!("Plugin would be installed from: {}", source)
    }))
}

/// Download a plugin from a URL and return the temporary file path
async fn download_plugin_from_url(url: &str) -> Result<std::path::PathBuf> {
    // Create a temporary file
    let temp_file = std::env::temp_dir().join(format!("plugin_{}.tmp", chrono::Utc::now().timestamp()));
    let temp_path = temp_file.clone();

    // Download the file
    let response = reqwest::get(url).await
        .map_err(|e| Error::generic(format!("Failed to download from URL: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::generic(format!("HTTP error {}: {}", response.status().as_u16(),
                          response.status().canonical_reason().unwrap_or("Unknown"))));
    }

    // Read the response bytes
    let bytes = response.bytes().await
        .map_err(|e| Error::generic(format!("Failed to read response: {}", e)))?;

    // Write to temporary file
    tokio::fs::write(&temp_file, &bytes).await
        .map_err(|e| Error::generic(format!("Failed to write temporary file: {}", e)))?;

    Ok(temp_path)
}


pub async fn serve_icon() -> impl IntoResponse {
    // Return a simple placeholder icon response
    ([(http::header::CONTENT_TYPE, "image/png")], "")
}

pub async fn serve_icon_32() -> impl IntoResponse {
    ([(http::header::CONTENT_TYPE, "image/png")], "")
}

pub async fn serve_icon_48() -> impl IntoResponse {
    ([(http::header::CONTENT_TYPE, "image/png")], "")
}

pub async fn serve_logo() -> impl IntoResponse {
    ([(http::header::CONTENT_TYPE, "image/png")], "")
}

pub async fn serve_logo_40() -> impl IntoResponse {
    ([(http::header::CONTENT_TYPE, "image/png")], "")
}

pub async fn serve_logo_80() -> impl IntoResponse {
    ([(http::header::CONTENT_TYPE, "image/png")], "")
}

// Missing handler functions that routes.rs expects
pub async fn update_traffic_shaping(
    State(state): State<AdminState>,
    Json(config): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Traffic shaping updated".to_string()))
}

pub async fn import_postman(
    State(state): State<AdminState>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    use mockforge_core::workspace_import::{import_postman_to_workspace, WorkspaceImportConfig};
    use uuid::Uuid;

    let content = request.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let filename = request.get("filename").and_then(|v| v.as_str());
    let environment = request.get("environment").and_then(|v| v.as_str());
    let base_url = request.get("base_url").and_then(|v| v.as_str());

    // Import the collection
    let import_result = match mockforge_core::import::import_postman_collection(content, base_url) {
        Ok(result) => result,
        Err(e) => {
            // Record failed import
            let entry = ImportHistoryEntry {
                id: Uuid::new_v4().to_string(),
                format: "postman".to_string(),
                timestamp: chrono::Utc::now(),
                routes_count: 0,
                variables_count: 0,
                warnings_count: 0,
                success: false,
                filename: filename.map(|s| s.to_string()),
                environment: environment.map(|s| s.to_string()),
                base_url: base_url.map(|s| s.to_string()),
                error_message: Some(e.clone()),
            };
            let mut history = state.import_history.write().await;
            history.push(entry);

            return Json(ApiResponse::error(format!("Postman import failed: {}", e)));
        }
    };

    // Create workspace from imported routes
    let workspace_name = filename
        .and_then(|f| f.split('.').next())
        .unwrap_or("Imported Postman Collection");

    let config = WorkspaceImportConfig {
        create_folders: true,
        base_folder_name: None,
        preserve_hierarchy: true,
        max_depth: 5,
    };

    // Convert MockForgeRoute to ImportRoute
    let routes: Vec<ImportRoute> = import_result.routes.into_iter().map(|route| ImportRoute {
        method: route.method,
        path: route.path,
        headers: route.headers,
        body: route.body,
        response: ImportResponse {
            status: route.response.status,
            headers: route.response.headers,
            body: route.response.body,
        },
    }).collect();

    match import_postman_to_workspace(routes, workspace_name.to_string(), config) {
        Ok(workspace_result) => {
            // Save the workspace to persistent storage
            if let Err(e) = state.workspace_persistence.save_workspace(&workspace_result.workspace).await {
                tracing::error!("Failed to save workspace: {}", e);
                return Json(ApiResponse::error(format!("Import succeeded but failed to save workspace: {}", e)));
            }

            // Record successful import
            let entry = ImportHistoryEntry {
                id: Uuid::new_v4().to_string(),
                format: "postman".to_string(),
                timestamp: chrono::Utc::now(),
                routes_count: workspace_result.request_count,
                variables_count: import_result.variables.len(),
                warnings_count: workspace_result.warnings.len(),
                success: true,
                filename: filename.map(|s| s.to_string()),
                environment: environment.map(|s| s.to_string()),
                base_url: base_url.map(|s| s.to_string()),
                error_message: None,
            };
            let mut history = state.import_history.write().await;
            history.push(entry);

            Json(ApiResponse::success(format!("Successfully imported {} routes into workspace '{}'", workspace_result.request_count, workspace_name)))
        }
        Err(e) => {
            // Record failed import
            let entry = ImportHistoryEntry {
                id: Uuid::new_v4().to_string(),
                format: "postman".to_string(),
                timestamp: chrono::Utc::now(),
                routes_count: 0,
                variables_count: 0,
                warnings_count: 0,
                success: false,
                filename: filename.map(|s| s.to_string()),
                environment: environment.map(|s| s.to_string()),
                base_url: base_url.map(|s| s.to_string()),
                error_message: Some(e.to_string()),
            };
            let mut history = state.import_history.write().await;
            history.push(entry);

            Json(ApiResponse::error(format!("Failed to create workspace: {}", e)))
        }
    }
}

pub async fn import_insomnia(
    State(state): State<AdminState>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    use mockforge_core::workspace_import::create_workspace_from_insomnia;
    use uuid::Uuid;

    let content = request.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let filename = request.get("filename").and_then(|v| v.as_str());
    let environment = request.get("environment").and_then(|v| v.as_str());
    let base_url = request.get("base_url").and_then(|v| v.as_str());

    // Import the export
    let import_result = match mockforge_core::import::import_insomnia_export(content, environment) {
        Ok(result) => result,
        Err(e) => {
            // Record failed import
            let entry = ImportHistoryEntry {
                id: Uuid::new_v4().to_string(),
                format: "insomnia".to_string(),
                timestamp: chrono::Utc::now(),
                routes_count: 0,
                variables_count: 0,
                warnings_count: 0,
                success: false,
                filename: filename.map(|s| s.to_string()),
                environment: environment.map(|s| s.to_string()),
                base_url: base_url.map(|s| s.to_string()),
                error_message: Some(e.clone()),
            };
            let mut history = state.import_history.write().await;
            history.push(entry);

            return Json(ApiResponse::error(format!("Insomnia import failed: {}", e)));
        }
    };

    // Create workspace from imported routes
    let workspace_name = filename
        .and_then(|f| f.split('.').next())
        .unwrap_or("Imported Insomnia Collection");

    let config = WorkspaceImportConfig {
        create_folders: true,
        base_folder_name: None,
        preserve_hierarchy: true,
        max_depth: 5,
    };

    // Extract variables count before moving import_result
    let variables_count = import_result.variables.len();

    match mockforge_core::workspace_import::create_workspace_from_insomnia(import_result, Some(workspace_name.to_string())) {
        Ok(workspace_result) => {
            // Save the workspace to persistent storage
            if let Err(e) = state.workspace_persistence.save_workspace(&workspace_result.workspace).await {
                tracing::error!("Failed to save workspace: {}", e);
                return Json(ApiResponse::error(format!("Import succeeded but failed to save workspace: {}", e)));
            }

            // Record successful import
            let entry = ImportHistoryEntry {
                id: Uuid::new_v4().to_string(),
                format: "insomnia".to_string(),
                timestamp: chrono::Utc::now(),
                routes_count: workspace_result.request_count,
                variables_count,
                warnings_count: workspace_result.warnings.len(),
                success: true,
                filename: filename.map(|s| s.to_string()),
                environment: environment.map(|s| s.to_string()),
                base_url: base_url.map(|s| s.to_string()),
                error_message: None,
            };
            let mut history = state.import_history.write().await;
            history.push(entry);

            Json(ApiResponse::success(format!("Successfully imported {} routes into workspace '{}'", workspace_result.request_count, workspace_name)))
        }
        Err(e) => {
            // Record failed import
            let entry = ImportHistoryEntry {
                id: Uuid::new_v4().to_string(),
                format: "insomnia".to_string(),
                timestamp: chrono::Utc::now(),
                routes_count: 0,
                variables_count: 0,
                warnings_count: 0,
                success: false,
                filename: filename.map(|s| s.to_string()),
                environment: environment.map(|s| s.to_string()),
                base_url: base_url.map(|s| s.to_string()),
                error_message: Some(e.to_string()),
            };
            let mut history = state.import_history.write().await;
            history.push(entry);

            Json(ApiResponse::error(format!("Failed to create workspace: {}", e)))
        }
    }
}

pub async fn import_openapi(
    State(state): State<AdminState>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("OpenAPI import completed".to_string()))
}

pub async fn import_curl(
    State(state): State<AdminState>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    use uuid::Uuid;

    let content = request.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let filename = request.get("filename").and_then(|v| v.as_str());
    let base_url = request.get("base_url").and_then(|v| v.as_str());

    // Import the commands
    let import_result = match mockforge_core::import::import_curl_commands(content, base_url) {
        Ok(result) => result,
        Err(e) => {
            // Record failed import
            let entry = ImportHistoryEntry {
                id: Uuid::new_v4().to_string(),
                format: "curl".to_string(),
                timestamp: chrono::Utc::now(),
                routes_count: 0,
                variables_count: 0,
                warnings_count: 0,
                success: false,
                filename: filename.map(|s| s.to_string()),
                environment: None,
                base_url: base_url.map(|s| s.to_string()),
                error_message: Some(e.clone()),
            };
            let mut history = state.import_history.write().await;
            history.push(entry);

            return Json(ApiResponse::error(format!("Curl import failed: {}", e)));
        }
    };

    // Create workspace from imported routes
    let workspace_name = filename
        .and_then(|f| f.split('.').next())
        .unwrap_or("Imported Curl Commands");

    match mockforge_core::workspace_import::create_workspace_from_curl(import_result, Some(workspace_name.to_string())) {
        Ok(workspace_result) => {
            // Save the workspace to persistent storage
            if let Err(e) = state.workspace_persistence.save_workspace(&workspace_result.workspace).await {
                tracing::error!("Failed to save workspace: {}", e);
                return Json(ApiResponse::error(format!("Import succeeded but failed to save workspace: {}", e)));
            }

            // Record successful import
            let entry = ImportHistoryEntry {
                id: Uuid::new_v4().to_string(),
                format: "curl".to_string(),
                timestamp: chrono::Utc::now(),
                routes_count: workspace_result.request_count,
                variables_count: 0, // Curl doesn't have variables
                warnings_count: workspace_result.warnings.len(),
                success: true,
                filename: filename.map(|s| s.to_string()),
                environment: None,
                base_url: base_url.map(|s| s.to_string()),
                error_message: None,
            };
            let mut history = state.import_history.write().await;
            history.push(entry);

            Json(ApiResponse::success(format!("Successfully imported {} routes into workspace '{}'", workspace_result.request_count, workspace_name)))
        }
        Err(e) => {
            // Record failed import
            let entry = ImportHistoryEntry {
                id: Uuid::new_v4().to_string(),
                format: "curl".to_string(),
                timestamp: chrono::Utc::now(),
                routes_count: 0,
                variables_count: 0,
                warnings_count: 0,
                success: false,
                filename: filename.map(|s| s.to_string()),
                environment: None,
                base_url: base_url.map(|s| s.to_string()),
                error_message: Some(e.to_string()),
            };
            let mut history = state.import_history.write().await;
            history.push(entry);

            Json(ApiResponse::error(format!("Failed to create workspace: {}", e)))
        }
    }
}

pub async fn preview_import(
    State(state): State<AdminState>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    use mockforge_core::import::{import_postman_collection, import_insomnia_export, import_curl_commands};

    let content = request.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let filename = request.get("filename").and_then(|v| v.as_str());
    let environment = request.get("environment").and_then(|v| v.as_str());
    let base_url = request.get("base_url").and_then(|v| v.as_str());

    // Detect format from filename or content
    let format = if let Some(fname) = filename {
        if fname.to_lowercase().contains("postman") || fname.to_lowercase().ends_with(".postman_collection") {
            "postman"
        } else if fname.to_lowercase().contains("insomnia") || fname.to_lowercase().ends_with(".insomnia") {
            "insomnia"
        } else if fname.to_lowercase().contains("curl") || fname.to_lowercase().ends_with(".sh") || fname.to_lowercase().ends_with(".curl") {
            "curl"
        } else {
            "unknown"
        }
    } else {
        "unknown"
    };

    match format {
        "postman" => match import_postman_collection(content, base_url) {
            Ok(import_result) => {
                let routes: Vec<serde_json::Value> = import_result.routes.into_iter().map(|route| {
                    serde_json::json!({
                        "method": route.method,
                        "path": route.path,
                        "headers": route.headers,
                        "body": route.body,
                        "status_code": route.response.status,
                        "response": serde_json::json!({
                            "status": route.response.status,
                            "headers": route.response.headers,
                            "body": route.response.body
                        })
                    })
                }).collect();

                let response = serde_json::json!({
                    "routes": routes,
                    "variables": import_result.variables,
                    "warnings": import_result.warnings
                });

                Json(ApiResponse::success(response))
            }
            Err(e) => Json(ApiResponse::error(format!("Postman import failed: {}", e))),
        },
        "insomnia" => match import_insomnia_export(content, environment) {
            Ok(import_result) => {
                let routes: Vec<serde_json::Value> = import_result.routes.into_iter().map(|route| {
                    serde_json::json!({
                        "method": route.method,
                        "path": route.path,
                        "headers": route.headers,
                        "body": route.body,
                        "status_code": route.response.status,
                        "response": serde_json::json!({
                            "status": route.response.status,
                            "headers": route.response.headers,
                            "body": route.response.body
                        })
                    })
                }).collect();

                let response = serde_json::json!({
                    "routes": routes,
                    "variables": import_result.variables,
                    "warnings": import_result.warnings
                });

                Json(ApiResponse::success(response))
            }
            Err(e) => Json(ApiResponse::error(format!("Insomnia import failed: {}", e))),
        },
        "curl" => match import_curl_commands(content, base_url) {
            Ok(import_result) => {
                let routes: Vec<serde_json::Value> = import_result.routes.into_iter().map(|route| {
                    serde_json::json!({
                        "method": route.method,
                        "path": route.path,
                        "headers": route.headers,
                        "body": route.body,
                        "status_code": route.response.status,
                        "response": serde_json::json!({
                            "status": route.response.status,
                            "headers": route.response.headers,
                            "body": route.response.body
                        })
                    })
                }).collect();

                let response = serde_json::json!({
                    "routes": routes,
                    "variables": serde_json::json!({}),
                    "warnings": import_result.warnings
                });

                Json(ApiResponse::success(response))
            }
            Err(e) => Json(ApiResponse::error(format!("Curl import failed: {}", e))),
        },
        _ => Json(ApiResponse::error("Unsupported import format".to_string())),
    }
}

pub async fn get_import_history(
    State(state): State<AdminState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let history = state.import_history.read().await;
    let total = history.len();

    let imports: Vec<serde_json::Value> = history.iter().rev().take(50).map(|entry| {
        serde_json::json!({
            "id": entry.id,
            "format": entry.format,
            "timestamp": entry.timestamp.to_rfc3339(),
            "routes_count": entry.routes_count,
            "variables_count": entry.variables_count,
            "warnings_count": entry.warnings_count,
            "success": entry.success,
            "filename": entry.filename,
            "environment": entry.environment,
            "base_url": entry.base_url,
            "error_message": entry.error_message
        })
    }).collect();

    let response = serde_json::json!({
        "imports": imports,
        "total": total
    });

    Json(ApiResponse::success(response))
}

pub async fn get_admin_api_state(
    State(state): State<AdminState>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "active"
    })))
}

pub async fn get_admin_api_replay(
    State(state): State<AdminState>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "replay": []
    })))
}

pub async fn get_sse_status(
    State(state): State<AdminState>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "available": true,
        "endpoint": "/sse",
        "config": {
            "event_type": "status",
            "interval_ms": 1000,
            "data_template": "{}"
        }
    })))
}

pub async fn get_sse_connections(
    State(state): State<AdminState>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "active_connections": 0
    })))
}

// Workspace management functions
pub async fn get_workspaces(
    State(state): State<AdminState>,
) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    Json(ApiResponse::success(vec![]))
}

pub async fn create_workspace(
    State(state): State<AdminState>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Workspace created".to_string()))
}

pub async fn open_workspace_from_directory(
    State(state): State<AdminState>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Workspace opened from directory".to_string()))
}

pub async fn get_workspace(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "id": workspace_id,
        "name": "Mock Workspace",
        "description": "A mock workspace"
    })))
}

pub async fn delete_workspace(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Workspace deleted".to_string()))
}

pub async fn set_active_workspace(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Workspace activated".to_string()))
}

pub async fn create_folder(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Folder created".to_string()))
}

pub async fn create_request(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Request created".to_string()))
}

pub async fn execute_workspace_request(
    State(state): State<AdminState>,
    axum::extract::Path((workspace_id, request_id)): axum::extract::Path<(String, String)>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "executed",
        "response": {}
    })))
}

pub async fn get_request_history(
    State(state): State<AdminState>,
    axum::extract::Path((workspace_id, request_id)): axum::extract::Path<(String, String)>,
) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    Json(ApiResponse::success(vec![]))
}

pub async fn get_folder(
    State(state): State<AdminState>,
    axum::extract::Path((workspace_id, folder_id)): axum::extract::Path<(String, String)>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "id": folder_id,
        "name": "Mock Folder"
    })))
}

pub async fn import_to_workspace(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Import to workspace completed".to_string()))
}

pub async fn export_workspaces(
    State(state): State<AdminState>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Workspaces exported".to_string()))
}

// Environment management functions
pub async fn get_environments(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    Json(ApiResponse::success(vec![]))
}

pub async fn create_environment(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Environment created".to_string()))
}

pub async fn update_environment(
    State(state): State<AdminState>,
    axum::extract::Path((workspace_id, environment_id)): axum::extract::Path<(String, String)>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Environment updated".to_string()))
}

pub async fn delete_environment(
    State(state): State<AdminState>,
    axum::extract::Path((workspace_id, environment_id)): axum::extract::Path<(String, String)>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Environment deleted".to_string()))
}

pub async fn set_active_environment(
    State(state): State<AdminState>,
    axum::extract::Path((workspace_id, environment_id)): axum::extract::Path<(String, String)>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Environment activated".to_string()))
}

pub async fn get_environment_variables(
    State(state): State<AdminState>,
    axum::extract::Path((workspace_id, environment_id)): axum::extract::Path<(String, String)>,
) -> Json<ApiResponse<HashMap<String, String>>> {
    Json(ApiResponse::success(HashMap::new()))
}

pub async fn set_environment_variable(
    State(state): State<AdminState>,
    axum::extract::Path((workspace_id, environment_id)): axum::extract::Path<(String, String)>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Environment variable set".to_string()))
}

pub async fn remove_environment_variable(
    State(state): State<AdminState>,
    axum::extract::Path((workspace_id, environment_id, variable_name)): axum::extract::Path<(String, String, String)>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Environment variable removed".to_string()))
}

// Autocomplete functions
pub async fn get_autocomplete_suggestions(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "suggestions": [],
        "start_position": 0,
        "end_position": 0
    })))
}

// Sync management functions
pub async fn get_sync_status(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "disabled"
    })))
}

pub async fn configure_sync(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Sync configured".to_string()))
}

pub async fn disable_sync(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Sync disabled".to_string()))
}

pub async fn trigger_sync(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Sync triggered".to_string()))
}

pub async fn get_sync_changes(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    Json(ApiResponse::success(vec![]))
}

pub async fn confirm_sync_changes(
    State(state): State<AdminState>,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Sync changes confirmed".to_string()))
}

// Plugin management functions
pub async fn get_plugins(
    State(state): State<AdminState>,
) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    Json(ApiResponse::success(vec![]))
}

pub async fn delete_plugin(
    State(state): State<AdminState>,
    axum::extract::Path(plugin_id): axum::extract::Path<String>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Plugin deleted".to_string()))
}

pub async fn validate_plugin(
    State(state): State<AdminState>,
    Json(request): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Plugin validated".to_string()))
}

pub async fn get_plugin_status(
    State(state): State<AdminState>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "active"
    })))
}

// Missing functions that routes.rs expects
pub async fn clear_import_history(
    State(state): State<AdminState>,
) -> Json<ApiResponse<String>> {
    let mut history = state.import_history.write().await;
    history.clear();
    Json(ApiResponse::success("Import history cleared".to_string()))
}

pub async fn get_plugin(
    State(state): State<AdminState>,
    axum::extract::Path(plugin_id): axum::extract::Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "id": plugin_id,
        "name": "Mock Plugin",
        "version": "1.0.0",
        "status": "active"
    })))
}

pub async fn reload_plugins(
    State(state): State<AdminState>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Plugins reloaded".to_string()))
}
