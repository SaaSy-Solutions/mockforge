//! HTTP to gRPC bridge implementation
//!
//! This module provides functionality to bridge HTTP requests to gRPC services,
//! allowing RESTful APIs to be generated dynamically from protobuf definitions.

pub mod route_generator;
pub mod handlers;
pub mod converters;

use crate::reflection::{MockReflectionProxy, ProxyConfig};
use crate::dynamic::proto_parser::ProtoService;
use converters::{ProtobufJsonConverter, ConversionError};
use route_generator::RouteGenerator;
use std::collections::HashMap;
use std::sync::Arc;
use std::future::Future;
use axum::{
    Router,
    routing::{post, get, put, delete, patch},
    http::Method,
    extract::{Path, Query, State},
    response::{Json, IntoResponse, Sse},
    body::Bytes,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tower_http::cors::{CorsLayer, Any};
use tracing::{debug, info, warn};
use futures_util::stream::Stream;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use std::pin::Pin;

/// Configuration for the HTTP bridge
#[derive(Debug, Clone)]
pub struct HttpBridgeConfig {
    /// Whether the HTTP bridge is enabled
    pub enabled: bool,
    /// Base path for HTTP routes (e.g., "/api")
    pub base_path: String,
    /// Whether to enable CORS
    pub enable_cors: bool,
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Timeout for bridge requests in seconds
    pub timeout_seconds: u64,
    /// Path pattern for service routes (e.g., "/{service}/{method}")
    pub route_pattern: String,
}

impl Default for HttpBridgeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_path: "/api".to_string(),
            enable_cors: true,
            max_request_size: 10 * 1024 * 1024, // 10MB
            timeout_seconds: 30,
            route_pattern: "/{service}/{method}".to_string(),
        }
    }
}

/// Query parameters for HTTP requests
#[derive(Debug, Deserialize)]
pub struct BridgeQuery {
    /// Streaming mode (none, server, client, bidirectional)
    #[serde(default)]
    pub stream: Option<String>,
    /// Metadata to pass to gRPC call as key=value pairs
    #[serde(flatten)]
    pub metadata: HashMap<String, String>,
}

/// HTTP response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeResponse<T> {
    /// Whether the request was successful
    pub success: bool,
    /// The response data
    pub data: Option<T>,
    /// Error message if success is false
    pub error: Option<String>,
    /// Metadata from the gRPC response
    pub metadata: HashMap<String, String>,
}

/// Statistics about the HTTP bridge
#[derive(Debug, Serialize, Clone)]
pub struct BridgeStats {
    /// Number of requests served
    pub requests_served: u64,
    /// Number of successful requests
    pub requests_successful: u64,
    /// Number of failed requests
    pub requests_failed: u64,
    /// Services available via the bridge
    pub available_services: Vec<String>,
}

/// The HTTP bridge that provides RESTful API access to gRPC services
pub struct HttpBridge {
    /// The reflection proxy that handles gRPC calls
    proxy: Arc<MockReflectionProxy>,
    /// Route generator for creating HTTP routes
    route_generator: RouteGenerator,
    /// JSON to protobuf converter
    converter: ProtobufJsonConverter,
    /// Bridge configuration
    config: HttpBridgeConfig,
    /// Statistics
    stats: Arc<std::sync::Mutex<BridgeStats>>,
}

impl HttpBridge {
    /// Create a new HTTP bridge
    pub fn new(
        proxy: Arc<MockReflectionProxy>,
        config: HttpBridgeConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let route_generator = RouteGenerator::new(config.clone());
        let converter = ProtobufJsonConverter::new(proxy.service_registry.descriptor_pool().clone());
        let available_services = proxy.service_names();

        let stats = BridgeStats {
            requests_served: 0,
            requests_successful: 0,
            requests_failed: 0,
            available_services,
        };

        Ok(Self {
            proxy,
            route_generator,
            converter,
            config,
            stats: Arc::new(std::sync::Mutex::new(stats)),
        })
    }

