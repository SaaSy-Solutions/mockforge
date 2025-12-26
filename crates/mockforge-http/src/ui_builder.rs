/// UI Builder API for MockForge
///
/// Provides REST endpoints for the low-code UI builder that allows visual
/// creation and editing of mock endpoints without writing code.
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use mockforge_core::config::ServerConfig;
use mockforge_core::import::asyncapi_import::import_asyncapi_spec;
use mockforge_core::import::openapi_import::import_openapi_spec;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::*;

/// Endpoint configuration for UI builder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// Unique identifier for the endpoint
    pub id: String,
    /// Protocol type for this endpoint
    pub protocol: Protocol,
    /// Human-readable endpoint name
    pub name: String,
    /// Optional endpoint description
    pub description: Option<String>,
    /// Whether this endpoint is currently enabled
    pub enabled: bool,
    /// Protocol-specific configuration
    pub config: EndpointProtocolConfig,
}

/// Supported protocols in UI builder
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    /// HTTP/REST protocol
    Http,
    /// gRPC protocol
    Grpc,
    /// WebSocket protocol
    Websocket,
    /// GraphQL protocol
    Graphql,
    /// MQTT protocol
    Mqtt,
    /// SMTP protocol
    Smtp,
    /// Kafka protocol
    Kafka,
    /// AMQP protocol
    Amqp,
    /// FTP protocol
    Ftp,
}

/// Protocol-specific endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EndpointProtocolConfig {
    /// HTTP endpoint configuration
    Http(HttpEndpointConfig),
    /// gRPC endpoint configuration
    Grpc(GrpcEndpointConfig),
    /// WebSocket endpoint configuration
    Websocket(WebsocketEndpointConfig),
}

/// HTTP endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpEndpointConfig {
    /// HTTP method (GET, POST, PUT, etc.)
    pub method: String,
    /// API path pattern
    pub path: String,
    /// Optional request validation and schema configuration
    pub request: Option<HttpRequestConfig>,
    /// Response configuration
    pub response: HttpResponseConfig,
    /// Optional behavior configuration (latency, failure injection, etc.)
    pub behavior: Option<EndpointBehavior>,
}

/// HTTP request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestConfig {
    /// Optional validation settings
    pub validation: Option<ValidationConfig>,
    /// Optional custom request headers
    pub headers: Option<Vec<HeaderConfig>>,
    /// Optional query parameter definitions
    pub query_params: Option<Vec<QueryParamConfig>>,
    /// Optional JSON schema for request body
    pub body_schema: Option<serde_json::Value>,
}

/// HTTP response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponseConfig {
    /// HTTP status code to return
    pub status: u16,
    /// Optional custom response headers
    pub headers: Option<Vec<HeaderConfig>>,
    /// Response body configuration
    pub body: ResponseBody,
}

/// Response body configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseBody {
    /// Static JSON response content
    Static {
        /// JSON value to return
        content: serde_json::Value,
    },
    /// Template-based response with variable expansion
    Template {
        /// Template string with variables (e.g., "{{uuid}}", "{{now}}")
        template: String,
    },
    /// Faker-generated response from schema
    Faker {
        /// JSON schema for data generation
        schema: serde_json::Value,
    },
    /// AI-generated response
    AI {
        /// Prompt for AI generation
        prompt: String,
    },
}

/// Header configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderConfig {
    /// Header name
    pub name: String,
    /// Header value
    pub value: String,
}

/// Query parameter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParamConfig {
    /// Parameter name
    pub name: String,
    /// Whether this parameter is required
    pub required: bool,
    /// Optional JSON schema for validation
    pub schema: Option<serde_json::Value>,
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Validation mode (off, warn, or enforce)
    pub mode: ValidationMode,
    /// Optional JSON schema for validation
    pub schema: Option<serde_json::Value>,
}

/// Validation mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationMode {
    /// Validation disabled
    Off,
    /// Validation enabled but only warns on errors
    Warn,
    /// Validation enabled and rejects invalid requests
    Enforce,
}

/// Endpoint behavior configuration (chaos engineering)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointBehavior {
    /// Optional latency injection configuration
    pub latency: Option<LatencyConfig>,
    /// Optional failure injection configuration
    pub failure: Option<FailureConfig>,
    /// Optional traffic shaping configuration
    pub traffic_shaping: Option<TrafficShapingConfig>,
}

/// Latency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyConfig {
    /// Base latency in milliseconds
    pub base_ms: u64,
    /// Random jitter to add to base latency (milliseconds)
    pub jitter_ms: u64,
    /// Distribution type for latency simulation
    pub distribution: LatencyDistribution,
}

/// Latency distribution type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LatencyDistribution {
    /// Fixed latency (base + uniform jitter)
    Fixed,
    /// Normal/Gaussian distribution
    Normal {
        /// Standard deviation in milliseconds
        std_dev_ms: f64,
    },
    /// Pareto distribution (for realistic network simulation)
    Pareto {
        /// Shape parameter for Pareto distribution
        shape: f64,
    },
}

/// Failure configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureConfig {
    /// Error injection rate (0.0 to 1.0, where 1.0 = 100%)
    pub error_rate: f64,
    /// List of HTTP status codes to randomly return on failure
    pub status_codes: Vec<u16>,
    /// Optional custom error message
    pub error_message: Option<String>,
}

/// Traffic shaping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficShapingConfig {
    /// Optional bandwidth limit in bytes per second
    pub bandwidth_limit_bps: Option<u64>,
    /// Optional packet loss rate (0.0 to 1.0)
    pub packet_loss_rate: Option<f64>,
}

/// gRPC endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcEndpointConfig {
    /// gRPC service name
    pub service: String,
    /// gRPC method name
    pub method: String,
    /// Path to proto file
    pub proto_file: String,
    /// Request message type name
    pub request_type: String,
    /// Response message type name
    pub response_type: String,
    /// Response configuration
    pub response: GrpcResponseConfig,
    /// Optional behavior configuration
    pub behavior: Option<EndpointBehavior>,
}

/// gRPC response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcResponseConfig {
    /// Response body configuration
    pub body: ResponseBody,
    /// Optional metadata (gRPC headers)
    pub metadata: Option<Vec<HeaderConfig>>,
}

/// WebSocket endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketEndpointConfig {
    /// WebSocket connection path
    pub path: String,
    /// Action to perform when connection is established
    pub on_connect: Option<WebsocketAction>,
    /// Action to perform when message is received
    pub on_message: Option<WebsocketAction>,
    /// Action to perform when connection is closed
    pub on_disconnect: Option<WebsocketAction>,
    /// Optional behavior configuration
    pub behavior: Option<EndpointBehavior>,
}

/// WebSocket action type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebsocketAction {
    /// Send a message to the client
    Send {
        /// Message body to send
        message: ResponseBody,
    },
    /// Broadcast a message to all connected clients
    Broadcast {
        /// Message body to broadcast
        message: ResponseBody,
    },
    /// Echo received messages back to sender
    Echo,
    /// Close the connection
    Close {
        /// WebSocket close code
        code: u16,
        /// Close reason message
        reason: String,
    },
}

/// Configuration validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the configuration is valid
    pub valid: bool,
    /// List of validation errors if any
    pub errors: Vec<ValidationError>,
    /// List of validation warnings if any
    pub warnings: Vec<String>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Field name that failed validation
    pub field: String,
    /// Error message describing the issue
    pub message: String,
}

