/// Management API for MockForge
///
/// Provides REST endpoints for controlling mocks, server configuration,
/// and integration with developer tools (VS Code extension, CI/CD, etc.)
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};
use mockforge_core::openapi::OpenApiSpec;
#[cfg(feature = "smtp")]
use mockforge_smtp::EmailSearchFilters;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::*;

/// Mock configuration representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockConfig {
    pub id: String,
    pub name: String,
    pub method: String,
    pub path: String,
    pub response: MockResponse,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

/// Mock response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResponse {
    pub body: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Server statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub active_mocks: usize,
    pub enabled_mocks: usize,
    pub registered_routes: usize,
}

/// Server configuration info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub version: String,
    pub port: u16,
    pub has_openapi_spec: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_path: Option<String>,
}

/// Shared state for the management API
#[derive(Clone)]
pub struct ManagementState {
    pub mocks: Arc<RwLock<Vec<MockConfig>>>,
    pub spec: Option<Arc<OpenApiSpec>>,
    pub spec_path: Option<String>,
    pub port: u16,
    pub start_time: std::time::Instant,
    pub request_counter: Arc<RwLock<u64>>,
    #[cfg(feature = "smtp")]
    pub smtp_registry: Option<Arc<mockforge_smtp::SmtpSpecRegistry>>,
    #[cfg(feature = "mqtt")]
    pub mqtt_broker: Option<Arc<mockforge_mqtt::MqttBroker>>,
}

impl ManagementState {
    pub fn new(spec: Option<Arc<OpenApiSpec>>, spec_path: Option<String>, port: u16) -> Self {
        Self {
            mocks: Arc::new(RwLock::new(Vec::new())),
            spec,
            spec_path,
            port,
            start_time: std::time::Instant::now(),
            request_counter: Arc::new(RwLock::new(0)),
            #[cfg(feature = "smtp")]
            smtp_registry: None,
            #[cfg(feature = "mqtt")]
            mqtt_broker: None,
        }
    }

    #[cfg(feature = "smtp")]
    pub fn with_smtp_registry(
        mut self,
        smtp_registry: Arc<mockforge_smtp::SmtpSpecRegistry>,
    ) -> Self {
        self.smtp_registry = Some(smtp_registry);
        self
    }

    #[cfg(feature = "mqtt")]
    pub fn with_mqtt_broker(mut self, mqtt_broker: Arc<mockforge_mqtt::MqttBroker>) -> Self {
        self.mqtt_broker = Some(mqtt_broker);
        self
    }
}

/// List all mocks
async fn list_mocks(State(state): State<ManagementState>) -> Json<serde_json::Value> {
    let mocks = state.mocks.read().await;
    Json(serde_json::json!({
        "mocks": *mocks,
        "total": mocks.len(),
        "enabled": mocks.iter().filter(|m| m.enabled).count()
    }))
}

