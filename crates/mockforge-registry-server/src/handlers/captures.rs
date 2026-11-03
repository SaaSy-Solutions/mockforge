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
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let rows = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(
            async move { Ok(CaptureSession::list_by_workspace(&mut **tx, workspace_id).await?) },
        )
    })
    .await?;
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
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    let row = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(CaptureSession::create(
                &mut **tx,
                workspace_id,
                &request.name,
                request.description.as_deref(),
                Some(user_id),
            )
            .await?)
        })
    })
    .await?;
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
    let (session, org_id) = load_authorized_session_with_org(&state, user_id, &headers, id).await?;
    if let MembersOp::Add { capture_id } = &op {
        ensure_capture_in_org(&state, org_id, *capture_id).await?;
    }
    let sid = session.id;
    let changed = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(match op {
                MembersOp::Add { capture_id } => {
                    CaptureSession::add_member(&mut **tx, sid, capture_id).await?
                }
                MembersOp::Remove { capture_id } => {
                    CaptureSession::remove_member(&mut **tx, sid, capture_id).await?
                }
            })
        })
    })
    .await?;
    Ok(Json(serde_json::json!({ "changed": changed })))
}

/// `DELETE /api/v1/capture-sessions/{id}`
pub async fn delete_session(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let (_, org_id) = load_authorized_session_with_org(&state, user_id, &headers, id).await?;
    let deleted = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(CaptureSession::delete(&mut **tx, id).await?) })
    })
    .await?;
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
/// Behavioral clone training remains unavailable until a real trainer is
/// connected to this route.
pub async fn train_clone_from_session(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
    Json(_request): Json<TrainCloneRequest>,
) -> ApiResult<Json<CloneModel>> {
    load_authorized_session_with_org(&state, user_id, &headers, session_id).await?;
    Err(ApiError::InvalidRequest("behavioral clone training isn't available yet".into()))
}

/// `POST /api/v1/capture-sessions/{id}/replay`
///
/// Replays the capture session against a required target URL.
#[derive(Debug, Deserialize)]
pub struct ReplaySessionRequest {
    #[serde(default)]
    pub target_url: Option<String>,
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
    let target_url = request
        .target_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::InvalidRequest("target_url must not be empty".into()))?
        .to_string();

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

    let payload = serde_json::json!({
        "session_id": session.id,
        "target_url": target_url,
    });

    if let Err(e) = crate::run_queue::enqueue(
        state.redis.as_ref(),
        crate::run_queue::EnqueuedJob {
            run_id: run.id,
            org_id: run.org_id,
            source_id: session.id,
            kind: "replay",
            payload,
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

/// A row that loaded but whose workspace fails the org check must answer
/// exactly like a missing row, or the difference tells a caller which ids
/// exist in other tenants. RLS already hides foreign rows; this keeps the
/// answer identical when RLS is bypassed (the owner-role rollback path).
/// Storage errors still propagate.
fn as_not_found(err: ApiError, message: &str) -> ApiError {
    match err {
        ApiError::InvalidRequest(_) => ApiError::InvalidRequest(message.into()),
        other => other,
    }
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

/// A session member must be a capture recorded by a deployment in `org_id`.
/// The replay executor ships every member's full request/response bodies
/// back into the session's run, so an unchecked id would let one org read
/// another org's traffic. A foreign or unknown capture reads the same.
async fn ensure_capture_in_org(state: &AppState, org_id: Uuid, capture_id: Uuid) -> ApiResult<()> {
    let owned = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            // `capture_id` is TEXT; the CASE guards the cast so a malformed
            // legacy row cannot fail the whole query with 22P02.
            Ok(sqlx::query_scalar::<_, bool>(
                r#"
                SELECT EXISTS (
                    SELECT 1
                      FROM runtime_captures rc
                      JOIN hosted_mocks hm ON hm.id = rc.deployment_id
                     WHERE hm.org_id = $1
                       AND CASE WHEN rc.capture_id ~* '^[0-9a-f]{8}-?[0-9a-f]{4}-?[0-9a-f]{4}-?[0-9a-f]{4}-?[0-9a-f]{12}$'
                                THEN rc.capture_id::uuid END = $2
                )
                "#,
            )
            .bind(org_id)
            .bind(capture_id)
            .fetch_one(&mut **tx)
            .await?)
        })
    })
    .await?;
    if !owned {
        return Err(ApiError::InvalidRequest("Capture not found".into()));
    }
    Ok(())
}

/// Load a capture session bound to the caller's org (a session in another
/// org reads as absent -> "Capture session not found"), then verify its
/// workspace. Returns the session and the authorized org id.
async fn load_authorized_session_with_org(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<(CaptureSession, Uuid)> {
    let org_id = resolve_org_id(state, user_id, headers).await?;
    let session = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(CaptureSession::find_by_id(&mut **tx, id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Capture session not found".into()))?;
    authorize_workspace_in_org(state, org_id, session.workspace_id)
        .await
        .map_err(|e| as_not_found(e, "Capture session not found"))?;
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
    authorize_workspace_in_org(state, org_id, model.workspace_id)
        .await
        .map_err(|e| as_not_found(e, "Clone model not found"))?;
    Ok((model, org_id))
}
