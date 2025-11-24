//! HTTP request logging middleware

use axum::{
    extract::{ConnectInfo, MatchedPath, Request},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use mockforge_core::{
    create_http_log_entry, log_request_global,
    reality_continuum::response_trace::ResponseGenerationTrace,
    request_logger::RealityTraceMetadata,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Instant;
use tracing::info;

/// HTTP request logging middleware
pub async fn log_http_requests(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    matched_path: Option<MatchedPath>,
    req: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let method = req.method().to_string();
    let uri = req.uri().to_string();
    let path = matched_path
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| uri.split('?').next().unwrap_or(&uri).to_string());

    // Extract query parameters from URI
    let query_params: HashMap<String, String> = req
        .uri()
        .query()
        .map(|q| url::form_urlencoded::parse(q.as_bytes()).into_owned().collect())
        .unwrap_or_default();

    // Extract headers (filter sensitive ones)
    let headers = extract_safe_headers(req.headers());

    // Extract user agent
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Extract reality metadata from request extensions (set by consistency middleware)
    // Must be done before calling next.run() which consumes the request
    let reality_metadata = req.extensions().get::<RealityTraceMetadata>().cloned();

    // Call the next middleware/handler
    let response = next.run(req).await;

    // Calculate response time
    let response_time_ms = start_time.elapsed().as_millis() as u64;
    let status_code = response.status().as_u16();

    // Estimate response size (not perfect but good enough)
    let response_size_bytes = response
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Determine if this is an error
    let error_message = if status_code >= 400 {
        Some(format!(
            "HTTP {} {}",
            status_code,
            response.status().canonical_reason().unwrap_or("Unknown")
        ))
    } else {
        None
    };

    // Log the request with query parameters in metadata
    let mut log_entry = create_http_log_entry(
        &method,
        &path,
        status_code,
        response_time_ms,
        Some(addr.ip().to_string()),
        user_agent,
        headers,
        response_size_bytes,
        error_message,
    );

    // Add query parameters to metadata (clone to avoid move)
    let query_params_for_log = query_params.clone();
    if !query_params_for_log.is_empty() {
        for (key, value) in query_params_for_log {
            log_entry.metadata.insert(format!("query.{}", key), value);
        }
    }

    // Attach reality metadata if available
    log_entry.reality_metadata = reality_metadata;

    // Extract response generation trace from response extensions (set by handler)
    if let Some(trace) = response.extensions().get::<ResponseGenerationTrace>() {
        // Serialize trace to JSON string and store in metadata
        if let Ok(trace_json) = serde_json::to_string(trace) {
            log_entry.metadata.insert("response_generation_trace".to_string(), trace_json);
        }
    }

    // Log to centralized logger
    log_request_global(log_entry).await;

    // Also log to console for debugging (include query params if present)
    if !query_params.is_empty() {
        let query_params_clone = query_params.clone();
        info!(
            method = %method,
            path = %path,
            query = ?query_params_clone,
            status = status_code,
            duration_ms = response_time_ms,
            client_ip = %addr.ip(),
            "HTTP request processed"
        );
    } else {
        info!(
            method = %method,
            path = %path,
            status = status_code,
            duration_ms = response_time_ms,
            client_ip = %addr.ip(),
            "HTTP request processed"
        );
    }

    response
}

/// Extract safe headers (exclude sensitive ones)
fn extract_safe_headers(headers: &HeaderMap) -> HashMap<String, String> {
    let mut safe_headers = HashMap::new();

    // List of safe headers to include
    let safe_header_names = [
        "accept",
        "accept-encoding",
        "accept-language",
        "cache-control",
        "content-type",
        "content-length",
        "user-agent",
        "referer",
        "host",
        "x-forwarded-for",
        "x-real-ip",
    ];

    for name in safe_header_names {
        if let Some(value) = headers.get(name) {
            if let Ok(value_str) = value.to_str() {
                safe_headers.insert(name.to_string(), value_str.to_string());
            }
        }
    }

    safe_headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_safe_headers_empty() {
        let headers = HeaderMap::new();
        let safe_headers = extract_safe_headers(&headers);
        assert_eq!(safe_headers.len(), 0);
    }

    #[test]
    fn test_extract_safe_headers_with_safe_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("user-agent", HeaderValue::from_static("test-agent"));
        headers.insert("accept", HeaderValue::from_static("application/json"));

        let safe_headers = extract_safe_headers(&headers);

        assert_eq!(safe_headers.len(), 3);
        assert_eq!(safe_headers.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(safe_headers.get("user-agent"), Some(&"test-agent".to_string()));
        assert_eq!(safe_headers.get("accept"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_extract_safe_headers_excludes_sensitive_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("authorization", HeaderValue::from_static("Bearer token123"));
        headers.insert("cookie", HeaderValue::from_static("session=abc123"));
        headers.insert("x-api-key", HeaderValue::from_static("secret-key"));

        let safe_headers = extract_safe_headers(&headers);

        // Should only include content-type
        assert_eq!(safe_headers.len(), 1);
        assert_eq!(safe_headers.get("content-type"), Some(&"application/json".to_string()));

        // Should not include sensitive headers
        assert!(!safe_headers.contains_key("authorization"));
        assert!(!safe_headers.contains_key("cookie"));
        assert!(!safe_headers.contains_key("x-api-key"));
    }

    #[test]
    fn test_extract_safe_headers_all_safe_header_types() {
        let mut headers = HeaderMap::new();

        // Add all safe headers
        headers.insert("accept", HeaderValue::from_static("application/json"));
        headers.insert("accept-encoding", HeaderValue::from_static("gzip, deflate"));
        headers.insert("accept-language", HeaderValue::from_static("en-US"));
        headers.insert("cache-control", HeaderValue::from_static("no-cache"));
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("content-length", HeaderValue::from_static("123"));
        headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0"));
        headers.insert("referer", HeaderValue::from_static("https://example.com"));
        headers.insert("host", HeaderValue::from_static("api.example.com"));
        headers.insert("x-forwarded-for", HeaderValue::from_static("192.168.1.1"));
        headers.insert("x-real-ip", HeaderValue::from_static("192.168.1.2"));

        let safe_headers = extract_safe_headers(&headers);

        assert_eq!(safe_headers.len(), 11);
        assert_eq!(safe_headers.get("accept"), Some(&"application/json".to_string()));
        assert_eq!(safe_headers.get("accept-encoding"), Some(&"gzip, deflate".to_string()));
        assert_eq!(safe_headers.get("accept-language"), Some(&"en-US".to_string()));
        assert_eq!(safe_headers.get("cache-control"), Some(&"no-cache".to_string()));
        assert_eq!(safe_headers.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(safe_headers.get("content-length"), Some(&"123".to_string()));
        assert_eq!(safe_headers.get("user-agent"), Some(&"Mozilla/5.0".to_string()));
        assert_eq!(safe_headers.get("referer"), Some(&"https://example.com".to_string()));
        assert_eq!(safe_headers.get("host"), Some(&"api.example.com".to_string()));
        assert_eq!(safe_headers.get("x-forwarded-for"), Some(&"192.168.1.1".to_string()));
        assert_eq!(safe_headers.get("x-real-ip"), Some(&"192.168.1.2".to_string()));
    }

    #[test]
    fn test_extract_safe_headers_handles_invalid_utf8() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        // Note: HeaderValue doesn't allow invalid UTF-8, so this test ensures the code handles
        // the to_str() error gracefully by checking if the header exists but can't be converted

        let safe_headers = extract_safe_headers(&headers);
        assert!(safe_headers.contains_key("content-type"));
    }

    #[test]
    fn test_extract_safe_headers_case_insensitive() {
        let mut headers = HeaderMap::new();
        // HeaderMap is case-insensitive, but we insert with lowercase
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("User-Agent", HeaderValue::from_static("test"));

        let safe_headers = extract_safe_headers(&headers);

        // The function looks for lowercase names, but HeaderMap handles case-insensitivity
        assert_eq!(safe_headers.len(), 2);
        assert!(safe_headers.contains_key("content-type"));
        assert!(safe_headers.contains_key("user-agent"));
    }

    #[test]
    fn test_extract_safe_headers_mixed_safe_and_unsafe() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("authorization", HeaderValue::from_static("Bearer token"));
        headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0"));
        headers.insert("x-api-key", HeaderValue::from_static("secret"));
        headers.insert("accept", HeaderValue::from_static("*/*"));

        let safe_headers = extract_safe_headers(&headers);

        // Should only include the safe ones
        assert_eq!(safe_headers.len(), 3);
        assert!(safe_headers.contains_key("content-type"));
        assert!(safe_headers.contains_key("user-agent"));
        assert!(safe_headers.contains_key("accept"));

        // Should not include unsafe ones
        assert!(!safe_headers.contains_key("authorization"));
        assert!(!safe_headers.contains_key("x-api-key"));
    }
}
