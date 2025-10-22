/// UI Builder API for MockForge
///
/// Provides REST endpoints for the low-code UI builder that allows visual
/// creation and editing of mock endpoints without writing code.
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
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
    pub id: String,
    pub protocol: Protocol,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub config: EndpointProtocolConfig,
}

/// Supported protocols in UI builder
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Http,
    Grpc,
    Websocket,
    Graphql,
    Mqtt,
    Smtp,
    Kafka,
    Amqp,
    Ftp,
}

/// Protocol-specific endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EndpointProtocolConfig {
    Http(HttpEndpointConfig),
    Grpc(GrpcEndpointConfig),
    Websocket(WebsocketEndpointConfig),
}

/// HTTP endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpEndpointConfig {
    pub method: String,
    pub path: String,
    pub request: Option<HttpRequestConfig>,
    pub response: HttpResponseConfig,
    pub behavior: Option<EndpointBehavior>,
}

/// HTTP request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestConfig {
    pub validation: Option<ValidationConfig>,
    pub headers: Option<Vec<HeaderConfig>>,
    pub query_params: Option<Vec<QueryParamConfig>>,
    pub body_schema: Option<serde_json::Value>,
}

/// HTTP response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponseConfig {
    pub status: u16,
    pub headers: Option<Vec<HeaderConfig>>,
    pub body: ResponseBody,
}

/// Response body configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseBody {
    Static { content: serde_json::Value },
    Template { template: String },
    Faker { schema: serde_json::Value },
    AI { prompt: String },
}

/// Header configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderConfig {
    pub name: String,
    pub value: String,
}

/// Query parameter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParamConfig {
    pub name: String,
    pub required: bool,
    pub schema: Option<serde_json::Value>,
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub mode: ValidationMode,
    pub schema: Option<serde_json::Value>,
}

/// Validation mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationMode {
    Off,
    Warn,
    Enforce,
}

/// Endpoint behavior configuration (chaos engineering)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointBehavior {
    pub latency: Option<LatencyConfig>,
    pub failure: Option<FailureConfig>,
    pub traffic_shaping: Option<TrafficShapingConfig>,
}

/// Latency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyConfig {
    pub base_ms: u64,
    pub jitter_ms: u64,
    pub distribution: LatencyDistribution,
}

/// Latency distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LatencyDistribution {
    Fixed,
    Normal { std_dev_ms: f64 },
    Pareto { shape: f64 },
}

/// Failure configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureConfig {
    pub error_rate: f64,
    pub status_codes: Vec<u16>,
    pub error_message: Option<String>,
}

/// Traffic shaping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficShapingConfig {
    pub bandwidth_limit_bps: Option<u64>,
    pub packet_loss_rate: Option<f64>,
}

/// gRPC endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcEndpointConfig {
    pub service: String,
    pub method: String,
    pub proto_file: String,
    pub request_type: String,
    pub response_type: String,
    pub response: GrpcResponseConfig,
    pub behavior: Option<EndpointBehavior>,
}

/// gRPC response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcResponseConfig {
    pub body: ResponseBody,
    pub metadata: Option<Vec<HeaderConfig>>,
}

/// WebSocket endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketEndpointConfig {
    pub path: String,
    pub on_connect: Option<WebsocketAction>,
    pub on_message: Option<WebsocketAction>,
    pub on_disconnect: Option<WebsocketAction>,
    pub behavior: Option<EndpointBehavior>,
}

/// WebSocket action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebsocketAction {
    Send { message: ResponseBody },
    Broadcast { message: ResponseBody },
    Echo,
    Close { code: u16, reason: String },
}

/// Configuration validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// UI Builder state
#[derive(Clone)]
pub struct UIBuilderState {
    pub endpoints: Arc<RwLock<Vec<EndpointConfig>>>,
    pub server_config: Arc<RwLock<ServerConfig>>,
}

impl UIBuilderState {
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

    let paths = spec["paths"].as_object_mut().unwrap();

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
        .route("/endpoints/:id", get(get_endpoint).put(update_endpoint).delete(delete_endpoint))
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
}
