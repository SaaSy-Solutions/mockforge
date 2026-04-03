//! Workspace CRUD handlers for cloud mode

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
        cloud_workspace::WorkspaceSummaryResponse, record_audit_event, AuditEventType,
        CloudWorkspace, FeatureType, FeatureUsage,
    },
    AppState,
};

/// List all workspaces for the user's organization
pub async fn list_workspaces(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<WorkspaceSummaryResponse>>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let workspaces = CloudWorkspace::find_by_org(pool, org_ctx.org_id)
        .await
        .map_err(ApiError::Database)?;

    let summaries: Vec<WorkspaceSummaryResponse> =
        workspaces.iter().map(|w| w.to_summary()).collect();

    Ok(Json(summaries))
}

/// Get a single workspace by ID
pub async fn get_workspace(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<WorkspaceSummaryResponse>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let workspace = CloudWorkspace::find_by_id(pool, id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

    if workspace.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Workspace does not belong to this organization".to_string(),
        ));
    }

    Ok(Json(workspace.to_summary()))
}

/// Create a new workspace
#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

pub async fn create_workspace(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<CreateWorkspaceRequest>,
) -> ApiResult<Json<WorkspaceSummaryResponse>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("Workspace name is required".to_string()));
    }

    let workspace = CloudWorkspace::create(
        pool,
        org_ctx.org_id,
        user_id,
        request.name.trim(),
        &request.description,
    )
    .await
    .map_err(ApiError::Database)?;

    let _ = FeatureUsage::record(
        pool,
        org_ctx.org_id,
        Some(user_id),
        FeatureType::WorkspaceCreate,
        Some(serde_json::json!({
            "workspace_id": workspace.id,
            "name": workspace.name,
        })),
    )
    .await;

    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::WorkspaceCreated,
        format!("Workspace '{}' created", workspace.name),
        Some(serde_json::json!({ "workspace_id": workspace.id, "name": workspace.name })),
        ip_address,
        user_agent,
    )
    .await;

    Ok(Json(workspace.to_summary()))
}

/// Update a workspace
#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub settings: Option<serde_json::Value>,
}

pub async fn update_workspace(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateWorkspaceRequest>,
) -> ApiResult<Json<WorkspaceSummaryResponse>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let existing = CloudWorkspace::find_by_id(pool, id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

    if existing.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Workspace does not belong to this organization".to_string(),
        ));
    }

    let workspace = CloudWorkspace::update(
        pool,
        id,
        request.name.as_deref(),
        request.description.as_deref(),
        request.is_active,
        request.settings.as_ref(),
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

    let _ = FeatureUsage::record(
        pool,
        org_ctx.org_id,
        Some(user_id),
        FeatureType::WorkspaceUpdate,
        Some(serde_json::json!({ "workspace_id": workspace.id })),
    )
    .await;

    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::WorkspaceUpdated,
        format!("Workspace '{}' updated", workspace.name),
        Some(serde_json::json!({ "workspace_id": workspace.id })),
        ip_address,
        user_agent,
    )
    .await;

    Ok(Json(workspace.to_summary()))
}

/// Delete a workspace
pub async fn delete_workspace(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let workspace = CloudWorkspace::find_by_id(pool, id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

    if workspace.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Workspace does not belong to this organization".to_string(),
        ));
    }

    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::WorkspaceDeleted,
        format!("Workspace '{}' deleted", workspace.name),
        Some(serde_json::json!({ "workspace_id": workspace.id, "name": workspace.name })),
        ip_address,
        user_agent,
    )
    .await;

    let _ = FeatureUsage::record(
        pool,
        org_ctx.org_id,
        Some(user_id),
        FeatureType::WorkspaceDelete,
        Some(serde_json::json!({ "workspace_id": workspace.id })),
    )
    .await;

    CloudWorkspace::delete(pool, id).await.map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({ "success": true })))
}
