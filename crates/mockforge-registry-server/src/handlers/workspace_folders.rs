//! Workspace folder + request CRUD handlers.
//!
//! These match the shapes the cloud UI already types: `FolderDetail` and
//! `RequestSummary` in `crates/mockforge-ui/ui/src/types/index.ts`.

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
        workspace_folder::{FolderSummaryResponse, WorkspaceFolder},
        workspace_request::{RequestSummaryResponse, WorkspaceRequest},
        CloudWorkspace,
    },
    AppState,
};

async fn require_workspace_owner(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<()> {
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
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct FolderDetailResponse {
    pub summary: FolderSummaryResponse,
    pub requests: Vec<RequestSummaryResponse>,
}

/// GET /api/v1/workspaces/{workspace_id}/folders/{folder_id}
pub async fn get_folder(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, folder_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace_owner(&state, user_id, &headers, workspace_id).await?;

    let folder = WorkspaceFolder::find_by_id(state.db.pool(), folder_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Folder not found".to_string()))?;
    if folder.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Folder does not belong to this workspace".to_string(),
        ));
    }

    let summary = folder.to_summary_response(state.db.pool()).await?;
    let requests = WorkspaceRequest::list_by_folder(state.db.pool(), folder_id)
        .await?
        .into_iter()
        .map(|r| r.to_summary())
        .collect::<Vec<_>>();

    Ok(Json(serde_json::json!({
        "folder": FolderDetailResponse { summary, requests },
    })))
}

#[derive(Debug, Deserialize)]
pub struct CreateFolderRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
}

/// POST /api/v1/workspaces/{workspace_id}/folders
pub async fn create_folder(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateFolderRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace_owner(&state, user_id, &headers, workspace_id).await?;

    let name = request.name.trim();
    if name.is_empty() {
        return Err(ApiError::InvalidRequest("Folder name is required".to_string()));
    }

    if let Some(parent_id) = request.parent_id {
        let parent = WorkspaceFolder::find_by_id(state.db.pool(), parent_id)
            .await?
            .ok_or_else(|| ApiError::InvalidRequest("Parent folder not found".to_string()))?;
        if parent.workspace_id != workspace_id {
            return Err(ApiError::InvalidRequest(
                "Parent folder does not belong to this workspace".to_string(),
            ));
        }
    }

    let folder = WorkspaceFolder::create(
        state.db.pool(),
        workspace_id,
        request.parent_id,
        name,
        &request.description,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "id": folder.id,
        "message": "Folder created",
    })))
}

#[derive(Debug, Deserialize)]
pub struct CreateRequestRequestBody {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub method: String,
    pub path: String,
    #[serde(default = "default_status_code")]
    pub status_code: i32,
    #[serde(default)]
    pub response_body: String,
    #[serde(default)]
    pub folder_id: Option<Uuid>,
    #[serde(default)]
    pub request_headers: Option<serde_json::Value>,
    #[serde(default)]
    pub response_headers: Option<serde_json::Value>,
}

fn default_status_code() -> i32 {
    200
}

/// POST /api/v1/workspaces/{workspace_id}/requests
pub async fn create_request(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateRequestRequestBody>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace_owner(&state, user_id, &headers, workspace_id).await?;

    let name = request.name.trim();
    let path = request.path.trim();
    if name.is_empty() || path.is_empty() {
        return Err(ApiError::InvalidRequest("Request name and path are required".to_string()));
    }

    if let Some(folder_id) = request.folder_id {
        let folder = WorkspaceFolder::find_by_id(state.db.pool(), folder_id)
            .await?
            .ok_or_else(|| ApiError::InvalidRequest("Folder not found".to_string()))?;
        if folder.workspace_id != workspace_id {
            return Err(ApiError::InvalidRequest(
                "Folder does not belong to this workspace".to_string(),
            ));
        }
    }

    let method_upper = request.method.to_uppercase();
    let req_headers = request.request_headers.unwrap_or(serde_json::json!({}));
    let resp_headers = request.response_headers.unwrap_or(serde_json::json!({}));

    let created = WorkspaceRequest::create(
        state.db.pool(),
        workspace_id,
        request.folder_id,
        name,
        &request.description,
        &method_upper,
        path,
        request.status_code,
        &request.response_body,
        &req_headers,
        &resp_headers,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "id": created.id,
        "message": "Request created",
    })))
}

/// DELETE /api/v1/workspaces/{workspace_id}/folders/{folder_id}
pub async fn delete_folder(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, folder_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace_owner(&state, user_id, &headers, workspace_id).await?;

    let folder = WorkspaceFolder::find_by_id(state.db.pool(), folder_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Folder not found".to_string()))?;
    if folder.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Folder does not belong to this workspace".to_string(),
        ));
    }

    WorkspaceFolder::delete(state.db.pool(), folder_id).await?;

    Ok(Json(serde_json::json!({
        "id": folder_id,
        "message": "Folder deleted",
    })))
}

/// DELETE /api/v1/workspaces/{workspace_id}/requests/{request_id}
pub async fn delete_request(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, request_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace_owner(&state, user_id, &headers, workspace_id).await?;

    let request = WorkspaceRequest::find_by_id(state.db.pool(), request_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Request not found".to_string()))?;
    if request.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Request does not belong to this workspace".to_string(),
        ));
    }

    WorkspaceRequest::delete(state.db.pool(), request_id).await?;

    Ok(Json(serde_json::json!({
        "id": request_id,
        "message": "Request deleted",
    })))
}
