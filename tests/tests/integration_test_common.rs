//! Common utilities and helpers for integration tests
//!
//! This module provides shared test infrastructure for integration tests,
//! including server setup, test clients, and common assertions.

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

/// Test server configuration
pub struct TestServerConfig {
    pub http_port: u16,
    pub ws_port: u16,
    pub grpc_port: u16,
    pub admin_enabled: bool,
    pub admin_port: u16,
}

impl Default for TestServerConfig {
    fn default() -> Self {
        Self {
            http_port: 3100,
            ws_port: 3101,
            grpc_port: 3102,
            admin_enabled: true,
            admin_port: 3103,
        }
    }
}

/// Wait for a server to be ready by checking if a port is listening
pub async fn wait_for_server(addr: SocketAddr, timeout: Duration) -> Result<(), String> {
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        match TcpListener::bind(addr).await {
            Ok(_) => return Ok(()),
            Err(_) => {
                // Port is in use, which means server is running
                // Try to connect to verify it's actually responding
                match tokio::net::TcpStream::connect(addr).await {
                    Ok(_) => return Ok(()),
                    Err(_) => {
                        sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                }
            }
        }
    }

    Err(format!("Server at {} did not become ready within {:?}", addr, timeout))
}

/// Check if a port is available for binding
pub async fn is_port_available(port: u16) -> bool {
    match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Find an available port starting from a base port
pub async fn find_available_port(base_port: u16) -> u16 {
    for port in base_port..(base_port + 100) {
        if is_port_available(port).await {
            return port;
        }
    }
    panic!("Could not find available port starting from {}", base_port);
}

/// HTTP test client helper
pub struct TestHttpClient {
    pub base_url: String,
    pub client: reqwest::Client,
}

impl TestHttpClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn get(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        self.client.get(&format!("{}{}", self.base_url, path)).send().await
    }

    pub async fn post(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.client.post(&format!("{}{}", self.base_url, path)).json(body).send().await
    }

    pub async fn put(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.client.put(&format!("{}{}", self.base_url, path)).json(body).send().await
    }

    pub async fn delete(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        self.client.delete(&format!("{}{}", self.base_url, path)).send().await
    }
}

/// Assert response status code
pub fn assert_status(response: &reqwest::Response, expected: u16) {
    let actual = response.status().as_u16();
    assert_eq!(
        actual, expected,
        "Expected status {}, got {} (response: {:?})",
        expected, actual, response
    );
}

/// Assert response JSON matches expected value
pub async fn assert_json_eq(response: &mut reqwest::Response, expected: &serde_json::Value) {
    let actual: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(
        actual,
        *expected,
        "JSON mismatch:\nExpected: {}\nActual: {}",
        serde_json::to_string_pretty(expected).unwrap(),
        serde_json::to_string_pretty(&actual).unwrap()
    );
}
