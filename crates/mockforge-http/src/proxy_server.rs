//! Browser/Mobile Proxy Server
//!
//! Provides an intercepting proxy for frontend/mobile clients with HTTPS support,
//! certificate injection, and comprehensive request/response logging.

use axum::{
    extract::{ConnectInfo, Request},
    http::{HeaderMap, Method, StatusCode, Uri},
    middleware::Next,
    response::Response,
    routing::{any, get},
    Router,
};
use mockforge_core::proxy::{config::ProxyConfig, handler::ProxyHandler};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Proxy server state
#[derive(Debug)]
pub struct ProxyServer {
    /// Proxy configuration
    config: Arc<RwLock<ProxyConfig>>,
    /// Request logging enabled
    log_requests: bool,
    /// Response logging enabled
    log_responses: bool,
    /// Request counter for logging
    request_counter: Arc<RwLock<u64>>,
}

impl ProxyServer {
    /// Create a new proxy server
    pub fn new(config: ProxyConfig, log_requests: bool, log_responses: bool) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            log_requests,
            log_responses,
            request_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Get the Axum router for the proxy server
    pub fn router(self) -> Router {
        let state = Arc::new(self);

        Router::new()
            // Health check endpoint
            .route("/proxy/health", get(health_check))
            // Catch-all proxy handler
            .route("/proxy/*path", any(proxy_handler))
            .route("/*path", any(proxy_handler))
            .with_state(state)
            .layer(axum::middleware::from_fn(logging_middleware))
    }
}

/// Health check endpoint for the proxy
async fn health_check() -> Result<Response<String>, StatusCode> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(r#"{"status":"healthy","service":"mockforge-proxy"}"#.to_string())
        .unwrap())
}

/// Main proxy handler that intercepts and forwards requests
async fn proxy_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    mut request: Request,
    axum::extract::State(state): axum::extract::State<Arc<ProxyServer>>,
) -> Result<Response<String>, StatusCode> {
    let config = state.config.read().await;

    // Check if proxy is enabled
    if !config.enabled {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // Determine if this request should be proxied
    if !config.should_proxy(&method, uri.path()) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Get the upstream URL
    let upstream_url = config.get_upstream_url(uri.path());
    let stripped_path = config.strip_prefix(uri.path());

    // Log the request if enabled
    if state.log_requests {
        let mut counter = state.request_counter.write().await;
        *counter += 1;
        let request_id = *counter;

        info!(
            request_id = request_id,
            method = %method,
            path = %uri.path(),
            upstream = %upstream_url,
            client_ip = %addr.ip(),
            "Proxy request intercepted"
        );
    }

    // Convert headers to HashMap for the proxy handler
    let mut header_map = std::collections::HashMap::new();
    for (key, value) in &headers {
        if let Ok(value_str) = value.to_str() {
            header_map.insert(key.to_string(), value_str.to_string());
        }
    }

    // Read request body
    let body_bytes = match axum::body::to_bytes(request.body_mut(), usize::MAX).await {
        Ok(bytes) => Some(bytes.to_vec()),
        Err(e) => {
            error!("Failed to read request body: {}", e);
            None
        }
    };

    // Create proxy handler and process the request
    let proxy_handler = ProxyHandler::new(config.clone());

    match proxy_handler
        .proxy_request(&method, &uri, &headers, body_bytes.as_deref())
        .await
    {
        Ok(proxy_response) => {
            // Log the response if enabled
            if state.log_responses {
                info!(
                    method = %method,
                    path = %uri.path(),
                    status = proxy_response.status_code,
                    "Proxy response sent"
                );
            }

            // Convert proxy response to Axum response
            let mut response_builder = Response::builder().status(proxy_response.status_code);

            // Add response headers
            for (key, value) in proxy_response.headers {
                if let Ok(header_name) = axum::http::HeaderName::try_from(key.as_str()) {
                    response_builder = response_builder.header(header_name, value);
                }
            }

            // Convert body to string
            let body_string = match proxy_response.body {
                Some(body_bytes) => String::from_utf8_lossy(&body_bytes).to_string(),
                None => String::new(),
            };

            response_builder
                .body(body_string)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
        Err(e) => {
            error!("Proxy request failed: {}", e);
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}

/// Middleware for logging requests and responses
async fn logging_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let start = std::time::Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();

    debug!(
        method = %method,
        uri = %uri,
        client_ip = %addr.ip(),
        "Request received"
    );

    let response = next.run(request).await;
    let duration = start.elapsed();

    debug!(
        method = %method,
        uri = %uri,
        status = %response.status(),
        duration_ms = duration.as_millis(),
        "Response sent"
    );

    response
}

/// Proxy statistics for monitoring
#[derive(Debug, Serialize)]
pub struct ProxyStats {
    /// Total requests processed
    pub total_requests: u64,
    /// Requests per second
    pub requests_per_second: f64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Error rate percentage
    pub error_rate_percent: f64,
}

/// Get proxy statistics
pub async fn get_proxy_stats(state: &ProxyServer) -> ProxyStats {
    let total_requests = *state.request_counter.read().await;

    // For now, return basic stats. In a real implementation,
    // you'd track more detailed metrics over time.
    ProxyStats {
        total_requests,
        requests_per_second: 0.0,  // Would need time-based tracking
        avg_response_time_ms: 0.0, // Would need timing data
        error_rate_percent: 0.0,   // Would need error tracking
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use mockforge_core::proxy::config::ProxyConfig;
    use std::net::SocketAddr;
    use tokio_test;

    #[tokio::test]
    async fn test_proxy_server_creation() {
        let config = ProxyConfig::default();
        let server = ProxyServer::new(config, true, true);

        // Test that the server can be created
        assert!(server.log_requests);
        assert!(server.log_responses);
    }

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec(),
        )
        .unwrap();

        assert!(body.contains("healthy"));
        assert!(body.contains("mockforge-proxy"));
    }

    #[tokio::test]
    async fn test_proxy_stats() {
        let config = ProxyConfig::default();
        let server = ProxyServer::new(config, false, false);

        let stats = get_proxy_stats(&server).await;
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.requests_per_second, 0.0);
        assert_eq!(stats.avg_response_time_ms, 0.0);
        assert_eq!(stats.error_rate_percent, 0.0);
    }
}
