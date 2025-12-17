//! Workspace management API handlers
//!
//! This module provides REST API endpoints for managing multi-tenant workspaces.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use mockforge_core::{
    workspace::MockEnvironmentName, MultiTenantWorkspaceRegistry, Workspace, WorkspaceStats,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// Workspace management state
#[derive(Debug, Clone)]
pub struct WorkspaceState {
    /// Multi-tenant workspace registry
    pub registry: Arc<tokio::sync::RwLock<MultiTenantWorkspaceRegistry>>,
}

impl WorkspaceState {
    /// Create a new workspace state
    pub fn new(registry: Arc<tokio::sync::RwLock<MultiTenantWorkspaceRegistry>>) -> Self {
        Self { registry }
    }
}

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Workspace list item for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceListItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub stats: WorkspaceStats,
    pub created_at: String,
    pub updated_at: String,
}

/// Workspace creation request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Workspace update request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

/// List all workspaces
pub async fn list_workspaces(
    State(state): State<WorkspaceState>,
) -> Result<Json<ApiResponse<Vec<WorkspaceListItem>>>, Response> {
    let registry = state.registry.read().await;

    match registry.list_workspaces() {
        Ok(workspaces) => {
            let items: Vec<WorkspaceListItem> = workspaces
                .into_iter()
                .map(|(id, tenant_ws)| WorkspaceListItem {
                    id,
                    name: tenant_ws.workspace.name.clone(),
                    description: tenant_ws.workspace.description.clone(),
                    enabled: tenant_ws.enabled,
                    stats: tenant_ws.stats.clone(),
                    created_at: tenant_ws.workspace.created_at.to_rfc3339(),
                    updated_at: tenant_ws.workspace.updated_at.to_rfc3339(),
                })
                .collect();

            Ok(Json(ApiResponse::success(items)))
        }
        Err(e) => {
            tracing::error!("Failed to list workspaces: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
                .into_response())
        }
    }
}

/// Get a specific workspace by ID
pub async fn get_workspace(
    State(state): State<WorkspaceState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<ApiResponse<WorkspaceListItem>>, Response> {
    let registry = state.registry.read().await;

    match registry.get_workspace(&workspace_id) {
        Ok(tenant_ws) => {
            let item = WorkspaceListItem {
                id: workspace_id.clone(),
                name: tenant_ws.workspace.name.clone(),
                description: tenant_ws.workspace.description.clone(),
                enabled: tenant_ws.enabled,
                stats: tenant_ws.stats.clone(),
                created_at: tenant_ws.workspace.created_at.to_rfc3339(),
                updated_at: tenant_ws.workspace.updated_at.to_rfc3339(),
            };

            Ok(Json(ApiResponse::success(item)))
        }
        Err(e) => {
            tracing::error!("Failed to get workspace {}: {}", workspace_id, e);
            Err((
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("Workspace '{}' not found", workspace_id)})),
            )
                .into_response())
        }
    }
}

/// Create a new workspace
pub async fn create_workspace(
    State(state): State<WorkspaceState>,
    Json(request): Json<CreateWorkspaceRequest>,
) -> Result<Json<ApiResponse<WorkspaceListItem>>, Response> {
    let mut registry = state.registry.write().await;

    // Check if workspace already exists
    if registry.workspace_exists(&request.id) {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"error": format!("Workspace '{}' already exists", request.id)})),
        )
            .into_response());
    }

    // Create new workspace
    let mut workspace = Workspace::new(request.name.clone());
    workspace.description = request.description.clone();

    match registry.register_workspace(request.id.clone(), workspace) {
        Ok(_) => {
            // Get the created workspace
            match registry.get_workspace(&request.id) {
                Ok(tenant_ws) => {
                    let item = WorkspaceListItem {
                        id: request.id.clone(),
                        name: tenant_ws.workspace.name.clone(),
                        description: tenant_ws.workspace.description.clone(),
                        enabled: tenant_ws.enabled,
                        stats: tenant_ws.stats.clone(),
                        created_at: tenant_ws.workspace.created_at.to_rfc3339(),
                        updated_at: tenant_ws.workspace.updated_at.to_rfc3339(),
                    };

                    tracing::info!("Created workspace: {}", request.id);
                    Ok(Json(ApiResponse::success(item)))
                }
                Err(e) => {
                    tracing::error!("Failed to retrieve created workspace: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Workspace created but failed to retrieve"})),
                    )
                        .into_response())
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to create workspace: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
                .into_response())
        }
    }
}

