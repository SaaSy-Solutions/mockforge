//! Cloud Test Generator background worker (#469 Phase 3).
//!
//! Drains `cloud_test_generation_jobs WHERE status = 'queued'` in queued_at
//! order. For each claimed row:
//!
//!   1. Look up the workspace + org so we know which plan + BYOK config to
//!      apply.
//!   2. Pick a provider via the existing `ai::provider::pick_provider`
//!      decision (Byok / Platform / Disabled — same pipeline AI Studio
//!      uses, so quota and billing semantics stay consistent).
//!   3. Sample recent `runtime_captures` matching the job's
//!      `captures_filter`. Phase 3 supports a minimal filter vocabulary
//!      (`method`, `path_contains`, `status_min`, `status_max`, `limit`);
//!      anything else is ignored. The worker is forgiving — invalid
//!      filters fall back to "most recent 25 captures for the workspace".
//!   4. Build a JSON-focused prompt asking the LLM to emit a
//!      `{ scenarios: [...] }` document.
//!   5. Call `ai::client::call_llm` (the same dispatch the AI Studio
//!      handlers use).
//!   6. Best-effort JSON parse on the response — accept either a raw
//!      JSON object, a code-fenced block, or fall back to a `{ raw: "..." }`
//!      wrapper so the user always gets *something* in the result column.
//!   7. Write `result` + flip status to 'succeeded' on success, or write
//!      `error` + flip to 'failed' on any of the failure paths above.
//!
//! State transitions are gated on `WHERE status = 'running'`, so a user
//! who cancels a job mid-flight (Phase 2's UI lets them) wins the race
//! — the worker's terminal write is a no-op.
//!
//! Interval defaults to 5s but can be overridden via
//! `TEST_GENERATION_WORKER_INTERVAL_SECS` (clamped to ≥1s).
//! Set `TEST_GENERATION_WORKER_DISABLED=1` to skip wiring the worker
//! (useful in tests + local dev).

use std::time::Duration;

use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    ai::{
        client::{call_llm, LlmCall, LlmResult},
        provider::{pick_provider, Provider},
    },
    handlers::settings::decrypt_api_key,
    AppState,
};
use mockforge_registry_core::models::{
    organization::Plan, test_generation_job::TestGenerationJob, BYOKConfig, Organization,
};

/// Default poll cadence. Test-generation jobs are interactive (user
/// queues from the UI and watches the timeline), so 5s feels live
/// without burning DB time on idle scans.
const DEFAULT_INTERVAL_SECS: u64 = 5;

/// Hard cap on captures we feed the LLM in one job. Each capture is
/// dozens of tokens; 25 keeps us under every modern provider's prompt
/// limit even after the system prompt and reply allowance.
const MAX_CAPTURES: i64 = 25;

/// Cap on the LLM completion size — generous enough for ~10 scenarios
/// in JSON, low enough that runaway responses can't burn the entire
/// org quota in one call.
const LLM_MAX_COMPLETION_TOKENS: u32 = 2_000;

/// Start the test-generation worker. No-op when
/// `TEST_GENERATION_WORKER_DISABLED=1`.
pub fn start_test_generation_worker(state: AppState) {
    if std::env::var("TEST_GENERATION_WORKER_DISABLED").as_deref() == Ok("1") {
        info!("test_generation_worker: disabled via env");
        return;
    }

    let interval_secs = std::env::var("TEST_GENERATION_WORKER_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(DEFAULT_INTERVAL_SECS);

    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(interval_secs));
        // Skip the immediate first tick — give the server a moment to
        // settle (db pool, env, etc.).
        tick.tick().await;
        loop {
            tick.tick().await;
            if let Err(e) = drain_queue(&state).await {
                error!("test_generation_worker: drain failed: {e:?}");
            }
        }
    });

    info!("Test generation worker started (every {interval_secs}s)");
}

