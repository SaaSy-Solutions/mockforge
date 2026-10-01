//! Flows handlers — unified scenario/orchestration/state-machine/chain
//! resource (cloud-enablement task #9 / Phase 1).
//!
//! Phase 1 surface: CRUD over `flows` + read access to `flow_versions`.
//! Triggering a run reuses the #4 test_runs lifecycle with `kind` =
//! 'scenario' / 'orchestration' / 'state_machine' / 'chain' — wiring
//! that path lives in a follow-up slice once the worker pool is up.
//!
//! Routes:
//!   GET    /api/v1/workspaces/{workspace_id}/flows[?kind=]
//!   POST   /api/v1/workspaces/{workspace_id}/flows
//!   GET    /api/v1/flows/{id}                              (with current version)
//!   PATCH  /api/v1/flows/{id}                              (rename / description)
//!   DELETE /api/v1/flows/{id}
//!   POST   /api/v1/flows/{id}/versions                     (save new version)
//!   GET    /api/v1/flows/{id}/versions                     (history)
//!   GET    /api/v1/flow-versions/{version_id}

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use mockforge_registry_core::models::flow::CreateFlow;
use mockforge_registry_core::models::test_run::EnqueueTestRun;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{CloudWorkspace, Flow, FlowVersion, TestRun},
    store::with_org_context,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListFlowsQuery {
    /// Optional kind filter, e.g. ?kind=scenario.
    #[serde(default)]
    pub kind: Option<String>,
}

/// `GET /api/v1/workspaces/{workspace_id}/flows`
pub async fn list_flows(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListFlowsQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Flow>>> {
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let kind = query.kind;
    let flows = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(Flow::list_by_workspace(&mut **tx, workspace_id, kind.as_deref()).await?)
        })
    })
    .await?;
    Ok(Json(flows))
}

#[derive(Debug, Deserialize)]
pub struct CreateFlowRequest {
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Initial version's config payload.
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct FlowWithVersion {
    #[serde(flatten)]
    pub flow: Flow,
    pub version: FlowVersion,
}

/// `POST /api/v1/workspaces/{workspace_id}/flows`
pub async fn create_flow(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateFlowRequest>,
) -> ApiResult<Json<FlowWithVersion>> {
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if !Flow::is_valid_kind(&request.kind) {
        return Err(ApiError::InvalidRequest(format!(
            "kind must be one of: {}",
            Flow::VALID_KINDS.join(", ")
        )));
    }

    let (flow, version) = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(Flow::create_with_initial_version(
                &mut **tx,
                CreateFlow {
                    workspace_id,
                    kind: &request.kind,
                    name: &request.name,
                    description: request.description.as_deref(),
                    config: &request.config,
                    created_by: Some(user_id),
                },
            )
            .await?)
        })
    })
    .await?;

    Ok(Json(FlowWithVersion { flow, version }))
}

/// `GET /api/v1/flows/{id}` — returns the flow plus its current version,
/// so the editor can render the canvas in one round-trip.
pub async fn get_flow(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<FlowWithVersion>> {
    let (flow, org_id) = load_authorized_flow_with_org(&state, user_id, &headers, id).await?;
    let version_id = flow
        .current_version_id
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Flow has no current version")))?;
    let version = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(FlowVersion::find_by_id(&mut **tx, version_id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Flow version row missing")))?;
    Ok(Json(FlowWithVersion { flow, version }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateFlowRequest {
    #[serde(default)]
    pub name: Option<String>,
    /// Outer Option = "field present"; inner = "set to NULL".
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub description: Option<Option<String>>,
}

/// `PATCH /api/v1/flows/{id}` — only renames/edits the description.
/// Saving config changes goes through `POST /flows/{id}/versions` so the
/// version history is preserved.
pub async fn update_flow(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateFlowRequest>,
) -> ApiResult<Json<Flow>> {
    let (_, org_id) = load_authorized_flow_with_org(&state, user_id, &headers, id).await?;

    let updated = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(Flow::rename(
                &mut **tx,
                id,
                request.name.as_deref(),
                request.description.as_ref().map(|d| d.as_deref()),
            )
            .await?)
        })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Flow not found".into()))?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/flows/{id}` — cascade-deletes all versions.
pub async fn delete_flow(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let (_, org_id) = load_authorized_flow_with_org(&state, user_id, &headers, id).await?;

    let deleted = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(Flow::delete(&mut **tx, id).await?) })
    })
    .await?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Flow not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[derive(Debug, Deserialize)]
pub struct SaveVersionRequest {
    pub config: serde_json::Value,
}

/// `POST /api/v1/flows/{id}/versions`
///
/// Inserts a new flow_version and points `flows.current_version_id` at
/// it in the same transaction. Old versions stay around for rollback.
pub async fn save_flow_version(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<SaveVersionRequest>,
) -> ApiResult<Json<FlowVersion>> {
    let (_, org_id) = load_authorized_flow_with_org(&state, user_id, &headers, id).await?;

    let version = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(Flow::save_new_version(&mut **tx, id, &request.config, Some(user_id)).await?)
        })
    })
    .await?;
    Ok(Json(version))
}

