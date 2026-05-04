//! Internal callback endpoints for the mockforge-test-runner worker.
//!
//! These three routes form the contract between the registry (state
//! owner) and the runner (work executor):
//! - `POST /api/v1/internal/test-runs/{id}/start`  — mark `running`
//! - `POST /api/v1/internal/test-runs/{id}/events` — append to event log
//! - `POST /api/v1/internal/test-runs/{id}/finish` — mark terminal +
//!   bump runner_seconds_used billing meter
//!
//! Auth: shared bearer token loaded from `MOCKFORGE_INTERNAL_API_TOKEN`.
//! mTLS is the planned production posture; for Phase 1 we ship the
//! token-based check so the runner can be exercised end-to-end while
//! cert plumbing lands separately.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    models::{TestRun, UsageCounter},
    AppState,
};

/// Verify the request carries the shared internal-API bearer token.
/// Returns InvalidRequest("Not found") for any auth failure — same
/// shape the public routes use for cross-org probes, so a leaked /
/// curl'd request can't enumerate which run-ids exist.
fn require_internal_auth(headers: &HeaderMap) -> ApiResult<()> {
    let configured = match std::env::var("MOCKFORGE_INTERNAL_API_TOKEN") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            return Err(ApiError::Internal(anyhow::anyhow!(
                "MOCKFORGE_INTERNAL_API_TOKEN not configured"
            )));
        }
    };
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::InvalidRequest("Not found".into()))?;
    // Constant-time comparison to avoid timing side channels on the
    // shared secret.
    if !constant_time_eq(provided.as_bytes(), configured.as_bytes()) {
        return Err(ApiError::InvalidRequest("Not found".into()));
    }
    Ok(())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// `POST /api/v1/internal/test-runs/{id}/start`
pub async fn run_started(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    require_internal_auth(&headers)?;
    let updated = TestRun::mark_running(state.db.pool(), id).await.map_err(ApiError::Database)?;
    if updated.is_none() {
        // Either the run doesn't exist or it's not in 'queued'. Either
        // way, idempotent no-op for the runner — return 200 so retries
        // don't escalate.
        return Ok(Json(serde_json::json!({ "status": "noop" })));
    }
    Ok(Json(serde_json::json!({ "status": "running" })))
}

#[derive(Debug, Deserialize)]
pub struct EventBody {
    /// Sequence number assigned by the runner. Must be unique within
    /// the run — the schema's UNIQUE(run_id, seq) enforces it.
    pub seq: i32,
    pub event_type: String,
    pub payload: serde_json::Value,
}

/// `POST /api/v1/internal/test-runs/{id}/events`
pub async fn run_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<EventBody>,
) -> ApiResult<Json<serde_json::Value>> {
    require_internal_auth(&headers)?;

    sqlx::query(
        "INSERT INTO test_run_events (run_id, seq, event_type, payload) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (run_id, seq) DO NOTHING",
    )
    .bind(id)
    .bind(body.seq)
    .bind(&body.event_type)
    .bind(&body.payload)
    .execute(state.db.pool())
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({ "appended": true })))
}

#[derive(Debug, Deserialize)]
pub struct FinishBody {
    /// Terminal status: one of passed | failed | cancelled | errored.
    pub status: String,
    pub runner_seconds: i32,
    #[serde(default)]
    pub summary: Option<serde_json::Value>,
}