/// UI Builder state
#[derive(Clone)]
pub struct UIBuilderState {
    /// Collection of endpoint configurations
    pub endpoints: Arc<RwLock<Vec<EndpointConfig>>>,
    /// Server configuration
    pub server_config: Arc<RwLock<ServerConfig>>,
}

impl UIBuilderState {
    /// Create a new UI builder state
    ///
    /// # Arguments
    /// * `server_config` - Server configuration for the mock server
    pub fn new(server_config: ServerConfig) -> Self {
        Self {
            endpoints: Arc::new(RwLock::new(Vec::new())),
            server_config: Arc::new(RwLock::new(server_config)),
        }
    }
}

/// List all endpoints
async fn list_endpoints(State(state): State<UIBuilderState>) -> Json<serde_json::Value> {
    let endpoints = state.endpoints.read().await;
    Json(serde_json::json!({
        "endpoints": *endpoints,
        "total": endpoints.len(),
        "enabled": endpoints.iter().filter(|e| e.enabled).count(),
        "by_protocol": {
            "http": endpoints.iter().filter(|e| e.protocol == Protocol::Http).count(),
            "grpc": endpoints.iter().filter(|e| e.protocol == Protocol::Grpc).count(),
            "websocket": endpoints.iter().filter(|e| e.protocol == Protocol::Websocket).count(),
        }
    }))
}

/// Get a specific endpoint by ID
async fn get_endpoint(
    State(state): State<UIBuilderState>,
    Path(id): Path<String>,
) -> Result<Json<EndpointConfig>, StatusCode> {
    let endpoints = state.endpoints.read().await;
    endpoints
        .iter()
        .find(|e| e.id == id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Create a new endpoint
async fn create_endpoint(
    State(state): State<UIBuilderState>,
    Json(mut endpoint): Json<EndpointConfig>,
) -> Result<Json<EndpointConfig>, StatusCode> {
    let mut endpoints = state.endpoints.write().await;

    // Generate ID if not provided
    if endpoint.id.is_empty() {
        endpoint.id = uuid::Uuid::new_v4().to_string();
    }

    // Check for duplicate ID
    if endpoints.iter().any(|e| e.id == endpoint.id) {
        return Err(StatusCode::CONFLICT);
    }

    info!(
        endpoint_id = %endpoint.id,
        protocol = ?endpoint.protocol,
        "Creating new endpoint"
    );

    endpoints.push(endpoint.clone());
    Ok(Json(endpoint))
}

/// Update an existing endpoint
async fn update_endpoint(
    State(state): State<UIBuilderState>,
    Path(id): Path<String>,
    Json(updated): Json<EndpointConfig>,
) -> Result<Json<EndpointConfig>, StatusCode> {
    let mut endpoints = state.endpoints.write().await;

    let endpoint = endpoints.iter_mut().find(|e| e.id == id).ok_or(StatusCode::NOT_FOUND)?;

    info!(
        endpoint_id = %id,
        protocol = ?updated.protocol,
        "Updating endpoint"
    );

    *endpoint = updated.clone();
    Ok(Json(updated))
}

/// Delete an endpoint
async fn delete_endpoint(
    State(state): State<UIBuilderState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let mut endpoints = state.endpoints.write().await;

    let index = endpoints.iter().position(|e| e.id == id).ok_or(StatusCode::NOT_FOUND)?;

    info!(endpoint_id = %id, "Deleting endpoint");

    endpoints.remove(index);
    Ok(StatusCode::NO_CONTENT)
}

/// Validate endpoint configuration
async fn validate_endpoint(
    State(_state): State<UIBuilderState>,
    Json(endpoint): Json<EndpointConfig>,
) -> Json<ValidationResult> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Validate based on protocol
    match &endpoint.config {
        EndpointProtocolConfig::Http(http_config) => {
            // Validate HTTP method
            let valid_methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
            if !valid_methods.contains(&http_config.method.to_uppercase().as_str()) {
                errors.push(ValidationError {
                    field: "method".to_string(),
                    message: format!("Invalid HTTP method: {}", http_config.method),
                });
            }

            // Validate path
            if !http_config.path.starts_with('/') {
                errors.push(ValidationError {
                    field: "path".to_string(),
                    message: "Path must start with /".to_string(),
                });
            }

            // Validate status code
            if !(100..600).contains(&http_config.response.status) {
                errors.push(ValidationError {
                    field: "status".to_string(),
                    message: "Status code must be between 100 and 599".to_string(),
                });
            }
        }
        EndpointProtocolConfig::Grpc(grpc_config) => {
            // Validate gRPC service and method names
            if grpc_config.service.is_empty() {
                errors.push(ValidationError {
                    field: "service".to_string(),
                    message: "Service name is required".to_string(),
                });
            }
            if grpc_config.method.is_empty() {
                errors.push(ValidationError {
                    field: "method".to_string(),
                    message: "Method name is required".to_string(),
                });
            }
        }
        EndpointProtocolConfig::Websocket(ws_config) => {
            // Validate WebSocket path
            if !ws_config.path.starts_with('/') {
                errors.push(ValidationError {
                    field: "path".to_string(),
                    message: "Path must start with /".to_string(),
                });
            }
        }
    }

    // Check for behavior configuration warnings
    if let Some(EndpointProtocolConfig::Http(http_config)) = Some(&endpoint.config) {
        if let Some(behavior) = &http_config.behavior {
            if let Some(failure) = &behavior.failure {
                if failure.error_rate > 0.5 {
                    warnings.push("High error rate configured (>50%)".to_string());
                }
            }
        }
    }

    Json(ValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
    })
}

/// Export configuration as YAML
async fn export_config(
    State(state): State<UIBuilderState>,
) -> Result<impl IntoResponse, StatusCode> {
    let server_config = state.server_config.read().await;

    match serde_yaml::to_string(&*server_config) {
        Ok(yaml) => Ok((StatusCode::OK, [("Content-Type", "application/x-yaml")], yaml)),
        Err(e) => {
            error!(error = %e, "Failed to serialize config to YAML");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Import configuration from YAML/JSON
#[derive(Debug, Deserialize)]
struct ImportRequest {
    config: String,
    format: ConfigFormat,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ConfigFormat {
    Yaml,
    Json,
}

async fn import_config(
    State(state): State<UIBuilderState>,
    Json(request): Json<ImportRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let config: ServerConfig = match request.format {
        ConfigFormat::Yaml => serde_yaml::from_str(&request.config).map_err(|e| {
            error!(error = %e, "Failed to parse YAML config");
            StatusCode::BAD_REQUEST
        })?,
        ConfigFormat::Json => serde_json::from_str(&request.config).map_err(|e| {
            error!(error = %e, "Failed to parse JSON config");
            StatusCode::BAD_REQUEST
        })?,
    };

    let mut server_config = state.server_config.write().await;
    *server_config = config;

    info!("Configuration imported successfully");

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Configuration imported successfully"
    })))
}

/// Get current server configuration
async fn get_config(State(state): State<UIBuilderState>) -> Json<ServerConfig> {
    let config = state.server_config.read().await;
    Json(config.clone())
}

