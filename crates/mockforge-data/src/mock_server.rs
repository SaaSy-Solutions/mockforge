//! Mock Server Mode Implementation
//!
//! This module provides MSW-style mock server capabilities that can serve
//! generated mock data based on OpenAPI specifications.

use crate::mock_generator::{MockDataGenerator, MockDataResult, MockGeneratorConfig, MockResponse};
use crate::{Error, Result};
use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

/// Configuration for the mock server
#[derive(Debug, Clone)]
pub struct MockServerConfig {
    /// Port to run the server on
    pub port: u16,
    /// Host to bind to
    pub host: String,
    /// OpenAPI specification
    pub openapi_spec: Value,
    /// Mock data generator configuration
    pub generator_config: MockGeneratorConfig,
    /// Whether to enable CORS
    pub enable_cors: bool,
    /// Custom response delays (in milliseconds)
    pub response_delays: HashMap<String, u64>,
    /// Whether to log all requests
    pub log_requests: bool,
}

impl Default for MockServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "127.0.0.1".to_string(),
            openapi_spec: json!({}),
            generator_config: MockGeneratorConfig::default(),
            enable_cors: true,
            response_delays: HashMap::new(),
            log_requests: true,
        }
    }
}

impl MockServerConfig {
    /// Create a new mock server configuration
    pub fn new(openapi_spec: Value) -> Self {
        Self {
            openapi_spec,
            ..Default::default()
        }
    }

    /// Set the port
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the host
    pub fn host(mut self, host: String) -> Self {
        self.host = host;
        self
    }

    /// Set the generator configuration
    pub fn generator_config(mut self, config: MockGeneratorConfig) -> Self {
        self.generator_config = config;
        self
    }

    /// Enable or disable CORS
    pub fn enable_cors(mut self, enabled: bool) -> Self {
        self.enable_cors = enabled;
        self
    }

    /// Add a response delay for a specific endpoint
    pub fn response_delay(mut self, endpoint: String, delay_ms: u64) -> Self {
        self.response_delays.insert(endpoint, delay_ms);
        self
    }

    /// Enable or disable request logging
    pub fn log_requests(mut self, enabled: bool) -> Self {
        self.log_requests = enabled;
        self
    }
}

/// Mock server that serves generated data based on OpenAPI specifications
#[derive(Debug)]
pub struct MockServer {
    /// Server configuration
    config: MockServerConfig,
    /// Generated mock data
    mock_data: Arc<MockDataResult>,
    /// Route handlers
    handlers: HashMap<String, MockResponse>,
}

impl MockServer {
    /// Create a new mock server
    pub fn new(config: MockServerConfig) -> Result<Self> {
        info!("Creating mock server with OpenAPI specification");

        // Generate mock data from the OpenAPI spec
        let mut generator = MockDataGenerator::with_config(config.generator_config.clone());
        let mock_data = generator.generate_from_openapi_spec(&config.openapi_spec)?;

        // Create handlers map from generated responses
        let mut handlers = HashMap::new();
        for (endpoint, response) in &mock_data.responses {
            handlers.insert(endpoint.clone(), response.clone());
        }

        Ok(Self {
            config,
            mock_data: Arc::new(mock_data),
            handlers,
        })
    }