/// Drain the queue until we either run out of work or a single job
/// fails to claim. We process jobs serially per tick — concurrency is a
/// future optimisation if the queue ever sustains a backlog.
async fn drain_queue(state: &AppState) -> Result<(), sqlx::Error> {
    loop {
        let claimed = TestGenerationJob::claim_next_queued(state.db.pool()).await?;
        let Some(job) = claimed else {
            return Ok(());
        };
        let job_id = job.id;
        debug!(%job_id, "test_generation_worker: claimed job");
        match process_job(state, job).await {
            Ok(result) => {
                let changed =
                    TestGenerationJob::complete_success(state.db.pool(), job_id, &result).await?;
                if !changed {
                    // Almost certainly a cancellation race — log and move on.
                    debug!(%job_id, "test_generation_worker: success write was no-op (likely cancelled)");
                }
            }
            Err(reason) => {
                let reason_str = reason.to_string();
                warn!(%job_id, error = %reason_str, "test_generation_worker: job failed");
                let _ = TestGenerationJob::complete_failure(
                    state.db.pool(),
                    job_id,
                    reason_str.as_str(),
                )
                .await;
            }
        }
    }
}

/// Run one job end-to-end. Returns the `result` JSON value to persist
/// on success, or an error whose `to_string()` lands in the `error`
/// column on failure.
async fn process_job(state: &AppState, job: TestGenerationJob) -> Result<Value, WorkerError> {
    // 1. Org context for plan + BYOK decision.
    let org = Organization::find_by_id(state.db.pool(), job.org_id)
        .await
        .map_err(|e| WorkerError::Internal(format!("org lookup failed: {e}")))?
        .ok_or_else(|| WorkerError::Internal("Organization missing for job".into()))?;
    let plan = org.plan();
    let is_paid_plan = matches!(plan, Plan::Pro | Plan::Team);

    // 2. BYOK config (the worker can't fall back to platform credits
    // without rate-limit accounting plumbing — Phase 4 if/when we need
    // platform-credit generations).
    let byok = load_byok_config(state, job.org_id).await?;
    let provider = pick_provider(is_paid_plan, byok);

    let Provider::Byok(byok_cfg) = provider else {
        return Err(WorkerError::ProviderUnavailable(match plan {
            Plan::Free => "Test generation requires a BYOK provider key on the Free plan. Add a key in Settings → BYOK.".into(),
            _ => "Test generation requires a BYOK provider key. Platform-credit generations are not yet supported — add a BYOK key in Settings → BYOK.".into(),
        }));
    };

    let api_key = decrypt_api_key(&byok_cfg.api_key)
        .map_err(|e| WorkerError::Internal(format!("BYOK key decrypt failed: {e:?}")))?;

    // 3. Sample captures.
    let filter = parse_filter(&job.captures_filter);
    let captures = fetch_captures(state.db.pool(), job.workspace_id, &filter)
        .await
        .map_err(|e| WorkerError::Internal(format!("capture sampling failed: {e}")))?;

    if captures.is_empty() {
        return Err(WorkerError::EmptyCorpus(
            "No matching captures found in this workspace. Record some traffic first or relax the filter.".into(),
        ));
    }

    // 4. Build the prompt.
    let (system, user) = build_prompt(&job.prompt, &captures);

    // 5. Call the LLM via the existing dispatch.
    let call = LlmCall {
        provider: byok_cfg.provider.clone(),
        model: byok_cfg.model.clone().unwrap_or_else(|| "gpt-4o-mini".into()),
        api_key,
        base_url: byok_cfg.base_url.clone(),
        system,
        user,
        temperature: 0.2,
        max_tokens: LLM_MAX_COMPLETION_TOKENS,
    };
    let llm_result = call_llm(call).await.map_err(|e| WorkerError::LlmCall(format!("{e:?}")))?;

    // 6. Parse + assemble the persisted result.
    let scenarios = parse_scenarios(&llm_result.content);
    Ok(build_result_value(&llm_result, &byok_cfg.provider, scenarios, captures.len()))
}

// --- filter handling ------------------------------------------------------

#[derive(Debug, Default)]
struct CaptureFilter {
    method: Option<String>,
    path_contains: Option<String>,
    status_min: Option<i32>,
    status_max: Option<i32>,
    limit: i64,
}

