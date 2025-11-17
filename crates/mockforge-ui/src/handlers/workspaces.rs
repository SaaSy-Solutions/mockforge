//! Workspace management API handlers
//!
//! This module provides REST API endpoints for managing multi-tenant workspaces.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use mockforge_core::{MultiTenantWorkspaceRegistry, TenantWorkspace, Workspace, WorkspaceStats};
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::MultiTenantConfig;

    fn create_test_state() -> WorkspaceState {
        let config = MultiTenantConfig::default();
        let registry = MultiTenantWorkspaceRegistry::new(config);
        WorkspaceState::new(Arc::new(tokio::sync::RwLock::new(registry)))
    }

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
}