/// Get a specific mock by ID
async fn get_mock(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> Result<Json<MockConfig>, StatusCode> {
    let mocks = state.mocks.read().await;
    mocks
        .iter()
        .find(|m| m.id == id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Create a new mock
async fn create_mock(
    State(state): State<ManagementState>,
    Json(mut mock): Json<MockConfig>,
) -> Result<Json<MockConfig>, StatusCode> {
    let mut mocks = state.mocks.write().await;

    // Generate ID if not provided
    if mock.id.is_empty() {
        mock.id = uuid::Uuid::new_v4().to_string();
    }

    // Check for duplicate ID
    if mocks.iter().any(|m| m.id == mock.id) {
        return Err(StatusCode::CONFLICT);
    }

    info!("Creating mock: {} {} {}", mock.method, mock.path, mock.id);
    mocks.push(mock.clone());
    Ok(Json(mock))
}

/// Update an existing mock
async fn update_mock(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    Json(updated_mock): Json<MockConfig>,
) -> Result<Json<MockConfig>, StatusCode> {
    let mut mocks = state.mocks.write().await;

    let position = mocks.iter().position(|m| m.id == id).ok_or(StatusCode::NOT_FOUND)?;

    info!("Updating mock: {}", id);
    mocks[position] = updated_mock.clone();
    Ok(Json(updated_mock))
}

/// Delete a mock
async fn delete_mock(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let mut mocks = state.mocks.write().await;

    let position = mocks.iter().position(|m| m.id == id).ok_or(StatusCode::NOT_FOUND)?;

    info!("Deleting mock: {}", id);
    mocks.remove(position);
    Ok(StatusCode::NO_CONTENT)
}

/// Get server statistics
async fn get_stats(State(state): State<ManagementState>) -> Json<ServerStats> {
    let mocks = state.mocks.read().await;
    let request_count = *state.request_counter.read().await;

    Json(ServerStats {
        uptime_seconds: state.start_time.elapsed().as_secs(),
        total_requests: request_count,
        active_mocks: mocks.len(),
        enabled_mocks: mocks.iter().filter(|m| m.enabled).count(),
        registered_routes: mocks.len(), // This could be enhanced with actual route registry info
    })
}

/// Get server configuration
async fn get_config(State(state): State<ManagementState>) -> Json<ServerConfig> {
    Json(ServerConfig {
        version: env!("CARGO_PKG_VERSION").to_string(),
        port: state.port,
        has_openapi_spec: state.spec.is_some(),
        spec_path: state.spec_path.clone(),
    })
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mockforge-management",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Export format for mock configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Yaml,
}

/// Export mocks in specified format
async fn export_mocks(
    State(state): State<ManagementState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<(StatusCode, String), StatusCode> {
    let mocks = state.mocks.read().await;

    let format = params
        .get("format")
        .map(|f| match f.as_str() {
            "yaml" | "yml" => ExportFormat::Yaml,
            _ => ExportFormat::Json,
        })
        .unwrap_or(ExportFormat::Json);

    match format {
        ExportFormat::Json => serde_json::to_string_pretty(&*mocks)
            .map(|json| (StatusCode::OK, json))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
        ExportFormat::Yaml => serde_yaml::to_string(&*mocks)
            .map(|yaml| (StatusCode::OK, yaml))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Import mocks from JSON/YAML
async fn import_mocks(
    State(state): State<ManagementState>,
    Json(mocks): Json<Vec<MockConfig>>,
) -> impl IntoResponse {
    let mut current_mocks = state.mocks.write().await;
    current_mocks.clear();
    current_mocks.extend(mocks);
    Json(serde_json::json!({ "status": "imported", "count": current_mocks.len() }))
}

#[cfg(feature = "smtp")]
/// List SMTP emails in mailbox
async fn list_smtp_emails(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(ref smtp_registry) = state.smtp_registry {
        match smtp_registry.get_emails() {
            Ok(emails) => (StatusCode::OK, Json(serde_json::json!(emails))),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve emails",
                    "message": e.to_string()
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "SMTP mailbox management not available",
                "message": "SMTP server is not enabled or registry not available."
            })),
        )
    }
}

/// Get specific SMTP email
#[cfg(feature = "smtp")]
async fn get_smtp_email(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Some(ref smtp_registry) = state.smtp_registry {
        match smtp_registry.get_email_by_id(&id) {
            Ok(Some(email)) => (StatusCode::OK, Json(serde_json::json!(email))),
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Email not found",
                    "id": id
                })),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve email",
                    "message": e.to_string()
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "SMTP mailbox management not available",
                "message": "SMTP server is not enabled or registry not available."
            })),
        )
    }
}

/// Clear SMTP mailbox
#[cfg(feature = "smtp")]
async fn clear_smtp_mailbox(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(ref smtp_registry) = state.smtp_registry {
        match smtp_registry.clear_mailbox() {
            Ok(()) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": "Mailbox cleared successfully"
                })),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to clear mailbox",
                    "message": e.to_string()
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "SMTP mailbox management not available",
                "message": "SMTP server is not enabled or registry not available."
            })),
        )
    }
}

/// Export SMTP mailbox
#[cfg(feature = "smtp")]
async fn export_smtp_mailbox(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let format = params.get("format").unwrap_or(&"json".to_string()).clone();
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "SMTP mailbox management not available via HTTP API",
            "message": "SMTP server runs separately from HTTP server. Use CLI commands to access mailbox.",
            "requested_format": format
        })),
    )
}

/// Search SMTP emails
#[cfg(feature = "smtp")]
async fn search_smtp_emails(
    State(state): State<ManagementState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    if let Some(ref smtp_registry) = state.smtp_registry {
        let filters = EmailSearchFilters {
            sender: params.get("sender").cloned(),
            recipient: params.get("recipient").cloned(),
            subject: params.get("subject").cloned(),
            body: params.get("body").cloned(),
            since: params
                .get("since")
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            until: params
                .get("until")
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            use_regex: params.get("regex").map(|s| s == "true").unwrap_or(false),
            case_sensitive: params.get("case_sensitive").map(|s| s == "true").unwrap_or(false),
        };

        match smtp_registry.search_emails(filters) {
            Ok(emails) => (StatusCode::OK, Json(serde_json::json!(emails))),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to search emails",
                    "message": e.to_string()
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "SMTP mailbox management not available",
                "message": "SMTP server is not enabled or registry not available."
            })),
        )
    }
}

/// MQTT broker statistics
#[cfg(feature = "mqtt")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttBrokerStats {
    pub connected_clients: usize,
    pub active_topics: usize,
    pub retained_messages: usize,
    pub total_subscriptions: usize,
}

/// MQTT management handlers
#[cfg(feature = "mqtt")]
async fn get_mqtt_stats(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.mqtt_broker {
        let connected_clients = broker.get_connected_clients().await.len();
        let active_topics = broker.get_active_topics().await.len();
        let stats = broker.get_topic_stats().await;

        let broker_stats = MqttBrokerStats {
            connected_clients,
            active_topics,
            retained_messages: stats.retained_messages,
            total_subscriptions: stats.total_subscriptions,
        };

        Json(broker_stats).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "MQTT broker not available").into_response()
    }
}

