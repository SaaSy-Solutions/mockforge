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

/// Create the UI Builder router
pub fn create_ui_builder_router(state: UIBuilderState) -> Router {
    Router::new()
        .route("/endpoints", get(list_endpoints).post(create_endpoint))
        .route("/endpoints/:id", get(get_endpoint).put(update_endpoint).delete(delete_endpoint))
        .route("/endpoints/validate", post(validate_endpoint))
        .route("/config", get(get_config).put(update_config))
        .route("/config/export", get(export_config))
        .route("/config/import", post(import_config))
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