/// Update an existing workspace
pub async fn update_workspace(
    State(state): State<WorkspaceState>,
    Path(workspace_id): Path<String>,
    Json(request): Json<UpdateWorkspaceRequest>,
) -> Result<Json<ApiResponse<WorkspaceListItem>>, Response> {
    let mut registry = state.registry.write().await;

    // Get existing workspace
    let mut tenant_ws = match registry.get_workspace(&workspace_id) {
        Ok(ws) => ws,
        Err(e) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("Workspace '{}' not found", workspace_id)})),
            )
                .into_response());
        }
    };

    // Update workspace fields
    if let Some(name) = request.name {
        tenant_ws.workspace.name = name;
    }

    if let Some(description) = request.description {
        tenant_ws.workspace.description = Some(description);
    }

    tenant_ws.workspace.updated_at = chrono::Utc::now();

    // Save updated workspace
    match registry.update_workspace(&workspace_id, tenant_ws.workspace.clone()) {
        Ok(_) => {
            // Handle enabled/disabled separately
            if let Some(enabled) = request.enabled {
                if let Err(e) = registry.set_workspace_enabled(&workspace_id, enabled) {
                    tracing::error!("Failed to set workspace enabled status: {}", e);
                }
            }

            // Get updated workspace
            match registry.get_workspace(&workspace_id) {
                Ok(updated_ws) => {
                    let item = WorkspaceListItem {
                        id: workspace_id.clone(),
                        name: updated_ws.workspace.name.clone(),
                        description: updated_ws.workspace.description.clone(),
                        enabled: updated_ws.enabled,
                        stats: updated_ws.stats.clone(),
                        created_at: updated_ws.workspace.created_at.to_rfc3339(),
                        updated_at: updated_ws.workspace.updated_at.to_rfc3339(),
                    };

                    tracing::info!("Updated workspace: {}", workspace_id);
                    Ok(Json(ApiResponse::success(item)))
                }
                Err(e) => {
                    tracing::error!("Failed to retrieve updated workspace: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Workspace updated but failed to retrieve"})),
                    )
                        .into_response())
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to update workspace: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
                .into_response())
        }
    }
}

/// Delete a workspace
pub async fn delete_workspace(
    State(state): State<WorkspaceState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<ApiResponse<String>>, Response> {
    let mut registry = state.registry.write().await;

    match registry.remove_workspace(&workspace_id) {
        Ok(_) => {
            tracing::info!("Deleted workspace: {}", workspace_id);
            Ok(Json(ApiResponse::success(format!(
                "Workspace '{}' deleted successfully",
                workspace_id
            ))))
        }
        Err(e) => {
            tracing::error!("Failed to delete workspace {}: {}", workspace_id, e);
            Err((StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))).into_response())
        }
    }
}

/// Get workspace statistics
pub async fn get_workspace_stats(
    State(state): State<WorkspaceState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<ApiResponse<WorkspaceStats>>, Response> {
    let registry = state.registry.read().await;

    match registry.get_workspace(&workspace_id) {
        Ok(tenant_ws) => Ok(Json(ApiResponse::success(tenant_ws.stats.clone()))),
        Err(e) => {
            tracing::error!("Failed to get workspace stats for {}: {}", workspace_id, e);
            Err((
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("Workspace '{}' not found", workspace_id)})),
            )
                .into_response())
        }
    }
}

/// Mock environment response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEnvironmentResponse {
    pub name: String,
    pub id: String,
    pub workspace_id: String,
    pub reality_config: Option<serde_json::Value>,
    pub chaos_config: Option<serde_json::Value>,
    pub drift_budget_config: Option<serde_json::Value>,
}

/// Mock environment manager response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEnvironmentManagerResponse {
    pub workspace_id: String,
    pub active_environment: Option<String>,
    pub environments: Vec<MockEnvironmentResponse>,
}

