//! Admin dashboard and server management handlers
//!
//! This module handles admin dashboard operations, server management,
//! metrics, logs, and configuration.

use axum::{
    extract::{Query, State},
    response::Json,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::handlers::AdminState;
use crate::models::*;

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
    State(_state): State<AdminState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<LogEntry>>> {
    // Parse query parameters for filtering
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(100);

    let method_filter = params.get("method").map(|s| s.to_string());
    let path_filter = params.get("path").map(|s| s.to_string());
    let status_filter = params.get("status").and_then(|s| s.parse::<u16>().ok());

    // Get recent logs from the centralized logger
    let request_logs = if let Some(global_logger) = mockforge_core::get_global_logger() {
        global_logger.get_recent_logs(Some(limit * 2)).await
    } else {
        Vec::new()
    };

    // Convert RequestLogEntry to LogEntry and apply filters
    let mut log_entries: Vec<LogEntry> = request_logs
        .into_iter()
        .filter(|log| {
            // Only include HTTP logs for now (matching the UI interface)
            log.server_type == "HTTP"
        })
        .filter(|log| {
            // Apply method filter
            method_filter.as_ref().is_none_or(|filter| log.method == *filter)
        })
        .filter(|log| {
            // Apply path filter (simple substring match)
            path_filter.as_ref().is_none_or(|filter| log.path.contains(filter))
        })
        .filter(|log| {
            // Apply status filter
            status_filter.is_none_or(|filter| log.status_code == filter)
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
    let metrics = state.metrics.read().await;
    let error_rate = 0.0; // Note: total_errors field doesn't exist in this RequestMetrics, setting to 0.0
                          // Note: Some fields from the original RequestMetrics aren't available, using defaults
    Json(ApiResponse::success(SimpleMetricsData {
        total_requests: metrics.total_requests,
        active_requests: metrics.active_connections, // Using active_connections as proxy
        average_response_time: 0.0, // This field doesn't exist in this RequestMetrics
        error_rate,
    }))
}

/// Update latency configuration
pub async fn update_latency(
    State(state): State<super::AdminState>,
    Json(config): Json<Value>,
) -> Json<ApiResponse<String>> {
    // Extract latency configuration from the JSON
    let base_ms = config.get("base_ms").and_then(|v| v.as_u64()).unwrap_or(50);
    let jitter_ms = config.get("jitter_ms").and_then(|v| v.as_u64()).unwrap_or(20);
    let tag_overrides = config
        .get("tag_overrides")
        .and_then(|v| v.as_object())
        .map(|obj| obj.iter().filter_map(|(k, v)| v.as_u64().map(|val| (k.clone(), val))).collect())
        .unwrap_or_default();

    // Update the configuration
    state.update_latency_config(base_ms, jitter_ms, tag_overrides).await;

    tracing::info!("Updated latency profile: base_ms={}, jitter_ms={}", base_ms, jitter_ms);
    Json(ApiResponse::success("Latency configuration updated".to_string()))
}

/// Update fault injection configuration
pub async fn update_faults(
    State(state): State<super::AdminState>,
    Json(config): Json<Value>,
) -> Json<ApiResponse<String>> {
    // Extract fault configuration from the JSON
    let enabled = config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let failure_rate = config.get("failure_rate").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let status_codes = config
        .get("status_codes")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u16)).collect())
        .unwrap_or_default();

    // Update the configuration
    state.update_fault_config(enabled, failure_rate, status_codes).await;

    tracing::info!("Updated fault config: enabled={}, failure_rate={}", enabled, failure_rate);
    Json(ApiResponse::success("Fault configuration updated".to_string()))
}

/// Update proxy configuration
pub async fn update_proxy(
    State(state): State<super::AdminState>,
    Json(config): Json<Value>,
) -> Json<ApiResponse<String>> {
    // Extract proxy configuration from the JSON
    let enabled = config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let upstream_url = config.get("upstream_url").and_then(|v| v.as_str()).map(|s| s.to_string());
    let timeout_seconds = config.get("timeout_seconds").and_then(|v| v.as_u64()).unwrap_or(30);

    // Update the configuration
    state.update_proxy_config(enabled, upstream_url.clone(), timeout_seconds).await;

    tracing::info!(
        "Updated proxy config: enabled={}, upstream_url={:?}, timeout_seconds={}",
        enabled,
        upstream_url,
        timeout_seconds
    );
    Json(ApiResponse::success("Proxy configuration updated".to_string()))
}

/// Clear logs
pub async fn clear_logs(State(_state): State<AdminState>) -> Json<ApiResponse<String>> {
    if let Some(global_logger) = mockforge_core::get_global_logger() {
        global_logger.clear_logs().await;
    }
    tracing::info!("Request logs cleared via admin UI");
    Json(ApiResponse::success("Logs cleared".to_string()))
}

/// Restart servers
pub async fn restart_servers(State(state): State<super::AdminState>) -> Json<ApiResponse<String>> {
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
        if let Err(e) = super::perform_server_restart(&state_clone).await {
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

/// Get restart status
pub async fn get_restart_status(
    State(state): State<super::AdminState>,
) -> Json<ApiResponse<super::RestartStatus>> {
    let status = state.get_restart_status().await;
    Json(ApiResponse::success(status))
}

/// Get configuration
pub async fn get_config(State(state): State<super::AdminState>) -> Json<ApiResponse<Value>> {
    let config = state.get_config().await;
    Json(ApiResponse::success(serde_json::to_value(config).unwrap_or_else(|_| json!({}))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state() -> super::AdminState {
        super::AdminState::new(None, None, None, None, false, 8080)
    }

    #[tokio::test]
    async fn test_get_restart_status() {
        let state = create_test_state();
        let response = get_restart_status(axum::extract::State(state)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_get_config() {
        let state = create_test_state();
        let response = get_config(axum::extract::State(state)).await;

        assert!(response.0.success);
    }
}
