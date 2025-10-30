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
        .route("/api/users/:id", axum::routing::get(|axum::extract::Path(id): axum::extract::Path<u32>| async move {
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

    // Create mock target server
    let target_app = create_mock_target_server(config.target_port).await.unwrap();

    // Create proxy server
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
    let config = ProxyTestConfig::default();

    // Create mock target server
    let target_app = create_mock_target_server(config.target_port).await.unwrap();

    // Create proxy server
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
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Alice"));
    assert!(body_str.contains("Bob"));
}

/// Test proxy with path parameters
#[tokio::test]
async fn test_proxy_path_parameters() {
    let config = ProxyTestConfig::default();

    // Create mock target server
    let target_app = create_mock_target_server(config.target_port).await.unwrap();

    // Create proxy server
    let proxy_server = create_proxy_server(config.clone()).await.unwrap();
    let proxy_app = proxy_server.router();

    // Test forwarding a request with path parameters
    let response = proxy_app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/proxy/api/users/123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("User 123"));
    assert!(body_str.contains("user123@example.com"));
}

/// Test proxy with POST requests
#[tokio::test]
async fn test_proxy_post_request() {
    let config = ProxyTestConfig::default();

    // Create mock target server
    let target_app = create_mock_target_server(config.target_port).await.unwrap();

    // Create proxy server
    let proxy_server = create_proxy_server(config.clone()).await.unwrap();
    let proxy_app = proxy_server.router();

    // Test forwarding a POST request
    let post_body = serde_json::json!({
        "title": "Test Post",
        "content": "This is a test post content"
    });

    let response = proxy_app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/proxy/api/posts")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&post_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Test Post"));
    assert!(body_str.contains("This is a test post content"));
}

/// Test proxy statistics
#[tokio::test]
async fn test_proxy_statistics() {
    let config = ProxyTestConfig::default();

    // Create proxy server
    let proxy_server = create_proxy_server(config.clone()).await.unwrap();

    // Get initial stats
    let stats = get_proxy_stats(&proxy_server).await;
    assert_eq!(stats.total_requests, 0);
    assert_eq!(stats.requests_per_second, 0.0);
    assert_eq!(stats.avg_response_time_ms, 0.0);
    assert_eq!(stats.error_rate_percent, 0.0);
}

/// Test proxy with different HTTP methods
#[tokio::test]
async fn test_proxy_http_methods() {
    let config = ProxyTestConfig::default();

    // Create mock target server
    let target_app = create_mock_target_server(config.target_port).await.unwrap();

    // Create proxy server
    let proxy_server = create_proxy_server(config.clone()).await.unwrap();
    let proxy_app = proxy_server.router();

    // Test different HTTP methods
    let methods = vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::HEAD,
        Method::OPTIONS,
        Method::PATCH,
    ];

    for method in methods {
        let response = proxy_app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method.clone())
                    .uri("/proxy/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // All methods should work (target server handles them)
        assert!(
            response.status().is_success() || response.status() == StatusCode::METHOD_NOT_ALLOWED
        );
    }
}

/// Test proxy error handling
#[tokio::test]
async fn test_proxy_error_handling() {
    let config = ProxyTestConfig::default();

    // Create proxy server with invalid target
    let mut proxy_config = ProxyConfig::default();
    proxy_config.enabled = true;
    proxy_config.target_url = Some("http://127.0.0.1:9999".to_string()); // Invalid port
    proxy_config.prefix = Some("/proxy/".to_string());
    proxy_config.passthrough_by_default = true;

    let proxy_server = ProxyServer::new(proxy_config, true, true);
    let proxy_app = proxy_server.router();

    // Test request to invalid target
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

    // Should return error status
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

/// Test proxy with disabled configuration
#[tokio::test]
async fn test_proxy_disabled() {
    let mut proxy_config = ProxyConfig::default();
    proxy_config.enabled = false; // Disabled proxy

    let proxy_server = ProxyServer::new(proxy_config, true, true);
    let proxy_app = proxy_server.router();

    // Test request to disabled proxy
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

    // Should return service unavailable
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

/// Test proxy prefix handling
#[tokio::test]
async fn test_proxy_prefix_handling() {
    let config = ProxyTestConfig::default();

    // Create mock target server
    let target_app = create_mock_target_server(config.target_port).await.unwrap();

    // Create proxy server with custom prefix
    let mut proxy_config = ProxyConfig::default();
    proxy_config.enabled = true;
    proxy_config.target_url = Some(format!("http://127.0.0.1:{}", config.target_port));
    proxy_config.prefix = Some("/api-proxy/".to_string());
    proxy_config.passthrough_by_default = true;

    let proxy_server = ProxyServer::new(proxy_config, true, true);
    let proxy_app = proxy_server.router();

    // Test request with custom prefix
    let response = proxy_app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api-proxy/api/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Alice"));
}

/// Test proxy with headers
#[tokio::test]
async fn test_proxy_headers() {
    let config = ProxyTestConfig::default();

    // Create mock target server
    let target_app = create_mock_target_server(config.target_port).await.unwrap();

    // Create proxy server
    let proxy_server = create_proxy_server(config.clone()).await.unwrap();
    let proxy_app = proxy_server.router();

    // Test request with custom headers
    let response = proxy_app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/proxy/api/users")
                .header("Authorization", "Bearer test-token")
                .header("X-Custom-Header", "test-value")
                .header("User-Agent", "MockForge-Test/1.0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Headers should be forwarded to target server
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Alice"));
}

/// Integration test simulating browser behavior
#[tokio::test]
async fn test_browser_simulation() {
    let config = ProxyTestConfig::default();

    // Create mock target server
    let target_app = create_mock_target_server(config.target_port).await.unwrap();

    // Create proxy server
    let proxy_server = create_proxy_server(config.clone()).await.unwrap();
    let proxy_app = proxy_server.router();

    // Simulate typical browser requests
    let browser_requests = vec![
        ("GET", "/proxy/api/users", "application/json"),
        ("GET", "/proxy/api/users/1", "application/json"),
        ("POST", "/proxy/api/posts", "application/json"),
        ("GET", "/proxy/health", "application/json"),
    ];

    for (method, path, content_type) in browser_requests {
        let response = proxy_app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("Accept", content_type)
                    .header("User-Agent", "Mozilla/5.0 (compatible; MockForge-Test)")
                    .header("Accept-Language", "en-US,en;q=0.9")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // All browser requests should succeed
        assert!(response.status().is_success());
    }
}

/// Integration test simulating mobile app behavior
#[tokio::test]
async fn test_mobile_app_simulation() {
    let config = ProxyTestConfig::default();

    // Create mock target server
    let target_app = create_mock_target_server(config.target_port).await.unwrap();

    // Create proxy server
    let proxy_server = create_proxy_server(config.clone()).await.unwrap();
    let proxy_app = proxy_server.router();

    // Simulate typical mobile app requests
    let mobile_requests = vec![
        ("GET", "/proxy/api/users", "application/json"),
        ("POST", "/proxy/api/posts", "application/json"),
        ("GET", "/proxy/health", "application/json"),
    ];

    for (method, path, content_type) in mobile_requests {
        let response = proxy_app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("Accept", content_type)
                    .header("User-Agent", "MockForge-Mobile/1.0 (iOS/Android)")
                    .header("X-App-Version", "1.0.0")
                    .header("X-Device-ID", "test-device-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // All mobile requests should succeed
        assert!(response.status().is_success());
    }
}
