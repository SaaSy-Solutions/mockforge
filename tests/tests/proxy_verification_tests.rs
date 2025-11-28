//! Browser/Mobile Proxy Verification Tests
//!
//! Tests to verify that the MockForge proxy works correctly with browsers
//! and mobile clients, including HTTPS certificate injection.

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, Uri},
    Router,
};
use mockforge_core::proxy::config::ProxyConfig;
use mockforge_http::proxy_server::{get_proxy_stats, ProxyServer};
use std::net::SocketAddr;
use tower::ServiceExt;

/// Test configuration for proxy verification
#[derive(Debug, Clone)]
pub struct ProxyTestConfig {
    pub proxy_port: u16,
    pub target_port: u16,
    pub use_https: bool,
    pub log_requests: bool,
    pub log_responses: bool,
}

impl Default for ProxyTestConfig {
    fn default() -> Self {
        Self {
            proxy_port: 8081,
            target_port: 3000,
            use_https: false,
            log_requests: true,
            log_responses: true,
        }
    }
}

/// Test helper for creating a mock target server
pub async fn create_mock_target_server(
    port: u16,
) -> Result<Router, Box<dyn std::error::Error + Send + Sync>> {
    let app = Router::new()
        .route("/api/users", axum::routing::get(|| async {
            axum::Json(serde_json::json!([
                {"id": 1, "name": "Alice", "email": "alice@example.com"},
                {"id": 2, "name": "Bob", "email": "bob@example.com"}
            ]))
        }))
        .route("/api/users/{id}", axum::routing::get(|axum::extract::Path(id): axum::extract::Path<u32>| async move {
            axum::Json(serde_json::json!({
                "id": id,
                "name": format!("User {}", id),
                "email": format!("user{}@example.com", id)
            }))
        }))
        .route("/api/posts", axum::routing::post(|axum::extract::Json(body): axum::extract::Json<serde_json::Value>| async move {
            axum::Json(serde_json::json!({
                "id": 123,
                "title": body.get("title").unwrap_or(&serde_json::Value::String("Default Title".to_string())),
                "content": body.get("content").unwrap_or(&serde_json::Value::String("Default Content".to_string())),
                "created_at": "2024-01-01T00:00:00Z"
            }))
        }))
        .route("/health", axum::routing::get(|| async {
            axum::Json(serde_json::json!({"status": "healthy", "service": "mock-target"}))
        }));

    Ok(app)
}

/// Test helper for creating a proxy server
pub async fn create_proxy_server(
    config: ProxyTestConfig,
) -> Result<ProxyServer, Box<dyn std::error::Error + Send + Sync>> {
    let mut proxy_config = ProxyConfig::default();
    proxy_config.enabled = true;
    proxy_config.target_url = Some(format!("http://127.0.0.1:{}", config.target_port));
    proxy_config.prefix = Some("/proxy/".to_string());
    proxy_config.passthrough_by_default = true;

    let proxy_server = ProxyServer::new(proxy_config, config.log_requests, config.log_responses);
    Ok(proxy_server)
}

/// Test HTTP proxy functionality
#[tokio::test]
async fn test_http_proxy_basic_functionality() {
    let config = ProxyTestConfig::default();

    // Create proxy server (no target server needed for health check)
    let proxy_server = create_proxy_server(config.clone()).await.unwrap();
    let proxy_app = proxy_server.router();

    // Test proxy health check
    let response = proxy_app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/proxy/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("healthy"));
    assert!(body_str.contains("mockforge-proxy"));
}

/// Test proxy request forwarding
#[tokio::test]
async fn test_proxy_request_forwarding() {
    // Use a random available port for target server
    let target_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_port = target_listener.local_addr().unwrap().port();

    let mut config = ProxyTestConfig::default();
    config.target_port = target_port;

    // Create and start mock target server
    let target_app = create_mock_target_server(target_port).await.unwrap();

    // Start target server in background
    let target_server_handle = tokio::spawn(async move {
        axum::serve(target_listener, target_app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Create proxy server with updated config
    let proxy_server = create_proxy_server(config.clone()).await.unwrap();
    let proxy_app = proxy_server.router();

    // Test forwarding a GET request
    let response = proxy_app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/proxy/api/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should forward to target server
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Expected 200 OK but got {} - proxy may not be forwarding correctly",
        response.status()
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Alice"));
    assert!(body_str.contains("Bob"));

    // Clean up - stop target server
    target_server_handle.abort();
}
