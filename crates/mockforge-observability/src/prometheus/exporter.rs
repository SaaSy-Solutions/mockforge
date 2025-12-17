//! Prometheus metrics exporter
//!
//! Provides HTTP endpoints for Prometheus to scrape metrics

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;
use tracing::{debug, error};

use super::metrics::MetricsRegistry;

/// Handler for the /metrics endpoint
pub async fn metrics_handler(
    State(registry): State<Arc<MetricsRegistry>>,
) -> Result<impl IntoResponse, MetricsError> {
    debug!("Serving Prometheus metrics");

    let encoder = TextEncoder::new();
    let metric_families = registry.registry().gather();

    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).map_err(|e| {
        error!("Failed to encode metrics: {}", e);
        MetricsError::EncodingError(e.to_string())
    })?;

    let body = String::from_utf8(buffer).map_err(|e| {
        error!("Failed to convert metrics to UTF-8: {}", e);
        MetricsError::EncodingError(e.to_string())
    })?;

    Ok((
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    ))
}

/// Health check endpoint for the metrics server
pub async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Create a router for the Prometheus metrics endpoint
pub fn prometheus_router(registry: Arc<MetricsRegistry>) -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .with_state(registry)
}

/// Error type for metrics operations
#[derive(Debug)]
pub enum MetricsError {
    EncodingError(String),
}

impl IntoResponse for MetricsError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            MetricsError::EncodingError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Encoding error: {}", msg))
            }
        };

        (status, message).into_response()
    }
}

impl std::fmt::Display for MetricsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricsError::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
        }
    }
}

impl std::error::Error for MetricsError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prometheus::MetricsRegistry;

    #[tokio::test]
    async fn test_metrics_handler() {
        let registry = Arc::new(MetricsRegistry::new());

        // Record some test metrics
        registry.record_http_request("GET", 200, 0.045);
        registry.record_http_request("POST", 201, 0.123);

        // Call the handler
        let result = metrics_handler(State(registry)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_handler() {
        let response = health_handler().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_prometheus_router_creation() {
        let registry = Arc::new(MetricsRegistry::new());
        let _router = prometheus_router(registry);
        // Router should be created successfully
    }

    #[test]
    fn test_metrics_error_display() {
        let error = MetricsError::EncodingError("test error message".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Encoding error"));
        assert!(display.contains("test error message"));
    }

    #[test]
    fn test_metrics_error_debug() {
        let error = MetricsError::EncodingError("test".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("EncodingError"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_metrics_error_is_error_trait() {
        let error = MetricsError::EncodingError("test".to_string());
        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &error;
    }

    #[tokio::test]
    async fn test_metrics_error_into_response() {
        let error = MetricsError::EncodingError("encoding failed".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_metrics_handler_with_various_metrics() {
        let registry = Arc::new(MetricsRegistry::new());

        // Record various types of metrics
        registry.record_http_request("GET", 200, 0.045);
        registry.record_http_request("POST", 201, 0.123);
        registry.record_http_request("DELETE", 204, 0.015);
        registry.record_http_request("PUT", 500, 0.500);
        registry.record_grpc_request("ListUsers", "OK", 0.025);
        registry.record_ws_message_sent();
        registry.record_ws_message_received();
        registry.record_plugin_execution("test-plugin", true, 0.010);
        registry.record_error("http", "timeout");
        registry.update_memory_usage(1024.0 * 1024.0 * 50.0);
        registry.update_cpu_usage(25.5);

        // Call the handler
        let result = metrics_handler(State(registry)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_handler_empty_registry() {
        let registry = Arc::new(MetricsRegistry::new());
        // No metrics recorded
        let result = metrics_handler(State(registry)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_handler_with_grpc_metrics() {
        let registry = Arc::new(MetricsRegistry::new());
        registry.record_grpc_request("GetUser", "OK", 0.025);
        registry.record_grpc_request("CreateUser", "INTERNAL", 0.150);
        registry.record_grpc_request_with_pillar("ListUsers", "OK", 0.050, "reality");

        let result = metrics_handler(State(registry)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_handler_with_websocket_metrics() {
        let registry = Arc::new(MetricsRegistry::new());
        registry.record_ws_connection_established();
        registry.record_ws_message_sent();
        registry.record_ws_message_received();
        registry.record_ws_error();
        registry.record_ws_connection_closed(60.0, "normal");

        let result = metrics_handler(State(registry)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_handler_with_smtp_metrics() {
        let registry = Arc::new(MetricsRegistry::new());
        registry.record_smtp_connection_established();
        registry.record_smtp_message_received();
        registry.record_smtp_message_stored();
        registry.record_smtp_error("auth_failed");
        registry.record_smtp_connection_closed();

        let result = metrics_handler(State(registry)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_handler_with_marketplace_metrics() {
        let registry = Arc::new(MetricsRegistry::new());
        registry.record_marketplace_publish("plugin", true, 2.5);
        registry.record_marketplace_download("template", true, 0.5);
        registry.record_marketplace_search("scenario", true, 0.1);
        registry.record_marketplace_error("plugin", "validation_failed");
        registry.update_marketplace_items_total("plugin", 150);

        let result = metrics_handler(State(registry)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_handler_with_workspace_metrics() {
        let registry = Arc::new(MetricsRegistry::new());
        registry.record_workspace_request("workspace-1", "GET", 200, 0.05);
        registry.update_workspace_active_routes("workspace-1", 10);
        registry.record_workspace_error("workspace-1", "timeout");
        registry.increment_workspace_routes("workspace-1");
        registry.decrement_workspace_routes("workspace-1");

        let result = metrics_handler(State(registry)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_handler_with_scenario_metrics() {
        let registry = Arc::new(MetricsRegistry::new());
        registry.set_scenario_mode(0); // healthy
        registry.record_chaos_trigger();
        registry.set_scenario_mode(3); // chaos

        let result = metrics_handler(State(registry)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_handler_with_path_metrics() {
        let registry = Arc::new(MetricsRegistry::new());
        registry.record_http_request_with_path("/api/users/123", "GET", 200, 0.05);
        registry.record_http_request_with_path("/api/users/456", "GET", 200, 0.06);
        registry.record_http_request_with_path_and_pillar(
            "/api/items",
            "POST",
            201,
            0.1,
            "reality",
        );

        let result = metrics_handler(State(registry)).await;
        assert!(result.is_ok());
    }
}