/// Update server configuration
async fn update_config(
    State(state): State<UIBuilderState>,
    Json(new_config): Json<ServerConfig>,
) -> Result<Json<ServerConfig>, StatusCode> {
    let mut config = state.server_config.write().await;
    *config = new_config.clone();

    info!("Server configuration updated");

    Ok(Json(new_config))
}

/// Import OpenAPI specification request
#[derive(Debug, Deserialize)]
struct ImportOpenApiRequest {
    content: String,
    base_url: Option<String>,
    auto_enable: Option<bool>,
}

/// Import OpenAPI specification response
#[derive(Debug, Serialize)]
struct ImportOpenApiResponse {
    success: bool,
    endpoints_created: usize,
    warnings: Vec<String>,
    spec_info: OpenApiSpecInfoResponse,
}

#[derive(Debug, Serialize)]
struct OpenApiSpecInfoResponse {
    title: String,
    version: String,
    description: Option<String>,
    openapi_version: String,
    servers: Vec<String>,
}

/// Import OpenAPI/Swagger specification and auto-generate endpoints
async fn import_openapi_spec_handler(
    State(state): State<UIBuilderState>,
    Json(request): Json<ImportOpenApiRequest>,
) -> Result<Json<ImportOpenApiResponse>, (StatusCode, Json<serde_json::Value>)> {
    info!("Importing OpenAPI specification");

    // Import the OpenAPI spec
    let import_result = import_openapi_spec(&request.content, request.base_url.as_deref())
        .map_err(|e| {
            error!(error = %e, "Failed to import OpenAPI spec");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Failed to import OpenAPI specification",
                    "details": e
                })),
            )
        })?;

    let auto_enable = request.auto_enable.unwrap_or(true);
    let mut endpoints = state.endpoints.write().await;
    let mut created_count = 0;

    // Convert imported routes to EndpointConfig
    for route in import_result.routes {
        let endpoint_id = uuid::Uuid::new_v4().to_string();

        // Convert response body to ResponseBody enum
        let response_body = ResponseBody::Static {
            content: route.response.body,
        };

        // Convert headers to HeaderConfig
        let response_headers: Option<Vec<HeaderConfig>> = if route.response.headers.is_empty() {
            None
        } else {
            Some(
                route
                    .response
                    .headers
                    .into_iter()
                    .map(|(name, value)| HeaderConfig { name, value })
                    .collect(),
            )
        };

        let endpoint = EndpointConfig {
            id: endpoint_id.clone(),
            protocol: Protocol::Http,
            name: format!("{} {}", route.method.to_uppercase(), route.path),
            description: Some(format!(
                "Auto-generated from OpenAPI spec: {} v{}",
                import_result.spec_info.title, import_result.spec_info.version
            )),
            enabled: auto_enable,
            config: EndpointProtocolConfig::Http(HttpEndpointConfig {
                method: route.method.to_uppercase(),
                path: route.path,
                request: None,
                response: HttpResponseConfig {
                    status: route.response.status,
                    headers: response_headers,
                    body: response_body,
                },
                behavior: None,
            }),
        };

        info!(
            endpoint_id = %endpoint_id,
            method = %endpoint.name,
            "Created endpoint from OpenAPI spec"
        );

        endpoints.push(endpoint);
        created_count += 1;
    }

    Ok(Json(ImportOpenApiResponse {
        success: true,
        endpoints_created: created_count,
        warnings: import_result.warnings,
        spec_info: OpenApiSpecInfoResponse {
            title: import_result.spec_info.title,
            version: import_result.spec_info.version,
            description: import_result.spec_info.description,
            openapi_version: import_result.spec_info.openapi_version,
            servers: import_result.spec_info.servers,
        },
    }))
}

/// Export endpoints as OpenAPI specification
async fn export_openapi_spec_handler(
    State(state): State<UIBuilderState>,
) -> Result<impl IntoResponse, StatusCode> {
    let endpoints = state.endpoints.read().await;
    let server_config = state.server_config.read().await;

    // Build OpenAPI spec from endpoints
    let http_host = &server_config.http.host;
    let http_port = server_config.http.port;

    let mut spec = serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "MockForge Generated API",
            "version": "1.0.0",
            "description": "API specification generated from MockForge endpoints"
        },
        "servers": [
            {
                "url": format!("http://{}:{}", http_host, http_port)
            }
        ],
        "paths": {}
    });

    let paths = spec["paths"].as_object_mut().expect("paths must be an object");

    // Group endpoints by path
    for endpoint in endpoints.iter() {
        if endpoint.protocol != Protocol::Http {
            continue; // Only export HTTP endpoints for now
        }

        if let EndpointProtocolConfig::Http(http_config) = &endpoint.config {
            let path = &http_config.path;

            if !paths.contains_key(path) {
                paths.insert(path.clone(), serde_json::json!({}));
            }

            let method = http_config.method.to_lowercase();

            // Build response body
            let response_body_content = match &http_config.response.body {
                ResponseBody::Static { content } => content.clone(),
                ResponseBody::Template { template } => serde_json::json!({
                    "type": "string",
                    "example": template
                }),
                ResponseBody::Faker { schema } => schema.clone(),
                ResponseBody::AI { prompt } => serde_json::json!({
                    "type": "string",
                    "description": prompt
                }),
            };

            let operation = serde_json::json!({
                "summary": &endpoint.name,
                "description": endpoint.description.as_ref().unwrap_or(&String::new()),
                "operationId": &endpoint.id,
                "responses": {
                    http_config.response.status.to_string(): {
                        "description": format!("Response with status {}", http_config.response.status),
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object"
                                },
                                "example": response_body_content
                            }
                        }
                    }
                }
            });

            paths[path][&method] = operation;
        }
    }

    match serde_json::to_string_pretty(&spec) {
        Ok(json) => Ok((StatusCode::OK, [("Content-Type", "application/json")], json)),
        Err(e) => {
            error!(error = %e, "Failed to serialize OpenAPI spec to JSON");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Import AsyncAPI specification request
#[derive(Debug, Deserialize)]
struct ImportAsyncApiRequest {
    content: String,
    base_url: Option<String>,
    auto_enable: Option<bool>,
}

/// Import AsyncAPI specification response
#[derive(Debug, Serialize)]
struct ImportAsyncApiResponse {
    success: bool,
    endpoints_created: usize,
    warnings: Vec<String>,
    spec_info: AsyncApiSpecInfoResponse,
}

#[derive(Debug, Serialize)]
struct AsyncApiSpecInfoResponse {
    title: String,
    version: String,
    description: Option<String>,
    asyncapi_version: String,
    servers: Vec<String>,
}

/// Import AsyncAPI specification and auto-generate WebSocket/MQTT/Kafka endpoints
async fn import_asyncapi_spec_handler(
    State(state): State<UIBuilderState>,
    Json(request): Json<ImportAsyncApiRequest>,
) -> Result<Json<ImportAsyncApiResponse>, (StatusCode, Json<serde_json::Value>)> {
    info!("Importing AsyncAPI specification");

    // Import the AsyncAPI spec
    let import_result = import_asyncapi_spec(&request.content, request.base_url.as_deref())
        .map_err(|e| {
            error!(error = %e, "Failed to import AsyncAPI spec");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Failed to import AsyncAPI specification",
                    "details": e
                })),
            )
        })?;

    let auto_enable = request.auto_enable.unwrap_or(true);
    let mut endpoints = state.endpoints.write().await;
    let mut created_count = 0;

    // Convert imported channels to EndpointConfig
    for channel in import_result.channels {
        let endpoint_id = uuid::Uuid::new_v4().to_string();

        // Map AsyncAPI protocol to MockForge protocol
        let (protocol, config) = match channel.protocol {
            mockforge_core::import::asyncapi_import::ChannelProtocol::Websocket => {
                // Create WebSocket endpoint
                let on_message = if let Some(op) = channel.operations.first() {
                    if let Some(example) = &op.example_message {
                        Some(WebsocketAction::Send {
                            message: ResponseBody::Static {
                                content: example.clone(),
                            },
                        })
                    } else {
                        Some(WebsocketAction::Echo)
                    }
                } else {
                    Some(WebsocketAction::Echo)
                };

                (
                    Protocol::Websocket,
                    EndpointProtocolConfig::Websocket(WebsocketEndpointConfig {
                        path: channel.path.clone(),
                        on_connect: None,
                        on_message,
                        on_disconnect: None,
                        behavior: None,
                    }),
                )
            }
            _ => {
                // For MQTT/Kafka/AMQP, we'll skip for now as they need special handling
                // Log a warning and continue
                warn!(
                    "Skipping channel '{}' with protocol {:?} - not yet supported for UI Builder",
                    channel.name, channel.protocol
                );
                continue;
            }
        };

        let endpoint = EndpointConfig {
            id: endpoint_id.clone(),
            protocol,
            name: format!("{} - {}", channel.name, channel.path),
            description: channel.description.or_else(|| {
                Some(format!(
                    "Auto-generated from AsyncAPI spec: {} v{}",
                    import_result.spec_info.title, import_result.spec_info.version
                ))
            }),
            enabled: auto_enable,
            config,
        };

        info!(
            endpoint_id = %endpoint_id,
            name = %endpoint.name,
            "Created endpoint from AsyncAPI spec"
        );

        endpoints.push(endpoint);
        created_count += 1;
    }

    Ok(Json(ImportAsyncApiResponse {
        success: true,
        endpoints_created: created_count,
        warnings: import_result.warnings,
        spec_info: AsyncApiSpecInfoResponse {
            title: import_result.spec_info.title,
            version: import_result.spec_info.version,
            description: import_result.spec_info.description,
            asyncapi_version: import_result.spec_info.asyncapi_version,
            servers: import_result.spec_info.servers,
        },
    }))
}