/// List all mock environments for a workspace
pub async fn list_mock_environments(
    State(state): State<WorkspaceState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<ApiResponse<MockEnvironmentManagerResponse>>, Response> {
    let registry = state.registry.read().await;

    match registry.get_workspace(&workspace_id) {
        Ok(tenant_ws) => {
            let mock_envs = tenant_ws.workspace.get_mock_environments();
            let environments: Vec<MockEnvironmentResponse> = mock_envs
                .list_environments()
                .into_iter()
                .map(|env| MockEnvironmentResponse {
                    name: env.name.as_str().to_string(),
                    id: env.id.clone(),
                    workspace_id: env.workspace_id.clone(),
                    reality_config: env
                        .reality_config
                        .as_ref()
                        .map(|c| serde_json::to_value(c).unwrap_or(serde_json::json!({}))),
                    chaos_config: env
                        .chaos_config
                        .as_ref()
                        .map(|c| serde_json::to_value(c).unwrap_or(serde_json::json!({}))),
                    drift_budget_config: env
                        .drift_budget_config
                        .as_ref()
                        .map(|c| serde_json::to_value(c).unwrap_or(serde_json::json!({}))),
                })
                .collect();

            let response = MockEnvironmentManagerResponse {
                workspace_id: workspace_id.clone(),
                active_environment: mock_envs.active_environment.map(|e| e.as_str().to_string()),
                environments,
            };

            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Failed to get workspace {}: {}", workspace_id, e);
            Err((
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("Workspace '{}' not found", workspace_id)})),
            )
                .into_response())
        }
    }
}

/// Get a specific mock environment
pub async fn get_mock_environment(
    State(state): State<WorkspaceState>,
    Path((workspace_id, env_name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<MockEnvironmentResponse>>, Response> {
    let registry = state.registry.read().await;

    let env_name_enum = match env_name.to_lowercase().as_str() {
        "dev" => MockEnvironmentName::Dev,
        "test" => MockEnvironmentName::Test,
        "prod" => MockEnvironmentName::Prod,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Invalid environment name: '{}'. Must be 'dev', 'test', or 'prod'", env_name)})),
            )
                .into_response());
        }
    };

    match registry.get_workspace(&workspace_id) {
        Ok(tenant_ws) => {
            match tenant_ws.workspace.get_mock_environment(env_name_enum) {
                Some(env) => {
                    let response = MockEnvironmentResponse {
                        name: env.name.as_str().to_string(),
                        id: env.id.clone(),
                        workspace_id: env.workspace_id.clone(),
                        reality_config: env.reality_config.as_ref().map(|c| serde_json::to_value(c).unwrap_or(serde_json::json!({}))),
                        chaos_config: env.chaos_config.as_ref().map(|c| serde_json::to_value(c).unwrap_or(serde_json::json!({}))),
                        drift_budget_config: env.drift_budget_config.as_ref().map(|c| serde_json::to_value(c).unwrap_or(serde_json::json!({}))),
                    };
                    Ok(Json(ApiResponse::success(response)))
                }
                None => Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": format!("Environment '{}' not found in workspace '{}'", env_name, workspace_id)})),
                )
                    .into_response()),
            }
        }
        Err(e) => {
            tracing::error!("Failed to get workspace {}: {}", workspace_id, e);
            Err((
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("Workspace '{}' not found", workspace_id)})),
            )
                .into_response())
        }
    }
}

/// Set active mock environment
#[derive(Debug, Clone, Deserialize)]
pub struct SetActiveEnvironmentRequest {
    pub environment: String,
}

