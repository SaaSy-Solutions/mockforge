//! HTTP metrics collection middleware
//!
//! Collects Prometheus metrics for all HTTP requests including:
//! - Request counts by method and status
//! - Request duration histograms
//! - In-flight request tracking
//! - Error counts

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use mockforge_observability::get_global_registry;
use std::time::Instant;
use tracing::debug;

/// Metrics collection middleware for HTTP requests
///
/// This middleware should be applied to all HTTP routes to collect comprehensive
/// metrics for Prometheus. It tracks:
/// - Total request counts (by method and status code)
/// - Request duration (as histograms for percentile calculations)
/// - In-flight requests
/// - Error rates
pub async fn collect_http_metrics(
    matched_path: Option<MatchedPath>,
    req: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let method = req.method().to_string();
    let uri_path = req.uri().path().to_string();
    let path = matched_path
        .as_ref()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or(uri_path);

    // Get metrics registry
    let registry = get_global_registry();

    // Track in-flight requests
    registry.increment_in_flight("http");
    debug!(
        method = %method,
        path = %path,
        "Starting HTTP request metrics collection"
    );

    // Process the request
    let response = next.run(req).await;

    // Decrement in-flight requests
    registry.decrement_in_flight("http");

    // Calculate metrics
    let duration = start_time.elapsed();
    let duration_seconds = duration.as_secs_f64();
    let status_code = response.status().as_u16();

    // Record metrics
    registry.record_http_request(&method, status_code, duration_seconds);

    // Record errors separately
    if status_code >= 400 {
        let error_type = if status_code >= 500 {
            "server_error"
        } else {
            "client_error"
        };
        registry.record_error("http", error_type);
    }

    debug!(
        method = %method,
        path = %path,
        status = status_code,
        duration_ms = duration.as_millis(),
        "HTTP request metrics recorded"
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        response::IntoResponse,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> impl IntoResponse {
        (StatusCode::OK, "test response")
    }

    #[tokio::test]
    async fn test_metrics_middleware_records_success() {
        let app = Router::new()
            .route("/test", axum::routing::get(test_handler))
            .layer(middleware::from_fn(collect_http_metrics));

        let request = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_middleware_records_errors() {
        async fn error_handler() -> impl IntoResponse {
            (StatusCode::INTERNAL_SERVER_ERROR, "error")
        }

        let app = Router::new()
            .route("/error", axum::routing::get(error_handler))
            .layer(middleware::from_fn(collect_http_metrics));

        let request = Request::builder()
            .uri("/error")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
