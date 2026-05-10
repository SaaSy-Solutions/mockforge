//! Internal endpoint for the AI-assisted contract drift second pass
//! (#348).
//!
//! `mockforge-test-runner`'s `ContractExecutor` calls this endpoint
//! after its structural diff so we can pick up sampled exchanges,
//! run them through an LLM, and return scored drift findings the
//! runner emits as additional `diff_finding` events. The runner can't
//! call the LLM itself because the BYOK key + platform key live on
//! the registry side and we don't want to plumb decryption through the
//! queue payload.
//!
//! Auth model: shared internal bearer token (`MOCKFORGE_INTERNAL_API_TOKEN`),
//! same as every other handler under `/api/v1/internal/*`.

use axum::{extract::State, http::HeaderMap, Json};
use chrono::{DateTime, Utc};
use mockforge_registry_core::models::Organization;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ai::contract_diff::{build_prompt, parse_findings, AiFinding, SampledExchange},
    error::{ApiError, ApiResult},
    handlers::ai_studio::{extract_json_payload, run_completion_for_org, PromptInputs},
    AppState,
};

/// Per-endpoint sample cap. The model needs at least 2-3 exchanges to
/// confidently call out drift; more than 5 just inflates the prompt
/// without much marginal signal. Hard-capped server-side so a runner
/// can't burn LLM credits by asking for hundreds of samples.
const MAX_SAMPLES_PER_ENDPOINT: i64 = 5;
const DEFAULT_SAMPLES_PER_ENDPOINT: i64 = 3;

/// How many endpoints we'll sample at once. The structural pass can
/// emit hundreds of findings on a busy workspace; AI scoring all of
/// them on every run is expensive. The runner is expected to send a
/// representative subset (e.g. the structural pass's "declared with
/// traffic" set, capped at this many).
const MAX_ENDPOINTS_PER_REQUEST: usize = 25;

