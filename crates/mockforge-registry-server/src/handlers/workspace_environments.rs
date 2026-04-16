//! Workspace environments handlers (Postman-style environments per workspace).
//!
//! These endpoints back the `EnvironmentManager` UI in the Config page and
//! the `EnvironmentIndicator` in the layout. They are gated to org members
//! who own the workspace and reuse the cloud workspace authorization
//! pattern from `cloud_workspaces.rs`.
//!
//! The local OSS admin server has its own `/__mockforge/workspaces/...`
//! routes; these are the cloud equivalents under `/api/v1/workspaces/...`.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    AppState,
};
use mockforge_registry_core::models::workspace_environment::{
    WorkspaceEnvironment, WorkspaceEnvironmentSummary, WorkspaceEnvironmentVariable,
};

// ====================================================================
// Authorization helper
// ====================================================================

/// Verify the caller is a member of the org that owns this workspace.
async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<()> {
    let workspace = state
        .store
        .find_cloud_workspace_by_id(workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

    let org_ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    if workspace.org_id != org_ctx.org_id {
        return Err(ApiError::PermissionDenied);
    }
    Ok(())
}

// ====================================================================
// Response types — match the frontend `EnvironmentSummary` shape
// ====================================================================

#[derive(Debug, Serialize)]
pub struct EnvironmentSummaryResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub variable_count: i64,
    pub is_global: bool,
    pub active: bool,
    pub color: Option<serde_json::Value>,
}

