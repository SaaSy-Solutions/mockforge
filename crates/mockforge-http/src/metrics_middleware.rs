//! HTTP metrics collection middleware
//!
//! Collects Prometheus metrics for all HTTP requests including:
//! - Request counts by method and status
//! - Request duration histograms
//! - In-flight request tracking
//! - Error counts
//! - Pillar dimension for usage tracking

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use mockforge_observability::get_global_registry;
use std::time::Instant;
use tracing::debug;

/// Determine pillar from endpoint path
///
/// Analyzes the request path to determine which pillar(s) the request belongs to.
/// This enables pillar-based usage tracking in telemetry.
fn determine_pillar_from_path(path: &str) -> &'static str {
    let path_lower = path.to_lowercase();

    // Reality pillar patterns
    if path_lower.contains("/reality")
        || path_lower.contains("/personas")
        || path_lower.contains("/chaos")
        || path_lower.contains("/fidelity")
        || path_lower.contains("/continuum") {
        return "reality";
    }

    // Contracts pillar patterns
    if path_lower.contains("/contracts")
        || path_lower.contains("/validation")
        || path_lower.contains("/drift")
        || path_lower.contains("/schema")
        || path_lower.contains("/sync") {
        return "contracts";
    }

    // DevX pillar patterns
    if path_lower.contains("/sdk")
        || path_lower.contains("/playground")
        || path_lower.contains("/plugins")
        || path_lower.contains("/cli")
        || path_lower.contains("/generator") {
        return "devx";
    }

    // Cloud pillar patterns
    if path_lower.contains("/registry")
        || path_lower.contains("/workspace")
        || path_lower.contains("/org")
        || path_lower.contains("/marketplace")
        || path_lower.contains("/collab") {
        return "cloud";
    }

    // AI pillar patterns
    if path_lower.contains("/ai")
        || path_lower.contains("/mockai")
        || path_lower.contains("/voice")
        || path_lower.contains("/llm")
        || path_lower.contains("/studio") {
        return "ai";
    }

    // Default to unknown if no pattern matches
    "unknown"
}

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
    let path = matched_path.as_ref().map(|mp| mp.as_str().to_string()).unwrap_or(uri_path);

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

    // Determine pillar from path
    let pillar = determine_pillar_from_path(&path);

    // Record metrics with pillar information
    registry.record_http_request_with_pillar(&method, status_code, duration_seconds, pillar);

    // Record errors separately with pillar
    if status_code >= 400 {
        let error_type = if status_code >= 500 {
            "server_error"
        } else {
            "client_error"
        };
        registry.record_error_with_pillar("http", error_type, pillar);
    }

    debug!(
        method = %method,
        path = %path,
        status = status_code,
        duration_ms = duration.as_millis(),
        pillar = pillar,
        "HTTP request metrics recorded with pillar dimension"
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
        use axum::Router;
        let app = Router::new()
            .route("/test", axum::routing::get(test_handler))
            .layer(middleware::from_fn(collect_http_metrics));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_middleware_records_errors() {
        async fn error_handler() -> impl IntoResponse {
            (StatusCode::INTERNAL_SERVER_ERROR, "error")
        }

        use axum::Router;
        let app = Router::new()
            .route("/error", axum::routing::get(error_handler))
            .layer(middleware::from_fn(collect_http_metrics));

        let request = Request::builder().uri("/error").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