/// `POST /api/v1/internal/test-runs/{id}/finish`
///
/// Marks the run terminal (idempotent — already-terminal rows are not
/// changed) and increments the org's runner_seconds_used billing meter
/// by the reported delta.
pub async fn run_finished(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<FinishBody>,
) -> ApiResult<Json<serde_json::Value>> {
    require_internal_auth(&headers)?;

    if !matches!(body.status.as_str(), "passed" | "failed" | "cancelled" | "errored") {
        return Err(ApiError::InvalidRequest(
            "status must be passed | failed | cancelled | errored".into(),
        ));
    }
    if body.runner_seconds < 0 {
        return Err(ApiError::InvalidRequest("runner_seconds must be non-negative".into()));
    }

    let run = TestRun::mark_finished(
        state.db.pool(),
        id,
        &body.status,
        body.runner_seconds,
        body.summary.as_ref(),
    )
    .await
    .map_err(ApiError::Database)?;

    if let Some(run) = run {
        // Bill the org for runner-seconds. Note we read org_id from
        // the test_runs row (set at queue-time), not from the request
        // — the runner can't lie about which org to charge.
        if body.runner_seconds > 0 {
            UsageCounter::increment_runner_seconds(
                state.db.pool(),
                run.org_id,
                body.runner_seconds as i64,
            )
            .await
            .map_err(ApiError::Database)?;
        }

        // Cross-table status mirroring: for kinds where the owning
        // resource has its own status column, flip it here so the UI's
        // resource view stays in sync without a separate poll. Each
        // mark_* helper is idempotent — already-terminal rows are not
        // changed.
        if let Err(e) = mirror_kind_status(&state, &run, body.summary.as_ref()).await {
            tracing::error!(
                run_id = %run.id,
                kind = %run.kind,
                error = %e,
                "failed to mirror run status onto owning resource — UI may show stale status",
            );
        }

        return Ok(Json(serde_json::json!({
            "status": run.status,
            "runner_seconds": run.runner_seconds,
        })));
    }
    // Already terminal or row gone — idempotent no-op.
    Ok(Json(serde_json::json!({ "status": "noop" })))
}

/// Cross-table mirror: when the test_run lifecycle finishes, also flip
/// the owning resource's status (snapshot.status, clone_model.status,
/// etc.) so the resource view doesn't show "capturing"/"training" for
/// rows whose backing run already passed/failed.
///
/// Each branch is a no-op if the kind doesn't have a paired status
/// column. All mark_* helpers are idempotent on the terminal value.
async fn mirror_kind_status(
    state: &AppState,
    run: &TestRun,
    summary: Option<&serde_json::Value>,
) -> sqlx::Result<()> {
    use mockforge_registry_core::models::chaos::CreateChaosCampaignReport;
    use mockforge_registry_core::models::{ChaosCampaignReport, CloneModel, Snapshot};

    let pool = state.db.pool();
    match run.kind.as_str() {
        "snapshot_capture" => {
            if run.status == "passed" {
                let storage_url = summary
                    .and_then(|s| s.get("storage_url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("synthetic://snapshot");
                let size_bytes =
                    summary.and_then(|s| s.get("size_bytes")).and_then(|v| v.as_i64()).unwrap_or(0);
                let manifest = summary.cloned().unwrap_or_else(|| serde_json::json!({}));
                Snapshot::mark_ready(pool, run.suite_id, storage_url, size_bytes, &manifest)
                    .await?;
            } else {
                Snapshot::mark_failed(pool, run.suite_id).await?;
            }
        }
        "behavioral_clone" => {
            if run.status == "passed" {
                let artifact_url = summary
                    .and_then(|s| s.get("artifact_url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("synthetic://clone-model");
                let metrics = summary.cloned().unwrap_or_else(|| serde_json::json!({}));
                CloneModel::mark_ready(
                    pool,
                    run.suite_id,
                    artifact_url,
                    &metrics,
                    run.runner_seconds.unwrap_or(0),
                )
                .await?;
            } else {
                CloneModel::mark_failed(pool, run.suite_id).await?;
            }
        }
        "chaos_campaign" => {
            // Persist the per-run report row so the campaign's history
            // tab has data, regardless of pass/fail. Aborted runs land
            // here too (status='failed'/'cancelled') so the UI can
            // show abort_reason.
            let aborted = !matches!(run.status.as_str(), "passed");
            let abort_reason: Option<String> = if aborted {
                summary
                    .and_then(|s| s.get("abort_reason"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .or_else(|| Some(run.status.clone()))
            } else {
                None
            };
            let fault_count = summary
                .and_then(|s| s.get("fault_count"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .clamp(0, i32::MAX as i64) as i32;
            let recommendations = summary.and_then(|s| s.get("recommendations").cloned());
            ChaosCampaignReport::create(
                pool,
                CreateChaosCampaignReport {
                    campaign_id: run.suite_id,
                    run_id: run.id,
                    fault_count,
                    aborted,
                    abort_reason: abort_reason.as_deref(),
                    summary,
                    recommendations: recommendations.as_ref(),
                },
            )
            .await?;
        }
        // Other kinds (unit/contract_diff/replay/flow.*) don't have a
        // separate per-resource status — the test_runs row is the
        // source of truth.
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ct_eq_returns_true_for_equal() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn ct_eq_returns_false_for_different() {
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }
}
