//! E2E test helpers and utilities
//!
//! Shared utilities for end-to-end testing across protocols and SDKs

use mockforge_test::{MockForgeServer, ServerConfig};
use mockforge_core::ServerConfig as CoreServerConfig;
use std::time::Duration;

/// Test server configuration for E2E tests
pub struct E2ETestServer {
    pub server: MockForgeServer,
    pub http_port: u16,
    pub admin_port: u16,
    pub ws_port: Option<u16>,
    pub grpc_port: Option<u16>,
}

impl E2ETestServer {
    /// Start a test server with default configuration
    pub async fn start() -> Result<Self, mockforge_test::Error> {
        Self::start_with_config(ServerConfig::default()).await
    }

    /// Start a test server with custom configuration
    pub async fn start_with_config(config: ServerConfig) -> Result<Self, mockforge_test::Error> {
        // Ensure admin is enabled
        let mut config = config;
        if !config.enable_admin {
            config.enable_admin = true;
        }
        if config.admin_port.is_none() {
            config.admin_port = Some(0); // Auto-assign admin port
        }

        let server = MockForgeServer::builder()
            .http_port(config.http_port)
            .admin_port(config.admin_port.unwrap_or(0))
            .ws_port(config.ws_port)
            .grpc_port(config.grpc_port)
            .enable_admin(true)
            .health_timeout(Duration::from_secs(30))
            .build()
            .await?;

        // Admin port defaults to 9080 if not specified, or we can try to detect it
        // For now, use default admin port
        let admin_port = config.admin_port.unwrap_or(9080);

        Ok(Self {
            http_port: server.http_port(),
            admin_port,
            ws_port: server.ws_port(),
            grpc_port: server.grpc_port(),
            server,
        })
    }

    /// Get the base HTTP URL
    pub fn http_url(&self) -> String {
        format!("http://localhost:{}", self.http_port)
    }

    /// Get the admin API URL
    pub fn admin_url(&self) -> String {
        format!("http://localhost:{}", self.admin_port)
    }

    /// Get the WebSocket URL
    pub fn ws_url(&self) -> Option<String> {
        self.ws_port.map(|port| format!("ws://localhost:{}", port))
    }

    /// Get the gRPC address
    pub fn grpc_addr(&self) -> Option<String> {
        self.grpc_port.map(|port| format!("http://localhost:{}", port))
    }

    /// Stop the server
    pub fn stop(self) -> Result<(), mockforge_test::Error> {
        self.server.stop()
    }
}

/// Create a basic HTTP server configuration for testing
pub fn http_test_config() -> CoreServerConfig {
    let mut config = CoreServerConfig::default();
    config.http.port = 0; // Auto-assign port
    config.http.enabled = true;
    config.admin.enabled = true;
    config.admin.port = 0; // Auto-assign port
    config
}

/// Create a WebSocket server configuration for testing
pub fn websocket_test_config() -> CoreServerConfig {
    let mut config = http_test_config();
    config.websocket.enabled = true;
    config.websocket.port = 0; // Auto-assign port
    config
}

/// Create a gRPC server configuration for testing
pub fn grpc_test_config() -> CoreServerConfig {
    let mut config = http_test_config();
    config.grpc.enabled = true;
    config.grpc.port = 0; // Auto-assign port
    config
}

/// Create a GraphQL server configuration for testing
pub fn graphql_test_config() -> CoreServerConfig {
    let mut config = http_test_config();
    config.graphql.enabled = true;
    config.graphql.port = 0; // Auto-assign port
    config
}

/// Wait for a condition to become true with timeout
pub async fn wait_for<F, Fut>(mut condition: F, timeout: Duration, interval: Duration) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if condition().await {
            return true;
        }
        tokio::time::sleep(interval).await;
    }
    false
}

/// Assert that a response has the expected status code
pub fn assert_status(response: &reqwest::Response, expected: u16) {
    assert_eq!(
        response.status().as_u16(),
        expected,
        "Expected status {}, got {}",
        expected,
        response.status()
    );
}

/// Assert that a response is JSON and matches expected structure
pub async fn assert_json_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, Box<dyn std::error::Error>> {
    assert!(response.headers().get("content-type").is_some());
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        content_type.contains("application/json"),
        "Expected JSON response, got {}",
        content_type
    );
    Ok(response.json().await?)
}