pub async fn set_active_mock_environment(
    State(state): State<WorkspaceState>,
    Path(workspace_id): Path<String>,
    Json(request): Json<SetActiveEnvironmentRequest>,
) -> Result<Json<ApiResponse<String>>, Response> {
    let mut registry = state.registry.write().await;

    let env_name = match request.environment.to_lowercase().as_str() {
        "dev" => MockEnvironmentName::Dev,
        "test" => MockEnvironmentName::Test,
        "prod" => MockEnvironmentName::Prod,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Invalid environment name: '{}'. Must be 'dev', 'test', or 'prod'", request.environment)})),
            )
                .into_response());
        }
    };

    match registry.get_workspace(&workspace_id) {
        Ok(mut tenant_ws) => {
            match tenant_ws.workspace.set_active_mock_environment(env_name) {
                Ok(_) => {
                    // Save the updated workspace
                    if let Err(e) =
                        registry.update_workspace(&workspace_id, tenant_ws.workspace.clone())
                    {
                        tracing::error!("Failed to save workspace: {}", e);
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({"error": "Failed to save workspace"})),
                        )
                            .into_response());
                    }

                    tracing::info!(
                        "Set active environment to '{}' for workspace '{}'",
                        request.environment,
                        workspace_id
                    );
                    Ok(Json(ApiResponse::success(format!(
                        "Active environment set to '{}'",
                        request.environment
                    ))))
                }
                Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()})))
                    .into_response()),
            }
        }
        Err(e) => {
            tracing::error!("Failed to get workspace {}: {}", workspace_id, e);
            Err((
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("Workspace '{}' not found", workspace_id)})),
            )
                .into_response())
        }
    }
}

/// Update mock environment configuration
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMockEnvironmentRequest {
    pub reality_config: Option<serde_json::Value>,
    pub chaos_config: Option<serde_json::Value>,
    pub drift_budget_config: Option<serde_json::Value>,
}

