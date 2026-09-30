//! Request ID propagation middleware
//!
//! Generates or propagates a unique request ID for each HTTP request.
//! The request ID is:
//! - Generated as a UUID if not provided in the `X-Request-ID` header
//! - Propagated from the incoming `X-Request-ID` header if present
//! - Added to all log entries via tracing spans
//! - Returned in the `X-Request-ID` response header

use axum::{
    extract::Request,
    http::{header::HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use tracing::Instrument;
use uuid::Uuid;

/// The header name for request ID
pub static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Request ID middleware
///
/// This middleware:
/// 1. Checks for an existing `X-Request-ID` header in the request
/// 2. If present and valid, uses it; otherwise generates a new UUID
/// 3. Creates a tracing span with the request ID
/// 4. Adds the request ID to the response headers
pub async fn request_id_middleware(request: Request, next: Next) -> Response {
    // Extract or generate request ID
    let request_id = extract_or_generate_request_id(&request);

    // Create a span with the request ID for structured logging
    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
    );

    // Run the rest of the chain inside the span. `instrument` re-enters the
    // span on every poll; a `span.enter()` guard held across the await would
    // corrupt the per-thread span stacks once the task hops worker threads.
    let mut response = async {
        tracing::debug!(request_id = %request_id, "Processing request");
        next.run(request).await
    }
    .instrument(span)
    .await;

    // Add request ID to response headers
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(X_REQUEST_ID.clone(), header_value);
    }

    response
}

/// Extract request ID from header or generate a new one
fn extract_or_generate_request_id(request: &Request) -> String {
    request
        .headers()
        .get(&X_REQUEST_ID)
        .and_then(|h| h.to_str().ok())
        .filter(|id| is_valid_request_id(id))
        .map(|id| id.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

/// Validate that a request ID is acceptable
/// Accepts UUIDs and other reasonable ID formats (alphanumeric with dashes, max 64 chars)
fn is_valid_request_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 64 {
        return false;
    }

    // Allow alphanumeric characters, dashes, and underscores
    id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Extension trait to get the request ID from response headers
pub trait RequestIdExt {
    fn request_id(&self) -> Option<&str>;
}

impl RequestIdExt for Response {
    fn request_id(&self) -> Option<&str> {
        self.headers().get(&X_REQUEST_ID).and_then(|h| h.to_str().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "OK"
    }

    fn create_test_router() -> Router {
        Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(request_id_middleware))
    }

    /// Regression: the middleware used to hold `span.enter()` across
    /// `next.run(..).await`. When the request task yielded and resumed on
    /// another worker thread, the thread-local span stacks went out of sync and
    /// `tracing-subscriber` panicked ("tried to clone a span that already
    /// closed") inside a handler, dropping the connection mid-request.
    #[test]
    fn request_span_survives_yielding_handlers_on_multi_thread_runtime() {
        let dispatch = tracing::Dispatch::new(tracing_subscriber::registry());
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .on_thread_start(move || std::mem::forget(tracing::dispatcher::set_default(&dispatch)))
            .build()
            .unwrap();

        runtime.block_on(async {
            // Mirrors what sqlx does on every query: wrap a future in
            // `Span::current()`, which clones the current span.
            async fn yielding_handler() -> &'static str {
                use tracing::Instrument;
                for _ in 0..16 {
                    tokio::task::yield_now().in_current_span().await;
                }
                "OK"
            }

            let app = Router::new()
                .route("/yield", get(yielding_handler))
                .layer(axum::middleware::from_fn(request_id_middleware));

            let requests: Vec<_> = (0..2000)
                .map(|_| {
                    let app = app.clone();
                    tokio::spawn(async move {
                        let request = Request::builder().uri("/yield").body(Body::empty()).unwrap();
                        app.oneshot(request).await.unwrap().status()
                    })
                })
                .collect();

            for request in requests {
                assert_eq!(request.await.expect("request task panicked"), StatusCode::OK);
            }
        });
    }

    #[test]
    fn test_is_valid_request_id_uuid() {
        let uuid = Uuid::new_v4().to_string();
        assert!(is_valid_request_id(&uuid));
    }

    #[test]
    fn test_is_valid_request_id_custom() {
        assert!(is_valid_request_id("req-12345"));
        assert!(is_valid_request_id("my_request_id"));
        assert!(is_valid_request_id("abc123"));
    }

    #[test]
    fn test_is_valid_request_id_empty() {
        assert!(!is_valid_request_id(""));
    }

    #[test]
    fn test_is_valid_request_id_too_long() {
        let long_id = "a".repeat(65);
        assert!(!is_valid_request_id(&long_id));
    }

    #[test]
    fn test_is_valid_request_id_invalid_chars() {
        assert!(!is_valid_request_id("req/id"));
        assert!(!is_valid_request_id("req id"));
        assert!(!is_valid_request_id("req\nid"));
    }

    #[tokio::test]
    async fn test_request_id_generated_when_missing() {
        let app = create_test_router();

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Should have X-Request-ID header
        let request_id = response.headers().get(&X_REQUEST_ID);
        assert!(request_id.is_some());

        // Should be a valid UUID
        let id_str = request_id.unwrap().to_str().unwrap();
        assert!(Uuid::parse_str(id_str).is_ok());
    }

    #[tokio::test]
    async fn test_request_id_propagated_when_present() {
        let app = create_test_router();
        let expected_id = "my-custom-request-id-123";

        let request = Request::builder()
            .uri("/test")
            .header("X-Request-ID", expected_id)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Should have the same X-Request-ID
        let request_id = response.headers().get(&X_REQUEST_ID).unwrap();
        assert_eq!(request_id.to_str().unwrap(), expected_id);
    }

    #[tokio::test]
    async fn test_request_id_regenerated_for_invalid_id() {
        let app = create_test_router();
        let invalid_id = "invalid/id with spaces";

        let request = Request::builder()
            .uri("/test")
            .header("X-Request-ID", invalid_id)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Should have a new UUID, not the invalid ID
        let request_id = response.headers().get(&X_REQUEST_ID).unwrap();
        let id_str = request_id.to_str().unwrap();
        assert_ne!(id_str, invalid_id);
        assert!(Uuid::parse_str(id_str).is_ok());
    }

    #[tokio::test]
    async fn test_request_id_ext_trait() {
        let app = create_test_router();

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Use the extension trait
        let request_id = response.request_id();
        assert!(request_id.is_some());
    }

    #[test]
    fn test_x_request_id_header_name() {
        assert_eq!(X_REQUEST_ID.as_str(), "x-request-id");
    }
}
