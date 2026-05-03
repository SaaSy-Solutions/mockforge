//! Capture sessions + behavioral-clone model handlers
//! (cloud-enablement task #6 / Phase 1).
//!
//! Phase 1 surface: capture-session CRUD + member management,
//! clone-model read paths + create-training row. Actual training
//! worker / replay endpoint / per-capture cloud-shipping land in
//! follow-up slices.
//!
//! Routes:
//!   GET    /api/v1/workspaces/{workspace_id}/capture-sessions
//!   POST   /api/v1/workspaces/{workspace_id}/capture-sessions
//!   PATCH  /api/v1/capture-sessions/{id}/members         (add/remove)
//!   DELETE /api/v1/capture-sessions/{id}
//!
//!   GET    /api/v1/workspaces/{workspace_id}/clone-models
//!   POST   /api/v1/capture-sessions/{id}/train          (enqueues training)
//!   GET    /api/v1/clone-models/{id}
//!   DELETE /api/v1/clone-models/{id}

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::usage::effective_limits,
    middleware::{resolve_org_context, AuthUser},
    models::{CaptureSession, CloneModel, CloudWorkspace},
    AppState,
};

// --- capture sessions ------------------------------------------------------

/// `GET /api/v1/workspaces/{workspace_id}/capture-sessions`
pub async fn list_sessions(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<CaptureSession>>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let rows = CaptureSession::list_by_workspace(state.db.pool(), workspace_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// `POST /api/v1/workspaces/{workspace_id}/capture-sessions`
pub async fn create_session(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateSessionRequest>,
) -> ApiResult<Json<CaptureSession>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    let row = CaptureSession::create(
        state.db.pool(),
        workspace_id,
        &request.name,
        request.description.as_deref(),
        Some(user_id),
    )
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(row))
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum MembersOp {
    Add { capture_id: Uuid },
    Remove { capture_id: Uuid },
}

/// `PATCH /api/v1/capture-sessions/{id}/members`
///
/// Body: `{"op": "add", "capture_id": "..."}` or
///       `{"op": "remove", "capture_id": "..."}`. Idempotent — repeated
/// adds/removes are no-ops.
pub async fn modify_session_members(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(op): Json<MembersOp>,
) -> ApiResult<Json<serde_json::Value>> {
    let session = load_authorized_session(&state, user_id, &headers, id).await?;
    let changed = match op {
        MembersOp::Add { capture_id } => {
            CaptureSession::add_member(state.db.pool(), session.id, capture_id)
                .await
                .map_err(ApiError::Database)?
        }
        MembersOp::Remove { capture_id } => {
            CaptureSession::remove_member(state.db.pool(), session.id, capture_id)
                .await
                .map_err(ApiError::Database)?
        }
    };
    Ok(Json(serde_json::json!({ "changed": changed })))
}

/// `DELETE /api/v1/capture-sessions/{id}`
pub async fn delete_session(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    load_authorized_session(&state, user_id, &headers, id).await?;
    let deleted = CaptureSession::delete(state.db.pool(), id).await.map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Capture session not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

// --- clone models ----------------------------------------------------------

/// `GET /api/v1/workspaces/{workspace_id}/clone-models`
pub async fn list_clone_models(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<CloneModel>>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let rows = CloneModel::list_by_workspace(state.db.pool(), workspace_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct TrainCloneRequest {
    pub name: String,
}

/// `POST /api/v1/capture-sessions/{id}/train`
///
/// Inserts a clone_model row in `training` state. Worker enqueue is a
/// follow-up slice; the row alone lets the UI render the in-progress
/// training card.
pub async fn train_clone_from_session(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<TrainCloneRequest>,
) -> ApiResult<Json<CloneModel>> {
    let session = load_authorized_session(&state, user_id, &headers, session_id).await?;
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }

    let workspace = CloudWorkspace::find_by_id(state.db.pool(), session.workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;

    // Plan-limit check on max_clone_models.
    let limits = effective_limits(&state, &load_org(&state, workspace.org_id).await?).await?;
    let max_clones = limits.get("max_clone_models").and_then(|v| v.as_i64()).unwrap_or(0);
    if max_clones == 0 {
        return Err(ApiError::ResourceLimitExceeded(
            "Behavioral cloning is not enabled on this plan".into(),
        ));
    }

    let row = CloneModel::create_training(
        state.db.pool(),
        workspace.org_id,
        session.workspace_id,
        Some(session.id),
        &request.name,
    )
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(row))
}

/// `GET /api/v1/clone-models/{id}`
pub async fn get_clone_model(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<CloneModel>> {
    let model = load_authorized_clone(&state, user_id, &headers, id).await?;
    Ok(Json(model))
}

/// `DELETE /api/v1/clone-models/{id}`
pub async fn delete_clone_model(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    load_authorized_clone(&state, user_id, &headers, id).await?;
    let deleted = CloneModel::delete(state.db.pool(), id).await.map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Clone model not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<()> {
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Workspace not found".into()));
    }
    Ok(())
}

async fn load_authorized_session(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<CaptureSession> {
    let session = CaptureSession::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Capture session not found".into()))?;
    authorize_workspace(state, user_id, headers, session.workspace_id).await?;
    Ok(session)
}

async fn load_authorized_clone(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<CloneModel> {
    let model = CloneModel::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Clone model not found".into()))?;
    authorize_workspace(state, user_id, headers, model.workspace_id).await?;
    Ok(model)
}

async fn load_org(
    state: &AppState,
    org_id: Uuid,
) -> ApiResult<mockforge_registry_core::models::Organization> {
    use mockforge_registry_core::models::Organization;
    Organization::find_by_id(state.db.pool(), org_id)
        .await
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("DB error loading org")))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".into()))
}