/// `POST /api/v1/flows/{id}/runs`
///
/// Triggers a flow execution. Reuses the #4 test_runs lifecycle with
/// `kind` = the flow's own kind (scenario / orchestration / state_machine
/// / chain), so it shares the runner pool, concurrency cap, and
/// runner_seconds metering with regular test runs.
pub async fn trigger_run(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<TestRun>> {
    // `org_id` is the flow's workspace org, verified against the caller.
    let (flow, org_id) = load_authorized_flow_with_org(&state, user_id, &headers, id).await?;

    let org = mockforge_registry_core::models::Organization::find_by_id(state.db.pool(), org_id)
        .await
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("DB error loading org")))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".into()))?;
    let limits = crate::handlers::usage::effective_limits(&state, &org).await?;
    let max_concurrent = limits.get("max_concurrent_runs").and_then(|v| v.as_i64()).unwrap_or(0);
    if max_concurrent == 0 {
        return Err(ApiError::ResourceLimitExceeded(
            "Test execution / flow runs are not enabled on this plan".into(),
        ));
    }
    // Concurrency cap + insert in one org-bound transaction.
    let flow_id = flow.id;
    let kind = flow.kind.clone();
    let run = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            if max_concurrent > 0 {
                let inflight = TestRun::count_inflight(tx, org_id).await?;
                if inflight.total() >= max_concurrent {
                    return Ok(Err(inflight.total()));
                }
            }
            Ok(Ok(TestRun::enqueue(
                &mut **tx,
                EnqueueTestRun {
                    suite_id: flow_id,
                    org_id,
                    kind: &kind,
                    triggered_by: "manual",
                    triggered_by_user: Some(user_id),
                    git_ref: None,
                    git_sha: None,
                },
            )
            .await?))
        })
    })
    .await?
    .map_err(|inflight_total| {
        ApiError::ResourceLimitExceeded(format!(
            "Concurrent run limit reached ({}/{}).",
            inflight_total, max_concurrent,
        ))
    })?;

    // Push payload includes the flow's current_version_id AND its
    // config — saves the runner a round trip to fetch what it needs to
    // execute. Loading the version is best-effort: a missing version
    // (FK should prevent it but races are possible) is logged and the
    // run still queues with whatever metadata we have, so the runner
    // can record the failure with context.
    let version_config: Option<serde_json::Value> = match flow.current_version_id {
        Some(vid) => match with_org_context(state.db.runtime_pool(), org_id, |tx| {
            Box::pin(async move { Ok(FlowVersion::find_by_id(&mut **tx, vid).await?) })
        })
        .await
        {
            Ok(Some(v)) => Some(v.config),
            Ok(None) => {
                tracing::warn!(
                    flow_id = %flow.id,
                    version_id = %vid,
                    "flow.current_version_id points at missing FlowVersion",
                );
                None
            }
            Err(e) => {
                tracing::warn!(
                    flow_id = %flow.id,
                    error = %e,
                    "failed to load FlowVersion for run payload",
                );
                None
            }
        },
        None => None,
    };

    if let Err(e) = crate::run_queue::enqueue(
        state.redis.as_ref(),
        crate::run_queue::EnqueuedJob {
            run_id: run.id,
            org_id: run.org_id,
            source_id: flow.id,
            kind: &flow.kind,
            payload: serde_json::json!({
                "flow_kind": flow.kind,
                "flow_name": flow.name,
                "current_version_id": flow.current_version_id,
                "config": version_config,
            }),
        },
    )
    .await
    {
        tracing::error!(run_id = %run.id, error = %e, "failed to enqueue flow run");
    }

    Ok(Json(run))
}

/// `GET /api/v1/flows/{id}/versions` — full version history, newest first.
pub async fn list_flow_versions(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<FlowVersion>>> {
    let (_, org_id) = load_authorized_flow_with_org(&state, user_id, &headers, id).await?;
    let versions = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(FlowVersion::list_by_flow(&mut **tx, id).await?) })
    })
    .await?;
    Ok(Json(versions))
}

/// `GET /api/v1/flow-versions/{version_id}` — fetch a specific older
/// version for diff/rollback views.
pub async fn get_flow_version(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<FlowVersion>> {
    // Resolve the caller's org first and read the version bound to it: a
    // version whose flow lives in another org reads as `None` under RLS and
    // gets the same "not found" as a missing id.
    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    let version = with_org_context(state.db.runtime_pool(), ctx.org_id, |tx| {
        Box::pin(async move { Ok(FlowVersion::find_by_id(&mut **tx, version_id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Flow version not found".into()))?;
    // Authorize against the parent flow.
    load_authorized_flow_with_org(&state, user_id, &headers, version.flow_id).await?;
    Ok(Json(version))
}

async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<Uuid> {
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    let workspace = find_workspace_in_org(state, ctx.org_id, workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Workspace not found".into()));
    }
    Ok(ctx.org_id)
}

/// Load a flow the caller may access, returning it with the authorized org
/// id. The flow is read bound to the caller's org, so a flow in another org
/// reads as `None` and gets the same "Flow not found" as a missing id.
async fn load_authorized_flow_with_org(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<(Flow, Uuid)> {
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    let flow = with_org_context(state.db.runtime_pool(), ctx.org_id, |tx| {
        Box::pin(async move { Ok(Flow::find_by_id(&mut **tx, id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Flow not found".into()))?;
    let workspace = find_workspace_in_org(state, ctx.org_id, flow.workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Flow not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Flow not found".into()));
    }
    Ok((flow, ctx.org_id))
}

/// Load a workspace bound to `org_id`; a workspace in another org is `None`.
async fn find_workspace_in_org(
    state: &AppState,
    org_id: Uuid,
    workspace_id: Uuid,
) -> ApiResult<Option<CloudWorkspace>> {
    Ok(with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(CloudWorkspace::find_by_id(&mut **tx, workspace_id).await?) })
    })
    .await?)
}

fn deserialize_double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}
