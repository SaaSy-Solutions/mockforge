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
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures_util::stream::{self, Stream};
use mockforge_registry_core::models::test_run::EnqueueTestRun;
use serde::Deserialize;
use std::convert::Infallible;
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

    // 4. Insert the test_runs row.
    let run = TestRun::enqueue(
        state.db.pool(),
        EnqueueTestRun {
            suite_id: suite.id,
            org_id: ctx.org_id,
            kind: &suite.kind,
            triggered_by,
            triggered_by_user: Some(user_id),
            git_ref: request.git_ref.as_deref(),
            git_sha: request.git_sha.as_deref(),
        },
    )
    .await
    .map_err(ApiError::Database)?;

    // 5. Push onto the Redis queue so mockforge-test-runner picks it up.
    // Failure to enqueue logs a warning but doesn't fail the request —
    // the row still exists and a future runner reconnect / retrigger
    // can consume it. That matches our other "Redis is optional" paths.
    let payload = serde_json::Value::Object(serde_json::Map::new());
    if let Err(e) = crate::run_queue::enqueue(
        state.redis.as_ref(),
        crate::run_queue::EnqueuedJob {
            run_id: run.id,
            org_id: run.org_id,
            source_id: suite.id,
            kind: &suite.kind,
            payload,
        },
    )
    .await
    {
        tracing::error!(run_id = %run.id, error = %e, "failed to enqueue test_run");
    }

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

/// `GET /api/v1/test-runs/{id}/stream`
///
/// Server-Sent Events stream of test_run_events as they're appended.
/// The runner writes events through the internal callback path; this
/// handler polls every ~1s, advances a `seq` cursor past whatever it's
/// emitted, and closes the stream once the run reaches a terminal
/// status (passed | failed | cancelled | errored).
///
/// Polling Postgres is fine for now — the per-run event volume is
/// bounded (hundreds, not thousands) and Phase 1 doesn't need a
/// pub/sub bus. A LISTEN/NOTIFY upgrade is a follow-up if perf
/// demands it.
pub async fn stream_run_events(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    // Authorize once up front so we don't leak existence to non-members.
    load_authorized_run(&state, user_id, &headers, id).await?;

    let pool = state.db.pool().clone();
    let cursor = EventCursor {
        run_id: id,
        pool,
        seq: 0,
        buffered: Vec::new(),
        terminal_emitted: false,
    };
    let stream = stream::unfold(cursor, advance_event_cursor);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// One step of the SSE stream: drain the buffer first; when empty,
/// poll the DB; when both DB and buffer are drained AND the run is
/// terminal, emit the final `done` event and stop.
async fn advance_event_cursor(
    mut cursor: EventCursor,
) -> Option<(Result<Event, Infallible>, EventCursor)> {
    if cursor.terminal_emitted {
        return None;
    }

    // Drain whatever's already buffered before going back to the DB.
    if let Some(row) = cursor.buffered.pop() {
        let payload = serde_json::json!({
            "seq": row.seq,
            "type": row.event_type,
            "payload": row.payload,
            "occurred_at": row.occurred_at,
        });
        let evt = Event::default().event(&row.event_type).data(payload.to_string());
        return Some((Ok(evt), cursor));
    }

    // 1s poll cadence — fast enough that the UI feels live, slow enough
    // that idle streams don't hammer the DB.
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    let events: Vec<TestRunEventRow> = match sqlx::query_as::<_, TestRunEventRow>(
        "SELECT seq, event_type, payload, occurred_at \
         FROM test_run_events \
         WHERE run_id = $1 AND seq > $2 \
         ORDER BY seq ASC LIMIT 200",
    )
    .bind(cursor.run_id)
    .bind(cursor.seq)
    .fetch_all(&cursor.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            let evt = Event::default()
                .event("stream_error")
                .data(serde_json::json!({ "error": e.to_string() }).to_string());
            cursor.terminal_emitted = true;
            return Some((Ok(evt), cursor));
        }
    };

    for row in &events {
        cursor.seq = row.seq.max(cursor.seq);
    }
    // Buffer is a stack; push reverse so pop() yields oldest first.
    cursor.buffered = events.into_iter().rev().collect();

    if let Some(row) = cursor.buffered.pop() {
        let payload = serde_json::json!({
            "seq": row.seq,
            "type": row.event_type,
            "payload": row.payload,
            "occurred_at": row.occurred_at,
        });
        let evt = Event::default().event(&row.event_type).data(payload.to_string());
        return Some((Ok(evt), cursor));
    }

    let terminal = matches!(
        sqlx::query_as::<_, (String,)>("SELECT status FROM test_runs WHERE id = $1")
            .bind(cursor.run_id)
            .fetch_optional(&cursor.pool)
            .await,
        Ok(Some((ref s,))) if matches!(
            s.as_str(),
            "passed" | "failed" | "cancelled" | "errored"
        )
    );

    if !terminal {
        // No new events, run still inflight — keep-alive sentinel.
        let evt = Event::default().event("ping").data("{}");
        return Some((Ok(evt), cursor));
    }

    let final_payload = match sqlx::query_as::<_, (String, Option<i32>, Option<serde_json::Value>)>(
        "SELECT status, runner_seconds, summary FROM test_runs WHERE id = $1",
    )
    .bind(cursor.run_id)
    .fetch_optional(&cursor.pool)
    .await
    {
        Ok(Some((status, runner_seconds, summary))) => serde_json::json!({
            "status": status,
            "runner_seconds": runner_seconds,
            "summary": summary,
        }),
        _ => serde_json::json!({ "status": "unknown" }),
    };
    cursor.terminal_emitted = true;
    let evt = Event::default().event("done").data(final_payload.to_string());
    Some((Ok(evt), cursor))
}

/// Per-stream poll state.
struct EventCursor {
    run_id: Uuid,
    pool: sqlx::PgPool,
    /// Highest seq we've emitted; the next DB poll asks for `> seq`.
    seq: i32,
    /// Events fetched but not yet emitted. Stack — reversed so pop()
    /// returns oldest first.
    buffered: Vec<TestRunEventRow>,
    terminal_emitted: bool,
}

#[derive(sqlx::FromRow)]
struct TestRunEventRow {
    seq: i32,
    event_type: String,
    payload: serde_json::Value,
    occurred_at: chrono::DateTime<chrono::Utc>,
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
