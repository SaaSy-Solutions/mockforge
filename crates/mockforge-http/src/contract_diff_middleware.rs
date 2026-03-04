//! Pillars: [Contracts]
//!
//! Contract diff middleware for capturing requests
//!
//! This middleware captures incoming HTTP requests for contract diff analysis.
//! It extracts request data and stores it in the capture manager.

use axum::http::HeaderMap;
use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use mockforge_core::{
    ai_contract_diff::CapturedRequest, request_capture::get_global_capture_manager,
};
use std::collections::HashMap;
use tracing::debug;

/// Maximum request body size to buffer for capture (1 MB).
const MAX_CAPTURE_BODY_SIZE: usize = 1024 * 1024;

/// Middleware to capture requests for contract diff analysis
pub async fn capture_for_contract_diff(req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let uri = req.uri().clone();
    let path = uri.path().to_string();
    let query = uri.query();

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
    let body_bytes = match axum::body::to_bytes(body, MAX_CAPTURE_BODY_SIZE).await {
        Ok(b) => b,
        Err(_) => {
            // Body too large or read error — forward without capturing body.
            let rebuilt = Request::from_parts(parts, Body::empty());
            return next.run(rebuilt).await;
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
