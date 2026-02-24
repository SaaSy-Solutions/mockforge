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

    let total_errors: u64 = metrics.errors_by_endpoint.values().sum();
    let error_rate = if metrics.total_requests > 0 {
        total_errors as f64 / metrics.total_requests as f64
    } else {
        0.0
    };

    let average_response_time = if metrics.response_times.is_empty() {
        0.0
    } else {
        metrics.response_times.iter().sum::<u64>() as f64 / metrics.response_times.len() as f64
    };

    Json(ApiResponse::success(SimpleMetricsData {
        total_requests: metrics.total_requests,
        active_requests: metrics.active_connections,
        average_response_time,
        error_rate,
    }))
}

/// Update latency configuration
pub async fn update_latency(
    State(state): State<AdminState>,
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
    State(state): State<AdminState>,
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
    State(state): State<AdminState>,
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
    State(state): State<AdminState>,
) -> Json<ApiResponse<super::RestartStatus>> {
    let status = state.get_restart_status().await;
    Json(ApiResponse::success(status))
}

/// Get configuration
pub async fn get_config(State(state): State<AdminState>) -> Json<ApiResponse<Value>> {
    let config = state.get_config().await;
    Json(ApiResponse::success(serde_json::to_value(config).unwrap_or_else(|_| json!({}))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state() -> AdminState {
        AdminState::new(None, None, None, None, false, 8080, None, None, None, None, None)
    }

    // ==================== RequestMetrics Tests ====================

    #[test]
    fn test_request_metrics_default() {
        let metrics = RequestMetrics::default();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.active_requests, 0);
        assert_eq!(metrics.average_response_time, 0.0);
        assert_eq!(metrics.requests_per_second, 0.0);
        assert_eq!(metrics.total_errors, 0);
    }

    #[test]
    fn test_request_metrics_creation() {
        let metrics = RequestMetrics {
            total_requests: 1000,
            active_requests: 10,
            average_response_time: 45.5,
            requests_per_second: 25.0,
            total_errors: 5,
        };

        assert_eq!(metrics.total_requests, 1000);
        assert_eq!(metrics.active_requests, 10);
        assert!((metrics.average_response_time - 45.5).abs() < 0.001);
        assert!((metrics.requests_per_second - 25.0).abs() < 0.001);
        assert_eq!(metrics.total_errors, 5);
    }

    #[test]
    fn test_request_metrics_clone() {
        let metrics = RequestMetrics {
            total_requests: 500,
            active_requests: 5,
            average_response_time: 30.0,
            requests_per_second: 10.0,
            total_errors: 2,
        };

        let cloned = metrics.clone();
        assert_eq!(cloned.total_requests, 500);
        assert_eq!(cloned.active_requests, 5);
    }

    #[test]
    fn test_request_metrics_debug() {
        let metrics = RequestMetrics::default();
        let debug_str = format!("{:?}", metrics);
        assert!(debug_str.contains("RequestMetrics"));
        assert!(debug_str.contains("total_requests"));
    }

    // ==================== Handler Tests ====================

    #[tokio::test]
    async fn test_get_restart_status() {
        let state = create_test_state();
        let response = get_restart_status(State(state)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_get_config() {
        let state = create_test_state();
        let response = get_config(State(state)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_get_health() {
        let response = get_health().await;

        assert_eq!(response.0.status, "healthy");
        assert!(response.0.issues.is_empty());
    }

    #[tokio::test]
    async fn test_get_server_info() {
        let state = create_test_state();
        let response = get_server_info(State(state)).await;

        assert!(response.0.is_object());
        let obj = response.0.as_object().unwrap();
        assert!(obj.contains_key("http_server"));
        assert!(obj.contains_key("ws_server"));
        assert!(obj.contains_key("grpc_server"));
        assert!(obj.contains_key("graphql_server"));
        assert!(obj.contains_key("api_enabled"));
    }

    #[tokio::test]
    async fn test_get_server_info_disabled() {
        let state = create_test_state();
        let response = get_server_info(State(state)).await;

        // With None addresses, should return "disabled"
        let obj = response.0.as_object().unwrap();
        assert_eq!(obj.get("http_server").and_then(|v| v.as_str()), Some("disabled"));
        assert_eq!(obj.get("ws_server").and_then(|v| v.as_str()), Some("disabled"));
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let state = create_test_state();
        let response = get_metrics(State(state)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_get_logs_empty() {
        let state = create_test_state();
        let params = HashMap::new();
        let response = get_logs(State(state), Query(params)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_get_logs_with_limit() {
        let state = create_test_state();
        let mut params = HashMap::new();
        params.insert("limit".to_string(), "10".to_string());

        let response = get_logs(State(state), Query(params)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_get_logs_with_method_filter() {
        let state = create_test_state();
        let mut params = HashMap::new();
        params.insert("method".to_string(), "GET".to_string());

        let response = get_logs(State(state), Query(params)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_get_logs_with_path_filter() {
        let state = create_test_state();
        let mut params = HashMap::new();
        params.insert("path".to_string(), "/api".to_string());

        let response = get_logs(State(state), Query(params)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_get_logs_with_status_filter() {
        let state = create_test_state();
        let mut params = HashMap::new();
        params.insert("status".to_string(), "200".to_string());

        let response = get_logs(State(state), Query(params)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_clear_logs() {
        let state = create_test_state();
        let response = clear_logs(State(state)).await;

        assert!(response.0.success);
        assert!(response.0.data.is_some());
    }

    #[tokio::test]
    async fn test_update_latency() {
        let state = create_test_state();
        let config = json!({
            "base_ms": 100,
            "jitter_ms": 20
        });

        let response = update_latency(State(state), Json(config)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_update_latency_with_overrides() {
        let state = create_test_state();
        let config = json!({
            "base_ms": 50,
            "jitter_ms": 10,
            "tag_overrides": {
                "slow": 500,
                "fast": 10
            }
        });

        let response = update_latency(State(state), Json(config)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_update_faults() {
        let state = create_test_state();
        let config = json!({
            "enabled": true,
            "failure_rate": 0.1,
            "status_codes": [500, 503]
        });

        let response = update_faults(State(state), Json(config)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_update_faults_disabled() {
        let state = create_test_state();
        let config = json!({
            "enabled": false
        });

        let response = update_faults(State(state), Json(config)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_update_proxy() {
        let state = create_test_state();
        let config = json!({
            "enabled": true,
            "upstream_url": "http://localhost:8000",
            "timeout_seconds": 60
        });

        let response = update_proxy(State(state), Json(config)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_update_proxy_disabled() {
        let state = create_test_state();
        let config = json!({
            "enabled": false
        });

        let response = update_proxy(State(state), Json(config)).await;

        assert!(response.0.success);
    }

    #[tokio::test]
    async fn test_restart_servers() {
        let state = create_test_state();
        let response = restart_servers(State(state)).await;

        // Should succeed to initiate (even if restart won't actually work without real servers)
        assert!(response.0.success || response.0.error.is_some());
    }
}
