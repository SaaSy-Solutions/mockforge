//! Cloud Test Generator handlers (#469) — Phase 1 data plane.
//!
//! Backs `cloudTestGeneratorApi` in the UI. Each row represents one async
//! LLM job that, in Phase 2, will be picked up by a background worker
//! that calls the org's BYOK provider key against a corpus of recent
//! `runtime_captures` rows and returns generated test scenarios.
//!
//! Phase 1 ships the data plane only:
//! * `POST /api/v1/workspaces/{workspace_id}/test-generation/jobs` — create a job in 'queued' state.
//! * `GET  /api/v1/workspaces/{workspace_id}/test-generation/jobs`             — list (newest first, capped 100).
//! * `GET  /api/v1/workspaces/{workspace_id}/test-generation/jobs/{job_id}`    — status / result poll.
//! * `POST /api/v1/workspaces/{workspace_id}/test-generation/jobs/{job_id}/cancel` — cancel a queued/running job.
//!
//! Rows created here sit in 'queued' until Phase 2 wires the worker.
//! That's intentional — the data shape and authorization model land
//! first so the UI client (Phase 1) and the worker (Phase 2) can land
//! independently against a stable contract.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    AppState,
};
use mockforge_registry_core::models::{
    test_generation_job::{CreateTestGenerationJob, TestGenerationJob},
    CloudWorkspace,
};

/// Hard cap on the list page size. The TestGeneratorPage is a poll-based
/// timeline, so showing more than ~100 jobs at once is rarely useful and
/// makes the index scan unbounded.
const LIST_LIMIT: i64 = 100;

/// Cap on prompt length. LLM providers all have a context window; bounding
/// here means we reject obvious abuse without round-tripping to the
/// provider only to get rejected upstream. 8 KB is comfortably under every
/// modern provider's prompt token limit.
const MAX_PROMPT_BYTES: usize = 8 * 1024;

#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    /// Optional natural-language description of what tests to generate.
    /// Empty allowed — Phase 2's worker falls back to a default prompt
    /// derived from `captures_filter`.
    #[serde(default)]
    pub prompt: String,
    /// JSON filter object describing which captures to feed the LLM.
    /// Forwarded verbatim to the worker; the worker owns the filter
    /// vocabulary. Bounded only by `MAX_FILTER_BYTES` below.
    #[serde(default)]
    pub captures_filter: Value,
}

/// Cap on the JSON-encoded filter. Same shape rationale as
/// `MAX_PROMPT_BYTES` — anything larger is almost certainly abuse and
/// would never round-trip through Phase 2's worker anyway.
const MAX_FILTER_BYTES: usize = 16 * 1024;

// --- POST /jobs -----------------------------------------------------------

pub async fn create_job(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<CreateJobRequest>,
) -> ApiResult<Json<TestGenerationJob>> {
    let workspace = authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    if body.prompt.len() > MAX_PROMPT_BYTES {
        return Err(ApiError::InvalidRequest(format!(
            "prompt exceeds {MAX_PROMPT_BYTES} byte limit"
        )));
    }
    let filter_size = serde_json::to_vec(&body.captures_filter).map(|v| v.len()).unwrap_or(0);
    if filter_size > MAX_FILTER_BYTES {
        return Err(ApiError::InvalidRequest(format!(
            "captures_filter exceeds {MAX_FILTER_BYTES} byte limit"
        )));
    }
    if !body.captures_filter.is_object() && !body.captures_filter.is_null() {
        return Err(ApiError::InvalidRequest("captures_filter must be a JSON object".into()));
    }

    // Normalise null → empty object so the DB row never carries `null`
    // (the column NOT-NULL default is `{}::jsonb`).
    let captures_filter = if body.captures_filter.is_null() {
        json!({})
    } else {
        body.captures_filter
    };

    let row = TestGenerationJob::create(
        state.db.pool(),
        CreateTestGenerationJob {
            workspace_id: workspace.id,
            org_id: workspace.org_id,
            prompt: &body.prompt,
            captures_filter: &captures_filter,
            created_by: Some(user_id),
        },
    )
    .await?;

    tracing::info!(
        job_id = %row.id,
        workspace_id = %workspace.id,
        org_id = %workspace.org_id,
        prompt_len = body.prompt.len(),
        "test-generation job queued"
    );

    Ok(Json(row))
}

// --- GET /jobs ------------------------------------------------------------

pub async fn list_jobs(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<TestGenerationJob>>> {
    let workspace = authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let jobs =
        TestGenerationJob::list_by_workspace(state.db.pool(), workspace.id, LIST_LIMIT).await?;
    Ok(Json(jobs))
}

// --- GET /jobs/{job_id} ---------------------------------------------------

pub async fn get_job(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((workspace_id, job_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<Json<TestGenerationJob>> {
    let workspace = authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let job = TestGenerationJob::find_in_workspace(state.db.pool(), workspace.id, job_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Job not found".into()))?;
    Ok(Json(job))
}

// --- POST /jobs/{job_id}/cancel -------------------------------------------

pub async fn cancel_job(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((workspace_id, job_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let workspace = authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let changed = TestGenerationJob::cancel(state.db.pool(), workspace.id, job_id).await?;
    Ok(Json(json!({
        "cancelled": changed,
    })))
}

// --- helpers --------------------------------------------------------------

/// Resolve `workspace_id` and check the caller's org owns it. Returns
/// the workspace so callers can read `workspace.org_id` without a second
/// fetch. Mirrors the helper in `handlers::captures` but returns the
/// workspace rather than just `()`.
async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<CloudWorkspace> {
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != workspace.org_id {
        // Same opaque response as the unknown-workspace case — don't leak
        // existence of cross-org workspace IDs.
        return Err(ApiError::InvalidRequest("Workspace not found".into()));
    }
    Ok(workspace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_request_defaults() {
        let req: CreateJobRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(req.prompt, "");
        assert!(req.captures_filter.is_null());
    }

    #[test]
    fn create_request_accepts_prompt_and_filter() {
        let req: CreateJobRequest =
            serde_json::from_str(r#"{"prompt":"gen tests","captures_filter":{"status":">=400"}}"#)
                .unwrap();
        assert_eq!(req.prompt, "gen tests");
        assert_eq!(req.captures_filter["status"], ">=400");
    }

    #[test]
    fn prompt_length_cap_is_8kb() {
        // Round number tied to the comment in MAX_PROMPT_BYTES — guards
        // against an accidental "let's set it to 4MB" change.
        assert_eq!(MAX_PROMPT_BYTES, 8 * 1024);
    }

    #[test]
    fn filter_size_cap_is_16kb() {
        assert_eq!(MAX_FILTER_BYTES, 16 * 1024);
    }

    #[test]
    fn list_limit_is_capped_at_100() {
        // The TestGeneratorPage is a poll-based timeline; >100 jobs at
        // once produces noisy UI and unbounded scans without a useful
        // value.
        assert_eq!(LIST_LIMIT, 100);
    }
}
