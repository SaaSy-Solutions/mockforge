//! Response body buffering middleware
//!
//! This middleware buffers response bodies so they can be read multiple times,
//! enabling downstream middleware to access the response body for analysis.

use axum::{body::Body, extract::Request, http::Response, middleware::Next};
use serde_json::Value;

/// Buffered response body
#[derive(Clone)]
pub struct BufferedResponse {
    /// Response status
    pub status: u16,
    /// Response headers
    pub headers: http::HeaderMap,
    /// Response body as bytes
    pub body: axum::body::Bytes,
}

impl BufferedResponse {
    /// Get response body as JSON value
    pub fn json(&self) -> Option<Value> {
        serde_json::from_slice(&self.body).ok()
    }

    /// Get response body as string
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }
}

/// Middleware to buffer response bodies
///
/// This middleware reads the entire response body into memory so it can be
/// accessed multiple times by downstream middleware. The buffered response
/// is stored in request extensions.
pub async fn buffer_response_middleware(req: Request, next: Next) -> Response<Body> {
    // Process request
    let response = next.run(req).await;

    // Event streams never end, so buffering one would hold the response
    // forever. Pass them through; drift tracking treats an unbuffered
    // response as having no JSON body.
    if is_event_stream(&response) {
        return response;
    }

    // Extract response parts
    let (parts, body) = response.into_parts();

    // Read body into bytes
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!("Failed to buffer response body: {}", e);
            // Return error response if body buffering fails
            return Response::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to buffer response"))
                .expect("static response body should never fail to build");
        }
    };

    // Create buffered response
    let buffered = BufferedResponse {
        status: parts.status.as_u16(),
        headers: parts.headers.clone(),
        body: body_bytes.clone(),
    };

    // Store in request extensions for downstream middleware
    // Note: We can't modify request extensions after the response is created,
    // so we'll store it in a way that can be accessed via a different mechanism
    // For now, we'll just recreate the response with the buffered body

    // Recreate response with buffered body
    let mut response_builder = Response::builder().status(parts.status).version(parts.version);

    // Copy headers
    for (name, value) in parts.headers.iter() {
        response_builder = response_builder.header(name, value);
    }

    // Add buffered response to response extensions
    let mut response = match response_builder.body(Body::from(body_bytes)) {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Failed to build response: {}", e);
            return Response::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to build response"))
                .expect("static response body should never fail to build");
        }
    };

    // Store buffered response in response extensions
    response.extensions_mut().insert(buffered);

    response
}

fn is_event_stream(response: &Response<Body>) -> bool {
    response
        .headers()
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.trim_start().to_ascii_lowercase().starts_with("text/event-stream"))
}

/// Extract buffered response from response extensions
pub fn get_buffered_response(response: &Response<Body>) -> Option<BufferedResponse> {
    response.extensions().get::<BufferedResponse>().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use futures::stream;
    use std::time::Duration;
    use tower::ServiceExt;

    fn app() -> Router {
        Router::new()
            .route("/json", get(|| async { axum::Json(serde_json::json!({"ok": true})) }))
            .route(
                "/events",
                get(|| async {
                    axum::response::sse::Sse::new(stream::pending::<
                        Result<axum::response::sse::Event, std::convert::Infallible>,
                    >())
                }),
            )
            .layer(axum::middleware::from_fn(buffer_response_middleware))
    }

    async fn call(path: &str) -> Response<Body> {
        let req = Request::builder().uri(path).body(Body::empty()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), app().oneshot(req))
            .await
            .unwrap_or_else(|_| panic!("{path} was held by the buffer"))
            .unwrap()
    }

    #[tokio::test]
    async fn buffers_finite_responses() {
        let response = call("/json").await;
        let buffered = get_buffered_response(&response).expect("JSON response is buffered");
        assert_eq!(buffered.json(), Some(serde_json::json!({"ok": true})));
    }

    #[tokio::test]
    async fn passes_event_streams_through_unbuffered() {
        let response = call("/events").await;
        assert_eq!(response.status(), http::StatusCode::OK);
        assert!(get_buffered_response(&response).is_none());
    }
}
