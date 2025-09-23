//! Admin dashboard and server management handlers
//!
//! This module handles admin dashboard operations, server management,
//! metrics, logs, and configuration.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::System;

use crate::models::*;
use mockforge_core::{Error, Result, CentralizedRequestLogger};

/// Request metrics for tracking
#[derive(Debug, Clone, Default)]
pub struct RequestMetrics {
    /// Total requests served
    pub total_requests: u64,
    /// Active requests currently being processed
    pub active_requests: u64,
    /// Average response time in milliseconds
    pub average_response_time: f64,
    /// Request rate per second
    pub requests_per_second: f64,
    /// Total errors encountered
    pub total_errors: u64,
}

/// Admin state containing server configuration and runtime data
#[derive(Debug, Clone)]
pub struct AdminState {
    /// HTTP server address
    pub http_server_addr: Option<std::net::SocketAddr>,
    /// WebSocket server address
    pub ws_server_addr: Option<std::net::SocketAddr>,
    /// gRPC server address
    pub grpc_server_addr: Option<std::net::SocketAddr>,
    /// GraphQL server address
    pub graphql_server_addr: Option<std::net::SocketAddr>,
    /// Whether API is enabled
    pub api_enabled: bool,
    /// Request metrics
    pub metrics: Arc<std::sync::Mutex<RequestMetrics>>,
    /// Centralized request logger (static reference)
    pub logger: &'static CentralizedRequestLogger,
}

impl AdminState {
    /// Create a new admin state
    pub fn new(
        http_server_addr: Option<std::net::SocketAddr>,
        ws_server_addr: Option<std::net::SocketAddr>,
        grpc_server_addr: Option<std::net::SocketAddr>,
        graphql_server_addr: Option<std::net::SocketAddr>,
        api_enabled: bool,
        logger: &'static CentralizedRequestLogger,
    ) -> Self {
        Self {
            http_server_addr,
            ws_server_addr,
            grpc_server_addr,
            graphql_server_addr,
            api_enabled,
            metrics: Arc::new(std::sync::Mutex::new(RequestMetrics::default())),
            logger,
        }
    }
}

/// Get dashboard data
pub async fn get_dashboard(State(state): State<AdminState>) -> Json<ApiResponse<DashboardData>> {
    let metrics = state.metrics.lock().unwrap();
    Json(ApiResponse::success(DashboardData {
        server_info: ServerInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_time: env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown"),
            git_sha: env!("VERGEN_GIT_SHA").unwrap_or("unknown"),
        },
        system_info: DashboardSystemInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            uptime: System::uptime() as u64,
            memory_usage: System::new_all().total_memory() - System::new_all().available_memory(),
        },
        metrics: SimpleMetricsData {
            total_requests: metrics.total_requests,
            active_requests: metrics.active_requests,
            average_response_time: metrics.average_response_time,
            error_rate: if metrics.total_requests == 0 {
                0.0
            } else {
                metrics.total_errors as f64 / metrics.total_requests as f64
            },
        },
    }))
}

/// Get server information
pub async fn get_server_info(State(state): State<AdminState>) -> Json<Value> {
    Json(json!({
        "http_server": state.http_server_addr.map(|addr| addr.to_string()).unwrap_or_else(|| "disabled".to_string()),
        "ws_server": state.ws_server_addr.map(|addr| addr.to_string()).unwrap_or_else(|| "disabled".to_string()),
        "grpc_server": state.grpc_server_addr.map(|addr| addr.to_string()).unwrap_or_else(|| "disabled".to_string()),
        "graphql_server": state.graphql_server_addr.map(|addr| addr.to_string()).unwrap_or_else(|| "disabled".to_string()),
        "api_enabled": state.api_enabled,
    }))
}

/// Get health check status
pub async fn get_health() -> Json<HealthCheck> {
    Json(HealthCheck {
        status: "healthy".to_string(),
        services: HashMap::new(),
        last_check: Utc::now(),
        issues: Vec::new(),
    })
}

/// Get logs
pub async fn get_logs(
    State(state): State<AdminState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<LogEntry>>> {
    // Parse query parameters for filtering
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(100);

    let method_filter = params.get("method").map(|s| s.to_string());
    let path_filter = params.get("path").map(|s| s.to_string());
    let status_filter = params.get("status").and_then(|s| s.parse::<u16>().ok());

    // Get recent logs from the centralized logger
    let request_logs = state.logger.get_recent_logs(Some(limit * 2)).await; // Get more to filter

    // Convert RequestLogEntry to LogEntry and apply filters
    let mut log_entries: Vec<LogEntry> = request_logs
        .into_iter()
        .filter(|log| {
            // Only include HTTP logs for now (matching the UI interface)
            log.server_type == "HTTP"
        })
        .filter(|log| {
            // Apply method filter
            method_filter.as_ref().map_or(true, |filter| log.method == *filter)
        })
        .filter(|log| {
            // Apply path filter (simple substring match)
            path_filter.as_ref().map_or(true, |filter| log.path.contains(filter))
        })
        .filter(|log| {
            // Apply status filter
            status_filter.map_or(true, |filter| log.status_code == filter)
        })
        .map(|log| LogEntry {
            timestamp: log.timestamp,
            status: log.status_code,
            method: log.method,
            url: log.path,
            response_time: log.response_time_ms,
            size: log.response_size_bytes,
        })
        .take(limit)
        .collect();

    // Sort by timestamp descending (most recent first)
    log_entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Json(ApiResponse::success(log_entries))
}

/// Get metrics data
pub async fn get_metrics(State(state): State<AdminState>) -> Json<ApiResponse<SimpleMetricsData>> {
    let metrics = state.metrics.lock().unwrap();
    let error_rate = if metrics.total_requests > 0 {
        metrics.total_errors as f64 / metrics.total_requests as f64
    } else {
        0.0
    };
    Json(ApiResponse::success(SimpleMetricsData {
        total_requests: metrics.total_requests,
        active_requests: metrics.active_requests,
        average_response_time: metrics.average_response_time,
        error_rate,
    }))
}

/// Update latency configuration (placeholder)
pub async fn update_latency(
    State(_state): State<AdminState>,
    Json(_config): Json<Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Latency configuration updated".to_string()))
}

/// Update fault injection configuration (placeholder)
pub async fn update_faults(
    State(_state): State<AdminState>,
    Json(_config): Json<Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Fault configuration updated".to_string()))
}

/// Update proxy configuration (placeholder)
pub async fn update_proxy(
    State(_state): State<AdminState>,
    Json(_config): Json<Value>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Proxy configuration updated".to_string()))
}

/// Clear logs (placeholder)
pub async fn clear_logs(State(_state): State<AdminState>) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Logs cleared".to_string()))
}

/// Restart servers (placeholder)
pub async fn restart_servers(State(_state): State<AdminState>) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Servers restarted".to_string()))
}

/// Get restart status (placeholder)
pub async fn get_restart_status() -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Ready".to_string()))
}

/// Get configuration (placeholder)
pub async fn get_config(State(_state): State<AdminState>) -> Json<ApiResponse<Value>> {
    Json(ApiResponse::success(json!({})))
}
