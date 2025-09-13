//! HTTP request logging middleware

use axum::{
    extract::{ConnectInfo, MatchedPath, Request},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use mockforge_core::{log_request_global, create_http_log_entry};
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
    
    // Extract headers (filter sensitive ones)
    let headers = extract_safe_headers(req.headers());
    
    // Extract user agent
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

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
        Some(format!("HTTP {} {}", status_code, response.status().canonical_reason().unwrap_or("Unknown")))
    } else {
        None
    };

    // Log the request
    let log_entry = create_http_log_entry(
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

    // Log to centralized logger
    log_request_global(log_entry).await;

    // Also log to console for debugging
    info!(
        method = %method,
        path = %path,
        status = status_code,
        duration_ms = response_time_ms,
        client_ip = %addr.ip(),
        "HTTP request processed"
    );

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