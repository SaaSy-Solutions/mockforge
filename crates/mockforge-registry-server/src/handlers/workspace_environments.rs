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
    store::with_org_context,
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

    let workspace = with_org_context(state.db.runtime_pool(), org_ctx.org_id, |tx| {
        Box::pin(async move { Ok(CloudWorkspace::find_by_id(&mut **tx, workspace_id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

    if workspace.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Workspace does not belong to this organization".to_string(),
        ));
    }

    Ok(workspace)
}

/// Load `environment_id` bound to `org_id` and confirm it belongs to
/// `workspace_id`. An environment in another org reads as `None` under RLS and
/// maps to the same "Environment not found" error as a missing row.
async fn find_env_in_workspace(
    state: &AppState,
    org_id: Uuid,
    workspace_id: Uuid,
    environment_id: Uuid,
) -> ApiResult<WorkspaceEnvironment> {
    let env = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(
            async move { Ok(WorkspaceEnvironment::find_by_id(&mut **tx, environment_id).await?) },
        )
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Environment not found".to_string()))?;
    if env.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Environment does not belong to this workspace".to_string(),
        ));
    }
    Ok(env)
}

/// `conn` must be bound to the workspace's org (callers run this inside
/// `with_org_context`).
async fn build_summary(
    conn: &mut sqlx::PgConnection,
    env: &WorkspaceEnvironment,
) -> sqlx::Result<EnvironmentSummaryResponse> {
    let variable_count = WorkspaceEnvironment::variable_count(&mut *conn, env.id).await?;
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
    let org_id = require_workspace(&state, user_id, &headers, workspace_id).await?.org_id;

    let summaries = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            let envs = WorkspaceEnvironment::list_by_workspace(&mut **tx, workspace_id).await?;
            let mut summaries = Vec::with_capacity(envs.len());
            for env in &envs {
                summaries.push(build_summary(tx, env).await?);
            }
            Ok(summaries)
        })
    })
    .await?;
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
    let org_id = require_workspace(&state, user_id, &headers, workspace_id).await?.org_id;

    let name = request.name.trim();
    if name.is_empty() {
        return Err(ApiError::InvalidRequest("Environment name is required".to_string()));
    }

    let color = request.color.unwrap_or_else(|| ColorInput {
        hex: String::new(),
        name: String::new(),
    });

    let name_owned = name.to_string();
    let description = request.description.clone();
    // The closure returns the raw sqlx result so the unique-violation mapping
    // below still sees the `sqlx::Error`.
    let env = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(WorkspaceEnvironment::create(
                tx,
                workspace_id,
                &name_owned,
                &description,
                &color.hex,
                &color.name,
            )
            .await)
        })
    })
    .await?
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
    let org_id = require_workspace(&state, user_id, &headers, workspace_id).await?.org_id;

    find_env_in_workspace(&state, org_id, workspace_id, environment_id).await?;

    let (color_hex, color_name) = match request.color {
        Some(c) => (Some(c.hex), Some(c.name)),
        None => (None, None),
    };

    with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(WorkspaceEnvironment::update(
                &mut **tx,
                environment_id,
                request.name.as_deref(),
                request.description.as_deref(),
                color_hex.as_deref(),
                color_name.as_deref(),
            )
            .await?)
        })
    })
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
    let org_id = require_workspace(&state, user_id, &headers, workspace_id).await?.org_id;

    find_env_in_workspace(&state, org_id, workspace_id, environment_id).await?;

    with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(WorkspaceEnvironment::delete(&mut **tx, environment_id).await?) })
    })
    .await?;
    Ok(Json(serde_json::json!({ "message": "Environment deleted" })))
}

/// POST /api/v1/workspaces/{workspace_id}/environments/{environment_id}/activate
pub async fn activate_environment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = require_workspace(&state, user_id, &headers, workspace_id).await?.org_id;

    let activated = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(WorkspaceEnvironment::set_active(&mut **tx, workspace_id, environment_id).await?)
        })
    })
    .await?;
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
    let org_id = require_workspace(&state, user_id, &headers, workspace_id).await?.org_id;

    with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(WorkspaceEnvironment::reorder(&mut **tx, workspace_id, &request.environment_ids)
                .await?)
        })
    })
    .await?;
    Ok(Json(serde_json::json!({ "message": "Environment order updated" })))
}

/// GET /api/v1/workspaces/{workspace_id}/environments/{environment_id}/variables
pub async fn list_variables(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = require_workspace(&state, user_id, &headers, workspace_id).await?.org_id;

    find_env_in_workspace(&state, org_id, workspace_id, environment_id).await?;

    let vars = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(WorkspaceEnvVariable::list_by_environment(&mut **tx, environment_id).await?)
        })
    })
    .await?;
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
    let org_id = require_workspace(&state, user_id, &headers, workspace_id).await?.org_id;

    find_env_in_workspace(&state, org_id, workspace_id, environment_id).await?;

    let key = request.key.trim();
    if key.is_empty() {
        return Err(ApiError::InvalidRequest("Variable name is required".to_string()));
    }

    let key = key.to_string();
    with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(WorkspaceEnvVariable::upsert(
                &mut **tx,
                environment_id,
                &key,
                &request.value,
                request.encrypted,
            )
            .await?)
        })
    })
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
    let org_id = require_workspace(&state, user_id, &headers, workspace_id).await?.org_id;

    find_env_in_workspace(&state, org_id, workspace_id, environment_id).await?;

    let deleted = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(WorkspaceEnvVariable::delete(&mut **tx, environment_id, &variable_name).await?)
        })
    })
    .await?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Variable not found".to_string()));
    }
    Ok(Json(serde_json::json!({ "message": "Variable deleted" })))
}
