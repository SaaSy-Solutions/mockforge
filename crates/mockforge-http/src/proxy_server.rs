//! Browser/Mobile Proxy Server
//!
//! Provides an intercepting proxy for frontend/mobile clients with HTTPS support,
//! certificate injection, and comprehensive request/response logging.

use axum::{
    extract::Request, http::StatusCode, middleware::Next, response::Response, routing::get, Router,
};
use mockforge_core::proxy::{body_transform::BodyTransformationMiddleware, config::ProxyConfig};
use serde::Serialize;
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
        let state_for_middleware = state.clone();

        Router::new()
            // Health check endpoint
            .route("/proxy/health", get(health_check))
            // Catch-all proxy handler - use fallback for all methods
            .fallback(proxy_handler)
            .with_state(state)
            .layer(axum::middleware::from_fn_with_state(state_for_middleware, logging_middleware))
    }
}

/// Health check endpoint for the proxy
async fn health_check() -> Result<Response<String>, StatusCode> {
    // Response builder should never fail with known-good values, but handle errors gracefully
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(r#"{"status":"healthy","service":"mockforge-proxy"}"#.to_string())
        .map_err(|e| {
            tracing::error!("Failed to build health check response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Main proxy handler that intercepts and forwards requests
async fn proxy_handler(
    axum::extract::State(state): axum::extract::State<Arc<ProxyServer>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Response<String>, StatusCode> {
    // Extract client address from request extensions (set by ConnectInfo middleware)
    let client_addr = request
        .extensions()
        .get::<SocketAddr>()
        .copied()
        .unwrap_or_else(|| std::net::SocketAddr::from(([0, 0, 0, 0], 0)));

    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();

    // Read request body early for conditional evaluation (consume the body)
    let body_bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => Some(bytes.to_vec()),
        Err(e) => {
            error!("Failed to read request body: {}", e);
            None
        }
    };

    let config = state.config.read().await;

    // Check if proxy is enabled
    if !config.enabled {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // Determine if this request should be proxied (with conditional evaluation)
    if !config.should_proxy_with_condition(&method, &uri, &headers, body_bytes.as_deref()) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Get the stripped path (without proxy prefix)
    let stripped_path = config.strip_prefix(uri.path());

    // Get the base upstream URL and construct the full URL
    let base_upstream_url = config.get_upstream_url(uri.path());
    let full_upstream_url =
        if stripped_path.starts_with("http://") || stripped_path.starts_with("https://") {
            stripped_path.clone()
        } else {
            let base = base_upstream_url.trim_end_matches('/');
            let path = stripped_path.trim_start_matches('/');
            let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
            if path.is_empty() || path == "/" {
                format!("{}{}", base, query)
            } else {
                format!("{}/{}", base, path) + &query
            }
        };

    // Create a new URI with the full upstream URL for the proxy handler
    let modified_uri = full_upstream_url.parse::<axum::http::Uri>().unwrap_or_else(|_| uri.clone());

    // Log the request if enabled
    if state.log_requests {
        let mut counter = state.request_counter.write().await;
        *counter += 1;
        let request_id = *counter;

        info!(
            request_id = request_id,
            method = %method,
            path = %uri.path(),
            upstream = %full_upstream_url,
            client_ip = %client_addr.ip(),
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

    // Use ProxyClient directly with the full upstream URL to bypass ProxyHandler's URL construction
    use mockforge_core::proxy::client::ProxyClient;
    let proxy_client = ProxyClient::new();

    // Convert method to reqwest method
    let reqwest_method = match method.as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        "PATCH" => reqwest::Method::PATCH,
        _ => {
            error!("Unsupported HTTP method: {}", method);
            return Err(StatusCode::METHOD_NOT_ALLOWED);
        }
    };

    // Add any configured headers
    for (key, value) in &config.headers {
        header_map.insert(key.clone(), value.clone());
    }

    // Apply request body transformations if configured
    let mut transformed_request_body = body_bytes.clone();
    if !config.request_replacements.is_empty() {
        let transform_middleware = BodyTransformationMiddleware::new(
            config.request_replacements.clone(),
            Vec::new(), // No response rules needed here
        );
        if let Err(e) =
            transform_middleware.transform_request_body(uri.path(), &mut transformed_request_body)
        {
            warn!("Failed to transform request body: {}", e);
            // Continue with original body if transformation fails
        }
    }

    match proxy_client
        .send_request(
            reqwest_method,
            &full_upstream_url,
            &header_map,
            transformed_request_body.as_deref(),
        )
        .await
    {
        Ok(response) => {
            let status = StatusCode::from_u16(response.status().as_u16())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

            // Log the response if enabled
            if state.log_responses {
                info!(
                    method = %method,
                    path = %uri.path(),
                    status = status.as_u16(),
                    "Proxy response sent"
                );
            }

            // Convert response headers
            let mut response_headers = axum::http::HeaderMap::new();
            for (name, value) in response.headers() {
                if let (Ok(header_name), Ok(header_value)) = (
                    axum::http::HeaderName::try_from(name.as_str()),
                    axum::http::HeaderValue::try_from(value.as_bytes()),
                ) {
                    response_headers.insert(header_name, header_value);
                }
            }

            // Read response body
            let response_body_bytes = response.bytes().await.map_err(|e| {
                error!("Failed to read proxy response body: {}", e);
                StatusCode::BAD_GATEWAY
            })?;

            // Apply response body transformations if configured
            let mut final_body_bytes = response_body_bytes.to_vec();
            {
                let config_for_response = state.config.read().await;
                if !config_for_response.response_replacements.is_empty() {
                    let transform_middleware = BodyTransformationMiddleware::new(
                        Vec::new(), // No request rules needed here
                        config_for_response.response_replacements.clone(),
                    );
                    let mut body_option = Some(final_body_bytes.clone());
                    if let Err(e) = transform_middleware.transform_response_body(
                        uri.path(),
                        status.as_u16(),
                        &mut body_option,
                    ) {
                        warn!("Failed to transform response body: {}", e);
                        // Continue with original body if transformation fails
                    } else if let Some(transformed_body) = body_option {
                        final_body_bytes = transformed_body;
                    }
                }
            }

            let body_string = String::from_utf8_lossy(&final_body_bytes).to_string();

            // Build Axum response
            let mut response_builder = Response::builder().status(status);
            for (name, value) in response_headers.iter() {
                response_builder = response_builder.header(name, value);
            }

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
    axum::extract::State(_state): axum::extract::State<Arc<ProxyServer>>,
    request: Request,
    next: Next,
) -> Response {
    let start = std::time::Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();

    // Extract client address from request extensions
    let client_addr = request
        .extensions()
        .get::<SocketAddr>()
        .copied()
        .unwrap_or_else(|| std::net::SocketAddr::from(([0, 0, 0, 0], 0)));

    debug!(
        method = %method,
        uri = %uri,
        client_ip = %client_addr.ip(),
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

        // Response body is already a String
        let body = response.into_body();

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