    /// Start the mock server
    pub async fn start(self) -> Result<()> {
        let config = self.config.clone();
        let app = self.create_router();
        let addr = SocketAddr::from(([127, 0, 0, 1], config.port));

        info!("Starting mock server on {}", addr);

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| Error::generic(format!("Failed to bind to {}: {}", addr, e)))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| Error::generic(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// Create the Axum router with all endpoints
    fn create_router(self) -> Router {
        let mock_data = Arc::clone(&self.mock_data);
        let config = Arc::new(self.config);
        let handlers = Arc::new(self.handlers);

        Router::new()
            // Add all OpenAPI endpoints dynamically
            .route("/", get(Self::root_handler))
            .route("/health", get(Self::health_handler))
            .route("/openapi.json", get(Self::openapi_handler))
            .route("/mock-data", get(Self::mock_data_handler))
            // Dynamic routes will be added based on OpenAPI spec
            .with_state(MockServerState {
                mock_data,
                config,
                handlers,
            })
    }

    /// Root handler - returns API information
    async fn root_handler() -> Json<Value> {
        Json(json!({
            "name": "MockForge Mock Server",
            "version": "1.0.0",
            "description": "Mock server powered by MockForge",
            "endpoints": {
                "/health": "Health check endpoint",
                "/openapi.json": "OpenAPI specification",
                "/mock-data": "Generated mock data"
            }
        }))
    }

    /// Health check handler
    async fn health_handler() -> Json<Value> {
        Json(json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "service": "mockforge-mock-server"
        }))
    }

    /// OpenAPI specification handler
    async fn openapi_handler(
        axum::extract::State(state): axum::extract::State<MockServerState>,
    ) -> Json<Value> {
        Json(serde_json::to_value(&state.mock_data.spec_info).unwrap_or(json!({})))
    }

    /// Mock data handler - returns all generated mock data
    async fn mock_data_handler(
        axum::extract::State(state): axum::extract::State<MockServerState>,
    ) -> Json<Value> {
        Json(json!({
            "schemas": state.mock_data.schemas,
            "responses": state.mock_data.responses,
            "warnings": state.mock_data.warnings
        }))
    }

    /// Generic endpoint handler that serves mock data based on the request
    ///
    /// This handler is kept for future use when implementing generic mock server
    /// functionality that doesn't require pre-defined routes.
    ///
    /// TODO: Integrate into mock server when generic routing is implemented
    #[allow(dead_code)] // TODO: Remove when generic handler is integrated
    async fn generic_handler(
        axum::extract::State(state): axum::extract::State<MockServerState>,
        method: axum::http::Method,
        path: axum::extract::Path<String>,
        query: Query<HashMap<String, String>>,
        _headers: HeaderMap,
    ) -> std::result::Result<Json<Value>, StatusCode> {
        let endpoint_key = format!("{} /{}", method.as_str().to_uppercase(), path.as_str());

        // Log request if enabled
        if state.config.log_requests {
            info!("Handling request: {} with query: {:?}", endpoint_key, query);
        }

        // Apply response delay if configured
        if let Some(delay) = state.config.response_delays.get(&endpoint_key) {
            tokio::time::sleep(tokio::time::Duration::from_millis(*delay)).await;
        }

        // Find matching handler
        if let Some(response) = state.handlers.get(&endpoint_key) {
            Ok(Json(response.body.clone()))
        } else {
            // Try to find a similar endpoint (for path parameters)
            let similar_endpoint = state
                .handlers
                .keys()
                .find(|key| Self::endpoints_match(key, &endpoint_key))
                .cloned();

            if let Some(endpoint) = similar_endpoint {
                if let Some(response) = state.handlers.get(&endpoint) {
                    Ok(Json(response.body.clone()))
                } else {
                    Err(StatusCode::NOT_FOUND)
                }
            } else {
                // Return a generic mock response
                let generic_response = json!({
                    "message": "Mock response",
                    "endpoint": endpoint_key,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "data": {}
                });
                Ok(Json(generic_response))
            }
        }
    }

    /// Check if two endpoints match (handles path parameters)
    ///
    /// TODO: Integrate into route matching system when advanced path parameter matching is implemented
    #[allow(dead_code)] // TODO: Remove when route matching system is updated
    pub fn endpoints_match(pattern: &str, request: &str) -> bool {
        // Simple pattern matching - in a real implementation,
        // you'd want more sophisticated path parameter matching
        let pattern_parts: Vec<&str> = pattern.split(' ').collect();
        let request_parts: Vec<&str> = request.split(' ').collect();

        if pattern_parts.len() != request_parts.len() {
            return false;
        }

        for (pattern_part, request_part) in pattern_parts.iter().zip(request_parts.iter()) {
            if pattern_part != request_part && !pattern_part.contains(":") {
                return false;
            }
        }

        true
    }
}

/// State shared across all handlers
#[derive(Debug, Clone)]
struct MockServerState {
    mock_data: Arc<MockDataResult>,
    // These fields are reserved for future mock server configuration
    // TODO: Integrate config and handlers when full mock server implementation is completed
    #[allow(dead_code)] // TODO: Remove when config integration is complete
    config: Arc<MockServerConfig>,
    #[allow(dead_code)] // TODO: Remove when handler registry is integrated
    handlers: Arc<HashMap<String, MockResponse>>,
}

