//! Workspace environment + variable handlers.
//!
//! Matches the self-hosted `/__mockforge/workspaces/{id}/environments/*` surface so the
//! cloud UI (which calls `/api/v1/workspaces/{id}/environments/*`) works end-to-end.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{
        workspace_environment::{
            EnvironmentSummaryResponse, WorkspaceEnvVariable, WorkspaceEnvironment,
        },
        CloudWorkspace,
    },
    AppState,
};

async fn require_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<CloudWorkspace> {
    let org_ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

    if workspace.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Workspace does not belong to this organization".to_string(),
        ));
    }

    Ok(workspace)
}

async fn build_summary(
    pool: &sqlx::PgPool,
    env: &WorkspaceEnvironment,
) -> ApiResult<EnvironmentSummaryResponse> {
    let variable_count = WorkspaceEnvironment::variable_count(pool, env.id).await?;
    Ok(EnvironmentSummaryResponse {
        id: env.id,
        name: env.name.clone(),
        description: env.description.clone(),
        variable_count,
        is_global: false,
        active: env.is_active,
        color: env.color_response(),
        order: env.sort_order,
    })
}

/// GET /api/v1/workspaces/{workspace_id}/environments
pub async fn list_environments(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    let envs = WorkspaceEnvironment::list_by_workspace(state.db.pool(), workspace_id).await?;
    let mut summaries = Vec::with_capacity(envs.len());
    for env in &envs {
        summaries.push(build_summary(state.db.pool(), env).await?);
    }
    let total = summaries.len();
    Ok(Json(serde_json::json!({
        "environments": summaries,
        "total": total,
    })))
}

#[derive(Debug, Deserialize)]
pub struct CreateEnvironmentRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub color: Option<ColorInput>,
}

#[derive(Debug, Deserialize)]
pub struct ColorInput {
    #[serde(default)]
    pub hex: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CreateEnvironmentResponse {
    pub id: Uuid,
    pub message: String,
}

/// POST /api/v1/workspaces/{workspace_id}/environments
pub async fn create_environment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateEnvironmentRequest>,
) -> ApiResult<Json<CreateEnvironmentResponse>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    let name = request.name.trim();
    if name.is_empty() {
        return Err(ApiError::InvalidRequest("Environment name is required".to_string()));
    }

    let color = request.color.unwrap_or_else(|| ColorInput {
        hex: String::new(),
        name: String::new(),
    });

    let env = WorkspaceEnvironment::create(
        state.db.pool(),
        workspace_id,
        name,
        &request.description,
        &color.hex,
        &color.name,
    )
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.is_unique_violation() => ApiError::InvalidRequest(
            format!("An environment named '{name}' already exists in this workspace"),
        ),
        other => ApiError::Database(other),
    })?;

    Ok(Json(CreateEnvironmentResponse {
        id: env.id,
        message: "Environment created".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnvironmentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<ColorInput>,
}

/// PUT /api/v1/workspaces/{workspace_id}/environments/{environment_id}
pub async fn update_environment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateEnvironmentRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    let existing = WorkspaceEnvironment::find_by_id(state.db.pool(), environment_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Environment not found".to_string()))?;
    if existing.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Environment does not belong to this workspace".to_string(),
        ));
    }

    let (color_hex, color_name) = match request.color {
        Some(c) => (Some(c.hex), Some(c.name)),
        None => (None, None),
    };

    WorkspaceEnvironment::update(
        state.db.pool(),
        environment_id,
        request.name.as_deref(),
        request.description.as_deref(),
        color_hex.as_deref(),
        color_name.as_deref(),
    )
    .await?;

    Ok(Json(serde_json::json!({ "message": "Environment updated" })))
}

/// DELETE /api/v1/workspaces/{workspace_id}/environments/{environment_id}
pub async fn delete_environment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    let existing = WorkspaceEnvironment::find_by_id(state.db.pool(), environment_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Environment not found".to_string()))?;
    if existing.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Environment does not belong to this workspace".to_string(),
        ));
    }

    WorkspaceEnvironment::delete(state.db.pool(), environment_id).await?;
    Ok(Json(serde_json::json!({ "message": "Environment deleted" })))
}

/// POST /api/v1/workspaces/{workspace_id}/environments/{environment_id}/activate
pub async fn activate_environment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    let activated =
        WorkspaceEnvironment::set_active(state.db.pool(), workspace_id, environment_id).await?;
    if activated.is_none() {
        return Err(ApiError::InvalidRequest("Environment not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "message": "Environment activated" })))
}

#[derive(Debug, Deserialize)]
pub struct EnvironmentOrderRequest {
    pub environment_ids: Vec<Uuid>,
}

/// PUT /api/v1/workspaces/{workspace_id}/environments/order
pub async fn reorder_environments(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<EnvironmentOrderRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    WorkspaceEnvironment::reorder(state.db.pool(), workspace_id, &request.environment_ids).await?;
    Ok(Json(serde_json::json!({ "message": "Environment order updated" })))
}

/// GET /api/v1/workspaces/{workspace_id}/environments/{environment_id}/variables
pub async fn list_variables(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    let env = WorkspaceEnvironment::find_by_id(state.db.pool(), environment_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Environment not found".to_string()))?;
    if env.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Environment does not belong to this workspace".to_string(),
        ));
    }

    let vars = WorkspaceEnvVariable::list_by_environment(state.db.pool(), environment_id).await?;
    let variables: Vec<_> = vars.iter().map(|v| v.to_response()).collect();
    Ok(Json(serde_json::json!({ "variables": variables })))
}

#[derive(Debug, Deserialize)]
pub struct SetVariableRequest {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub encrypted: bool,
}

/// POST /api/v1/workspaces/{workspace_id}/environments/{environment_id}/variables
pub async fn set_variable(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<SetVariableRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    let env = WorkspaceEnvironment::find_by_id(state.db.pool(), environment_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Environment not found".to_string()))?;
    if env.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Environment does not belong to this workspace".to_string(),
        ));
    }

    let key = request.key.trim();
    if key.is_empty() {
        return Err(ApiError::InvalidRequest("Variable name is required".to_string()));
    }

    WorkspaceEnvVariable::upsert(
        state.db.pool(),
        environment_id,
        key,
        &request.value,
        request.encrypted,
    )
    .await?;

    Ok(Json(serde_json::json!({ "message": "Variable saved" })))
}

/// DELETE /api/v1/workspaces/{workspace_id}/environments/{environment_id}/variables/{name}
pub async fn delete_variable(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id, variable_name)): Path<(Uuid, Uuid, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    let env = WorkspaceEnvironment::find_by_id(state.db.pool(), environment_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Environment not found".to_string()))?;
    if env.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Environment does not belong to this workspace".to_string(),
        ));
    }

    let deleted =
        WorkspaceEnvVariable::delete(state.db.pool(), environment_id, &variable_name).await?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Variable not found".to_string()));
    }
    Ok(Json(serde_json::json!({ "message": "Variable deleted" })))
}
