//! HTTP tracing middleware for distributed tracing
//!
//! Creates OpenTelemetry spans for HTTP requests with proper context propagation

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use mockforge_tracing::{
    create_request_span, extract_from_axum_headers, inject_into_axum_headers, record_error,
    record_success, Protocol,
};
use opentelemetry::{trace::TraceContextExt, KeyValue};
use std::time::Instant;
use tracing::debug;

/// Tracing middleware for HTTP requests
///
/// This middleware:
/// - Extracts trace context from incoming request headers (W3C Trace Context)
/// - Creates a span for the request
/// - Records span attributes (method, path, status, duration)
/// - Injects trace context into response headers
/// - Records errors with proper span status
pub async fn http_tracing_middleware(
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
        .unwrap_or(uri_path.clone());

    // Extract trace context from headers
    let parent_ctx = extract_from_axum_headers(req.headers());

    // Create span for this request
    let mut span = create_request_span(
        Protocol::Http,
        &format!("{} {}", method, path),
        vec![
            KeyValue::new("http.method", method.clone()),
            KeyValue::new("http.route", path.clone()),
            KeyValue::new("http.url", uri_path.clone()),
        ],
    );

    debug!(
        method = %method,
        path = %path,
        "Created trace span for HTTP request"
    );

    // Process the request
    let mut response = next.run(req).await;

    // Calculate metrics
    let duration = start_time.elapsed();
    let status_code = response.status().as_u16();

    // Record span attributes
    let attributes = vec![
        KeyValue::new("http.status_code", status_code as i64),
        KeyValue::new("http.duration_ms", duration.as_millis() as i64),
    ];

    // Record error or success on the span before attaching to context
    if status_code >= 400 {
        record_error(
            &mut span,
            &format!("HTTP {}: {}", status_code, response.status().canonical_reason().unwrap_or("Unknown")),
        );
    } else {
        record_success(&mut span, attributes);
    }

    // Attach span to context
    let ctx = parent_ctx.with_span(span);

    // Inject trace context into response headers
    inject_into_axum_headers(&ctx, response.headers_mut());

    debug!(
        method = %method,
        path = %path,
        status = status_code,
        duration_ms = duration.as_millis(),
        "Completed trace span for HTTP request"
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
    async fn test_tracing_middleware_creates_span() {
        // Initialize tracer for test
        use mockforge_tracing::TracingConfig;
        use opentelemetry::global;
        use opentelemetry_sdk::propagation::TraceContextPropagator;

        global::set_text_map_propagator(TraceContextPropagator::new());

        let app = Router::new()
            .route("/test", axum::routing::get(test_handler))
            .layer(middleware::from_fn(http_tracing_middleware));

        let request = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify trace context was injected into response headers
        assert!(response.headers().contains_key("traceparent"));
    }

    #[tokio::test]
    async fn test_tracing_middleware_propagates_context() {
        use opentelemetry::global;
        use opentelemetry_sdk::propagation::TraceContextPropagator;

        global::set_text_map_propagator(TraceContextPropagator::new());

        let app = Router::new()
            .route("/test", axum::routing::get(test_handler))
            .layer(middleware::from_fn(http_tracing_middleware));

        // Send request with existing trace context
        let request = Request::builder()
            .uri("/test")
            .header(
                "traceparent",
                "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            )
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify trace context was propagated
        let traceparent = response
            .headers()
            .get("traceparent")
            .and_then(|v| v.to_str().ok());

        assert!(traceparent.is_some());
        // Trace ID should be preserved
        assert!(traceparent.unwrap().contains("0af7651916cd43dd8448eb211c80319c"));
    }
}
