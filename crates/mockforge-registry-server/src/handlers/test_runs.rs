//! Test run handlers (cloud-enablement task #4 / Phase 2).
//!
//! Phase 2 surface only — enqueue + read paths. The actual worker
//! (mockforge-test-runner crate) consuming the Redis queue and
//! transitioning runs through running → terminal lives in a follow-up
//! slice, as does the SSE event stream.
//!
//! Routes:
//!   POST   /api/v1/test-suites/{id}/runs        (trigger)
//!   GET    /api/v1/test-suites/{id}/runs        (history for one suite)
//!   GET    /api/v1/organizations/{org_id}/test-runs   (cross-suite list)
//!   GET    /api/v1/test-runs/{id}
//!   POST   /api/v1/test-runs/{id}/cancel

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use mockforge_registry_core::models::test_run::EnqueueTestRun;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::usage::effective_limits,
    middleware::{resolve_org_context, AuthUser},
    models::{TestRun, TestSuite},
    AppState,
};

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 500;

#[derive(Debug, Deserialize)]
pub struct TriggerRunRequest {
    /// Source of the trigger. Defaults to `manual`. Workers/CI should
    /// set explicitly so the UI can distinguish.
    #[serde(default)]
    pub triggered_by: Option<String>,
    #[serde(default)]
    pub git_ref: Option<String>,
    #[serde(default)]
    pub git_sha: Option<String>,
}

/// `POST /api/v1/test-suites/{id}/runs`
///
/// Enqueues a run in `queued` state. Concurrency-cap enforced
/// pre-enqueue against `max_concurrent_runs` from the org's plan
/// limits. Runner-minute quota is enforced by the worker at run start
/// (we can't predict run duration here).
pub async fn trigger_run(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(suite_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<TriggerRunRequest>,
) -> ApiResult<Json<TestRun>> {
    // 1. Load the suite + verify org membership.
    let suite = TestSuite::find_by_id(state.db.pool(), suite_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Test suite not found".into()))?;

    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;

    // Suite is workspace-scoped; verify the workspace belongs to caller's org.
    let workspace = mockforge_registry_core::models::CloudWorkspace::find_by_id(
        state.db.pool(),
        suite.workspace_id,
    )
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    if workspace.org_id != ctx.org_id {
        return Err(ApiError::InvalidRequest("Test suite not found".into()));
    }

    // 2. Concurrency cap.
    let limits = effective_limits(&state, &ctx.org).await?;
    let max_concurrent = limits.get("max_concurrent_runs").and_then(|v| v.as_i64()).unwrap_or(0);
    if max_concurrent == 0 {
        return Err(ApiError::ResourceLimitExceeded(
            "Test execution is not enabled on this plan — upgrade to Pro or Team to run tests"
                .into(),
        ));
    }
    if max_concurrent > 0 {
        let inflight = TestRun::count_inflight(state.db.pool(), ctx.org_id)
            .await
            .map_err(ApiError::Database)?;
        if inflight.total() >= max_concurrent {
            return Err(ApiError::ResourceLimitExceeded(format!(
                "Concurrent run limit reached ({}/{}). Wait for a run to finish or upgrade your plan.",
                inflight.total(),
                max_concurrent,
            )));
        }
    }

    // 3. Validate triggered_by source label.
    let triggered_by = request.triggered_by.as_deref().unwrap_or("manual");
    if !is_valid_trigger_source(triggered_by) {
        return Err(ApiError::InvalidRequest(
            "triggered_by must be one of: manual, schedule, ci, webhook".into(),
        ));
    }

    // 4. Enqueue.
    let run = TestRun::enqueue(
        state.db.pool(),
        EnqueueTestRun {
            suite_id: suite.id,
            org_id: ctx.org_id,
            triggered_by,
            triggered_by_user: Some(user_id),
            git_ref: request.git_ref.as_deref(),
            git_sha: request.git_sha.as_deref(),
        },
    )
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(run))
}

#[derive(Debug, Deserialize)]
pub struct ListRunsQuery {
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `GET /api/v1/test-suites/{id}/runs`
pub async fn list_suite_runs(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(suite_id): Path<Uuid>,
    Query(query): Query<ListRunsQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<TestRun>>> {
    // Verify suite belongs to caller's org via the workspace check.
    let suite = TestSuite::find_by_id(state.db.pool(), suite_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Test suite not found".into()))?;
    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    let workspace = mockforge_registry_core::models::CloudWorkspace::find_by_id(
        state.db.pool(),
        suite.workspace_id,
    )
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    if workspace.org_id != ctx.org_id {
        return Err(ApiError::InvalidRequest("Test suite not found".into()));
    }

    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let runs = TestRun::list_by_suite(state.db.pool(), suite.id, limit)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(runs))
}

#[derive(Debug, Deserialize)]
pub struct ListOrgRunsQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `GET /api/v1/organizations/{org_id}/test-runs`
pub async fn list_org_runs(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    Query(query): Query<ListOrgRunsQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<TestRun>>> {
    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != org_id {
        return Err(ApiError::InvalidRequest("Cannot list runs for a different org".into()));
    }

    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let runs = TestRun::list_by_org(state.db.pool(), org_id, query.status.as_deref(), limit)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(runs))
}

/// `GET /api/v1/test-runs/{id}`
pub async fn get_run(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<TestRun>> {
    let run = load_authorized_run(&state, user_id, &headers, id).await?;
    Ok(Json(run))
}

/// `POST /api/v1/test-runs/{id}/cancel`
pub async fn cancel_run(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<TestRun>> {
    load_authorized_run(&state, user_id, &headers, id).await?;
    let updated = TestRun::cancel(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            ApiError::InvalidRequest(
                "Run is not cancellable (already terminal or not found)".into(),
            )
        })?;
    Ok(Json(updated))
}

/// Fetch a run and verify the caller belongs to its org.
/// Cross-org reads return "not found" rather than "forbidden" to avoid
/// leaking existence.
async fn load_authorized_run(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<TestRun> {
    let run = TestRun::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Test run not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != run.org_id {
        return Err(ApiError::InvalidRequest("Test run not found".into()));
    }
    Ok(run)
}

fn is_valid_trigger_source(s: &str) -> bool {
    matches!(s, "manual" | "schedule" | "ci" | "webhook")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_source_accepts_canonical_values() {
        assert!(is_valid_trigger_source("manual"));
        assert!(is_valid_trigger_source("schedule"));
        assert!(is_valid_trigger_source("ci"));
        assert!(is_valid_trigger_source("webhook"));
    }

    #[test]
    fn trigger_source_rejects_others() {
        assert!(!is_valid_trigger_source("MANUAL"));
        assert!(!is_valid_trigger_source(""));
        assert!(!is_valid_trigger_source("api"));
    }
}