/// Tolerant parse of the job's `captures_filter` JSON. Phase 3 supports
/// a deliberately small vocabulary — anything else is ignored so we
/// don't have to coordinate filter schema changes between Phase 2's UI
/// and this worker.
fn parse_filter(raw: &Value) -> CaptureFilter {
    let mut f = CaptureFilter {
        limit: MAX_CAPTURES,
        ..CaptureFilter::default()
    };
    let Some(obj) = raw.as_object() else {
        return f;
    };
    if let Some(s) = obj.get("method").and_then(|v| v.as_str()) {
        f.method = Some(s.to_uppercase());
    }
    if let Some(s) = obj.get("path_contains").and_then(|v| v.as_str()) {
        f.path_contains = Some(s.to_string());
    }
    if let Some(n) = obj.get("status_min").and_then(|v| v.as_i64()) {
        f.status_min = Some(n.clamp(100, 599) as i32);
    }
    if let Some(n) = obj.get("status_max").and_then(|v| v.as_i64()) {
        f.status_max = Some(n.clamp(100, 599) as i32);
    }
    if let Some(n) = obj.get("limit").and_then(|v| v.as_i64()) {
        f.limit = n.clamp(1, MAX_CAPTURES);
    }
    f
}

#[derive(Debug, Clone, FromRow)]
struct CaptureSample {
    method: String,
    path: String,
    #[sqlx(rename = "effective_status")]
    status: i32,
    duration_ms: i32,
}

async fn fetch_captures(
    pool: &PgPool,
    workspace_id: Uuid,
    filter: &CaptureFilter,
) -> sqlx::Result<Vec<CaptureSample>> {
    sqlx::query_as::<_, CaptureSample>(
        r#"
        SELECT method, path,
               COALESCE(response_status_code, status_code, 0) AS effective_status,
               COALESCE(duration_ms, 0) AS duration_ms
        FROM runtime_captures
        WHERE workspace_id = $1
          AND ($2::text IS NULL OR UPPER(method) = $2)
          AND ($3::text IS NULL OR position($3 IN path) > 0)
          AND ($4::int IS NULL
               OR COALESCE(response_status_code, status_code, 0) >= $4)
          AND ($5::int IS NULL
               OR COALESCE(response_status_code, status_code, 0) <= $5)
        ORDER BY occurred_at DESC
        LIMIT $6
        "#,
    )
    .bind(workspace_id)
    .bind(filter.method.as_deref())
    .bind(filter.path_contains.as_deref())
    .bind(filter.status_min)
    .bind(filter.status_max)
    .bind(filter.limit)
    .fetch_all(pool)
    .await
}

// --- prompt assembly ------------------------------------------------------

/// Build the (system, user) prompt pair. The system prompt forces JSON
/// output; the user prompt embeds the captures as a compact CSV-ish
/// list so we don't burn tokens on pretty-printing.
fn build_prompt(user_prompt: &str, captures: &[CaptureSample]) -> (String, String) {
    let system = "You are a senior test engineer. Given a sample of recent API requests, you propose concise test scenarios that would catch realistic regressions. Output ONLY a single JSON object on one line of the form: {\"scenarios\": [{\"name\": \"...\", \"description\": \"...\", \"method\": \"GET\", \"path\": \"/foo\", \"expected_status\": 200, \"rationale\": \"...\"}, ...]}. No prose, no code fences, no explanation. Up to 10 scenarios.".to_string();

    let mut lines = String::with_capacity(64 * captures.len());
    lines.push_str("Recent captures (method | path | status | duration_ms):\n");
    for c in captures {
        lines.push_str(&format!("{} {} {} {}\n", c.method, c.path, c.status, c.duration_ms));
    }
    let extra = if user_prompt.trim().is_empty() {
        String::new()
    } else {
        format!("\n\nFocus area from the user:\n{}", user_prompt.trim())
    };
    let user = format!("{lines}{extra}");
    (system, user)
}

