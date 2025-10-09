//! Recording middleware for HTTP requests

use crate::recorder::Recorder;
use axum::{
    body::{Body, Bytes},
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use http_body_util::BodyExt;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Instant};
use tracing::{debug, error};

/// Middleware layer for recording HTTP requests and responses
pub async fn recording_middleware(
    recorder: Arc<Recorder>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    // Extract trace context if available
    let trace_id = req
        .headers()
        .get("traceparent")
        .and_then(|v| v.to_str().ok())
        .and_then(extract_trace_id);

    let span_id = req
        .headers()
        .get("traceparent")
        .and_then(|v| v.to_str().ok())
        .and_then(extract_span_id);

    // Extract request details
    let method = req.method().to_string();
    let uri = req.uri().clone();
    let path = uri.path().to_string();
    let query = uri.query().map(|q| q.to_string());

    // Clone headers
    let headers: HashMap<String, String> = req
        .headers()
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|s| (k.as_str().to_string(), s.to_string()))
        })
        .collect();

    // Extract body (need to consume and recreate the request)
    let (parts, body) = req.into_parts();
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!("Failed to read request body: {}", e);
            Bytes::new()
        }
    };

    // Record the request
    let start = Instant::now();
    let request_id = match recorder
        .record_http_request(
            &method,
            &path,
            query.as_deref(),
            &headers,
            if body_bytes.is_empty() {
                None
            } else {
                Some(&body_bytes)
            },
            Some(&addr.ip().to_string()),
            trace_id.as_deref(),
            span_id.as_deref(),
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to record request: {}", e);
            // Continue processing even if recording fails
            uuid::Uuid::new_v4().to_string()
        }
    };

    debug!("Recorded request: {} {} {}", request_id, method, path);

    // Reconstruct request with body
    let req = Request::from_parts(parts, Body::from(body_bytes));

    // Pass to next handler
    let response = next.run(req).await;

    // Extract response details
    let (parts, body) = response.into_parts();
    let status_code = parts.status.as_u16() as i32;

    // Extract response headers
    let response_headers: HashMap<String, String> = parts
        .headers
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|s| (k.as_str().to_string(), s.to_string()))
        })
        .collect();

    // Extract response body
    let response_body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!("Failed to read response body: {}", e);
            Bytes::new()
        }
    };

    // Calculate duration
    let duration_ms = start.elapsed().as_millis() as i64;

    // Record the response
    if let Err(e) = recorder
        .record_http_response(
            &request_id,
            status_code,
            &response_headers,
            if response_body_bytes.is_empty() {
                None
            } else {
                Some(&response_body_bytes)
            },
            duration_ms,
        )
        .await
    {
        error!("Failed to record response: {}", e);
    }

    debug!(
        "Recorded response: {} status={} duration={}ms",
        request_id, status_code, duration_ms
    );

    // Reconstruct response with body
    Response::from_parts(parts, Body::from(response_body_bytes))
}

/// Extract trace ID from W3C traceparent header
/// Format: 00-{trace_id}-{parent_id}-{flags}
fn extract_trace_id(traceparent: &str) -> Option<String> {
    let parts: Vec<&str> = traceparent.split('-').collect();
    if parts.len() >= 2 {
        Some(parts[1].to_string())
    } else {
        None
    }
}

/// Extract span ID from W3C traceparent header
fn extract_span_id(traceparent: &str) -> Option<String> {
    let parts: Vec<&str> = traceparent.split('-').collect();
    if parts.len() >= 3 {
        Some(parts[2].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_trace_id() {
        let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let trace_id = extract_trace_id(traceparent);
        assert_eq!(trace_id, Some("4bf92f3577b34da6a3ce929d0e0e4736".to_string()));
    }

    #[test]
    fn test_extract_span_id() {
        let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let span_id = extract_span_id(traceparent);
        assert_eq!(span_id, Some("00f067aa0ba902b7".to_string()));
    }

    #[test]
    fn test_invalid_traceparent() {
        let trace_id = extract_trace_id("invalid");
        assert_eq!(trace_id, None);
    }
}
