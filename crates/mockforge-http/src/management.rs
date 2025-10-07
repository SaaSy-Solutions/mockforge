/// Management API for MockForge
///
/// Provides REST endpoints for controlling mocks, server configuration,
/// and integration with developer tools (VS Code extension, CI/CD, etc.)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use mockforge_core::openapi::OpenApiSpec;
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
        }
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

    let position = mocks
        .iter()
        .position(|m| m.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;

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

    let position = mocks
        .iter()
        .position(|m| m.id == id)
        .ok_or(StatusCode::NOT_FOUND)?;

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

    let format = params.get("format")
        .and_then(|f| match f.as_str() {
            "yaml" | "yml" => Some(ExportFormat::Yaml),
            _ => Some(ExportFormat::Json),
        })
        .unwrap_or(ExportFormat::Json);

    match format {
        ExportFormat::Json => {
            serde_json::to_string_pretty(&*mocks)
                .map(|json| (StatusCode::OK, json))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
        ExportFormat::Yaml => {
            serde_yaml::to_string(&*mocks)
                .map(|yaml| (StatusCode::OK, yaml))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Import mocks from JSON/YAML
async fn import_mocks(
    State(state): State<ManagementState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    body: String,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let format = params.get("format")
        .and_then(|f| match f.as_str() {
            "yaml" | "yml" => Some(ExportFormat::Yaml),
            _ => Some(ExportFormat::Json),
        })
        .unwrap_or(ExportFormat::Json);

    let imported_mocks: Vec<MockConfig> = match format {
        ExportFormat::Json => serde_json::from_str(&body)
            .map_err(|_| StatusCode::BAD_REQUEST)?,
        ExportFormat::Yaml => serde_yaml::from_str(&body)
            .map_err(|_| StatusCode::BAD_REQUEST)?,
    };

    let mut mocks = state.mocks.write().await;

    // Determine import strategy
    let merge = params.get("merge")
        .map(|v| v == "true")
        .unwrap_or(false);

    if merge {
        // Merge: add new mocks, update existing ones by ID
        for imported in imported_mocks {
            if let Some(pos) = mocks.iter().position(|m| m.id == imported.id) {
                mocks[pos] = imported;
            } else {
                mocks.push(imported);
            }
        }
    } else {
        // Replace: clear existing and add imported
        *mocks = imported_mocks;
    }

    info!("Imported {} mocks (merge: {})", mocks.len(), merge);

    Ok(Json(serde_json::json!({
        "success": true,
        "imported": mocks.len(),
        "merge": merge
    })))
}

/// Build the management API router
pub fn management_router(state: ManagementState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/stats", get(get_stats))
        .route("/config", get(get_config))
        .route("/mocks", get(list_mocks))
        .route("/mocks", post(create_mock))
        .route("/mocks/:id", get(get_mock))
        .route("/mocks/:id", put(update_mock))
        .route("/mocks/:id", delete(delete_mock))
        .route("/export", get(export_mocks))
        .route("/import", post(import_mocks))
        .with_state(state)
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