/// Best-effort parse of the LLM reply into a JSON scenarios array.
/// Strategy:
///   1. Try parsing the whole reply as JSON.
///   2. If that fails, look for the first `{` / `}` pair and parse the
///      substring.
///   3. If that still fails, return None — the caller wraps the raw
///      content so the user can still see what came back.
fn parse_scenarios(content: &str) -> Option<Value> {
    let trimmed = content.trim();
    // Strip common code-fence wrappers since some providers ignore the
    // "no fences" instruction.
    let cleaned = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(v) = serde_json::from_str::<Value>(cleaned) {
        return Some(v);
    }

    // Locate the outermost balanced { ... } substring and try again.
    if let (Some(start), Some(end)) = (cleaned.find('{'), cleaned.rfind('}')) {
        if end > start {
            if let Ok(v) = serde_json::from_str::<Value>(&cleaned[start..=end]) {
                return Some(v);
            }
        }
    }
    None
}

/// Wrap the LLM output for persistence. We always include the raw
/// content so the UI can show what the provider said even when JSON
/// parse fails.
fn build_result_value(
    llm: &LlmResult,
    provider: &str,
    parsed: Option<Value>,
    captures_sampled: usize,
) -> Value {
    json!({
        "scenarios": parsed.as_ref().and_then(|v| v.get("scenarios").cloned()),
        "raw_parsed": parsed,
        "raw_content": llm.content,
        "model_meta": {
            "provider": provider,
            "prompt_tokens": llm.prompt_tokens,
            "completion_tokens": llm.completion_tokens,
        },
        "captures_sampled": captures_sampled,
    })
}

// --- BYOK lookup ----------------------------------------------------------

/// Read the org's BYOK config — same shape as
/// `handlers::ai_studio::load_byok_config`, copied here so the worker
/// doesn't depend on a `pub(crate)` helper that may move.
async fn load_byok_config(
    state: &AppState,
    org_id: Uuid,
) -> Result<Option<BYOKConfig>, WorkerError> {
    let setting = state
        .store
        .get_org_setting(org_id, "byok")
        .await
        .map_err(|e| WorkerError::Internal(format!("byok lookup failed: {e:?}")))?;
    let Some(setting) = setting else {
        return Ok(None);
    };
    let cfg: BYOKConfig = match serde_json::from_value(setting.setting_value) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    if !cfg.enabled || cfg.api_key.is_empty() {
        return Ok(None);
    }
    Ok(Some(cfg))
}

// --- errors ---------------------------------------------------------------

#[derive(Debug)]
enum WorkerError {
    ProviderUnavailable(String),
    EmptyCorpus(String),
    LlmCall(String),
    Internal(String),
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProviderUnavailable(m)
            | Self::EmptyCorpus(m)
            | Self::LlmCall(m)
            | Self::Internal(m) => write!(f, "{m}"),
        }
    }
}

// --- captures-filter deserializer (test-only) -----------------------------
//
// `parse_filter` is the runtime path; this struct only exists so tests can
// round-trip a filter through serde_json.
#[cfg(test)]
#[derive(Debug, serde::Deserialize)]
struct FilterRoundTrip {
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    path_contains: Option<String>,
    #[serde(default)]
    status_min: Option<i32>,
    #[serde(default)]
    status_max: Option<i32>,
    #[serde(default)]
    limit: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_filter_empty_defaults_to_max_captures() {
        let f = parse_filter(&json!({}));
        assert!(f.method.is_none());
        assert!(f.path_contains.is_none());
        assert!(f.status_min.is_none());
        assert!(f.status_max.is_none());
        assert_eq!(f.limit, MAX_CAPTURES);
    }

    #[test]
    fn parse_filter_normalises_method_uppercase_and_clamps_limit() {
        let f = parse_filter(&json!({
            "method": "post",
            "limit": 1_000_000,
        }));
        assert_eq!(f.method.as_deref(), Some("POST"));
        assert_eq!(f.limit, MAX_CAPTURES, "limit clamped to MAX_CAPTURES");
    }

    #[test]
    fn parse_filter_clamps_status_to_http_range() {
        let f = parse_filter(&json!({
            "status_min": 50,
            "status_max": 999,
        }));
        assert_eq!(f.status_min, Some(100));
        assert_eq!(f.status_max, Some(599));
    }

    #[test]
    fn parse_filter_round_trips_path_contains() {
        let f = parse_filter(&json!({"path_contains": "/auth/"}));
        assert_eq!(f.path_contains.as_deref(), Some("/auth/"));
    }

