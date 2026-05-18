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

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures_util::stream::{self, Stream};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
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

// --- SSE: GET /jobs/{job_id}/stream --------------------------------------

/// `GET /api/v1/workspaces/{workspace_id}/test-generation/jobs/{job_id}/stream`
///
/// Server-Sent Events stream of the job's lifecycle. The UI's polling
/// path (every 5s) works fine; this endpoint exists for clients that
/// want sub-second update latency without burning the UI render path
/// on a tight `setInterval`.
///
/// Pattern mirrors `handlers::test_runs::stream_run_events`: poll the
/// underlying row every 1s, emit a `status_update` event whenever the
/// shape changes (status / started_at / finished_at / has-result /
/// has-error), terminate after one final event once status reaches a
/// terminal value.
///
/// Polling Postgres (vs LISTEN/NOTIFY) is the right tradeoff today:
/// per-org job rate is bounded (interactive UI feature, not a firehose)
/// and the 1s cadence is invisible to the user. A pub/sub upgrade can
/// land later if usage demands it.
pub async fn stream_job(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((workspace_id, job_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    // Authorize once up front so we don't leak existence to non-members.
    let workspace = authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    let cursor = JobStreamCursor {
        pool: state.db.pool().clone(),
        workspace_id: workspace.id,
        job_id,
        last_snapshot: None,
        terminal_emitted: false,
    };

    let stream = stream::unfold(cursor, advance_job_stream);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// Snapshot of the fields we care about for change-detection. Two
/// snapshots compare equal iff a client wouldn't see a difference.
#[derive(Debug, Clone, PartialEq, Eq)]
struct JobSnapshot {
    status: String,
    started_at_set: bool,
    finished_at_set: bool,
    has_result: bool,
    has_error: bool,
}

impl JobSnapshot {
    fn from_job(j: &TestGenerationJob) -> Self {
        Self {
            status: j.status.clone(),
            started_at_set: j.started_at.is_some(),
            finished_at_set: j.finished_at.is_some(),
            has_result: j.result.is_some(),
            has_error: j.error.is_some(),
        }
    }

    fn is_terminal(&self) -> bool {
        matches!(self.status.as_str(), "succeeded" | "failed" | "cancelled")
    }
}

struct JobStreamCursor {
    pool: PgPool,
    workspace_id: Uuid,
    job_id: Uuid,
    last_snapshot: Option<JobSnapshot>,
    terminal_emitted: bool,
}

/// One step of the SSE stream:
///   - Poll the job row.
///   - If the row is gone, emit `not_found` and terminate.
///   - If the snapshot is unchanged, sleep and re-poll on the next
///     iteration (no event emitted; SSE keep-alive comments cover idle
///     connections).
///   - If the snapshot changed, emit a `status_update` event with the
///     full row payload.
///   - If the new snapshot is terminal, mark the cursor so the next
///     iteration returns None.
async fn advance_job_stream(
    mut cursor: JobStreamCursor,
) -> Option<(Result<Event, Infallible>, JobStreamCursor)> {
    if cursor.terminal_emitted {
        return None;
    }

    // 1s cadence; matches the test_runs SSE handler. Skip the sleep on
    // the very first poll so the initial snapshot lands immediately.
    if cursor.last_snapshot.is_some() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    let job = match TestGenerationJob::find_in_workspace(
        &cursor.pool,
        cursor.workspace_id,
        cursor.job_id,
    )
    .await
    {
        Ok(Some(j)) => j,
        Ok(None) => {
            let evt = Event::default()
                .event("not_found")
                .data(json!({ "job_id": cursor.job_id }).to_string());
            cursor.terminal_emitted = true;
            return Some((Ok(evt), cursor));
        }
        Err(e) => {
            let evt = Event::default()
                .event("stream_error")
                .data(json!({ "error": e.to_string() }).to_string());
            cursor.terminal_emitted = true;
            return Some((Ok(evt), cursor));
        }
    };

    let snapshot = JobSnapshot::from_job(&job);
    let unchanged = cursor.last_snapshot.as_ref().is_some_and(|s| s == &snapshot);

    if unchanged {
        // No new data — emit a heartbeat keep-alive comment by yielding
        // a `ping` event so the client knows the connection's still live
        // without burning bytes on a full payload. The browser's
        // EventSource ignores unknown event types unless explicitly
        // listened to, which is what we want.
        let evt = Event::default().event("ping").data("{}");
        return Some((Ok(evt), cursor));
    }

    let terminal = snapshot.is_terminal();
    cursor.last_snapshot = Some(snapshot);
    if terminal {
        cursor.terminal_emitted = true;
    }

    let payload =
        serde_json::to_value(&job).unwrap_or_else(|_| json!({ "error": "serialization failed" }));
    let evt = Event::default().event("status_update").data(payload.to_string());
    Some((Ok(evt), cursor))
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
