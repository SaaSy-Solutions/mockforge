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
    models::{FitnessFunction, TestRun, UsageCounter},
    AppState,
};
use axum::extract::Query;
use serde::Serialize;

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
        "fitness_evaluation" => {
            // Architectural fitness functions (#355). Two responsibilities:
            //   1. Persist the per-run measurement row so the page can
            //      render a "last 30 evaluations" timeline + flip
            //      `fitness_functions.last_status` for the inline pill.
            //   2. Raise an incident on non-passing terminal status so
            //      the existing dispatcher fires notification channels.
            //
            // The two are independent: a recording failure shouldn't
            // skip the alert, and an alerting failure shouldn't drop
            // the historical record. Best-effort + log on either side.
            let measured_value =
                summary.and_then(|s| s.get("measured_value")).and_then(|v| v.as_f64());
            let threshold_value =
                summary.and_then(|s| s.get("threshold_value")).and_then(|v| v.as_f64());

            // Map the `test_runs.status` axis onto the `fitness_evaluations.status`
            // axis. `errored` and `cancelled` are operationally distinct
            // from `failed`: we don't have a measurement, so the row
            // gets `unknown` rather than implying the assertion failed
            // when it never ran.
            let eval_status = match run.status.as_str() {
                "passed" => "pass",
                "failed" => "fail",
                _ => "unknown",
            };

            if let Err(e) = FitnessFunction::record_evaluation(
                pool,
                run.suite_id,
                eval_status,
                measured_value,
                threshold_value,
            )
            .await
            {
                tracing::warn!(
                    run_id = %run.id,
                    function_id = %run.suite_id,
                    error = %e,
                    "failed to record fitness evaluation history",
                );
            }

            // Alerting on `failed` and `errored`. Cancelled stays
            // user-initiated and shouldn't auto-page; passed is fine.
            // Mirrors the smoke wiring (issue #392).
            if !matches!(run.status.as_str(), "passed" | "cancelled") {
                use mockforge_registry_core::models::incident::RaiseIncidentInput;
                use mockforge_registry_core::models::Incident;

                let breaking_count = summary
                    .and_then(|s| s.get("breaking_count"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let findings_count = summary
                    .and_then(|s| s.get("findings_count"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let function_name = summary
                    .and_then(|s| s.get("function_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let severity = if run.status == "errored" {
                    "medium"
                } else {
                    "high"
                };
                let title = if run.status == "errored" {
                    "Fitness function evaluation errored".to_string()
                } else if !function_name.is_empty() {
                    format!("Fitness function '{}' failed", function_name)
                } else if breaking_count > 0 {
                    format!(
                        "{} breaking fitness violation{}",
                        breaking_count,
                        if breaking_count == 1 { "" } else { "s" },
                    )
                } else {
                    "Fitness function failed".to_string()
                };
                let dedupe_key = run.suite_id.to_string();
                let source_ref = run.id.to_string();
                let description = serde_json::json!({
                    "fitness_function_id": run.suite_id,
                    "run_id": run.id,
                    "run_status": run.status,
                    "function_name": function_name,
                    "measured_value": measured_value,
                    "threshold_value": threshold_value,
                    "breaking_count": breaking_count,
                    "findings_count": findings_count,
                })
                .to_string();

                Incident::raise(
                    pool,
                    RaiseIncidentInput {
                        org_id: run.org_id,
                        // workspace_id stays None — same caveat as smoke (#392):
                        // the TestRun row carries org + suite_id, not workspace.
                        // A follow-up could JOIN through fitness_functions to
                        // populate this so workspace-filtered routing rules
                        // apply; for now org-wide rules are the right scope.
                        workspace_id: None,
                        source: "fitness_function",
                        source_ref: Some(&source_ref),
                        dedupe_key: &dedupe_key,
                        severity,
                        title: &title,
                        description: Some(&description),
                    },
                )
                .await?;
            }
        }
        // Other kinds (unit/contract_diff/replay/flow.*) don't have a
        // separate per-resource status — the test_runs row is the
        // source of truth.
        _ => {}
    }
    Ok(())
}

/// Wire-format row returned by the capture-exchanges endpoint. Pulls
/// the columns the replay executor needs from runtime_captures, joined
/// against capture_session_members so only exchanges in the requested
/// session are returned.
#[allow(missing_docs)] // wire-format struct; columns documented in migration
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct CaptureExchangeRow {
    pub capture_id: String,
    pub method: String,
    pub path: String,
    pub query_params: Option<String>,
    pub request_headers: String,
    pub request_body: Option<String>,
    pub request_body_encoding: String,
    pub response_status_code: Option<i32>,
    pub response_headers: Option<String>,
    pub response_body: Option<String>,
    pub response_body_encoding: Option<String>,
    pub duration_ms: Option<i64>,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
}

/// `GET /api/v1/internal/tunnel-reservations/by-subdomain/{subdomain}`
///
/// Internal-only — the tunnel relay (mockforge-tunnel deployed as a
/// Fly app) calls this to authorize incoming subdomain claims. Returns
/// the reservation row when the subdomain exists + is in
/// status='reserved', else 404. Lets the relay enforce the cloud
/// reservation table without copying the schema into its own store.
pub async fn get_tunnel_reservation_by_subdomain(
    State(state): State<AppState>,
    Path(subdomain): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    use mockforge_registry_core::models::TunnelReservation;
    require_internal_auth(&headers)?;

    let row = TunnelReservation::find_by_subdomain(state.db.pool(), &subdomain)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Subdomain not reserved".into()))?;

    // Echo back the fields the relay needs to authorize the connection
    // and route to the right backend. We deliberately don't include
    // workspace_id or other internals the relay shouldn't need.
    Ok(Json(serde_json::json!({
        "id": row.id,
        "org_id": row.org_id,
        "name": row.name,
        "subdomain": row.subdomain,
        "custom_domain": row.custom_domain,
        "custom_domain_verified": row.custom_domain_verified,
        "status": row.status,
    })))
}

/// `POST /api/v1/internal/hosted-mocks/{id}/chaos`
///
/// Internal proxy that forwards a chaos-toggle request to the hosted
/// mock's admin endpoint. The chaos executor calls this for
/// target_kind=hosted_mock; we resolve the deployment's internal Fly
/// URL and POST to its `/__mockforge/chaos/toggle`. Lets the runner
/// inject real faults without needing direct network access to the
/// container's admin port.
#[derive(Debug, Deserialize)]
pub struct ChaosToggleRequest {
    pub enabled: bool,
}

pub async fn proxy_chaos_toggle(
    State(state): State<AppState>,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<ChaosToggleRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    use mockforge_registry_core::models::HostedMock;
    require_internal_auth(&headers)?;

    let deployment = HostedMock::find_by_id(state.db.pool(), deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".into()))?;

    let base = deployment
        .internal_url
        .as_deref()
        .or(deployment.deployment_url.as_deref())
        .ok_or_else(|| {
            ApiError::InvalidRequest(
                "Deployment has neither internal_url nor deployment_url".into(),
            )
        })?;

    // Trim any trailing path; we always target /__mockforge/chaos/toggle.
    let target = format!(
        "{}/__mockforge/chaos/toggle",
        base.trim_end_matches('/').trim_end_matches("/__mockforge")
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("mockforge-registry-chaos-proxy/1.0")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = client
        .post(&target)
        .json(&serde_json::json!({ "enabled": body.enabled }))
        .send()
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("chaos proxy fetch failed: {e}")))?;

    let status = resp.status();
    let payload: serde_json::Value = resp.json().await.unwrap_or_else(|_| serde_json::json!({}));
    if !status.is_success() {
        return Err(ApiError::InvalidRequest(format!(
            "deployment refused chaos toggle: HTTP {status}"
        )));
    }
    Ok(Json(serde_json::json!({
        "enabled": body.enabled,
        "deployment_response": payload,
    })))
}

/// One (method, path) tuple observed in the workspace's recent
/// runtime_captures, with hit count.
#[allow(missing_docs)]
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct WorkspaceEndpointHit {
    pub method: String,
    pub path: String,
    pub hits: i64,
}

/// `GET /api/v1/internal/workspaces/{id}/endpoint-hits`
///
/// Internal-only — the contract diff executor calls this to compare
/// actual traffic against the declared OpenAPI spec. Returns each
/// (method, path) combo seen in the workspace's recent captures, with
/// hit count, ordered by hits desc.
pub async fn get_workspace_endpoint_hits(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<WorkspaceEndpointHit>>> {
    require_internal_auth(&headers)?;
    let rows = sqlx::query_as::<_, WorkspaceEndpointHit>(
        r#"
        SELECT rc.method,
               rc.path,
               COUNT(*) AS hits
          FROM runtime_captures rc
         WHERE rc.workspace_id = $1
           AND rc.occurred_at >= NOW() - INTERVAL '24 hours'
         GROUP BY rc.method, rc.path
         ORDER BY hits DESC
         LIMIT 500
        "#,
    )
    .bind(workspace_id)
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

/// `GET /api/v1/internal/capture-sessions/{id}/exchanges`
///
/// Internal-only — the replay executor calls this to fetch the
/// captured exchanges it should replay against the target URL.
/// Cross-deployment because runtime_captures rows can come from any
/// hosted-mock in the org; the session itself owns the authoritative
/// list via capture_session_members.
pub async fn get_capture_exchanges(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<CaptureExchangeRow>>> {
    require_internal_auth(&headers)?;
    let rows = sqlx::query_as::<_, CaptureExchangeRow>(
        r#"
        SELECT rc.capture_id,
               rc.method,
               rc.path,
               rc.query_params,
               rc.request_headers,
               rc.request_body,
               rc.request_body_encoding,
               rc.response_status_code,
               rc.response_headers,
               rc.response_body,
               rc.response_body_encoding,
               rc.duration_ms,
               rc.occurred_at
          FROM runtime_captures rc
          JOIN capture_session_members csm
            ON csm.capture_id = rc.capture_id::uuid
         WHERE csm.session_id = $1
         ORDER BY rc.occurred_at ASC
         LIMIT 1000
        "#,
    )
    .bind(session_id)
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

/// `GET /api/v1/internal/fitness-functions/{id}`
///
/// Internal-only — the FitnessExecutor (`mockforge-test-runner`) calls
/// this to fetch the function definition (kind + config) before
/// dispatching to the kind-specific evaluator. Returns the full
/// `FitnessFunction` row; the executor reads `kind` for dispatch and
/// pulls everything else (deployment_id, threshold, window) from
/// `config`.
pub async fn get_fitness_function(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<FitnessFunction>> {
    require_internal_auth(&headers)?;
    let row = FitnessFunction::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Fitness function not found".into()))?;
    Ok(Json(row))
}

/// Latency aggregate for a deployment over a window. All values in
/// milliseconds. `count` is the number of request-log rows that fed
/// the aggregate; `error_count` is rows with status >= 500.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct DeploymentLatencyStats {
    pub count: i64,
    pub error_count: i64,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
    pub max_ms: Option<f64>,
    pub avg_ms: Option<f64>,
}

/// Query params for the latency-stats endpoint. Both bounded — the
/// window can't exceed 24h (matches the `runtime_request_logs`
/// retention model) and the path filter caps at 256 chars.
#[derive(Debug, Deserialize)]
pub struct LatencyStatsQuery {
    /// Look-back window. Defaults to 60 minutes when missing; clamped
    /// to `[1, 1440]` (1 minute to 24 hours).
    #[serde(default)]
    pub window_minutes: Option<i64>,
    /// Optional path filter. Exact match when set; full deployment
    /// when omitted. The fitness executor passes this through from
    /// the function's `config.path` if the user specified one.
    #[serde(default)]
    pub path: Option<String>,
}

/// `GET /api/v1/internal/deployments/{id}/latency-stats`
///
/// Internal-only — the FitnessExecutor calls this from the runner
/// when evaluating `kind='latency_threshold'` checks. Returns
/// percentile latencies (p50/p95/p99/max), avg, count, and error
/// count over the requested window. Any of the percentile fields can
/// be `None` when the count is zero (no traffic).
pub async fn get_deployment_latency_stats(
    State(state): State<AppState>,
    Path(deployment_id): Path<Uuid>,
    Query(params): Query<LatencyStatsQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<DeploymentLatencyStats>> {
    require_internal_auth(&headers)?;
    let window = params.window_minutes.unwrap_or(60).clamp(1, 1440);
    let path_filter = params.path.as_deref().filter(|s| !s.is_empty() && s.len() <= 256);

    // Postgres `percentile_cont(WITHIN GROUP)` for percentiles —
    // standard library function, no extension required. The casts to
    // `DOUBLE PRECISION` keep the result type predictable across PG
    // versions (some return numeric).
    // Use sqlx::FromRow on the response struct directly so we don't
    // have to spell out a 7-arity tuple type (clippy::type_complexity).
    let stats = sqlx::query_as::<_, DeploymentLatencyStatsRow>(
        r#"
        SELECT
            COUNT(*)::bigint                                                     AS count,
            COUNT(*) FILTER (WHERE status >= 500)::bigint                         AS error_count,
            (percentile_cont(0.50) WITHIN GROUP (ORDER BY latency_ms))::float8    AS p50_ms,
            (percentile_cont(0.95) WITHIN GROUP (ORDER BY latency_ms))::float8    AS p95_ms,
            (percentile_cont(0.99) WITHIN GROUP (ORDER BY latency_ms))::float8    AS p99_ms,
            MAX(latency_ms)::float8                                                AS max_ms,
            AVG(latency_ms)::float8                                                AS avg_ms
        FROM runtime_request_logs
        WHERE deployment_id = $1
          AND occurred_at >= NOW() - ($2::bigint * INTERVAL '1 minute')
          AND ($3::text IS NULL OR path = $3)
        "#,
    )
    .bind(deployment_id)
    .bind(window)
    .bind(path_filter)
    .fetch_one(state.db.pool())
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(DeploymentLatencyStats {
        count: stats.count,
        error_count: stats.error_count,
        p50_ms: stats.p50_ms,
        p95_ms: stats.p95_ms,
        p99_ms: stats.p99_ms,
        max_ms: stats.max_ms,
        avg_ms: stats.avg_ms,
    }))
}

/// SQL row shape for the latency-stats query. Identical fields to the
/// public `DeploymentLatencyStats` response struct, but with sqlx's
/// FromRow derive so we don't have to spell out a 7-arity tuple type.
#[derive(Debug, sqlx::FromRow)]
struct DeploymentLatencyStatsRow {
    count: i64,
    error_count: i64,
    p50_ms: Option<f64>,
    p95_ms: Option<f64>,
    p99_ms: Option<f64>,
    max_ms: Option<f64>,
    avg_ms: Option<f64>,
}

/// Aggregate of contract-diff findings for a monitored service, used
/// by `kind='contract_stability'` fitness checks. Counts findings by
/// severity over the configured window.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct MonitoredServiceContractStability {
    pub breaking_count: i64,
    pub non_breaking_count: i64,
    pub cosmetic_count: i64,
    pub run_count: i64,
    /// Most recent diff-run start timestamp inside the window. `None`
    /// when no runs have happened — distinguishes "no data" from
    /// "lots of data but no findings". ISO-8601 string for wire
    /// stability across runner builds.
    pub latest_run_at: Option<String>,
}

/// Query params for the contract-stability endpoint.
#[derive(Debug, Deserialize)]
pub struct ContractStabilityQuery {
    /// Look-back window. Default 24h since contract diffs run far
    /// less frequently than smoke / latency probes (typically once
    /// per deploy or once per day on a schedule). Clamped to
    /// `[1, 10080]` (1 minute to one week).
    #[serde(default)]
    pub window_minutes: Option<i64>,
}

/// `GET /api/v1/internal/monitored-services/{id}/contract-stability`
///
/// Internal-only — the FitnessExecutor calls this when evaluating
/// `kind='contract_stability'` checks. Aggregates `contract_diff_findings`
/// joined to `contract_diff_runs.monitored_service_id` over the
/// requested window.
pub async fn get_monitored_service_contract_stability(
    State(state): State<AppState>,
    Path(monitored_service_id): Path<Uuid>,
    Query(params): Query<ContractStabilityQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<MonitoredServiceContractStability>> {
    require_internal_auth(&headers)?;
    // 1 week ceiling — contract diffs are expensive and a longer
    // window gives misleading "stable" signals when an old breaking
    // finding has already been fixed.
    let window = params.window_minutes.unwrap_or(1_440).clamp(1, 10_080);

    let row = sqlx::query_as::<_, ContractStabilityRow>(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE f.severity = 'breaking')::bigint     AS breaking_count,
            COUNT(*) FILTER (WHERE f.severity = 'non_breaking')::bigint AS non_breaking_count,
            COUNT(*) FILTER (WHERE f.severity = 'cosmetic')::bigint     AS cosmetic_count,
            (SELECT COUNT(*) FROM contract_diff_runs
                WHERE monitored_service_id = $1
                  AND started_at >= NOW() - ($2::bigint * INTERVAL '1 minute'))::bigint
                                                                         AS run_count,
            (SELECT MAX(started_at) FROM contract_diff_runs
                WHERE monitored_service_id = $1
                  AND started_at >= NOW() - ($2::bigint * INTERVAL '1 minute'))
                                                                         AS latest_run_at
        FROM contract_diff_findings f
        JOIN contract_diff_runs r ON f.run_id = r.id
        WHERE r.monitored_service_id = $1
          AND r.started_at >= NOW() - ($2::bigint * INTERVAL '1 minute')
        "#,
    )
    .bind(monitored_service_id)
    .bind(window)
    .fetch_one(state.db.pool())
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(MonitoredServiceContractStability {
        breaking_count: row.breaking_count,
        non_breaking_count: row.non_breaking_count,
        cosmetic_count: row.cosmetic_count,
        run_count: row.run_count,
        latest_run_at: row.latest_run_at.map(|t| t.to_rfc3339()),
    }))
}

/// SQL row shape for contract-stability. Severity counts come from
/// the FILTERed COUNTs on `contract_diff_findings`; run_count + latest
/// timestamp come from a correlated subquery so we get sensible
/// values even when there are zero findings (i.e. count rows from
/// `contract_diff_runs` directly rather than relying on the JOIN).
#[derive(Debug, sqlx::FromRow)]
struct ContractStabilityRow {
    breaking_count: i64,
    non_breaking_count: i64,
    cosmetic_count: i64,
    run_count: i64,
    latest_run_at: Option<chrono::DateTime<chrono::Utc>>,
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
