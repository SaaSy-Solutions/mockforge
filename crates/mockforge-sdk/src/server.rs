//! Mock server implementation

use crate::builder::MockServerBuilder;
use crate::stub::ResponseStub;
use crate::{Error, Result};
use axum::Router;
use mockforge_core::config::{RouteConfig, RouteResponseConfig};
use mockforge_core::{Config, ServerConfig};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::task::JoinHandle;

/// A mock server that can be embedded in tests
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
    pub fn new() -> MockServerBuilder {
        MockServerBuilder::new()
    }

    /// Create a mock server from configuration
    pub(crate) async fn from_config(
        server_config: ServerConfig,
        _core_config: Config,
    ) -> Result<Self> {
        let port = server_config.http.port;
        let host = server_config.http.host.clone();

        let address: SocketAddr = format!("{}:{}", host, port)
            .parse()
            .map_err(|e| Error::InvalidConfig(format!("Invalid address: {}", e)))?;

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
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
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
                .map_err(|e| Error::General(format!("Failed to create HTTP client: {}", e)))?;

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
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get the server base URL
    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    /// Check if the server is running
    pub fn is_running(&self) -> bool {
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
