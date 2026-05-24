//! Pillars: [Contracts]
//!
//! Contract diff middleware for capturing requests
//!
//! This middleware captures incoming HTTP requests for contract diff analysis.
//! It extracts request data and stores it in the capture manager.

use axum::http::header::CONTENT_LENGTH;
use axum::http::HeaderMap;
use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use mockforge_core::{
    ai_contract_diff::CapturedRequest, request_capture::get_global_capture_manager,
};
use std::collections::HashMap;
use tracing::debug;

/// Maximum request body size to buffer for capture.
///
/// Issue #79 — Srikanth reported `200 OK` returned mid-upload on 10 MB
/// chunked PATCH requests against MockForge. Root cause was right here:
/// the old buffer limit was 1 MiB and the over-limit branch silently
/// substituted `Body::empty()` before forwarding, so every downstream
/// handler (Json/Bytes extractors, the OpenAPI route handler) saw an
/// empty body and either parsed-error-then-responded or responded with
/// a default — *before* the client finished uploading. The bumped
/// default plus the cleaner "skip capture, forward original body"
/// branch below fix both the limit and the empty-body-substitution.
fn max_capture_body_size() -> usize {
    const DEFAULT_MB: usize = 10;
    std::env::var("MOCKFORGE_CONTRACT_DIFF_MAX_BODY_MB")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MB)
        .saturating_mul(1024 * 1024)
}

/// Middleware to capture requests for contract diff analysis
pub async fn capture_for_contract_diff(req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let uri = req.uri().clone();
    let path = uri.path().to_string();
    let query = uri.query();
    let max_body = max_capture_body_size();

    // Issue #79 — if the request advertises a Content-Length larger than
    // the capture cap, skip the capture entirely (and don't consume the
    // body). This avoids the previous bug where over-limit bodies were
    // replaced with Body::empty() before being forwarded to the handler.
    let content_length = req
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok());
    if let Some(len) = content_length {
        if len > max_body {
            debug!(
                "contract_diff: skipping capture for {} {} — content-length {} exceeds cap {}",
                method, path, len, max_body
            );
            return next.run(req).await;
        }
    }

    // Extract headers
    let headers = extract_headers_for_capture(req.headers());

    // Extract query parameters
    let query_params = if let Some(query) = query {
        parse_query_params(query)
    } else {
        HashMap::new()
    };

    // Buffer the request body so we can capture it and still forward it.
    let (parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, max_body).await {
        Ok(b) => b,
        Err(_) => {
            // Chunked body that exceeded the cap (no Content-Length to
            // pre-check). The body has been partially consumed and we
            // can't put it back. The least-bad behaviour is a 413
            // PayloadTooLarge so the caller knows the request was
            // rejected — emitting `Body::empty()` here used to silently
            // truncate the request and cause the handler to respond
            // before the client finished uploading. Issue #79.
            return Response::builder()
                .status(axum::http::StatusCode::PAYLOAD_TOO_LARGE)
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/json",
                )
                .body(Body::from(format!(
                    r#"{{"error":"PAYLOAD_TOO_LARGE","message":"chunked request body exceeded contract_diff capture cap (~{} MiB); raise MOCKFORGE_CONTRACT_DIFF_MAX_BODY_MB or send Content-Length"}}"#,
                    max_body / (1024 * 1024)
                )))
                .unwrap_or_else(|_| {
                    Response::new(Body::from("payload too large"))
                });
        }
    };

    // Try to parse body as JSON for structured capture
    let captured_body = if !body_bytes.is_empty() {
        serde_json::from_slice::<serde_json::Value>(&body_bytes).ok()
    } else {
        None
    };

    // Reconstruct the request with the buffered body
    let rebuilt = Request::from_parts(parts, Body::from(body_bytes));

    // Call the next middleware/handler
    let response = next.run(rebuilt).await;

    // Extract response status
    let status_code = response.status().as_u16();

    // Create captured request with body
    let mut captured = CapturedRequest::new(&method, &path, "proxy_middleware")
        .with_headers(headers)
        .with_query_params(query_params)
        .with_response(status_code, None);

    if let Some(body_value) = captured_body {
        captured = captured.with_body(body_value);
    }

    // Capture the request (fire and forget)
    if let Some(capture_manager) = get_global_capture_manager() {
        if let Err(e) = capture_manager.capture(captured).await {
            debug!("Failed to capture request for contract diff: {}", e);
        }
    }

    response
}

/// Extract headers for capture (excluding sensitive ones)
fn extract_headers_for_capture(headers: &HeaderMap) -> HashMap<String, String> {
    let mut captured_headers = HashMap::new();

    // Safe headers to capture
    let safe_headers = [
        "accept",
        "accept-encoding",
        "accept-language",
        "content-type",
        "content-length",
        "user-agent",
        "referer",
        "origin",
        "x-requested-with",
    ];

    for header_name in safe_headers {
        if let Some(value) = headers.get(header_name) {
            if let Ok(value_str) = value.to_str() {
                captured_headers.insert(header_name.to_string(), value_str.to_string());
            }
        }
    }

    captured_headers
}

/// Parse query string into HashMap
fn parse_query_params(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();

    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let decoded_key = urlencoding::decode(key).unwrap_or_else(|_| key.into());
            let decoded_value = urlencoding::decode(value).unwrap_or_else(|_| value.into());
            params.insert(decoded_key.to_string(), decoded_value.to_string());
        }
    }

    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_headers_for_capture() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("authorization", HeaderValue::from_static("Bearer token"));
        headers.insert("accept", HeaderValue::from_static("application/json"));

        let captured = extract_headers_for_capture(&headers);

        assert_eq!(captured.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(captured.get("accept"), Some(&"application/json".to_string()));
        assert!(!captured.contains_key("authorization")); // Should exclude sensitive headers
    }

    #[test]
    fn test_parse_query_params() {
        let query = "name=John&age=30&city=New%20York";
        let params = parse_query_params(query);

        assert_eq!(params.get("name"), Some(&"John".to_string()));
        assert_eq!(params.get("age"), Some(&"30".to_string()));
        assert_eq!(params.get("city"), Some(&"New York".to_string()));
    }

    #[test]
    fn test_parse_query_params_empty() {
        let params = parse_query_params("");
        assert!(params.is_empty());
    }
}
