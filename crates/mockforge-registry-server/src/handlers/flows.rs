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
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let flows = Flow::list_by_workspace(state.db.pool(), workspace_id, query.kind.as_deref())
        .await
        .map_err(ApiError::Database)?;
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
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if !Flow::is_valid_kind(&request.kind) {
        return Err(ApiError::InvalidRequest(format!(
            "kind must be one of: {}",
            Flow::VALID_KINDS.join(", ")
        )));
    }

    let (flow, version) = Flow::create_with_initial_version(
        state.db.pool(),
        CreateFlow {
            workspace_id,
            kind: &request.kind,
            name: &request.name,
            description: request.description.as_deref(),
            config: &request.config,
            created_by: Some(user_id),
        },
    )
    .await
    .map_err(ApiError::Database)?;

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
    let flow = load_authorized_flow(&state, user_id, &headers, id).await?;
    let version_id = flow
        .current_version_id
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Flow has no current version")))?;
    let version = FlowVersion::find_by_id(state.db.pool(), version_id)
        .await
        .map_err(ApiError::Database)?
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
    load_authorized_flow(&state, user_id, &headers, id).await?;

    let updated = Flow::rename(
        state.db.pool(),
        id,
        request.name.as_deref(),
        request.description.as_ref().map(|d| d.as_deref()),
    )
    .await
    .map_err(ApiError::Database)?
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
    load_authorized_flow(&state, user_id, &headers, id).await?;

    let deleted = Flow::delete(state.db.pool(), id).await.map_err(ApiError::Database)?;
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
    load_authorized_flow(&state, user_id, &headers, id).await?;

    let version = Flow::save_new_version(state.db.pool(), id, &request.config, Some(user_id))
        .await
        .map_err(ApiError::Database)?;
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
    let flow = load_authorized_flow(&state, user_id, &headers, id).await?;
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), flow.workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;

    let org = mockforge_registry_core::models::Organization::find_by_id(
        state.db.pool(),
        workspace.org_id,
    )
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
    if max_concurrent > 0 {
        let inflight = TestRun::count_inflight(state.db.pool(), workspace.org_id)
            .await
            .map_err(ApiError::Database)?;
        if inflight.total() >= max_concurrent {
            return Err(ApiError::ResourceLimitExceeded(format!(
                "Concurrent run limit reached ({}/{}).",
                inflight.total(),
                max_concurrent,
            )));
        }
    }

    let run = TestRun::enqueue(
        state.db.pool(),
        EnqueueTestRun {
            suite_id: flow.id,
            org_id: workspace.org_id,
            kind: &flow.kind,
            triggered_by: "manual",
            triggered_by_user: Some(user_id),
            git_ref: None,
            git_sha: None,
        },
    )
    .await
    .map_err(ApiError::Database)?;

    // Push payload includes the flow's current_version_id so the
    // runner-side FlowExecutor knows which config to load.
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
    load_authorized_flow(&state, user_id, &headers, id).await?;
    let versions = FlowVersion::list_by_flow(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?;
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
    let version = FlowVersion::find_by_id(state.db.pool(), version_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Flow version not found".into()))?;
    // Authorize against the parent flow.
    load_authorized_flow(&state, user_id, &headers, version.flow_id).await?;
    Ok(Json(version))
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

async fn load_authorized_flow(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<Flow> {
    let flow = Flow::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Flow not found".into()))?;
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), flow.workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Flow not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Flow not found".into()));
    }
    Ok(flow)
}

fn deserialize_double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}