pub async fn update_mock_environment(
    State(state): State<WorkspaceState>,
    Path((workspace_id, env_name)): Path<(String, String)>,
    Json(request): Json<UpdateMockEnvironmentRequest>,
) -> Result<Json<ApiResponse<MockEnvironmentResponse>>, Response> {
    let mut registry = state.registry.write().await;

    let env_name_enum = match env_name.to_lowercase().as_str() {
        "dev" => MockEnvironmentName::Dev,
        "test" => MockEnvironmentName::Test,
        "prod" => MockEnvironmentName::Prod,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Invalid environment name: '{}'. Must be 'dev', 'test', or 'prod'", env_name)})),
            )
                .into_response());
        }
    };

    match registry.get_workspace(&workspace_id) {
        Ok(mut tenant_ws) => {
            // Parse the configs from JSON
            let reality_config =
                request.reality_config.and_then(|v| serde_json::from_value(v).ok());
            let chaos_config = request.chaos_config.and_then(|v| serde_json::from_value(v).ok());
            let drift_budget_config =
                request.drift_budget_config.and_then(|v| serde_json::from_value(v).ok());

            // Update the environment config
            match tenant_ws.workspace.set_mock_environment_config(
                env_name_enum,
                reality_config,
                chaos_config,
                drift_budget_config,
            ) {
                Ok(_) => {
                    // Save the updated workspace
                    if let Err(e) =
                        registry.update_workspace(&workspace_id, tenant_ws.workspace.clone())
                    {
                        tracing::error!("Failed to save workspace: {}", e);
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({"error": "Failed to save workspace"})),
                        )
                            .into_response());
                    }

                    // Get the updated environment
                    match tenant_ws.workspace.get_mock_environment(env_name_enum) {
                        Some(env) => {
                            let response = MockEnvironmentResponse {
                                name: env.name.as_str().to_string(),
                                id: env.id.clone(),
                                workspace_id: env.workspace_id.clone(),
                                reality_config: env.reality_config.as_ref().map(|c| {
                                    serde_json::to_value(c).unwrap_or(serde_json::json!({}))
                                }),
                                chaos_config: env.chaos_config.as_ref().map(|c| {
                                    serde_json::to_value(c).unwrap_or(serde_json::json!({}))
                                }),
                                drift_budget_config: env.drift_budget_config.as_ref().map(|c| {
                                    serde_json::to_value(c).unwrap_or(serde_json::json!({}))
                                }),
                            };
                            tracing::info!(
                                "Updated environment '{}' for workspace '{}'",
                                env_name,
                                workspace_id
                            );
                            Ok(Json(ApiResponse::success(response)))
                        }
                        None => Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({"error": "Failed to retrieve updated environment"})),
                        )
                            .into_response()),
                    }
                }
                Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()})))
                    .into_response()),
            }
        }
        Err(e) => {
            tracing::error!("Failed to get workspace {}: {}", workspace_id, e);
            Err((
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("Workspace '{}' not found", workspace_id)})),
            )
                .into_response())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::MultiTenantConfig;

    fn create_test_state() -> WorkspaceState {
        let config = MultiTenantConfig::default();
        let registry = MultiTenantWorkspaceRegistry::new(config);
        WorkspaceState::new(Arc::new(tokio::sync::RwLock::new(registry)))
    }

    // ==================== WorkspaceState Tests ====================

    #[test]
    fn test_workspace_state_creation() {
        let state = create_test_state();
        // State is created - this verifies the type is correct
        let _ = state;
    }

    #[test]
    fn test_workspace_state_clone() {
        let state = create_test_state();
        let cloned = state.clone();
        // Both states reference the same registry
        let _ = cloned;
    }

    #[test]
    fn test_workspace_state_debug() {
        let state = create_test_state();
        let debug = format!("{:?}", state);
        assert!(debug.contains("WorkspaceState"));
    }

    // ==================== ApiResponse Tests ====================

    #[test]
    fn test_api_response_success() {
        let response: ApiResponse<String> = ApiResponse::success("test data".to_string());
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<String> = ApiResponse::error("test error".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn test_api_response_serialization() {
        let response = ApiResponse::success("data".to_string());
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("data"));
    }

    #[test]
    fn test_api_response_error_serialization() {
        let response: ApiResponse<()> = ApiResponse::error("something went wrong".to_string());
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("something went wrong"));
    }

    // ==================== CreateWorkspaceRequest Tests ====================

    #[test]
    fn test_create_workspace_request_minimal() {
        let request = CreateWorkspaceRequest {
            id: "ws-123".to_string(),
            name: "My Workspace".to_string(),
            description: None,
        };

        assert_eq!(request.id, "ws-123");
        assert_eq!(request.name, "My Workspace");
        assert!(request.description.is_none());
    }

    #[test]
    fn test_create_workspace_request_full() {
        let request = CreateWorkspaceRequest {
            id: "ws-456".to_string(),
            name: "Full Workspace".to_string(),
            description: Some("A complete workspace".to_string()),
        };

        assert!(request.description.is_some());
    }

    #[test]
    fn test_create_workspace_request_deserialization() {
        let json = r#"{
            "id": "test-ws",
            "name": "Test",
            "description": "Test workspace"
        }"#;

        let request: CreateWorkspaceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "test-ws");
        assert_eq!(request.name, "Test");
    }

    // ==================== UpdateWorkspaceRequest Tests ====================

    #[test]
    fn test_update_workspace_request_empty() {
        let request = UpdateWorkspaceRequest {
            name: None,
            description: None,
            enabled: None,
        };

        assert!(request.name.is_none());
        assert!(request.description.is_none());
        assert!(request.enabled.is_none());
    }

    #[test]
    fn test_update_workspace_request_partial() {
        let request = UpdateWorkspaceRequest {
            name: Some("New Name".to_string()),
            description: None,
            enabled: Some(false),
        };

        assert!(request.name.is_some());
        assert!(request.enabled.is_some());
    }

    #[test]
    fn test_update_workspace_request_deserialization() {
        let json = r#"{
            "name": "Updated",
            "enabled": true
        }"#;

        let request: UpdateWorkspaceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Updated".to_string()));
        assert_eq!(request.enabled, Some(true));
    }

    // ==================== WorkspaceListItem Tests ====================

    #[test]
    fn test_workspace_list_item_creation() {
        let item = WorkspaceListItem {
            id: "item-1".to_string(),
            name: "Test Item".to_string(),
            description: Some("Description".to_string()),
            enabled: true,
            stats: WorkspaceStats::default(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-02T00:00:00Z".to_string(),
        };

        assert_eq!(item.id, "item-1");
        assert!(item.enabled);
    }

    #[test]
    fn test_workspace_list_item_serialization() {
        let item = WorkspaceListItem {
            id: "ser-test".to_string(),
            name: "Serialize Test".to_string(),
            description: None,
            enabled: false,
            stats: WorkspaceStats::default(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("ser-test"));
        assert!(json.contains("Serialize Test"));
    }

    #[test]
    fn test_workspace_list_item_clone() {
        let item = WorkspaceListItem {
            id: "clone-test".to_string(),
            name: "Clone Test".to_string(),
            description: None,
            enabled: true,
            stats: WorkspaceStats::default(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let cloned = item.clone();
        assert_eq!(cloned.id, item.id);
        assert_eq!(cloned.enabled, item.enabled);
    }

    // ==================== MockEnvironmentResponse Tests ====================

    #[test]
    fn test_mock_environment_response_creation() {
        let response = MockEnvironmentResponse {
            name: "dev".to_string(),
            id: "env-123".to_string(),
            workspace_id: "ws-456".to_string(),
            reality_config: None,
            chaos_config: None,
            drift_budget_config: None,
        };

        assert_eq!(response.name, "dev");
        assert_eq!(response.id, "env-123");
    }

    #[test]
    fn test_mock_environment_response_with_configs() {
        let response = MockEnvironmentResponse {
            name: "test".to_string(),
            id: "env-test".to_string(),
            workspace_id: "ws-test".to_string(),
            reality_config: Some(serde_json::json!({"level": "high"})),
            chaos_config: Some(serde_json::json!({"enabled": true})),
            drift_budget_config: Some(serde_json::json!({"max_drift": 0.1})),
        };

        assert!(response.reality_config.is_some());
        assert!(response.chaos_config.is_some());
        assert!(response.drift_budget_config.is_some());
    }

    #[test]
    fn test_mock_environment_response_serialization() {
        let response = MockEnvironmentResponse {
            name: "prod".to_string(),
            id: "env-prod".to_string(),
            workspace_id: "ws-prod".to_string(),
            reality_config: None,
            chaos_config: None,
            drift_budget_config: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("prod"));
        assert!(json.contains("env-prod"));
    }

    // ==================== MockEnvironmentManagerResponse Tests ====================

    #[test]
    fn test_mock_environment_manager_response_empty() {
        let response = MockEnvironmentManagerResponse {
            workspace_id: "ws-empty".to_string(),
            active_environment: None,
            environments: vec![],
        };

        assert!(response.active_environment.is_none());
        assert!(response.environments.is_empty());
    }

    #[test]
    fn test_mock_environment_manager_response_with_environments() {
        let response = MockEnvironmentManagerResponse {
            workspace_id: "ws-full".to_string(),
            active_environment: Some("dev".to_string()),
            environments: vec![
                MockEnvironmentResponse {
                    name: "dev".to_string(),
                    id: "env-dev".to_string(),
                    workspace_id: "ws-full".to_string(),
                    reality_config: None,
                    chaos_config: None,
                    drift_budget_config: None,
                },
                MockEnvironmentResponse {
                    name: "test".to_string(),
                    id: "env-test".to_string(),
                    workspace_id: "ws-full".to_string(),
                    reality_config: None,
                    chaos_config: None,
                    drift_budget_config: None,
                },
            ],
        };

        assert_eq!(response.active_environment, Some("dev".to_string()));
        assert_eq!(response.environments.len(), 2);
    }

    // ==================== SetActiveEnvironmentRequest Tests ====================

    #[test]
    fn test_set_active_environment_request_creation() {
        let request = SetActiveEnvironmentRequest {
            environment: "prod".to_string(),
        };

        assert_eq!(request.environment, "prod");
    }

    #[test]
    fn test_set_active_environment_request_deserialization() {
        let json = r#"{"environment": "test"}"#;
        let request: SetActiveEnvironmentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.environment, "test");
    }

    // ==================== UpdateMockEnvironmentRequest Tests ====================

    #[test]
    fn test_update_mock_environment_request_empty() {
        let request = UpdateMockEnvironmentRequest {
            reality_config: None,
            chaos_config: None,
            drift_budget_config: None,
        };

        assert!(request.reality_config.is_none());
    }

    #[test]
    fn test_update_mock_environment_request_with_configs() {
        let request = UpdateMockEnvironmentRequest {
            reality_config: Some(serde_json::json!({"level": "medium"})),
            chaos_config: Some(serde_json::json!({"rate": 0.5})),
            drift_budget_config: None,
        };

        assert!(request.reality_config.is_some());
        assert!(request.chaos_config.is_some());
    }

    // ==================== Handler Tests ====================

    #[tokio::test]
    async fn test_create_workspace() {
        let state = create_test_state();

        let request = CreateWorkspaceRequest {
            id: "test".to_string(),
            name: "Test Workspace".to_string(),
            description: Some("Test description".to_string()),
        };

        let result = create_workspace(State(state.clone()), Json(request)).await.unwrap();

        assert!(result.0.success);
        assert_eq!(result.0.data.as_ref().unwrap().id, "test");
    }

    #[tokio::test]
    async fn test_list_workspaces() {
        let state = create_test_state();

        // Create a workspace first
        let request = CreateWorkspaceRequest {
            id: "test".to_string(),
            name: "Test Workspace".to_string(),
            description: None,
        };

        let _ = create_workspace(State(state.clone()), Json(request)).await;

        let result = list_workspaces(State(state)).await.unwrap();

        assert!(result.0.success);
        assert!(!result.0.data.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_workspace() {
        let state = create_test_state();

        // Create a workspace first
        let request = CreateWorkspaceRequest {
            id: "get-test".to_string(),
            name: "Get Test Workspace".to_string(),
            description: None,
        };

        let _ = create_workspace(State(state.clone()), Json(request)).await;

        let result = get_workspace(State(state), Path("get-test".to_string())).await.unwrap();

        assert!(result.0.success);
        assert_eq!(result.0.data.as_ref().unwrap().id, "get-test");
    }

    #[tokio::test]
    async fn test_get_workspace_not_found() {
        let state = create_test_state();

        let result = get_workspace(State(state), Path("nonexistent".to_string())).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_duplicate_workspace() {
        let state = create_test_state();

        let request = CreateWorkspaceRequest {
            id: "duplicate".to_string(),
            name: "First".to_string(),
            description: None,
        };

        let _ = create_workspace(State(state.clone()), Json(request)).await;

        let request2 = CreateWorkspaceRequest {
            id: "duplicate".to_string(),
            name: "Second".to_string(),
            description: None,
        };

        let result = create_workspace(State(state), Json(request2)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_workspace() {
        let state = create_test_state();

        // Create a workspace first
        let request = CreateWorkspaceRequest {
            id: "delete-test".to_string(),
            name: "Delete Test".to_string(),
            description: None,
        };

        let _ = create_workspace(State(state.clone()), Json(request)).await;

        let result = delete_workspace(State(state.clone()), Path("delete-test".to_string())).await;

        assert!(result.is_ok());
        assert!(result.unwrap().0.success);

        // Verify workspace is gone
        let get_result = get_workspace(State(state), Path("delete-test".to_string())).await;
        assert!(get_result.is_err());
    }

    #[tokio::test]
    async fn test_update_workspace() {
        let state = create_test_state();

        // Create a workspace first
        let create_request = CreateWorkspaceRequest {
            id: "update-test".to_string(),
            name: "Original Name".to_string(),
            description: None,
        };

        let _ = create_workspace(State(state.clone()), Json(create_request)).await;

        // Update the workspace
        let update_request = UpdateWorkspaceRequest {
            name: Some("Updated Name".to_string()),
            description: Some("New description".to_string()),
            enabled: Some(false),
        };

        let result = update_workspace(
            State(state.clone()),
            Path("update-test".to_string()),
            Json(update_request),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.0.success);
        assert_eq!(response.0.data.as_ref().unwrap().name, "Updated Name");
    }

    #[tokio::test]
    async fn test_get_workspace_stats() {
        let state = create_test_state();

        // Create a workspace first
        let request = CreateWorkspaceRequest {
            id: "stats-test".to_string(),
            name: "Stats Test".to_string(),
            description: None,
        };

        let _ = create_workspace(State(state.clone()), Json(request)).await;

        let result = get_workspace_stats(State(state), Path("stats-test".to_string())).await;

        assert!(result.is_ok());
        assert!(result.unwrap().0.success);
    }
}