#[cfg(feature = "mqtt")]
async fn get_mqtt_clients(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.mqtt_broker {
        let clients = broker.get_connected_clients().await;
        Json(serde_json::json!({
            "clients": clients
        }))
        .into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "MQTT broker not available").into_response()
    }
}

#[cfg(feature = "mqtt")]
async fn get_mqtt_topics(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.mqtt_broker {
        let topics = broker.get_active_topics().await;
        Json(serde_json::json!({
            "topics": topics
        }))
        .into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "MQTT broker not available").into_response()
    }
}

#[cfg(feature = "mqtt")]
async fn disconnect_mqtt_client(
    State(state): State<ManagementState>,
    Path(client_id): Path<String>,
) -> impl IntoResponse {
    if let Some(broker) = &state.mqtt_broker {
        match broker.disconnect_client(&client_id).await {
            Ok(_) => {
                (StatusCode::OK, format!("Client '{}' disconnected", client_id)).into_response()
            }
            Err(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to disconnect client: {}", e))
                    .into_response()
            }
        }
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "MQTT broker not available").into_response()
    }
}

/// Build the management API router
pub fn management_router(state: ManagementState) -> Router {
    let router = Router::new()
        .route("/health", get(health_check))
        .route("/stats", get(get_stats))
        .route("/config", get(get_config))
        .route("/mocks", get(list_mocks))
        .route("/mocks", post(create_mock))
        .route("/mocks/{id}", get(get_mock))
        .route("/mocks/{id}", put(update_mock))
        .route("/mocks/{id}", delete(delete_mock))
        .route("/export", get(export_mocks))
        .route("/import", post(import_mocks));

    #[cfg(feature = "smtp")]
    let router = router
        .route("/smtp/mailbox", get(list_smtp_emails))
        .route("/smtp/mailbox", delete(clear_smtp_mailbox))
        .route("/smtp/mailbox/{id}", get(get_smtp_email))
        .route("/smtp/mailbox/export", get(export_smtp_mailbox))
        .route("/smtp/mailbox/search", get(search_smtp_emails));

    #[cfg(not(feature = "smtp"))]
    let router = router;

    #[cfg(feature = "mqtt")]
    let router = router
        .route("/mqtt/stats", get(get_mqtt_stats))
        .route("/mqtt/clients", get(get_mqtt_clients))
        .route("/mqtt/topics", get(get_mqtt_topics))
        .route("/mqtt/clients/{client_id}", delete(disconnect_mqtt_client));

    #[cfg(not(feature = "mqtt"))]
    let router = router;

    router.with_state(state)
}

/// Build the management API router with UI Builder support
pub fn management_router_with_ui_builder(
    state: ManagementState,
    server_config: mockforge_core::config::ServerConfig,
) -> Router {
    use crate::ui_builder::{create_ui_builder_router, UIBuilderState};

    // Create the base management router
    let management = management_router(state);

    // Create UI Builder state and router
    let ui_builder_state = UIBuilderState::new(server_config);
    let ui_builder = create_ui_builder_router(ui_builder_state);

    // Nest UI Builder under /ui-builder
    management.nest("/ui-builder", ui_builder)
}

/// Build management router with spec import API
pub fn management_router_with_spec_import(state: ManagementState) -> Router {
    use crate::spec_import::{spec_import_router, SpecImportState};

    // Create base management router
    let management = management_router(state);

    // Merge with spec import router
    Router::new()
        .merge(management)
        .merge(spec_import_router(SpecImportState::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get_mock() {
        let state = ManagementState::new(None, None, 3000);

        let mock = MockConfig {
            id: "test-1".to_string(),
            name: "Test Mock".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            response: MockResponse {
                body: serde_json::json!({"message": "test"}),
                headers: None,
            },
            enabled: true,
            latency_ms: None,
            status_code: Some(200),
        };

        // Create mock
        {
            let mut mocks = state.mocks.write().await;
            mocks.push(mock.clone());
        }

        // Get mock
        let mocks = state.mocks.read().await;
        let found = mocks.iter().find(|m| m.id == "test-1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Mock");
    }

    #[tokio::test]
    async fn test_server_stats() {
        let state = ManagementState::new(None, None, 3000);

        // Add some mocks
        {
            let mut mocks = state.mocks.write().await;
            mocks.push(MockConfig {
                id: "1".to_string(),
                name: "Mock 1".to_string(),
                method: "GET".to_string(),
                path: "/test1".to_string(),
                response: MockResponse {
                    body: serde_json::json!({}),
                    headers: None,
                },
                enabled: true,
                latency_ms: None,
                status_code: Some(200),
            });
            mocks.push(MockConfig {
                id: "2".to_string(),
                name: "Mock 2".to_string(),
                method: "POST".to_string(),
                path: "/test2".to_string(),
                response: MockResponse {
                    body: serde_json::json!({}),
                    headers: None,
                },
                enabled: false,
                latency_ms: None,
                status_code: Some(201),
            });
        }

        let mocks = state.mocks.read().await;
        assert_eq!(mocks.len(), 2);
        assert_eq!(mocks.iter().filter(|m| m.enabled).count(), 1);
    }
}
