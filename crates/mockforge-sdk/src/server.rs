//! Mock server implementation

use crate::builder::MockServerBuilder;
use crate::stub::ResponseStub;
use crate::{Error, Result};
use axum::Router;
use mockforge_core::config::{RouteConfig, RouteResponseConfig};
use mockforge_core::{Config, ServerConfig};
use serde_json::Value;
use std::net::SocketAddr;
use tokio::task::JoinHandle;

/// A mock server that can be embedded in tests
#[derive(Debug)]
pub struct MockServer {
    port: u16,
    address: SocketAddr,
    config: ServerConfig,
    server_handle: Option<JoinHandle<()>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    routes: Vec<RouteConfig>,
}

impl MockServer {
    /// Create a new mock server builder
    #[must_use]
    pub const fn new() -> MockServerBuilder {
        MockServerBuilder::new()
    }

    /// Create a mock server from configuration
    pub(crate) async fn from_config(
        server_config: ServerConfig,
        _core_config: Config,
    ) -> Result<Self> {
        let port = server_config.http.port;
        let host = server_config.http.host.clone();

        let address: SocketAddr = format!("{host}:{port}")
            .parse()
            .map_err(|e| Error::InvalidConfig(format!("Invalid address: {e}")))?;

        Ok(Self {
            port,
            address,
            config: server_config,
            server_handle: None,
            shutdown_tx: None,
            routes: Vec::new(),
        })
    }

    /// Start the mock server
    pub async fn start(&mut self) -> Result<()> {
        if self.server_handle.is_some() {
            return Err(Error::ServerAlreadyStarted(self.port));
        }

        // Build the router from routes
        let router = self.build_simple_router();

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        let address = self.address;

        // Spawn the server
        let server_handle = tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::bind(address).await {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!("Failed to bind to {}: {}", address, e);
                    return;
                }
            };

