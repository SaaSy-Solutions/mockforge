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

use mockforge_registry_core::models::test_run::EnqueueTestRun;

use crate::{
    error::{ApiError, ApiResult},
    handlers::usage::effective_limits,
    middleware::{resolve_org_context, AuthUser},
    models::{CaptureSession, CloneModel, CloudWorkspace, TestRun},
    store::with_org_context,
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
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let rows = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(CloneModel::list_by_workspace(&mut **tx, workspace_id).await?) })
    })
    .await?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct TrainCloneRequest {
    pub name: String,
}

/// `POST /api/v1/capture-sessions/{id}/train`
///
/// Creates the clone_models row in `training` state AND enqueues a
/// `behavioral_clone` test_run so the worker actually picks it up. The
/// CloneTrainExecutor reports back via internal callbacks (status flips
/// to terminal, runner_seconds metered).
pub async fn train_clone_from_session(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<TrainCloneRequest>,
) -> ApiResult<Json<CloneModel>> {
    let (session, org_id) =
        load_authorized_session_with_org(&state, user_id, &headers, session_id).await?;
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }

    let limits = effective_limits(&state, &load_org(&state, org_id).await?).await?;
    let max_clones = limits.get("max_clone_models").and_then(|v| v.as_i64()).unwrap_or(0);
    if max_clones == 0 {
        return Err(ApiError::ResourceLimitExceeded(
            "Behavioral cloning is not enabled on this plan".into(),
        ));
    }

    let (workspace_id, sid) = (session.workspace_id, session.id);
    let name = request.name.clone();
    let (row, run) = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            let row =
                CloneModel::create_training(&mut **tx, org_id, workspace_id, Some(sid), &name)
                    .await?;
            let run = TestRun::enqueue(
                &mut **tx,
                EnqueueTestRun {
                    suite_id: row.id,
                    org_id,
                    kind: "behavioral_clone",
                    triggered_by: "manual",
                    triggered_by_user: Some(user_id),
                    git_ref: None,
                    git_sha: None,
                },
            )
            .await?;
            Ok((row, run))
        })
    })
    .await?;

    if let Err(e) = crate::run_queue::enqueue(
        state.redis.as_ref(),
        crate::run_queue::EnqueuedJob {
            run_id: run.id,
            org_id: run.org_id,
            source_id: row.id,
            kind: "behavioral_clone",
            payload: serde_json::json!({
                "session_id": session.id,
                "clone_model_id": row.id,
                "name": request.name,
            }),
        },
    )
    .await
    {
        tracing::error!(run_id = %run.id, error = %e, "failed to enqueue behavioral_clone run");
    }

    Ok(Json(row))
}

/// `POST /api/v1/capture-sessions/{id}/replay`
///
/// Triggers a synthetic replay of the capture session against a target.
/// Reuses the test_runs lifecycle with kind='replay'. The runner-side
/// ReplayExecutor synthesizes per-capture events until real impl
/// (real HTTP replay against `target_url`) lands.
#[derive(Debug, Deserialize)]
pub struct ReplaySessionRequest {
    #[serde(default)]
    pub target_url: Option<String>,
    /// How many synthetic captures the executor should pretend to replay.
    /// Optional — defaults to 5 on the runner side. Ignored once real
    /// replay lands.
    #[serde(default)]
    pub synthetic_captures: Option<u32>,
}

pub async fn replay_capture_session(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ReplaySessionRequest>,
) -> ApiResult<Json<TestRun>> {
    let (session, org_id) =
        load_authorized_session_with_org(&state, user_id, &headers, session_id).await?;

    let sid = session.id;
    let run = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(TestRun::enqueue(
                &mut **tx,
                EnqueueTestRun {
                    suite_id: sid,
                    org_id,
                    kind: "replay",
                    triggered_by: "manual",
                    triggered_by_user: Some(user_id),
                    git_ref: None,
                    git_sha: None,
                },
            )
            .await?)
        })
    })
    .await?;

    let mut payload = serde_json::Map::new();
    payload.insert("session_id".into(), serde_json::json!(session.id));
    if let Some(url) = request.target_url.as_deref() {
        payload.insert("target_url".into(), serde_json::Value::String(url.to_string()));
    }
    if let Some(n) = request.synthetic_captures {
        payload.insert("synthetic_captures".into(), serde_json::json!(n));
    }

    if let Err(e) = crate::run_queue::enqueue(
        state.redis.as_ref(),
        crate::run_queue::EnqueuedJob {
            run_id: run.id,
            org_id: run.org_id,
            source_id: session.id,
            kind: "replay",
            payload: serde_json::Value::Object(payload),
        },
    )
    .await
    {
        tracing::error!(run_id = %run.id, error = %e, "failed to enqueue replay run");
    }

    Ok(Json(run))
}

/// `GET /api/v1/clone-models/{id}`
pub async fn get_clone_model(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<CloneModel>> {
    let (model, _) = load_authorized_clone(&state, user_id, &headers, id).await?;
    Ok(Json(model))
}

/// `DELETE /api/v1/clone-models/{id}`
pub async fn delete_clone_model(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let (_, org_id) = load_authorized_clone(&state, user_id, &headers, id).await?;
    let deleted = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(CloneModel::delete(&mut **tx, id).await?) })
    })
    .await?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Clone model not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// Resolve the caller's org and verify `workspace_id` belongs to it.
/// Returns the org id so callers can bind it for follow-up queries.
async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<Uuid> {
    let org_id = resolve_org_id(state, user_id, headers).await?;
    authorize_workspace_in_org(state, org_id, workspace_id).await?;
    Ok(org_id)
}

async fn resolve_org_id(state: &AppState, user_id: Uuid, headers: &HeaderMap) -> ApiResult<Uuid> {
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    Ok(ctx.org_id)
}

/// Verify `workspace_id` belongs to `org_id`. The lookup is bound to that
/// org, so a workspace in another org reads as absent ("Workspace not
/// found", same as an explicit mismatch).
async fn authorize_workspace_in_org(
    state: &AppState,
    org_id: Uuid,
    workspace_id: Uuid,
) -> ApiResult<()> {
    let workspace = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(CloudWorkspace::find_by_id(&mut **tx, workspace_id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    if org_id != workspace.org_id {
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
    Ok(load_authorized_session_with_org(state, user_id, headers, id).await?.0)
}

/// Like [`load_authorized_session`], also returning the authorized org id.
async fn load_authorized_session_with_org(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<(CaptureSession, Uuid)> {
    let session = CaptureSession::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Capture session not found".into()))?;
    let org_id = authorize_workspace(state, user_id, headers, session.workspace_id).await?;
    Ok((session, org_id))
}

/// Load a clone model bound to the caller's org (a model in another org
/// reads as absent), then verify its workspace. Returns the org id.
async fn load_authorized_clone(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<(CloneModel, Uuid)> {
    let org_id = resolve_org_id(state, user_id, headers).await?;
    let model = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(CloneModel::find_by_id(&mut **tx, id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Clone model not found".into()))?;
    authorize_workspace_in_org(state, org_id, model.workspace_id).await?;
    Ok((model, org_id))
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