impl From<WorkspaceEnvironmentSummary> for EnvironmentSummaryResponse {
    fn from(s: WorkspaceEnvironmentSummary) -> Self {
        Self {
            id: s.id,
            name: s.name,
            description: s.description,
            variable_count: s.variable_count,
            is_global: s.is_global,
            active: s.is_active,
            color: s.color,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EnvironmentListResponse {
    pub environments: Vec<EnvironmentSummaryResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct EnvironmentVariableResponse {
    pub name: String,
    pub value: String,
    pub is_secret: bool,
}

impl From<WorkspaceEnvironmentVariable> for EnvironmentVariableResponse {
    fn from(v: WorkspaceEnvironmentVariable) -> Self {
        Self {
            name: v.name,
            value: v.value,
            is_secret: v.is_secret,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EnvironmentVariablesResponse {
    pub variables: Vec<EnvironmentVariableResponse>,
}

// ====================================================================
// Request types
// ====================================================================

#[derive(Debug, Deserialize)]
pub struct CreateEnvironmentRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub color: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct CreateEnvironmentResponse {
    pub id: Uuid,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnvironmentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct SetVariableRequest {
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub is_secret: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnvironmentsOrderRequest {
    pub environment_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// ====================================================================
// Handlers
// ====================================================================

/// `GET /api/v1/workspaces/{workspace_id}/environments`
pub async fn list_environments(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<EnvironmentListResponse>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    let summaries = state.store.list_workspace_environments(workspace_id).await?;
    let environments: Vec<EnvironmentSummaryResponse> =
        summaries.into_iter().map(Into::into).collect();
    let total = environments.len();
    Ok(Json(EnvironmentListResponse {
        environments,
        total,
    }))
}

/// `POST /api/v1/workspaces/{workspace_id}/environments`
pub async fn create_environment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateEnvironmentRequest>,
) -> ApiResult<Json<CreateEnvironmentResponse>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    let name = request.name.trim();
    if name.is_empty() {
        return Err(ApiError::InvalidRequest("Environment name is required".to_string()));
    }

    let env = state
        .store
        .create_workspace_environment(
            workspace_id,
            name,
            &request.description,
            request.color.as_ref(),
        )
        .await?;

    Ok(Json(CreateEnvironmentResponse {
        id: env.id,
        message: "Environment created".to_string(),
    }))
}

/// `PUT /api/v1/workspaces/{workspace_id}/environments/{environment_id}`
pub async fn update_environment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateEnvironmentRequest>,
) -> ApiResult<Json<MessageResponse>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let env = require_env_in_workspace(&state, workspace_id, environment_id).await?;
    if env.is_global && request.name.is_some() {
        return Err(ApiError::InvalidRequest("Cannot rename the global environment".to_string()));
    }

    state
        .store
        .update_workspace_environment(
            environment_id,
            request.name.as_deref(),
            request.description.as_deref(),
            request.color.as_ref(),
        )
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Environment not found".to_string()))?;

    Ok(Json(MessageResponse {
        message: "Environment updated".to_string(),
    }))
}

/// `DELETE /api/v1/workspaces/{workspace_id}/environments/{environment_id}`
pub async fn delete_environment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<MessageResponse>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let env = require_env_in_workspace(&state, workspace_id, environment_id).await?;
    if env.is_global {
        return Err(ApiError::InvalidRequest("Cannot delete the global environment".to_string()));
    }

    state.store.delete_workspace_environment(environment_id).await?;
    Ok(Json(MessageResponse {
        message: "Environment deleted".to_string(),
    }))
}

/// `POST /api/v1/workspaces/{workspace_id}/environments/{environment_id}/activate`
pub async fn activate_environment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<MessageResponse>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    require_env_in_workspace(&state, workspace_id, environment_id).await?;

    state
        .store
        .set_active_workspace_environment(workspace_id, environment_id)
        .await?;
    Ok(Json(MessageResponse {
        message: "Environment activated".to_string(),
    }))
}

/// `PUT /api/v1/workspaces/{workspace_id}/environments/order`
pub async fn reorder_environments(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<UpdateEnvironmentsOrderRequest>,
) -> ApiResult<Json<MessageResponse>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    state
        .store
        .reorder_workspace_environments(workspace_id, &request.environment_ids)
        .await?;
    Ok(Json(MessageResponse {
        message: "Environment order updated".to_string(),
    }))
}

/// `GET /api/v1/workspaces/{workspace_id}/environments/{environment_id}/variables`
pub async fn list_variables(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<EnvironmentVariablesResponse>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    require_env_in_workspace(&state, workspace_id, environment_id).await?;

    let variables = state.store.list_workspace_environment_variables(environment_id).await?;
    Ok(Json(EnvironmentVariablesResponse {
        variables: variables.into_iter().map(Into::into).collect(),
    }))
}

/// `POST /api/v1/workspaces/{workspace_id}/environments/{environment_id}/variables`
pub async fn set_variable(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<SetVariableRequest>,
) -> ApiResult<Json<MessageResponse>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    require_env_in_workspace(&state, workspace_id, environment_id).await?;

    let name = request.name.trim();
    if name.is_empty() {
        return Err(ApiError::InvalidRequest("Variable name is required".to_string()));
    }

    state
        .store
        .upsert_workspace_environment_variable(
            environment_id,
            name,
            &request.value,
            request.is_secret,
        )
        .await?;
    Ok(Json(MessageResponse {
        message: "Variable set".to_string(),
    }))
}

/// `DELETE /api/v1/workspaces/{workspace_id}/environments/{environment_id}/variables/{variable_name}`
pub async fn remove_variable(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id, variable_name)): Path<(Uuid, Uuid, String)>,
) -> ApiResult<Json<MessageResponse>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    require_env_in_workspace(&state, workspace_id, environment_id).await?;

    state
        .store
        .delete_workspace_environment_variable(environment_id, &variable_name)
        .await?;
    Ok(Json(MessageResponse {
        message: "Variable removed".to_string(),
    }))
}

// ====================================================================
// Internal helpers
// ====================================================================

async fn require_env_in_workspace(
    state: &AppState,
    workspace_id: Uuid,
    environment_id: Uuid,
) -> ApiResult<WorkspaceEnvironment> {
    let env = state
        .store
        .find_workspace_environment_by_id(environment_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Environment not found".to_string()))?;
    if env.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Environment does not belong to this workspace".to_string(),
        ));
    }
    Ok(env)
}
