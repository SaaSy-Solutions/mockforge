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
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::usage::effective_limits,
    middleware::{resolve_org_context, AuthUser},
    models::{CloudWorkspace, Snapshot, UsageCounter},
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

    // Capture the workspace state synchronously. For now the manifest
    // *is* the snapshot — we don't yet have S3/blob storage, so the
    // JSON lands inline in snapshots.manifest. Sub-second on a typical
    // workspace, so the request stays interactive without a background
    // worker round-trip. When blob storage lands, the manifest stays as
    // the index and bulky byte-level data moves out via storage_url.
    let (manifest, size_bytes) = match build_workspace_manifest(state.db.pool(), workspace_id).await
    {
        Ok((m, s)) => (m, s),
        Err(e) => {
            tracing::error!(snapshot_id = %snapshot.id, error = %e, "manifest build failed");
            // Flip status to 'failed' so list_by_workspace reflects reality.
            let _ = Snapshot::mark_failed(state.db.pool(), snapshot.id).await;
            return Err(ApiError::Database(e));
        }
    };

    let storage_url = format!("inline-manifest://snapshot/{}", snapshot.id);
    match Snapshot::mark_ready(state.db.pool(), snapshot.id, &storage_url, size_bytes, &manifest)
        .await
    {
        Ok(Some(ready)) => {
            // Storage metering is a gauge (set_snapshot_bytes) not a
            // counter; updating it correctly requires reading the
            // current size_bytes sum across all ready snapshots, which
            // would race with other captures landing concurrently. The
            // usage_threshold_checker worker reconciles the gauge from
            // the source of truth, so we leave it alone here.
            let _ = ctx; // ctx no longer load-bearing post-mark_ready
            Ok(Json(ready))
        }
        Ok(None) => Ok(Json(snapshot)), // already terminal — return what we have
        Err(e) => {
            let _ = Snapshot::mark_failed(state.db.pool(), snapshot.id).await;
            Err(ApiError::Database(e))
        }
    }
}

/// Build a JSON manifest of the workspace's authoritative state.
/// Includes the resources a "restore" would want to recreate: services,
/// fixtures, scenarios, environments, federation links, folders.
/// Returns (manifest, byte_count) so the caller can bill storage usage.
async fn build_workspace_manifest(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
) -> sqlx::Result<(serde_json::Value, i64)> {
    use mockforge_registry_core::models::{
        flow::Flow, mock_environment::MockEnvironment, ChaosCampaign,
    };

    // Each list is best-effort — if a resource family fails to load we
    // log + include an empty array. A partial snapshot is more useful
    // than no snapshot at all, and `restored_partial: true` in the
    // manifest tells a future restore worker to be cautious.
    let mut partial = false;
    let environments = match MockEnvironment::list_by_workspace(pool, workspace_id).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(workspace_id = %workspace_id, error = %e, "snapshot: mock_environments fetch failed");
            partial = true;
            Vec::new()
        }
    };
    let flows = match Flow::list_by_workspace(pool, workspace_id, None).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(workspace_id = %workspace_id, error = %e, "snapshot: flows fetch failed");
            partial = true;
            Vec::new()
        }
    };
    let chaos = match ChaosCampaign::list_by_workspace(pool, workspace_id).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(workspace_id = %workspace_id, error = %e, "snapshot: chaos fetch failed");
            partial = true;
            Vec::new()
        }
    };

    // Raw services / fixtures table dumps via sqlx so we don't need a
    // model API for every column — the manifest is forward-compatible
    // because new columns just appear in the JSON.
    let services = sqlx::query_as::<_, (Uuid, serde_json::Value)>(
        "SELECT id, to_jsonb(s) AS doc FROM services s WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        tracing::warn!(workspace_id = %workspace_id, error = %e, "snapshot: services fetch failed");
        partial = true;
        Vec::new()
    });
    let fixtures = sqlx::query_as::<_, (Uuid, serde_json::Value)>(
        "SELECT id, to_jsonb(f) AS doc FROM fixtures f WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| {
        tracing::warn!(workspace_id = %workspace_id, error = %e, "snapshot: fixtures fetch failed");
        partial = true;
        Vec::new()
    });

    let manifest = serde_json::json!({
        "schema_version": 1,
        "captured_at": chrono::Utc::now(),
        "workspace_id": workspace_id,
        "partial": partial,
        "counts": {
            "services": services.len(),
            "fixtures": fixtures.len(),
            "environments": environments.len(),
            "flows": flows.len(),
            "chaos_campaigns": chaos.len(),
        },
        "services": services.into_iter().map(|(_, doc)| doc).collect::<Vec<_>>(),
        "fixtures": fixtures.into_iter().map(|(_, doc)| doc).collect::<Vec<_>>(),
        "environments": environments,
        "flows": flows,
        "chaos_campaigns": chaos,
    });

    let bytes = manifest.to_string().len() as i64;
    Ok((manifest, bytes))
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