    #[test]
    fn parse_filter_handles_non_object_input() {
        // Defensive: the column NOT-NULL default is '{}::jsonb', but
        // upstream code paths or future migrations could produce other
        // shapes. We want to fall back to defaults rather than panic.
        let f = parse_filter(&Value::Null);
        assert!(f.method.is_none());
        let f = parse_filter(&json!("not an object"));
        assert!(f.method.is_none());
    }

    #[test]
    fn build_prompt_includes_captures_and_user_focus() {
        let captures = vec![
            CaptureSample {
                method: "GET".into(),
                path: "/users".into(),
                status: 200,
                duration_ms: 12,
            },
            CaptureSample {
                method: "POST".into(),
                path: "/users".into(),
                status: 201,
                duration_ms: 34,
            },
        ];
        let (system, user) = build_prompt("focus on auth", &captures);
        assert!(system.contains("scenarios"));
        assert!(user.contains("GET /users 200 12"));
        assert!(user.contains("POST /users 201 34"));
        assert!(user.contains("focus on auth"));
    }

    #[test]
    fn build_prompt_omits_user_focus_block_when_empty() {
        let captures = vec![CaptureSample {
            method: "GET".into(),
            path: "/".into(),
            status: 200,
            duration_ms: 1,
        }];
        let (_, user) = build_prompt("   ", &captures);
        assert!(!user.contains("Focus area"));
    }

    #[test]
    fn parse_scenarios_accepts_plain_json() {
        let v = parse_scenarios(r#"{"scenarios": [{"name": "ok"}]}"#).unwrap();
        assert_eq!(v["scenarios"][0]["name"], "ok");
    }

    #[test]
    fn parse_scenarios_strips_code_fences() {
        let raw = "```json\n{\"scenarios\": [1, 2]}\n```";
        let v = parse_scenarios(raw).unwrap();
        assert_eq!(v["scenarios"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn parse_scenarios_recovers_from_prose_wrapper() {
        let raw =
            "Sure! Here you go:\n{\"scenarios\": [\"a\", \"b\"]}\nLet me know if you need more.";
        let v = parse_scenarios(raw).unwrap();
        assert_eq!(v["scenarios"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn parse_scenarios_returns_none_on_unrecoverable_input() {
        assert!(parse_scenarios("not json at all").is_none());
    }

    #[test]
    fn build_result_value_preserves_raw_content_when_parse_fails() {
        let llm = LlmResult {
            content: "garbage".into(),
            prompt_tokens: 100,
            completion_tokens: 50,
        };
        let v = build_result_value(&llm, "openai", None, 5);
        assert_eq!(v["raw_content"], "garbage");
        assert!(v["scenarios"].is_null());
        assert!(v["raw_parsed"].is_null());
        assert_eq!(v["model_meta"]["prompt_tokens"], 100);
        assert_eq!(v["captures_sampled"], 5);
    }

    #[test]
    fn build_result_value_hoists_scenarios_field() {
        let llm = LlmResult {
            content: r#"{"scenarios": [{"name": "happy"}]}"#.into(),
            prompt_tokens: 0,
            completion_tokens: 0,
        };
        let parsed = parse_scenarios(&llm.content);
        let v = build_result_value(&llm, "openai", parsed, 3);
        assert_eq!(v["scenarios"][0]["name"], "happy");
        assert_eq!(v["captures_sampled"], 3);
    }

    #[test]
    fn worker_error_display_unwraps_message() {
        assert_eq!(format!("{}", WorkerError::EmptyCorpus("no rows".into())), "no rows");
    }

    #[test]
    fn filter_round_trip_smoke() {
        // Just confirm the test-only deserializer compiles + parses the
        // documented vocabulary. Real parsing goes through parse_filter
        // which is non-strict by design.
        let f: FilterRoundTrip = serde_json::from_value(json!({
            "method": "GET",
            "status_min": 400,
            "limit": 10,
        }))
        .unwrap();
        assert_eq!(f.method.as_deref(), Some("GET"));
        assert_eq!(f.status_min, Some(400));
        assert_eq!(f.limit, Some(10));
        assert!(f.path_contains.is_none());
    }
}
