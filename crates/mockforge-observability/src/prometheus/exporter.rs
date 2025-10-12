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
}