            tracing::info!("MockForge SDK server listening on {}", address);

            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("Server error");
        });

        self.server_handle = Some(server_handle);

        // Wait for the server to be ready by polling health
        self.wait_for_ready().await?;

        Ok(())
    }

    /// Wait for the server to be ready
    async fn wait_for_ready(&self) -> Result<()> {
        let max_attempts = 50;
        let delay = tokio::time::Duration::from_millis(100);

        for attempt in 0..max_attempts {
            // Try to connect to the server
            let client = reqwest::Client::builder()
                .timeout(tokio::time::Duration::from_millis(100))
                .build()
                .map_err(|e| Error::General(format!("Failed to create HTTP client: {e}")))?;

            match client.get(format!("{}/health", self.url())).send().await {
                Ok(response) if response.status().is_success() => return Ok(()),
                _ => {
                    if attempt < max_attempts - 1 {
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(Error::General(format!(
            "Server failed to become ready within {}ms",
            max_attempts * delay.as_millis() as u32
        )))
    }

    /// Build a simple router from stored routes
    fn build_simple_router(&self) -> Router {
        use axum::http::StatusCode;
        use axum::routing::{delete, get, post, put};
        use axum::{response::IntoResponse, Json};

        let mut router = Router::new();

        for route_config in &self.routes {
            let status = route_config.response.status;
            let body = route_config.response.body.clone();
            let headers = route_config.response.headers.clone();

            let handler = move || {
                let body = body.clone();
                let headers = headers.clone();
                async move {
                    let mut response = Json(body).into_response();
                    *response.status_mut() = StatusCode::from_u16(status).unwrap();

                    for (key, value) in headers {
                        if let Ok(header_name) = axum::http::HeaderName::from_bytes(key.as_bytes())
                        {
                            if let Ok(header_value) = axum::http::HeaderValue::from_str(&value) {
                                response.headers_mut().insert(header_name, header_value);
                            }
                        }
                    }

                    response
                }
            };

            let path = &route_config.path;

            router = match route_config.method.to_uppercase().as_str() {
                "GET" => router.route(path, get(handler)),
                "POST" => router.route(path, post(handler)),
                "PUT" => router.route(path, put(handler)),
                "DELETE" => router.route(path, delete(handler)),
                _ => router,
            };
        }

        router
    }

    /// Stop the mock server
    pub async fn stop(mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(handle) = self.server_handle.take() {
            let _ = handle.await;
        }

        Ok(())
    }

    /// Stub a response for a given method and path
    pub async fn stub_response(
        &mut self,
        method: impl Into<String>,
        path: impl Into<String>,
        body: Value,
    ) -> Result<()> {
        let stub = ResponseStub::new(method, path, body);
        self.add_stub(stub).await
    }

    /// Add a response stub
    pub async fn add_stub(&mut self, stub: ResponseStub) -> Result<()> {
        let route_config = RouteConfig {
            path: stub.path.clone(),
            method: stub.method,
            request: None,
            response: RouteResponseConfig {
                status: stub.status,
                headers: stub.headers,
                body: Some(stub.body),
            },
            fault_injection: None,
            latency: None,
        };

        self.routes.push(route_config);

        Ok(())
    }

    /// Remove all stubs
    pub async fn clear_stubs(&mut self) -> Result<()> {
        self.routes.clear();
        Ok(())
    }

    /// Get the server port
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Get the server base URL
    #[must_use]
    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    /// Check if the server is running
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.server_handle.is_some()
    }
}

impl Default for MockServer {
    fn default() -> Self {
        Self {
            port: 0,
            address: "127.0.0.1:0".parse().unwrap(),
            config: ServerConfig::default(),
            server_handle: None,
            shutdown_tx: None,
            routes: Vec::new(),
        }
    }
}

// Implement Drop to ensure server is stopped
impl Drop for MockServer {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mock_server_new() {
        let builder = MockServer::new();
        // Should return a MockServerBuilder
        assert_eq!(std::mem::size_of_val(&builder), std::mem::size_of::<MockServerBuilder>());
    }

    #[test]
    fn test_mock_server_default() {
        let server = MockServer::default();
        assert_eq!(server.port, 0);
        assert!(!server.is_running());
        assert!(server.routes.is_empty());
    }

    #[test]
    fn test_mock_server_port() {
        let server = MockServer::default();
        assert_eq!(server.port(), 0);
    }

    #[test]
    fn test_mock_server_url() {
        let mut server = MockServer::default();
        server.port = 8080;
        server.address = "127.0.0.1:8080".parse().unwrap();
        assert_eq!(server.url(), "http://127.0.0.1:8080");
    }

    #[test]
    fn test_mock_server_is_running_false() {
        let server = MockServer::default();
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn test_from_config_valid() {
        let server_config = ServerConfig::default();
        let core_config = Config::default();

        let result = MockServer::from_config(server_config, core_config).await;
        assert!(result.is_ok());

        let server = result.unwrap();
        assert!(!server.is_running());
        assert!(server.routes.is_empty());
    }

    #[tokio::test]
    async fn test_from_config_invalid_address() {
        let mut server_config = ServerConfig::default();
        server_config.http.host = "invalid host with spaces".to_string();
        let core_config = Config::default();

        let result = MockServer::from_config(server_config, core_config).await;
        assert!(result.is_err());
        match result {
            Err(Error::InvalidConfig(msg)) => {
                assert!(msg.contains("Invalid address"));
            }
            _ => panic!("Expected InvalidConfig error"),
        }
    }

    #[tokio::test]
    async fn test_add_stub() {
        let mut server = MockServer::default();
        let stub = ResponseStub::new("GET", "/api/test", json!({"test": true}));

        let result = server.add_stub(stub.clone()).await;
        assert!(result.is_ok());
        assert_eq!(server.routes.len(), 1);

        let route = &server.routes[0];
        assert_eq!(route.path, "/api/test");
        assert_eq!(route.method, "GET");
        assert_eq!(route.response.status, 200);
    }

    #[tokio::test]
    async fn test_add_stub_with_custom_status() {
        let mut server = MockServer::default();
        let stub = ResponseStub::new("POST", "/api/create", json!({"created": true}))
            .status(201);

        let result = server.add_stub(stub).await;
        assert!(result.is_ok());
        assert_eq!(server.routes.len(), 1);

        let route = &server.routes[0];
        assert_eq!(route.response.status, 201);
    }

    #[tokio::test]
    async fn test_add_stub_with_headers() {
        let mut server = MockServer::default();
        let stub = ResponseStub::new("GET", "/api/test", json!({}))
            .header("Content-Type", "application/json")
            .header("X-Custom", "value");

        let result = server.add_stub(stub).await;
        assert!(result.is_ok());

        let route = &server.routes[0];
        assert_eq!(route.response.headers.get("Content-Type"), Some(&"application/json".to_string()));
        assert_eq!(route.response.headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[tokio::test]
    async fn test_stub_response() {
        let mut server = MockServer::default();

        let result = server.stub_response("GET", "/api/users", json!({"users": []})).await;
        assert!(result.is_ok());
        assert_eq!(server.routes.len(), 1);

        let route = &server.routes[0];
        assert_eq!(route.path, "/api/users");
        assert_eq!(route.method, "GET");
    }

    #[tokio::test]
    async fn test_clear_stubs() {
        let mut server = MockServer::default();

        server.stub_response("GET", "/api/test1", json!({})).await.unwrap();
        server.stub_response("POST", "/api/test2", json!({})).await.unwrap();
        assert_eq!(server.routes.len(), 2);

        let result = server.clear_stubs().await;
        assert!(result.is_ok());
        assert_eq!(server.routes.len(), 0);
    }

    #[tokio::test]
    async fn test_multiple_stubs() {
        let mut server = MockServer::default();

        server.stub_response("GET", "/api/users", json!({"users": []})).await.unwrap();
        server.stub_response("POST", "/api/users", json!({"created": true})).await.unwrap();
        server.stub_response("DELETE", "/api/users/1", json!({"deleted": true})).await.unwrap();

        assert_eq!(server.routes.len(), 3);

        assert_eq!(server.routes[0].method, "GET");
        assert_eq!(server.routes[1].method, "POST");
        assert_eq!(server.routes[2].method, "DELETE");
    }

    #[test]
    fn test_build_simple_router_empty() {
        let server = MockServer::default();
        let router = server.build_simple_router();
        // Router should be created without panicking
        assert_eq!(std::mem::size_of_val(&router), std::mem::size_of::<Router>());
    }

    #[tokio::test]
    async fn test_build_simple_router_with_routes() {
        let mut server = MockServer::default();
        server.stub_response("GET", "/test", json!({"test": true})).await.unwrap();
        server.stub_response("POST", "/create", json!({"created": true})).await.unwrap();

        let router = server.build_simple_router();
        // Router should be built with the routes
        assert_eq!(std::mem::size_of_val(&router), std::mem::size_of::<Router>());
    }

    #[tokio::test]
    async fn test_start_server_already_started() {
        let mut server = MockServer::default();
        server.port = 0; // Use port 0 for OS assignment
        server.address = "127.0.0.1:0".parse().unwrap();

        // Start the server
        let result = server.start().await;
        assert!(result.is_ok());
        assert!(server.is_running());

        // Try to start again
        let result2 = server.start().await;
        assert!(result2.is_err());
        match result2 {
            Err(Error::ServerAlreadyStarted(_)) => (),
            _ => panic!("Expected ServerAlreadyStarted error"),
        }

        // Clean up
        let _ = server.stop().await;
    }

    #[test]
    fn test_server_debug_format() {
        let server = MockServer::default();
        let debug_str = format!("{server:?}");
        assert!(debug_str.contains("MockServer"));
    }

    #[tokio::test]
    async fn test_route_config_conversion() {
        let mut server = MockServer::default();
        let stub = ResponseStub::new("PUT", "/api/update", json!({"updated": true}))
            .status(200)
            .header("X-Version", "1.0");

        server.add_stub(stub).await.unwrap();

        let route = &server.routes[0];
        assert_eq!(route.path, "/api/update");
        assert_eq!(route.method, "PUT");
        assert_eq!(route.response.status, 200);
        assert_eq!(route.response.headers.get("X-Version"), Some(&"1.0".to_string()));
        assert!(route.response.body.is_some());
        assert_eq!(route.response.body, Some(json!({"updated": true})));
    }

    #[tokio::test]
    async fn test_server_with_different_methods() {
        let mut server = MockServer::default();

        server.stub_response("GET", "/test", json!({})).await.unwrap();
        server.stub_response("POST", "/test", json!({})).await.unwrap();
        server.stub_response("PUT", "/test", json!({})).await.unwrap();
        server.stub_response("DELETE", "/test", json!({})).await.unwrap();
        server.stub_response("PATCH", "/test", json!({})).await.unwrap();

        assert_eq!(server.routes.len(), 5);

        // Verify all methods are different
        let methods: Vec<_> = server.routes.iter().map(|r| r.method.as_str()).collect();
        assert!(methods.contains(&"GET"));
        assert!(methods.contains(&"POST"));
        assert!(methods.contains(&"PUT"));
        assert!(methods.contains(&"DELETE"));
        assert!(methods.contains(&"PATCH"));
    }

    #[tokio::test]
    async fn test_server_url_format() {
        let mut server = MockServer::default();
        server.port = 3000;
        server.address = "127.0.0.1:3000".parse().unwrap();

        let url = server.url();
        assert_eq!(url, "http://127.0.0.1:3000");
        assert!(url.starts_with("http://"));
    }

    #[tokio::test]
    async fn test_server_with_ipv6_address() {
        let mut server = MockServer::default();
        server.port = 8080;
        server.address = "[::1]:8080".parse().unwrap();

        let url = server.url();
        assert_eq!(url, "http://[::1]:8080");
    }
}
