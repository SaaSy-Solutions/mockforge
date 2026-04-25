//! Workspace CRUD handlers for cloud mode

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
    models::{
        cloud_workspace::WorkspaceSummaryResponse,
        workspace_folder::{FolderSummaryResponse, WorkspaceFolder},
        workspace_request::{RequestSummaryResponse, WorkspaceRequest},
        AuditEventType, FeatureType,
    },
    AppState,
};

async fn summarize_workspace(
    pool: &sqlx::PgPool,
    workspace: &crate::models::CloudWorkspace,
) -> ApiResult<WorkspaceSummaryResponse> {
    let folder_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workspace_folders WHERE workspace_id = $1")
            .bind(workspace.id)
            .fetch_one(pool)
            .await?;
    let request_count = WorkspaceRequest::count_in_workspace(pool, workspace.id).await?;

    let mut summary = workspace.to_summary();
    summary.folder_count = folder_count;
    summary.request_count = request_count;
    Ok(summary)
}

/// List all workspaces for the user's organization
pub async fn list_workspaces(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<WorkspaceSummaryResponse>>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let workspaces = state.store.list_cloud_workspaces_by_org(org_ctx.org_id).await?;

    let mut summaries: Vec<WorkspaceSummaryResponse> = Vec::with_capacity(workspaces.len());
    for ws in &workspaces {
        summaries.push(summarize_workspace(state.db.pool(), ws).await?);
    }

    Ok(Json(summaries))
}

/// Detailed workspace shape consumed by WorkspacesPage when a user clicks into a workspace.
#[derive(Debug, Serialize)]
pub struct WorkspaceDetailResponse {
    pub summary: WorkspaceSummaryResponse,
    pub folders: Vec<FolderSummaryResponse>,
    pub requests: Vec<RequestSummaryResponse>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceResponseEnvelope {
    pub workspace: WorkspaceDetailResponse,
}

/// Get a single workspace by ID (detail view including folders + top-level requests).
pub async fn get_workspace(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<WorkspaceResponseEnvelope>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let workspace = state
        .store
        .find_cloud_workspace_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

    if workspace.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Workspace does not belong to this organization".to_string(),
        ));
    }

    let pool = state.db.pool();
    let summary = summarize_workspace(pool, &workspace).await?;

    let folders = WorkspaceFolder::list_by_workspace(pool, id).await?;
    let mut folder_summaries: Vec<FolderSummaryResponse> = Vec::with_capacity(folders.len());
    for f in &folders {
        folder_summaries.push(f.to_summary_response(pool).await?);
    }

    let top_level_requests = WorkspaceRequest::list_by_workspace(pool, id)
        .await?
        .into_iter()
        .map(|r| r.to_summary())
        .collect::<Vec<_>>();

    Ok(Json(WorkspaceResponseEnvelope {
        workspace: WorkspaceDetailResponse {
            summary,
            folders: folder_summaries,
            requests: top_level_requests,
        },
    }))
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
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("Workspace name is required".to_string()));
    }

    let workspace = state
        .store
        .create_cloud_workspace(org_ctx.org_id, user_id, request.name.trim(), &request.description)
        .await?;

    state
        .store
        .record_feature_usage(
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

    state
        .store
        .record_audit_event(
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
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let existing = state
        .store
        .find_cloud_workspace_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

    if existing.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Workspace does not belong to this organization".to_string(),
        ));
    }

    let workspace = state
        .store
        .update_cloud_workspace(
            id,
            request.name.as_deref(),
            request.description.as_deref(),
            request.is_active,
            request.settings.as_ref(),
        )
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

    state
        .store
        .record_feature_usage(
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

    state
        .store
        .record_audit_event(
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
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let workspace = state
        .store
        .find_cloud_workspace_by_id(id)
        .await?
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

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::WorkspaceDeleted,
            format!("Workspace '{}' deleted", workspace.name),
            Some(serde_json::json!({ "workspace_id": workspace.id, "name": workspace.name })),
            ip_address,
            user_agent,
        )
        .await;

    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::WorkspaceDelete,
            Some(serde_json::json!({ "workspace_id": workspace.id })),
        )
        .await;

    state.store.delete_cloud_workspace(id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
