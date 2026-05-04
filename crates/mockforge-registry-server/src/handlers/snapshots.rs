//! Time Travel snapshot handlers (cloud-enablement task #10 / Phase 1).
//!
//! Phase 1 surface only — capture-trigger + read paths + delete. The
//! actual capture worker (consumes 'capturing' rows from the test_runs
//! queue with kind='snapshot_capture') and restore worker land in
//! follow-up slices.
//!
//! Routes:
//!   GET    /api/v1/workspaces/{workspace_id}/snapshots
//!   POST   /api/v1/workspaces/{workspace_id}/snapshots         (trigger capture)
//!   GET    /api/v1/snapshots/{id}
//!   DELETE /api/v1/snapshots/{id}

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{Duration, Utc};
use mockforge_registry_core::models::snapshot::CreateSnapshot;
use mockforge_registry_core::models::test_run::EnqueueTestRun;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::usage::effective_limits,
    middleware::{resolve_org_context, AuthUser},
    models::{CloudWorkspace, Snapshot, TestRun, UsageCounter},
    AppState,
};

const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 500;

#[derive(Debug, Deserialize)]
pub struct ListSnapshotsQuery {
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `GET /api/v1/workspaces/{workspace_id}/snapshots`
pub async fn list_snapshots(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListSnapshotsQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Snapshot>>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let snapshots = Snapshot::list_by_workspace(state.db.pool(), workspace_id, limit)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(snapshots))
}

#[derive(Debug, Deserialize)]
pub struct CaptureSnapshotRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub hosted_deployment_id: Option<Uuid>,
    /// Defaults to "manual". Other valid values: "schedule", "pre_chaos",
    /// "pre_restore" — used by internal callers, not external API users.
    #[serde(default)]
    pub triggered_by: Option<String>,
}

/// `POST /api/v1/workspaces/{workspace_id}/snapshots`
///
/// Inserts a row in `capturing` state and (eventually) enqueues the
/// capture worker. Worker enqueue is a follow-up slice; the row alone
/// is enough for the UI to render an in-progress capture.
pub async fn capture_snapshot(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CaptureSnapshotRequest>,
) -> ApiResult<Json<Snapshot>> {
    let ctx = authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    // Plan-limit checks.
    let limits = effective_limits(&state, &ctx.org).await?;
    let max_snapshots = limits.get("max_snapshots").and_then(|v| v.as_i64()).unwrap_or(0);
    if max_snapshots == 0 {
        return Err(ApiError::ResourceLimitExceeded(
            "Time Travel snapshots are not enabled on this plan".into(),
        ));
    }
    if max_snapshots > 0 {
        let used = Snapshot::count_by_workspace(state.db.pool(), workspace_id)
            .await
            .map_err(ApiError::Database)?;
        if used >= max_snapshots {
            return Err(ApiError::ResourceLimitExceeded(format!(
                "Snapshot limit reached ({used}/{max_snapshots}). Delete an old \
                 snapshot or upgrade your plan."
            )));
        }
    }

    // triggered_by validation. Only `manual` is accepted on the public
    // route; the schedule worker / chaos/restore hooks call the model
    // directly and don't go through this handler.
    let triggered_by = request.triggered_by.as_deref().unwrap_or("manual");
    if triggered_by != "manual" {
        return Err(ApiError::InvalidRequest(
            "triggered_by must be 'manual' for user-initiated captures".into(),
        ));
    }

    // expires_at = created_at + plan retention days.
    let retention_days =
        limits.get("snapshot_retention_days").and_then(|v| v.as_i64()).unwrap_or(7);
    let expires_at = if retention_days > 0 {
        Some(Utc::now() + Duration::days(retention_days))
    } else {
        None
    };

    let snapshot = Snapshot::create(
        state.db.pool(),
        CreateSnapshot {
            workspace_id,
            hosted_deployment_id: request.hosted_deployment_id,
            name: request.name.as_deref(),
            description: request.description.as_deref(),
            triggered_by,
            triggered_by_user: Some(user_id),
            expires_at,
        },
    )
    .await
    .map_err(ApiError::Database)?;

    // Pair the snapshot row with a test_runs row so it shares the runner
    // pool + concurrency cap + runner_seconds metering with everything
    // else. Snapshot.status remains 'capturing' until the executor
    // reports back via Snapshot::mark_ready (separate slice — for now
    // the worker only updates test_runs, leaving snapshot.status as a
    // known follow-up gap).
    let run = TestRun::enqueue(
        state.db.pool(),
        EnqueueTestRun {
            suite_id: snapshot.id,
            org_id: ctx.org_id,
            triggered_by: "manual",
            triggered_by_user: Some(user_id),
            git_ref: None,
            git_sha: None,
        },
    )
    .await
    .map_err(ApiError::Database)?;

    if let Err(e) = crate::run_queue::enqueue(
        state.redis.as_ref(),
        crate::run_queue::EnqueuedJob {
            run_id: run.id,
            org_id: run.org_id,
            source_id: snapshot.id,
            kind: "snapshot_capture",
            payload: serde_json::json!({
                "workspace_id": workspace_id,
                "hosted_deployment_id": request.hosted_deployment_id,
            }),
        },
    )
    .await
    {
        tracing::error!(run_id = %run.id, error = %e, "failed to enqueue snapshot_capture run");
    }

    Ok(Json(snapshot))
}

/// `GET /api/v1/snapshots/{id}`
pub async fn get_snapshot(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Snapshot>> {
    let snapshot = load_authorized_snapshot(&state, user_id, &headers, id).await?;
    Ok(Json(snapshot))
}

/// `DELETE /api/v1/snapshots/{id}`
///
/// Removes the row. Re-syncs the `usage_counters.snapshot_bytes_stored`
/// gauge so the dashboard meter reflects reality immediately. Blob
/// reclaim from object storage happens asynchronously in a follow-up
/// slice (the worker reads orphaned storage_url values).
pub async fn delete_snapshot(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let snapshot = load_authorized_snapshot(&state, user_id, &headers, id).await?;
    let workspace_id = snapshot.workspace_id;

    let deleted = Snapshot::delete(state.db.pool(), id).await.map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Snapshot not found".into()));
    }

    // Re-sync the storage gauge for the org.
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    let bytes = Snapshot::sum_ready_bytes_by_workspace(state.db.pool(), workspace_id)
        .await
        .map_err(ApiError::Database)?;
    UsageCounter::set_snapshot_bytes(state.db.pool(), workspace.org_id, bytes)
        .await
        .map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// Verify caller belongs to the workspace's org.
async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<crate::middleware::org_context::OrgContext> {
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Workspace not found".into()));
    }
    Ok(ctx)
}

/// Fetch a snapshot and verify caller belongs to its workspace's org.
async fn load_authorized_snapshot(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<Snapshot> {
    let snapshot = Snapshot::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Snapshot not found".into()))?;
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), snapshot.workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Snapshot not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Snapshot not found".into()));
    }
    Ok(snapshot)
}