/// `GET /api/v1/snapshots/{id}/diff?against=current`
///
/// Compares the snapshot's manifest against either the workspace's
/// current state (`against=current`, default) or another snapshot
/// (`against=<other_snapshot_id>`). Returns per-resource counts of
/// added/removed/changed plus the actual diff lists so the UI can
/// render a side-by-side review before the user commits to a restore.
#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    #[serde(default)]
    pub against: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct ResourceDiff {
    pub added: Vec<serde_json::Value>,
    pub removed: Vec<serde_json::Value>,
    pub changed: Vec<DiffPair>,
}

#[derive(Debug, serde::Serialize)]
pub struct DiffPair {
    pub from: serde_json::Value,
    pub to: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
pub struct SnapshotDiff {
    pub snapshot_id: Uuid,
    pub against_kind: String,
    pub against_snapshot_id: Option<Uuid>,
    pub services: ResourceDiff,
    pub fixtures: ResourceDiff,
    pub flows: ResourceDiff,
    pub environments: ResourceDiff,
    pub chaos_campaigns: ResourceDiff,
}

pub async fn diff_snapshot(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<DiffQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<SnapshotDiff>> {
    let snapshot = load_authorized_snapshot(&state, user_id, &headers, id).await?;
    let snapshot_manifest = snapshot.manifest.clone().unwrap_or_else(|| serde_json::json!({}));

    let against_str = query.against.as_deref().unwrap_or("current");
    let (against_kind, against_id, against_manifest) = if against_str == "current" {
        let (m, _) = build_workspace_manifest(state.db.pool(), snapshot.workspace_id)
            .await
            .map_err(ApiError::Database)?;
        ("current".to_string(), None, m)
    } else {
        let other_id = Uuid::parse_str(against_str).map_err(|_| {
            ApiError::InvalidRequest("'against' must be 'current' or a snapshot UUID".into())
        })?;
        let other = load_authorized_snapshot(&state, user_id, &headers, other_id).await?;
        if other.workspace_id != snapshot.workspace_id {
            return Err(ApiError::InvalidRequest(
                "Cannot diff snapshots across different workspaces".into(),
            ));
        }
        let m = other.manifest.clone().unwrap_or_else(|| serde_json::json!({}));
        ("snapshot".to_string(), Some(other_id), m)
    };

    Ok(Json(SnapshotDiff {
        snapshot_id: snapshot.id,
        against_kind,
        against_snapshot_id: against_id,
        services: diff_resource(&snapshot_manifest, &against_manifest, "services"),
        fixtures: diff_resource(&snapshot_manifest, &against_manifest, "fixtures"),
        flows: diff_resource(&snapshot_manifest, &against_manifest, "flows"),
        environments: diff_resource(&snapshot_manifest, &against_manifest, "environments"),
        chaos_campaigns: diff_resource(&snapshot_manifest, &against_manifest, "chaos_campaigns"),
    }))
}

/// Diff one resource family between two manifests by `id`. Resources
/// in `from` but not `to` are "removed"; resources in `to` but not
/// `from` are "added"; same id with different content is "changed".
fn diff_resource(from: &serde_json::Value, to: &serde_json::Value, key: &str) -> ResourceDiff {
    let from_list = from.get(key).and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let to_list = to.get(key).and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let from_by_id: std::collections::HashMap<String, serde_json::Value> = from_list
        .iter()
        .filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(|s| (s.to_string(), v.clone())))
        .collect();
    let to_by_id: std::collections::HashMap<String, serde_json::Value> = to_list
        .iter()
        .filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(|s| (s.to_string(), v.clone())))
        .collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for (id, v) in &from_by_id {
        match to_by_id.get(id) {
            None => removed.push(v.clone()),
            Some(other) if other != v => changed.push(DiffPair {
                from: v.clone(),
                to: other.clone(),
            }),
            Some(_) => {} // identical
        }
    }
    for (id, v) in &to_by_id {
        if !from_by_id.contains_key(id) {
            added.push(v.clone());
        }
    }

    ResourceDiff {
        added,
        removed,
        changed,
    }
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