    /// Create the HTTP router with all bridge routes
    pub fn create_router(&self) -> Router<Arc<HttpBridge>> {
        let mut router = Router::new();

        // Add CORS if enabled
        if self.config.enable_cors {
            router = router.layer(
                CorsLayer::new()
                    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH])
                    .allow_headers(Any)
                    .allow_origin(Any)
            );
        }

        // Add state containing self reference
        let bridge = Arc::new(self.clone());
        router = router.with_state(bridge);

        // Add health check endpoint
        router = router.route(
            &format!("{}/health", self.config.base_path),
            get(Self::health_check)
        );

        // Add statistics endpoint
        router = router.route(
            &format!("{}/stats", self.config.base_path),
            get(Self::get_stats)
        );

        // Add services listing endpoint
        router = router.route(
            &format!("{}/services", self.config.base_path),
            get(Self::list_services)
        );

        // Add OpenAPI documentation endpoint
        router = router.route(
            &format!("{}/docs", self.config.base_path),
            get(Self::get_openapi_spec)
        );

        // Create dynamic bridge endpoints for all registered services
        let registry = self.proxy.service_registry();

        // Add a generic route that handles all service/method combinations
        // The route pattern supports both GET (for streaming) and POST (for unary) requests
        router = router.route(&self.config.route_pattern, post(Self::handle_generic_bridge_request));
        router = router.route(&self.config.route_pattern, get(Self::handle_generic_bridge_request));

        let available_services = registry.service_names();
        let total_methods = registry.services.values().map(|s| s.service().methods.len()).sum::<usize>();
        info!("Created HTTP bridge router with {} services and {} dynamic endpoints",
              available_services.len(), total_methods);

        router
    }

    /// Health check handler
    async fn health_check(State(bridge): State<Arc<HttpBridge>>) -> axum::response::Json<serde_json::Value> {
        axum::response::Json(serde_json::json!({"status": "ok", "bridge": "healthy"}))
    }

    /// Get statistics handler
    async fn get_stats(State(bridge): State<Arc<HttpBridge>>) -> axum::response::Json<serde_json::Value> {
        let stats = bridge.stats.lock().unwrap();
        axum::response::Json(serde_json::json!({
            "requests_served": stats.requests_served,
            "requests_successful": stats.requests_successful,
            "requests_failed": stats.requests_failed,
            "available_services": stats.available_services
        }))
    }

    /// List services handler
    async fn list_services(State(bridge): State<Arc<HttpBridge>>) -> axum::response::Json<serde_json::Value> {
        Self::list_services_static(&bridge).await
    }

    /// Get OpenAPI spec handler
    async fn get_openapi_spec(State(bridge): State<Arc<HttpBridge>>) -> axum::response::Json<serde_json::Value> {
        Self::get_openapi_spec_static(&bridge).await
    }

    /// Generic bridge request handler that routes to specific services/methods
    async fn handle_generic_bridge_request(
        State(state): State<Arc<HttpBridge>>,
        Path(path_params): Path<HashMap<String, String>>,
        Query(query): Query<BridgeQuery>,
        body: Bytes,
    ) -> axum::response::Response {
        // Extract service and method from path parameters
        let service_name = match path_params.get("service") {
            Some(name) => name,
            None => {
                let error_response = BridgeResponse::<Value> {
                    success: false,
                    data: None,
                    error: Some("Missing 'service' parameter in path".to_string()),
                    metadata: HashMap::new(),
                };
                return (axum::http::StatusCode::BAD_REQUEST, Json(error_response)).into_response();
            }
        };

        let method_name = match path_params.get("method") {
            Some(name) => name,
            None => {
                let error_response = BridgeResponse::<Value> {
                    success: false,
                    data: None,
                    error: Some("Missing 'method' parameter in path".to_string()),
                    metadata: HashMap::new(),
                };
                return (axum::http::StatusCode::BAD_REQUEST, Json(error_response)).into_response();
            }
        };

        // Get method information from the registry
        let registry = state.proxy.service_registry();
        let service_opt = registry.get(service_name);
        let method_info = if let Some(service) = service_opt {
            service.service().methods.iter()
                .find(|m| m.name == *method_name)
        } else {
            let error_response = BridgeResponse::<Value> {
                success: false,
                data: None,
                error: Some(format!("Service '{}' not found", service_name)),
                metadata: HashMap::new(),
            };
            return (axum::http::StatusCode::NOT_FOUND, Json(error_response)).into_response();
        };

        let method_info = match method_info {
            Some(method) => method,
            None => {
                let error_response = BridgeResponse::<Value> {
                    success: false,
                    data: None,
                    error: Some(format!("Method '{}' not found in service '{}'", method_name, service_name)),
                    metadata: HashMap::new(),
                };
                return (axum::http::StatusCode::NOT_FOUND, Json(error_response)).into_response();
            }
        };

        // Update stats
        {
            let mut stats = state.stats.lock().unwrap();
            stats.requests_served += 1;
        }

        // Handle the request
        let result = Self::handle_bridge_request(
            &state.proxy,
            &state.converter,
            service_name,
            method_name,
            method_info.client_streaming,
            method_info.server_streaming,
            Query(query),
            body,
        ).await;

        match result {
            Ok(response) => {
                // Update successful stats
                {
                    let mut stats = state.stats.lock().unwrap();
                    stats.requests_successful += 1;
                }
                (axum::http::StatusCode::OK, Json(response)).into_response()
            }
            Err(err) => {
                // Update failed stats
                {
                    let mut stats = state.stats.lock().unwrap();
                    stats.requests_failed += 1;
                }
                warn!("Bridge request failed for {}.{}: {}", service_name, method_name, err);
                let error_response = BridgeResponse::<Value> {
                    success: false,
                    data: None,
                    error: Some(err.to_string()),
                    metadata: HashMap::new(),
                };
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
            }
        }
    }

    /// Create a handler function for a specific gRPC method
    fn create_bridge_handler(
        &self,
        service_name: String,
        method_name: String,
        client_streaming: bool,
        server_streaming: bool,
    ) -> Box<dyn Fn(
        State<Arc<Self>>,
        Path<HashMap<String, String>>,
        Query<BridgeQuery>,
        Bytes,
    ) -> Pin<Box<dyn Future<Output = axum::response::Response> + Send>> + Send + Sync> {
        Box::new(move |state: State<Arc<Self>>,
              path: Path<HashMap<String, String>>,
              query: Query<BridgeQuery>,
              body: Bytes| {
            let service_name = service_name.clone();
            let method_name = method_name.clone();
            let stats = state.stats.clone();
            let proxy = state.proxy.clone();
            let converter = state.converter.clone();

            Box::pin(async move {
                // Update stats
                {
                    let mut stats = stats.lock().unwrap();
                    stats.requests_served += 1;
                }

                // Handle the request
                let result = Self::handle_bridge_request(
                    &proxy,
                    &converter,
                    &service_name,
                    &method_name,
                    client_streaming,
                    server_streaming,
                    query,
                    body,
                ).await;

                match result {
                    Ok(response) => {
                        // Update successful stats
                        {
                            let mut stats = stats.lock().unwrap();
                            stats.requests_successful += 1;
                        }
                        (axum::http::StatusCode::OK, Json(response)).into_response()
                    }
                    Err(err) => {
                        // Update failed stats
                        {
                            let mut stats = stats.lock().unwrap();
                            stats.requests_failed += 1;
                        }
                        warn!("Bridge request failed for {}.{}: {}", service_name, method_name, err);
                        let error_response = BridgeResponse::<Value> {
                            success: false,
                            data: None,
                            error: Some(err.to_string()),
                            metadata: HashMap::new(),
                        };
                        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
                    }
                }
            })
        })
    }

    /// Get bridge statistics (static method for handler)
    async fn get_stats_static(bridge: &Arc<HttpBridge>) -> axum::response::Json<serde_json::Value> {
        let stats = bridge.stats.lock().unwrap();
        axum::response::Json(serde_json::json!({
            "requests_served": stats.requests_served,
            "requests_successful": stats.requests_successful,
            "requests_failed": stats.requests_failed,
            "available_services": stats.available_services
        }))
    }

    /// List available services (static method for handler)
    async fn list_services_static(bridge: &Arc<HttpBridge>) -> axum::response::Json<serde_json::Value> {
        let services = bridge.proxy.service_names();
        axum::response::Json(serde_json::json!({
            "services": services
        }))
    }

    /// Get OpenAPI spec (static method for handler)
    async fn get_openapi_spec_static(bridge: &Arc<HttpBridge>) -> axum::response::Json<serde_json::Value> {
        use std::collections::HashMap;
        use crate::dynamic::proto_parser::ProtoService;

        // Extract services from the service registry
        let services: HashMap<String, ProtoService> = bridge.proxy.service_registry().services.iter()
            .map(|(name, dyn_service)| (name.clone(), dyn_service.service().clone()))
            .collect();

        // Generate OpenAPI spec using the route generator
        let spec = bridge.route_generator.generate_openapi_spec(&services);
        axum::response::Json(spec)
    }

    /// Handle a bridge request by calling the appropriate gRPC method
    async fn handle_bridge_request(
        proxy: &MockReflectionProxy,
        converter: &ProtobufJsonConverter,
        service_name: &str,
        method_name: &str,
        client_streaming: bool,
        server_streaming: bool,
        query: Query<BridgeQuery>,
        body: Bytes,
    ) -> Result<BridgeResponse<Value>, Box<dyn std::error::Error + Send + Sync>> {
        debug!("Handling bridge request for {}.{}", service_name, method_name);

        // Parse JSON request body
        let json_request: Value = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&body).map_err(|e| {
                Box::<dyn std::error::Error + Send + Sync>::from(format!("Failed to parse JSON request: {}", e))
            })?
        };

        // Call appropriate gRPC method based on streaming type
        if server_streaming {
            // Handle streaming response
            Self::handle_streaming_request(proxy, converter, service_name, method_name, json_request).await
        } else {
            // Handle unary request
            Self::handle_unary_request(proxy, converter, service_name, method_name, json_request).await
        }
    }

    /// Handle unary request (no streaming)
    async fn handle_unary_request(
        proxy: &MockReflectionProxy,
        converter: &ProtobufJsonConverter,
        service_name: &str,
        method_name: &str,
        json_request: Value,
    ) -> Result<BridgeResponse<Value>, Box<dyn std::error::Error + Send + Sync>> {
        // Get method descriptor from the service registry
        let registry = proxy.service_registry();
        let service_registry = registry.clone();

        // Find the service and method
        let service_opt = service_registry.get(service_name);
        if service_opt.is_none() {
            return Err(format!("Service '{}' not found", service_name).into());
        }

        let service = service_opt.unwrap();
        let method_opt = service.service().methods.iter().find(|m| m.name == method_name);
        if method_opt.is_none() {
            return Err(format!("Method '{}' not found in service '{}'", method_name, service_name).into());
        }

        // For now, create a generic response since we don't have full descriptor integration
        // In a complete implementation, this would:
        // 1. Get input/output descriptor from proto parser
        // 2. Convert JSON to protobuf message
        // 3. Call the actual gRPC method via proxy
        // 4. Convert protobuf response back to JSON

        // Create a mock response for demonstration
        let json_response = serde_json::json!({
            "message": format!("Hello! This is a mock response from {}.{} bridge", service_name, method_name),
            "request_data": json_request,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        Ok(BridgeResponse {
            success: true,
            data: Some(json_response),
            error: None,
            metadata: HashMap::new(),
        })
    }

    /// Handle streaming request (returns SSE stream)
    async fn handle_streaming_request(
        proxy: &MockReflectionProxy,
        converter: &ProtobufJsonConverter,
        service_name: &str,
        method_name: &str,
        json_request: Value,
    ) -> Result<BridgeResponse<Value>, Box<dyn std::error::Error + Send + Sync>> {
        // For now, return an error indicating streaming is not yet implemented via HTTP
        // Full streaming implementation would use Server-Sent Events
        Err("Streaming responses via HTTP bridge are not yet implemented".into())
    }


}

impl Clone for HttpBridge {
    fn clone(&self) -> Self {
        Self {
            proxy: self.proxy.clone(),
            route_generator: self.route_generator.clone(),
            converter: self.converter.clone(),
            config: self.config.clone(),
            stats: self.stats.clone(),
        }
    }
}