/// Resolve tokens in a ResponseBody
pub async fn resolve_response_body_tokens(
    body: &ResponseBody,
) -> Result<serde_json::Value, String> {
    use crate::token_response::resolve_response_tokens;

    match body {
        ResponseBody::Static { content } => resolve_response_tokens(content.clone()).await,
        ResponseBody::Template { template } => {
            // Templates can also contain tokens, parse and resolve
            let value = serde_json::Value::String(template.clone());
            resolve_response_tokens(value).await
        }
        ResponseBody::Faker { schema } => {
            // Faker schemas may contain tokens
            resolve_response_tokens(schema.clone()).await
        }
        ResponseBody::AI { prompt: _ } => {
            // AI prompts are handled separately by the AI handler
            // Return as-is since AI generation happens at response time
            Ok(serde_json::json!({"_ai_prompt": true}))
        }
    }
}

/// Create the UI Builder router
pub fn create_ui_builder_router(state: UIBuilderState) -> Router {
    Router::new()
        .route("/endpoints", get(list_endpoints).post(create_endpoint))
        .route(
            "/endpoints/{id}",
            get(get_endpoint).put(update_endpoint).delete(delete_endpoint),
        )
        .route("/endpoints/validate", post(validate_endpoint))
        .route("/config", get(get_config).put(update_config))
        .route("/config/export", get(export_config))
        .route("/config/import", post(import_config))
        .route("/openapi/import", post(import_openapi_spec_handler))
        .route("/openapi/export", get(export_openapi_spec_handler))
        .route("/asyncapi/import", post(import_asyncapi_spec_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Helper Functions ====================

    fn create_test_http_endpoint() -> EndpointConfig {
        EndpointConfig {
            id: "test-1".to_string(),
            protocol: Protocol::Http,
            name: "Test Endpoint".to_string(),
            description: Some("A test endpoint".to_string()),
            enabled: true,
            config: EndpointProtocolConfig::Http(HttpEndpointConfig {
                method: "GET".to_string(),
                path: "/test".to_string(),
                request: None,
                response: HttpResponseConfig {
                    status: 200,
                    headers: None,
                    body: ResponseBody::Static {
                        content: serde_json::json!({"message": "Hello"}),
                    },
                },
                behavior: None,
            }),
        }
    }

    // ==================== Protocol Tests ====================

    #[test]
    fn test_protocol_http_serialization() {
        let protocol = Protocol::Http;
        let json = serde_json::to_string(&protocol).unwrap();
        assert_eq!(json, "\"http\"");
    }

    #[test]
    fn test_protocol_grpc_serialization() {
        let protocol = Protocol::Grpc;
        let json = serde_json::to_string(&protocol).unwrap();
        assert_eq!(json, "\"grpc\"");
    }

    #[test]
    fn test_protocol_websocket_serialization() {
        let protocol = Protocol::Websocket;
        let json = serde_json::to_string(&protocol).unwrap();
        assert_eq!(json, "\"websocket\"");
    }

    #[test]
    fn test_protocol_graphql_serialization() {
        let protocol = Protocol::Graphql;
        let json = serde_json::to_string(&protocol).unwrap();
        assert_eq!(json, "\"graphql\"");
    }

    #[test]
    fn test_protocol_mqtt_serialization() {
        let protocol = Protocol::Mqtt;
        let json = serde_json::to_string(&protocol).unwrap();
        assert_eq!(json, "\"mqtt\"");
    }

    #[test]
    fn test_protocol_smtp_serialization() {
        let protocol = Protocol::Smtp;
        let json = serde_json::to_string(&protocol).unwrap();
        assert_eq!(json, "\"smtp\"");
    }

    #[test]
    fn test_protocol_kafka_serialization() {
        let protocol = Protocol::Kafka;
        let json = serde_json::to_string(&protocol).unwrap();
        assert_eq!(json, "\"kafka\"");
    }

    #[test]
    fn test_protocol_amqp_serialization() {
        let protocol = Protocol::Amqp;
        let json = serde_json::to_string(&protocol).unwrap();
        assert_eq!(json, "\"amqp\"");
    }

    #[test]
    fn test_protocol_ftp_serialization() {
        let protocol = Protocol::Ftp;
        let json = serde_json::to_string(&protocol).unwrap();
        assert_eq!(json, "\"ftp\"");
    }

    #[test]
    fn test_protocol_deserialization() {
        let json = "\"http\"";
        let protocol: Protocol = serde_json::from_str(json).unwrap();
        assert_eq!(protocol, Protocol::Http);
    }

    #[test]
    fn test_protocol_equality() {
        assert_eq!(Protocol::Http, Protocol::Http);
        assert_ne!(Protocol::Http, Protocol::Grpc);
    }

    #[test]
    fn test_protocol_clone() {
        let protocol = Protocol::Websocket;
        let cloned = protocol.clone();
        assert_eq!(protocol, cloned);
    }

    // ==================== ValidationMode Tests ====================

    #[test]
    fn test_validation_mode_off_serialization() {
        let mode = ValidationMode::Off;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"off\"");
    }

    #[test]
    fn test_validation_mode_warn_serialization() {
        let mode = ValidationMode::Warn;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"warn\"");
    }

    #[test]
    fn test_validation_mode_enforce_serialization() {
        let mode = ValidationMode::Enforce;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"enforce\"");
    }

    #[test]
    fn test_validation_mode_deserialization() {
        let json = "\"enforce\"";
        let mode: ValidationMode = serde_json::from_str(json).unwrap();
        assert!(matches!(mode, ValidationMode::Enforce));
    }

    // ==================== HeaderConfig Tests ====================

    #[test]
    fn test_header_config_creation() {
        let header = HeaderConfig {
            name: "Content-Type".to_string(),
            value: "application/json".to_string(),
        };
        assert_eq!(header.name, "Content-Type");
        assert_eq!(header.value, "application/json");
    }

    #[test]
    fn test_header_config_serialization() {
        let header = HeaderConfig {
            name: "X-Custom-Header".to_string(),
            value: "custom-value".to_string(),
        };
        let json = serde_json::to_string(&header).unwrap();
        let deserialized: HeaderConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(header.name, deserialized.name);
        assert_eq!(header.value, deserialized.value);
    }

    #[test]
    fn test_header_config_clone() {
        let header = HeaderConfig {
            name: "Authorization".to_string(),
            value: "Bearer token".to_string(),
        };
        let cloned = header.clone();
        assert_eq!(header.name, cloned.name);
        assert_eq!(header.value, cloned.value);
    }

    // ==================== QueryParamConfig Tests ====================

    #[test]
    fn test_query_param_config_required() {
        let param = QueryParamConfig {
            name: "page".to_string(),
            required: true,
            schema: Some(serde_json::json!({"type": "integer"})),
        };
        assert!(param.required);
        assert!(param.schema.is_some());
    }

    #[test]
    fn test_query_param_config_optional() {
        let param = QueryParamConfig {
            name: "filter".to_string(),
            required: false,
            schema: None,
        };
        assert!(!param.required);
        assert!(param.schema.is_none());
    }

    #[test]
    fn test_query_param_config_serialization() {
        let param = QueryParamConfig {
            name: "limit".to_string(),
            required: true,
            schema: Some(serde_json::json!({"type": "integer", "maximum": 100})),
        };
        let json = serde_json::to_string(&param).unwrap();
        let deserialized: QueryParamConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(param.name, deserialized.name);
        assert_eq!(param.required, deserialized.required);
    }

    // ==================== ValidationConfig Tests ====================

    #[test]
    fn test_validation_config_with_schema() {
        let config = ValidationConfig {
            mode: ValidationMode::Enforce,
            schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                }
            })),
        };
        assert!(matches!(config.mode, ValidationMode::Enforce));
        assert!(config.schema.is_some());
    }

    #[test]
    fn test_validation_config_without_schema() {
        let config = ValidationConfig {
            mode: ValidationMode::Off,
            schema: None,
        };
        assert!(matches!(config.mode, ValidationMode::Off));
        assert!(config.schema.is_none());
    }

    // ==================== LatencyConfig Tests ====================

    #[test]
    fn test_latency_config_fixed() {
        let config = LatencyConfig {
            base_ms: 100,
            jitter_ms: 20,
            distribution: LatencyDistribution::Fixed,
        };
        assert_eq!(config.base_ms, 100);
        assert_eq!(config.jitter_ms, 20);
    }

    #[test]
    fn test_latency_config_normal_distribution() {
        let config = LatencyConfig {
            base_ms: 50,
            jitter_ms: 10,
            distribution: LatencyDistribution::Normal { std_dev_ms: 15.0 },
        };
        assert!(matches!(config.distribution, LatencyDistribution::Normal { .. }));
    }

    #[test]
    fn test_latency_config_pareto_distribution() {
        let config = LatencyConfig {
            base_ms: 75,
            jitter_ms: 25,
            distribution: LatencyDistribution::Pareto { shape: 1.5 },
        };
        assert!(matches!(config.distribution, LatencyDistribution::Pareto { .. }));
    }

    #[test]
    fn test_latency_config_serialization() {
        let config = LatencyConfig {
            base_ms: 200,
            jitter_ms: 50,
            distribution: LatencyDistribution::Fixed,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: LatencyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.base_ms, deserialized.base_ms);
        assert_eq!(config.jitter_ms, deserialized.jitter_ms);
    }

    // ==================== FailureConfig Tests ====================

    #[test]
    fn test_failure_config_creation() {
        let config = FailureConfig {
            error_rate: 0.1,
            status_codes: vec![500, 502, 503],
            error_message: Some("Server error".to_string()),
        };
        assert!((config.error_rate - 0.1).abs() < 0.001);
        assert_eq!(config.status_codes.len(), 3);
        assert!(config.error_message.is_some());
    }

    #[test]
    fn test_failure_config_high_error_rate() {
        let config = FailureConfig {
            error_rate: 0.75,
            status_codes: vec![500],
            error_message: None,
        };
        assert!(config.error_rate > 0.5);
    }

    #[test]
    fn test_failure_config_serialization() {
        let config = FailureConfig {
            error_rate: 0.25,
            status_codes: vec![429, 500],
            error_message: Some("Rate limited".to_string()),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: FailureConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.status_codes, deserialized.status_codes);
    }

    // ==================== TrafficShapingConfig Tests ====================

    #[test]
    fn test_traffic_shaping_config_bandwidth_limit() {
        let config = TrafficShapingConfig {
            bandwidth_limit_bps: Some(1024 * 1024), // 1 MB/s
            packet_loss_rate: None,
        };
        assert_eq!(config.bandwidth_limit_bps, Some(1024 * 1024));
    }

    #[test]
    fn test_traffic_shaping_config_packet_loss() {
        let config = TrafficShapingConfig {
            bandwidth_limit_bps: None,
            packet_loss_rate: Some(0.05),
        };
        assert!(config.packet_loss_rate.is_some());
    }

    #[test]
    fn test_traffic_shaping_config_both_enabled() {
        let config = TrafficShapingConfig {
            bandwidth_limit_bps: Some(500_000),
            packet_loss_rate: Some(0.02),
        };
        assert!(config.bandwidth_limit_bps.is_some());
        assert!(config.packet_loss_rate.is_some());
    }

    // ==================== EndpointBehavior Tests ====================

    #[test]
    fn test_endpoint_behavior_with_latency_only() {
        let behavior = EndpointBehavior {
            latency: Some(LatencyConfig {
                base_ms: 100,
                jitter_ms: 10,
                distribution: LatencyDistribution::Fixed,
            }),
            failure: None,
            traffic_shaping: None,
        };
        assert!(behavior.latency.is_some());
        assert!(behavior.failure.is_none());
    }

    #[test]
    fn test_endpoint_behavior_with_failure_only() {
        let behavior = EndpointBehavior {
            latency: None,
            failure: Some(FailureConfig {
                error_rate: 0.1,
                status_codes: vec![500],
                error_message: None,
            }),
            traffic_shaping: None,
        };
        assert!(behavior.failure.is_some());
    }

    #[test]
    fn test_endpoint_behavior_full_config() {
        let behavior = EndpointBehavior {
            latency: Some(LatencyConfig {
                base_ms: 50,
                jitter_ms: 10,
                distribution: LatencyDistribution::Fixed,
            }),
            failure: Some(FailureConfig {
                error_rate: 0.05,
                status_codes: vec![503],
                error_message: None,
            }),
            traffic_shaping: Some(TrafficShapingConfig {
                bandwidth_limit_bps: Some(100_000),
                packet_loss_rate: Some(0.01),
            }),
        };
        assert!(behavior.latency.is_some());
        assert!(behavior.failure.is_some());
        assert!(behavior.traffic_shaping.is_some());
    }

    // ==================== ResponseBody Tests ====================

    #[test]
    fn test_response_body_static() {
        let body = ResponseBody::Static {
            content: serde_json::json!({"message": "Hello"}),
        };
        if let ResponseBody::Static { content } = body {
            assert_eq!(content["message"], "Hello");
        } else {
            panic!("Expected Static response body");
        }
    }

    #[test]
    fn test_response_body_template() {
        let body = ResponseBody::Template {
            template: "Hello, {{name}}!".to_string(),
        };
        if let ResponseBody::Template { template } = body {
            assert!(template.contains("{{name}}"));
        } else {
            panic!("Expected Template response body");
        }
    }

    #[test]
    fn test_response_body_faker() {
        let body = ResponseBody::Faker {
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {"type": "integer"},
                    "name": {"type": "string"}
                }
            }),
        };
        if let ResponseBody::Faker { schema } = body {
            assert_eq!(schema["type"], "object");
        } else {
            panic!("Expected Faker response body");
        }
    }

    #[test]
    fn test_response_body_ai() {
        let body = ResponseBody::AI {
            prompt: "Generate a user profile".to_string(),
        };
        if let ResponseBody::AI { prompt } = body {
            assert!(prompt.contains("user profile"));
        } else {
            panic!("Expected AI response body");
        }
    }

    // ==================== HttpRequestConfig Tests ====================

    #[test]
    fn test_http_request_config_with_validation() {
        let config = HttpRequestConfig {
            validation: Some(ValidationConfig {
                mode: ValidationMode::Enforce,
                schema: Some(serde_json::json!({"type": "object"})),
            }),
            headers: None,
            query_params: None,
            body_schema: None,
        };
        assert!(config.validation.is_some());
    }

    #[test]
    fn test_http_request_config_with_headers() {
        let config = HttpRequestConfig {
            validation: None,
            headers: Some(vec![HeaderConfig {
                name: "Authorization".to_string(),
                value: "Bearer token".to_string(),
            }]),
            query_params: None,
            body_schema: None,
        };
        assert_eq!(config.headers.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_http_request_config_with_query_params() {
        let config = HttpRequestConfig {
            validation: None,
            headers: None,
            query_params: Some(vec![
                QueryParamConfig {
                    name: "page".to_string(),
                    required: true,
                    schema: None,
                },
                QueryParamConfig {
                    name: "limit".to_string(),
                    required: false,
                    schema: None,
                },
            ]),
            body_schema: None,
        };
        assert_eq!(config.query_params.as_ref().unwrap().len(), 2);
    }

    // ==================== HttpResponseConfig Tests ====================

    #[test]
    fn test_http_response_config_ok() {
        let config = HttpResponseConfig {
            status: 200,
            headers: None,
            body: ResponseBody::Static {
                content: serde_json::json!({"success": true}),
            },
        };
        assert_eq!(config.status, 200);
    }

    #[test]
    fn test_http_response_config_not_found() {
        let config = HttpResponseConfig {
            status: 404,
            headers: Some(vec![HeaderConfig {
                name: "X-Error-Code".to_string(),
                value: "NOT_FOUND".to_string(),
            }]),
            body: ResponseBody::Static {
                content: serde_json::json!({"error": "Not found"}),
            },
        };
        assert_eq!(config.status, 404);
        assert!(config.headers.is_some());
    }

    #[test]
    fn test_http_response_config_server_error() {
        let config = HttpResponseConfig {
            status: 500,
            headers: None,
            body: ResponseBody::Static {
                content: serde_json::json!({"error": "Internal server error"}),
            },
        };
        assert_eq!(config.status, 500);
    }

    // ==================== HttpEndpointConfig Tests ====================

    #[test]
    fn test_http_endpoint_config_get() {
        let config = HttpEndpointConfig {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            request: None,
            response: HttpResponseConfig {
                status: 200,
                headers: None,
                body: ResponseBody::Static {
                    content: serde_json::json!([]),
                },
            },
            behavior: None,
        };
        assert_eq!(config.method, "GET");
        assert!(config.path.starts_with('/'));
    }

    #[test]
    fn test_http_endpoint_config_post_with_request() {
        let config = HttpEndpointConfig {
            method: "POST".to_string(),
            path: "/api/users".to_string(),
            request: Some(HttpRequestConfig {
                validation: Some(ValidationConfig {
                    mode: ValidationMode::Enforce,
                    schema: Some(serde_json::json!({
                        "type": "object",
                        "required": ["name", "email"]
                    })),
                }),
                headers: None,
                query_params: None,
                body_schema: None,
            }),
            response: HttpResponseConfig {
                status: 201,
                headers: None,
                body: ResponseBody::Static {
                    content: serde_json::json!({"id": 1}),
                },
            },
            behavior: None,
        };
        assert_eq!(config.method, "POST");
        assert!(config.request.is_some());
    }

    // ==================== GrpcEndpointConfig Tests ====================

    #[test]
    fn test_grpc_endpoint_config_creation() {
        let config = GrpcEndpointConfig {
            service: "users.UserService".to_string(),
            method: "GetUser".to_string(),
            proto_file: "/path/to/user.proto".to_string(),
            request_type: "GetUserRequest".to_string(),
            response_type: "GetUserResponse".to_string(),
            response: GrpcResponseConfig {
                body: ResponseBody::Static {
                    content: serde_json::json!({"id": 1, "name": "John"}),
                },
                metadata: None,
            },
            behavior: None,
        };
        assert_eq!(config.service, "users.UserService");
        assert_eq!(config.method, "GetUser");
    }

    #[test]
    fn test_grpc_endpoint_config_with_metadata() {
        let config = GrpcEndpointConfig {
            service: "example.ExampleService".to_string(),
            method: "DoSomething".to_string(),
            proto_file: "/path/to/example.proto".to_string(),
            request_type: "Request".to_string(),
            response_type: "Response".to_string(),
            response: GrpcResponseConfig {
                body: ResponseBody::Static {
                    content: serde_json::json!({}),
                },
                metadata: Some(vec![HeaderConfig {
                    name: "x-request-id".to_string(),
                    value: "12345".to_string(),
                }]),
            },
            behavior: None,
        };
        assert!(config.response.metadata.is_some());
    }

    // ==================== WebsocketEndpointConfig Tests ====================

    #[test]
    fn test_websocket_endpoint_config_basic() {
        let config = WebsocketEndpointConfig {
            path: "/ws".to_string(),
            on_connect: None,
            on_message: Some(WebsocketAction::Echo),
            on_disconnect: None,
            behavior: None,
        };
        assert_eq!(config.path, "/ws");
    }

    #[test]
    fn test_websocket_endpoint_config_with_send() {
        let config = WebsocketEndpointConfig {
            path: "/notifications".to_string(),
            on_connect: Some(WebsocketAction::Send {
                message: ResponseBody::Static {
                    content: serde_json::json!({"type": "connected"}),
                },
            }),
            on_message: None,
            on_disconnect: None,
            behavior: None,
        };
        assert!(config.on_connect.is_some());
    }

    #[test]
    fn test_websocket_endpoint_config_with_broadcast() {
        let config = WebsocketEndpointConfig {
            path: "/chat".to_string(),
            on_connect: None,
            on_message: Some(WebsocketAction::Broadcast {
                message: ResponseBody::Template {
                    template: "{{message}}".to_string(),
                },
            }),
            on_disconnect: None,
            behavior: None,
        };
        if let Some(WebsocketAction::Broadcast { .. }) = config.on_message {
            // Test passes
        } else {
            panic!("Expected Broadcast action");
        }
    }

    #[test]
    fn test_websocket_endpoint_config_with_close() {
        let config = WebsocketEndpointConfig {
            path: "/stream".to_string(),
            on_connect: None,
            on_message: None,
            on_disconnect: Some(WebsocketAction::Close {
                code: 1000,
                reason: "Normal closure".to_string(),
            }),
            behavior: None,
        };
        if let Some(WebsocketAction::Close { code, reason }) = config.on_disconnect {
            assert_eq!(code, 1000);
            assert_eq!(reason, "Normal closure");
        } else {
            panic!("Expected Close action");
        }
    }

    // ==================== ValidationResult Tests ====================

    #[test]
    fn test_validation_result_valid() {
        let result = ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        };
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validation_result_with_errors() {
        let result = ValidationResult {
            valid: false,
            errors: vec![ValidationError {
                field: "method".to_string(),
                message: "Invalid HTTP method".to_string(),
            }],
            warnings: vec![],
        };
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_validation_result_with_warnings() {
        let result = ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec!["High error rate configured".to_string()],
        };
        assert!(result.valid);
        assert_eq!(result.warnings.len(), 1);
    }

    // ==================== ValidationError Tests ====================

    #[test]
    fn test_validation_error_creation() {
        let error = ValidationError {
            field: "path".to_string(),
            message: "Path must start with /".to_string(),
        };
        assert_eq!(error.field, "path");
        assert!(error.message.contains('/'));
    }

    #[test]
    fn test_validation_error_clone() {
        let error = ValidationError {
            field: "status".to_string(),
            message: "Invalid status code".to_string(),
        };
        let cloned = error.clone();
        assert_eq!(error.field, cloned.field);
        assert_eq!(error.message, cloned.message);
    }

    // ==================== UIBuilderState Tests ====================

    #[test]
    fn test_ui_builder_state_creation() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);
        // State should be created without panic
        let _ = state;
    }

    #[test]
    fn test_ui_builder_state_clone() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);
        let cloned = state.clone();
        // Both should exist without panic
        let _ = (state, cloned);
    }

    // ==================== EndpointConfig Tests ====================

    #[test]
    fn test_endpoint_serialization() {
        let endpoint = EndpointConfig {
            id: "test-1".to_string(),
            protocol: Protocol::Http,
            name: "Test Endpoint".to_string(),
            description: Some("A test endpoint".to_string()),
            enabled: true,
            config: EndpointProtocolConfig::Http(HttpEndpointConfig {
                method: "GET".to_string(),
                path: "/test".to_string(),
                request: None,
                response: HttpResponseConfig {
                    status: 200,
                    headers: None,
                    body: ResponseBody::Static {
                        content: serde_json::json!({"message": "Hello"}),
                    },
                },
                behavior: None,
            }),
        };

        let json = serde_json::to_string(&endpoint).unwrap();
        let deserialized: EndpointConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(endpoint.id, deserialized.id);
        assert_eq!(endpoint.protocol, deserialized.protocol);
    }

    #[test]
    fn test_endpoint_config_disabled() {
        let endpoint = EndpointConfig {
            id: "disabled-1".to_string(),
            protocol: Protocol::Http,
            name: "Disabled Endpoint".to_string(),
            description: None,
            enabled: false,
            config: EndpointProtocolConfig::Http(HttpEndpointConfig {
                method: "GET".to_string(),
                path: "/disabled".to_string(),
                request: None,
                response: HttpResponseConfig {
                    status: 200,
                    headers: None,
                    body: ResponseBody::Static {
                        content: serde_json::json!({}),
                    },
                },
                behavior: None,
            }),
        };
        assert!(!endpoint.enabled);
        assert!(endpoint.description.is_none());
    }

    #[test]
    fn test_endpoint_config_grpc() {
        let endpoint = EndpointConfig {
            id: "grpc-1".to_string(),
            protocol: Protocol::Grpc,
            name: "gRPC Endpoint".to_string(),
            description: Some("A gRPC endpoint".to_string()),
            enabled: true,
            config: EndpointProtocolConfig::Grpc(GrpcEndpointConfig {
                service: "test.Service".to_string(),
                method: "Call".to_string(),
                proto_file: "/test.proto".to_string(),
                request_type: "Request".to_string(),
                response_type: "Response".to_string(),
                response: GrpcResponseConfig {
                    body: ResponseBody::Static {
                        content: serde_json::json!({}),
                    },
                    metadata: None,
                },
                behavior: None,
            }),
        };
        assert_eq!(endpoint.protocol, Protocol::Grpc);
    }

    #[test]
    fn test_endpoint_config_websocket() {
        let endpoint = EndpointConfig {
            id: "ws-1".to_string(),
            protocol: Protocol::Websocket,
            name: "WebSocket Endpoint".to_string(),
            description: None,
            enabled: true,
            config: EndpointProtocolConfig::Websocket(WebsocketEndpointConfig {
                path: "/ws".to_string(),
                on_connect: None,
                on_message: Some(WebsocketAction::Echo),
                on_disconnect: None,
                behavior: None,
            }),
        };
        assert_eq!(endpoint.protocol, Protocol::Websocket);
    }

    #[test]
    fn test_validation() {
        // Test invalid HTTP method
        let endpoint = EndpointConfig {
            id: "test-1".to_string(),
            protocol: Protocol::Http,
            name: "Test".to_string(),
            description: None,
            enabled: true,
            config: EndpointProtocolConfig::Http(HttpEndpointConfig {
                method: "INVALID".to_string(),
                path: "/test".to_string(),
                request: None,
                response: HttpResponseConfig {
                    status: 200,
                    headers: None,
                    body: ResponseBody::Static {
                        content: serde_json::json!({}),
                    },
                },
                behavior: None,
            }),
        };

        // Validation would catch this in the async function
        assert_eq!(endpoint.protocol, Protocol::Http);
    }

    // ==================== ConfigFormat Tests ====================

    #[test]
    fn test_config_format_yaml_deserialization() {
        let json = r#"{"config": "test", "format": "yaml"}"#;
        let request: ImportRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(request.format, ConfigFormat::Yaml));
    }

    #[test]
    fn test_config_format_json_deserialization() {
        let json = r#"{"config": "test", "format": "json"}"#;
        let request: ImportRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(request.format, ConfigFormat::Json));
    }

    // ==================== Async Handler Tests ====================

    #[tokio::test]
    async fn test_list_endpoints_empty() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);
        let result = list_endpoints(State(state)).await;
        let response = result.0;
        assert_eq!(response["total"], 0);
        assert_eq!(response["enabled"], 0);
    }

    #[tokio::test]
    async fn test_create_and_get_endpoint() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let endpoint = create_test_http_endpoint();
        let create_result = create_endpoint(State(state.clone()), Json(endpoint.clone())).await;
        assert!(create_result.is_ok());

        let get_result = get_endpoint(State(state), Path("test-1".to_string())).await;
        assert!(get_result.is_ok());
        assert_eq!(get_result.unwrap().0.id, "test-1");
    }

    #[tokio::test]
    async fn test_get_endpoint_not_found() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let result = get_endpoint(State(state), Path("nonexistent".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_create_endpoint_duplicate_id() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let endpoint = create_test_http_endpoint();
        let _ = create_endpoint(State(state.clone()), Json(endpoint.clone())).await;

        // Try to create with same ID
        let result = create_endpoint(State(state), Json(endpoint)).await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_create_endpoint_auto_generate_id() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let mut endpoint = create_test_http_endpoint();
        endpoint.id = String::new(); // Empty ID should be auto-generated

        let result = create_endpoint(State(state), Json(endpoint)).await;
        assert!(result.is_ok());
        let created = result.unwrap().0;
        assert!(!created.id.is_empty());
    }

    #[tokio::test]
    async fn test_update_endpoint() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let endpoint = create_test_http_endpoint();
        let _ = create_endpoint(State(state.clone()), Json(endpoint.clone())).await;

        let mut updated = endpoint.clone();
        updated.name = "Updated Name".to_string();

        let result = update_endpoint(State(state), Path("test-1".to_string()), Json(updated)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.name, "Updated Name");
    }

    #[tokio::test]
    async fn test_update_endpoint_not_found() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let endpoint = create_test_http_endpoint();
        let result =
            update_endpoint(State(state), Path("nonexistent".to_string()), Json(endpoint)).await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_endpoint() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let endpoint = create_test_http_endpoint();
        let _ = create_endpoint(State(state.clone()), Json(endpoint)).await;

        let result = delete_endpoint(State(state), Path("test-1".to_string())).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_delete_endpoint_not_found() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let result = delete_endpoint(State(state), Path("nonexistent".to_string())).await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_validate_endpoint_valid_http() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let endpoint = create_test_http_endpoint();
        let result = validate_endpoint(State(state), Json(endpoint)).await;
        assert!(result.0.valid);
        assert!(result.0.errors.is_empty());
    }

    #[tokio::test]
    async fn test_validate_endpoint_invalid_method() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let mut endpoint = create_test_http_endpoint();
        if let EndpointProtocolConfig::Http(ref mut http_config) = endpoint.config {
            http_config.method = "INVALID".to_string();
        }

        let result = validate_endpoint(State(state), Json(endpoint)).await;
        assert!(!result.0.valid);
        assert!(!result.0.errors.is_empty());
    }

    #[tokio::test]
    async fn test_validate_endpoint_invalid_path() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let mut endpoint = create_test_http_endpoint();
        if let EndpointProtocolConfig::Http(ref mut http_config) = endpoint.config {
            http_config.path = "no-leading-slash".to_string();
        }

        let result = validate_endpoint(State(state), Json(endpoint)).await;
        assert!(!result.0.valid);
    }

    #[tokio::test]
    async fn test_validate_endpoint_invalid_status() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let mut endpoint = create_test_http_endpoint();
        if let EndpointProtocolConfig::Http(ref mut http_config) = endpoint.config {
            http_config.response.status = 999; // Invalid status code
        }

        let result = validate_endpoint(State(state), Json(endpoint)).await;
        assert!(!result.0.valid);
    }

    #[tokio::test]
    async fn test_validate_endpoint_high_error_rate_warning() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let mut endpoint = create_test_http_endpoint();
        if let EndpointProtocolConfig::Http(ref mut http_config) = endpoint.config {
            http_config.behavior = Some(EndpointBehavior {
                latency: None,
                failure: Some(FailureConfig {
                    error_rate: 0.75,
                    status_codes: vec![500],
                    error_message: None,
                }),
                traffic_shaping: None,
            });
        }

        let result = validate_endpoint(State(state), Json(endpoint)).await;
        assert!(result.0.valid); // Still valid, just has warnings
        assert!(!result.0.warnings.is_empty());
    }

    #[tokio::test]
    async fn test_validate_grpc_endpoint_empty_service() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let endpoint = EndpointConfig {
            id: "grpc-test".to_string(),
            protocol: Protocol::Grpc,
            name: "gRPC Test".to_string(),
            description: None,
            enabled: true,
            config: EndpointProtocolConfig::Grpc(GrpcEndpointConfig {
                service: String::new(), // Empty service name
                method: "Method".to_string(),
                proto_file: "/test.proto".to_string(),
                request_type: "Request".to_string(),
                response_type: "Response".to_string(),
                response: GrpcResponseConfig {
                    body: ResponseBody::Static {
                        content: serde_json::json!({}),
                    },
                    metadata: None,
                },
                behavior: None,
            }),
        };

        let result = validate_endpoint(State(state), Json(endpoint)).await;
        assert!(!result.0.valid);
    }

    #[tokio::test]
    async fn test_validate_websocket_endpoint_invalid_path() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let endpoint = EndpointConfig {
            id: "ws-test".to_string(),
            protocol: Protocol::Websocket,
            name: "WebSocket Test".to_string(),
            description: None,
            enabled: true,
            config: EndpointProtocolConfig::Websocket(WebsocketEndpointConfig {
                path: "no-slash".to_string(), // Invalid path
                on_connect: None,
                on_message: None,
                on_disconnect: None,
                behavior: None,
            }),
        };

        let result = validate_endpoint(State(state), Json(endpoint)).await;
        assert!(!result.0.valid);
    }

    #[tokio::test]
    async fn test_get_config() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let result = get_config(State(state)).await;
        // Should return the default config
        let _ = result.0;
    }

    #[tokio::test]
    async fn test_update_config() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config.clone());

        let result = update_config(State(state), Json(config)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_export_config() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        let result = export_config(State(state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_endpoints_with_multiple_protocols() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);

        // Add HTTP endpoint
        let http_endpoint = create_test_http_endpoint();
        let _ = create_endpoint(State(state.clone()), Json(http_endpoint)).await;

        // Add WebSocket endpoint
        let ws_endpoint = EndpointConfig {
            id: "ws-1".to_string(),
            protocol: Protocol::Websocket,
            name: "WS Endpoint".to_string(),
            description: None,
            enabled: true,
            config: EndpointProtocolConfig::Websocket(WebsocketEndpointConfig {
                path: "/ws".to_string(),
                on_connect: None,
                on_message: Some(WebsocketAction::Echo),
                on_disconnect: None,
                behavior: None,
            }),
        };
        let _ = create_endpoint(State(state.clone()), Json(ws_endpoint)).await;

        let result = list_endpoints(State(state)).await;
        let response = result.0;
        assert_eq!(response["total"], 2);
        assert_eq!(response["by_protocol"]["http"], 1);
        assert_eq!(response["by_protocol"]["websocket"], 1);
    }

    // ==================== Router Tests ====================

    #[test]
    fn test_create_ui_builder_router() {
        let config = ServerConfig::default();
        let state = UIBuilderState::new(config);
        let router = create_ui_builder_router(state);
        // Router should be created without panic
        let _ = router;
    }
}