#[derive(Debug, Deserialize)]
pub struct EndpointSpec {
    /// HTTP method (case-insensitive — normalised to uppercase before
    /// the runtime_captures lookup).
    pub method: String,
    /// Path as the spec declares it. Matched literally against
    /// `runtime_captures.path`; we don't expand parameter templates
    /// (`/users/{id}` → `/users/42`) here because the runner is in a
    /// better position to resolve them via its own pattern matching.
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct ScoreRequest {
    pub org_id: Uuid,
    pub workspace_id: Uuid,
    /// Excerpt of the OpenAPI spec scoped to the endpoints under
    /// review. The runner trims this to keep the prompt bounded.
    pub spec_excerpt: String,
    pub endpoints: Vec<EndpointSpec>,
    /// Optional override; defaults to [`DEFAULT_SAMPLES_PER_ENDPOINT`]
    /// and is hard-capped at [`MAX_SAMPLES_PER_ENDPOINT`].
    #[serde(default)]
    pub max_samples_per_endpoint: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ScoreResponse {
    pub findings: Vec<AiFinding>,
    /// Tokens consumed by this scoring call. Mostly informational for
    /// the runner so it can log it; metering already happened
    /// server-side via `record_ai_usage`.
    pub tokens_used: u64,
    /// `byok` | `platform` | `disabled` — same set used by AI Studio.
    pub provider: &'static str,
    /// `true` when no exchanges were found across any endpoint, so the
    /// runner can emit a "skipped — no traffic" log instead of a noisy
    /// empty result.
    pub no_traffic: bool,
}

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

/// Sub-set of `runtime_captures` columns the AI scorer cares about.
/// Anything outside this list (headers, query_params, request_id,
/// trace_id, etc.) is intentionally dropped — the model only consults
/// method, path, status, and bodies, so widening the row type would
/// just inflate token cost.
#[derive(sqlx::FromRow)]
struct SampleRow {
    method: String,
    path: String,
    status_code: Option<i32>,
    request_body: Option<String>,
    response_body: Option<String>,
    #[allow(dead_code)] // ordering only; not embedded in the prompt
    occurred_at: DateTime<Utc>,
}

/// Pull up to `limit` recent capture rows for one (workspace, method,
/// path) tuple. Newest first. Bodies come back verbatim — the prompt
/// builder handles truncation.
async fn fetch_samples(
    state: &AppState,
    workspace_id: Uuid,
    method: &str,
    path: &str,
    limit: i64,
) -> ApiResult<Vec<SampledExchange>> {
    let rows: Vec<SampleRow> = sqlx::query_as(
        r#"
        SELECT method,
               path,
               status_code,
               request_body,
               response_body,
               occurred_at
          FROM runtime_captures
         WHERE workspace_id = $1
           AND UPPER(method) = $2
           AND path = $3
           AND occurred_at >= NOW() - INTERVAL '24 hours'
         ORDER BY occurred_at DESC
         LIMIT $4
        "#,
    )
    .bind(workspace_id)
    .bind(method.to_uppercase())
    .bind(path)
    .bind(limit)
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::Database)?;

    Ok(rows
        .into_iter()
        .map(|r| SampledExchange {
            method: r.method,
            path: r.path,
            status_code: r.status_code,
            request_body: r.request_body,
            response_body: r.response_body,
        })
        .collect())
}

/// `POST /api/v1/internal/contract-diff/score`
///
/// Internal — runner-only. Returns a structured set of LLM-scored
/// drift findings the runner emits as `diff_finding` events.
pub async fn score_contract_drift(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ScoreRequest>,
) -> ApiResult<Json<ScoreResponse>> {
    require_internal_auth(&headers)?;

    if request.endpoints.is_empty() {
        return Ok(Json(ScoreResponse {
            findings: Vec::new(),
            tokens_used: 0,
            provider: "disabled",
            no_traffic: true,
        }));
    }

    let endpoints: Vec<EndpointSpec> =
        request.endpoints.into_iter().take(MAX_ENDPOINTS_PER_REQUEST).collect();

    let limit = request
        .max_samples_per_endpoint
        .unwrap_or(DEFAULT_SAMPLES_PER_ENDPOINT)
        .clamp(1, MAX_SAMPLES_PER_ENDPOINT);

    // Pull samples for each endpoint, then flatten into one big list
    // for the prompt. The model ranks per-endpoint via the
    // (method, path) header in each block, so flattening doesn't lose
    // the grouping.
    let mut all_samples: Vec<SampledExchange> = Vec::new();
    for ep in &endpoints {
        let mut samples =
            fetch_samples(&state, request.workspace_id, &ep.method, &ep.path, limit).await?;
        all_samples.append(&mut samples);
    }

    if all_samples.is_empty() {
        return Ok(Json(ScoreResponse {
            findings: Vec::new(),
            tokens_used: 0,
            provider: "disabled",
            no_traffic: true,
        }));
    }

    let org = Organization::find_by_id(state.db.pool(), request.org_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".into()))?;

    let (system, user) = build_prompt(&request.spec_excerpt, &all_samples);

    // Conservative budget: the system prompt teaches the model to emit
    // a small JSON array, and we only sample up to ~25 endpoints × 5
    // exchanges, so 4k completion tokens is plenty. Temperature kept
    // low so the model treats this as classification rather than
    // creative writing.
    let prompt = PromptInputs {
        system,
        user,
        model: None,
        temperature: 0.2,
        max_tokens: 4096,
    };
    let (raw_text, meta) = run_completion_for_org(&state, &org, prompt).await?;

    let findings = match extract_json_payload(&raw_text) {
        Some(json) => parse_findings(&json),
        None => Vec::new(),
    };

    Ok(Json(ScoreResponse {
        findings,
        tokens_used: meta.tokens_used,
        provider: meta.provider,
        no_traffic: false,
    }))
}