/// Builder for creating mock servers
#[derive(Debug)]
pub struct MockServerBuilder {
    config: MockServerConfig,
}

impl MockServerBuilder {
    /// Create a new mock server builder
    pub fn new(openapi_spec: Value) -> Self {
        Self {
            config: MockServerConfig::new(openapi_spec),
        }
    }

    /// Set the port
    pub fn port(mut self, port: u16) -> Self {
        self.config = self.config.port(port);
        self
    }

    /// Set the host
    pub fn host(mut self, host: String) -> Self {
        self.config = self.config.host(host);
        self
    }

    /// Set the generator configuration
    pub fn generator_config(mut self, config: MockGeneratorConfig) -> Self {
        self.config = self.config.generator_config(config);
        self
    }

    /// Enable or disable CORS
    pub fn enable_cors(mut self, enabled: bool) -> Self {
        self.config = self.config.enable_cors(enabled);
        self
    }

    /// Add a response delay for a specific endpoint
    pub fn response_delay(mut self, endpoint: String, delay_ms: u64) -> Self {
        self.config = self.config.response_delay(endpoint, delay_ms);
        self
    }

    /// Enable or disable request logging
    pub fn log_requests(mut self, enabled: bool) -> Self {
        self.config = self.config.log_requests(enabled);
        self
    }

    /// Build the mock server
    pub fn build(self) -> Result<MockServer> {
        MockServer::new(self.config)
    }
}

/// Quick function to start a mock server
pub async fn start_mock_server(openapi_spec: Value, port: u16) -> Result<()> {
    let server = MockServerBuilder::new(openapi_spec).port(port).build()?;

    server.start().await
}

/// Quick function to start a mock server with custom configuration
pub async fn start_mock_server_with_config(config: MockServerConfig) -> Result<()> {
    let server = MockServer::new(config)?;
    server.start().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mock_server_config_default() {
        let config = MockServerConfig::default();

        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "127.0.0.1");
        assert!(config.enable_cors);
        assert!(config.log_requests);
        assert!(config.response_delays.is_empty());
    }

    #[test]
    fn test_mock_server_config_new() {
        let spec = json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            }
        });

        let config = MockServerConfig::new(spec);

        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "127.0.0.1");
        assert!(config.enable_cors);
    }

    #[test]
    fn test_mock_server_config_builder_methods() {
        let spec = json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            }
        });

        let config = MockServerConfig::new(spec)
            .port(8080)
            .host("0.0.0.0".to_string())
            .enable_cors(false)
            .response_delay("/api/users".to_string(), 100)
            .log_requests(false);

        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
        assert!(!config.enable_cors);
        assert!(!config.log_requests);
        assert!(config.response_delays.contains_key("/api/users"));
        assert_eq!(config.response_delays.get("/api/users"), Some(&100));
    }

    #[test]
    fn test_mock_server_builder() {
        let spec = json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            }
        });

        let builder = MockServerBuilder::new(spec)
            .port(8080)
            .host("0.0.0.0".to_string())
            .enable_cors(false);

        assert_eq!(builder.config.port, 8080);
        assert_eq!(builder.config.host, "0.0.0.0");
        assert!(!builder.config.enable_cors);
    }

    #[test]
    fn test_endpoints_match_exact() {
        assert!(MockServer::endpoints_match("GET /api/users", "GET /api/users"));
        assert!(!MockServer::endpoints_match("GET /api/users", "POST /api/users"));
        assert!(!MockServer::endpoints_match("GET /api/users", "GET /api/products"));
    }

    #[test]
    fn test_endpoints_match_with_params() {
        // This is a simplified test - real path parameter matching would be more complex
        assert!(MockServer::endpoints_match("GET /api/users/:id", "GET /api/users/123"));
        assert!(MockServer::endpoints_match("GET /api/users/:id", "GET /api/users/abc"));
    }

    #[tokio::test]
    async fn test_mock_server_creation() {
        let spec = json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/api/users": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": "List of users",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "properties": {
                                                "users": {
                                                    "type": "array",
                                                    "items": {
                                                        "type": "object",
                                                        "properties": {
                                                            "id": {"type": "string"},
                                                            "name": {"type": "string"},
                                                            "email": {"type": "string"}
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let config = MockServerConfig::new(spec);
        let server = MockServer::new(config);

        assert!(server.is_ok());
    }
}
