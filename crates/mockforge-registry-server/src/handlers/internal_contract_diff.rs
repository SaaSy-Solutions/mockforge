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
    store::with_org_context,
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
    /// Persisted run identity. Older runners may omit this and remain anonymous.
    #[serde(default)]
    pub run_id: Option<Uuid>,
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
    /// Legacy claimed actor: accepted only when it matches the persisted run.
    /// New runners send run_id instead of supplying an actor.
    #[serde(default)]
    pub triggered_by_user_id: Option<Uuid>,
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

/// Resolve attribution from trusted persisted state before reading captures.
async fn resolve_scoring_actor(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    workspace_id: Uuid,
    run_id: Option<Uuid>,
    claimed_actor: Option<Uuid>,
) -> ApiResult<Option<Uuid>> {
    let (owned_workspace, actor) = with_org_context(pool, org_id, |tx| {
        Box::pin(async move {
            let owned_workspace: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id=$1 AND org_id=$2)",
            )
            .bind(workspace_id)
            .bind(org_id)
            .fetch_one(&mut **tx)
            .await?;
            let actor: Option<Option<Uuid>> = if let Some(run_id) = run_id {
                sqlx::query_scalar(
                    "SELECT r.triggered_by_user FROM test_runs r \
                     JOIN test_suites s ON s.id=r.suite_id \
                     JOIN workspaces w ON w.id=s.workspace_id \
                     WHERE r.id=$1 AND r.org_id=$2 AND w.org_id=$2 \
                     AND r.kind='contract_diff' \
                     AND (s.workspace_id=$3 OR s.target_workspace_id=$3)",
                )
                .bind(run_id)
                .bind(org_id)
                .bind(workspace_id)
                .fetch_optional(&mut **tx)
                .await?
            } else {
                Some(None)
            };
            Ok((owned_workspace, actor))
        })
    })
    .await?;
    if !owned_workspace || actor.is_none() {
        return Err(ApiError::InvalidRequest("Not found".into()));
    }
    let actor = actor.flatten();
    if claimed_actor.is_some() && claimed_actor != actor {
        return Err(ApiError::InvalidRequest("Not found".into()));
    }
    Ok(actor)
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

    let actor_user_id = resolve_scoring_actor(
        state.db.runtime_pool(),
        request.org_id,
        request.workspace_id,
        request.run_id,
        request.triggered_by_user_id,
    )
    .await?;

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
    let (raw_text, meta) = run_completion_for_org(&state, &org, actor_user_id, prompt).await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database::Database,
        models::Plan,
        store::{PgRegistryStore, RegistryStore},
    };

    #[tokio::test]
    #[ignore = "requires isolated TEST_DATABASE_URL and NOBYPASSRLS APP_DATABASE_URL"]
    async fn scoring_actor_comes_from_owned_run_and_preserves_anonymous_jobs() {
        let db = Database::connect(&std::env::var("TEST_DATABASE_URL").unwrap()).await.unwrap();
        db.migrate().await.unwrap();
        let role: (bool, bool) = sqlx::query_as(
            "SELECT rolsuper, rolbypassrls FROM pg_roles WHERE rolname=current_user",
        )
        .fetch_one(db.runtime_pool())
        .await
        .unwrap();
        assert_eq!(role, (false, false), "test must exercise enforced tenant context");
        let store = PgRegistryStore::new(db.pool().clone());
        let mut orgs = Vec::new();
        let mut workspaces = Vec::new();
        for _ in 0..2 {
            let tag = Uuid::new_v4().simple().to_string();
            let user = store
                .create_user(&tag, &format!("{tag}@example.invalid"), "disabled-fixture-login")
                .await
                .unwrap();
            let org = store
                .create_organization("Actor fixture", &tag, user.id, Plan::Free)
                .await
                .unwrap();
            let workspace = Uuid::new_v4();
            sqlx::query("INSERT INTO workspaces(id,org_id,name,created_by) VALUES($1,$2,'Actor fixture',$3)")
                .bind(workspace).bind(org.id).bind(user.id).execute(db.pool()).await.unwrap();
            orgs.push(org);
            workspaces.push(workspace);
        }
        let suite = Uuid::new_v4();
        sqlx::query("INSERT INTO test_suites(id,workspace_id,name,kind,config) VALUES($1,$2,'Actor fixture','contract_diff','{}'::jsonb)")
            .bind(suite).bind(workspaces[0]).execute(db.pool()).await.unwrap();
        let manual = Uuid::new_v4();
        let scheduled = Uuid::new_v4();
        for (run, actor, source) in [
            (manual, Some(orgs[0].owner_id), "manual"),
            (scheduled, None, "schedule"),
        ] {
            sqlx::query("INSERT INTO test_runs(id,suite_id,org_id,kind,triggered_by,triggered_by_user,status) VALUES($1,$2,$3,'contract_diff',$4,$5,'running')")
                .bind(run).bind(suite).bind(orgs[0].id).bind(source).bind(actor).execute(db.pool()).await.unwrap();
        }
        let pool = db.runtime_pool();
        let other_workspace = Uuid::new_v4();
        sqlx::query("INSERT INTO workspaces(id,org_id,name,created_by) VALUES($1,$2,'Other actor fixture',$3)")
            .bind(other_workspace).bind(orgs[0].id).bind(orgs[0].owner_id).execute(db.pool()).await.unwrap();
        assert!(resolve_scoring_actor(pool, orgs[0].id, other_workspace, Some(manual), None)
            .await
            .is_err());
        sqlx::query("UPDATE test_suites SET target_workspace_id=$2 WHERE id=$1")
            .bind(suite)
            .bind(other_workspace)
            .execute(db.pool())
            .await
            .unwrap();
        assert_eq!(
            resolve_scoring_actor(pool, orgs[0].id, other_workspace, Some(manual), None)
                .await
                .unwrap(),
            Some(orgs[0].owner_id)
        );
        assert_eq!(
            resolve_scoring_actor(
                pool,
                orgs[0].id,
                workspaces[0],
                Some(manual),
                Some(orgs[0].owner_id)
            )
            .await
            .unwrap(),
            Some(orgs[0].owner_id)
        );
        assert_eq!(
            resolve_scoring_actor(pool, orgs[0].id, workspaces[0], Some(manual), None)
                .await
                .unwrap(),
            Some(orgs[0].owner_id)
        );
        assert_eq!(
            resolve_scoring_actor(pool, orgs[0].id, workspaces[0], Some(scheduled), None)
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            resolve_scoring_actor(pool, orgs[0].id, workspaces[0], None, None)
                .await
                .unwrap(),
            None
        );
        for (org, workspace, run, claim) in [
            (orgs[1].id, workspaces[1], Some(manual), None),
            (orgs[0].id, workspaces[1], Some(manual), None),
            (orgs[0].id, workspaces[0], Some(Uuid::new_v4()), None),
            (orgs[0].id, workspaces[0], Some(manual), Some(orgs[1].owner_id)),
            (orgs[0].id, workspaces[0], None, Some(orgs[0].owner_id)),
        ] {
            assert!(
                matches!(resolve_scoring_actor(pool, org, workspace, run, claim).await, Err(ApiError::InvalidRequest(message)) if message == "Not found")
            );
        }
        sqlx::query("UPDATE test_runs SET kind='unit' WHERE id=$1")
            .bind(manual)
            .execute(db.pool())
            .await
            .unwrap();
        assert!(resolve_scoring_actor(pool, orgs[0].id, workspaces[0], Some(manual), None)
            .await
            .is_err());
    }

    #[tokio::test]
    #[ignore = "requires isolated Postgres, NOBYPASSRLS runtime and local fixture LLM"]
    async fn scoring_handler_persists_manual_and_anonymous_ai_audits() {
        use crate::{
            circuit_breaker::CircuitBreakerRegistry, config::Config, storage::PluginStorage,
        };
        use axum::{routing::post, Router};
        use serde_json::json;
        use std::sync::Arc;

        let endpoint = std::env::var("MOCKFORGE_PLATFORM_LLM_ENDPOINT").unwrap();
        assert!(endpoint.starts_with("http://127.0.0.1:"), "only a local fixture is allowed");
        let db = Database::connect(&std::env::var("TEST_DATABASE_URL").unwrap()).await.unwrap();
        db.migrate().await.unwrap();
        let role: (bool, bool) =
            sqlx::query_as("SELECT rolsuper,rolbypassrls FROM pg_roles WHERE rolname=current_user")
                .fetch_one(db.runtime_pool())
                .await
                .unwrap();
        assert_eq!(role, (false, false));
        let owner_store = PgRegistryStore::new(db.pool().clone());
        let tag = Uuid::new_v4().simple().to_string();
        let owner = owner_store
            .create_user(&tag, &format!("{tag}@example.invalid"), "disabled-fixture-login")
            .await
            .unwrap();
        let org = owner_store
            .create_organization("AI audit fixture", &tag, owner.id, Plan::Pro)
            .await
            .unwrap();
        let workspace = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workspaces(id,org_id,name,created_by) VALUES($1,$2,'AI audit fixture',$3)",
        )
        .bind(workspace)
        .bind(org.id)
        .bind(owner.id)
        .execute(db.pool())
        .await
        .unwrap();
        let suite = Uuid::new_v4();
        sqlx::query("INSERT INTO test_suites(id,workspace_id,name,kind,config) VALUES($1,$2,'AI audit fixture','contract_diff','{}'::jsonb)")
            .bind(suite).bind(workspace).execute(db.pool()).await.unwrap();
        let manual = Uuid::new_v4();
        let scheduled = Uuid::new_v4();
        for (run, actor, source) in [
            (manual, Some(owner.id), "manual"),
            (scheduled, None, "schedule"),
        ] {
            sqlx::query("INSERT INTO test_runs(id,suite_id,org_id,kind,triggered_by,triggered_by_user,status) VALUES($1,$2,$3,'contract_diff',$4,$5,'running')")
                .bind(run).bind(suite).bind(org.id).bind(source).bind(actor).execute(db.pool()).await.unwrap();
        }
        let deployment = Uuid::new_v4();
        sqlx::query("INSERT INTO hosted_mocks(id,org_id,name,slug,config_json,status) VALUES($1,$2,'AI audit fixture',$3,'{}'::jsonb,'active')")
            .bind(deployment).bind(org.id).bind(&tag).execute(db.pool()).await.unwrap();
        sqlx::query("INSERT INTO runtime_captures(deployment_id,capture_id,protocol,occurred_at,method,path,request_headers,request_body_encoding,status_code,workspace_id,source) VALUES($1,$2,'http',NOW(),'GET','/audit','{}','utf8',200,$3,'hosted')")
            .bind(deployment).bind(Uuid::new_v4().to_string()).bind(workspace).execute(db.pool()).await.unwrap();

        let config = Config::load().unwrap();
        let storage = PluginStorage::new(&config).await.unwrap();
        let state = AppState {
            store: Arc::new(
                PgRegistryStore::new(db.runtime_pool().clone()).with_owner_pool(db.pool().clone()),
            ),
            db: db.clone(),
            config,
            storage,
            metrics: Arc::new(mockforge_observability::prometheus::MetricsRegistry::new()),
            analytics_db: None,
            redis: None,
            circuit_breakers: CircuitBreakerRegistry::new(),
            platform_signing: None,
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/score", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().route("/score", post(score_contract_drift)).with_state(state),
            )
            .await
            .unwrap();
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        let token = std::env::var("MOCKFORGE_INTERNAL_API_TOKEN").unwrap();
        for run in [Some(manual), Some(scheduled), None] {
            let response = client
                .post(&url)
                .bearer_auth(&token)
                .json(&json!({
                    "run_id":run,"org_id":org.id,"workspace_id":workspace,
                    "spec_excerpt":"openapi: 3.0.0","endpoints":[{"method":"GET","path":"/audit"}]
                }))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::OK);
            let body: serde_json::Value = response.json().await.unwrap();
            assert_eq!(body["provider"], "platform");
            assert_eq!(body["tokens_used"], 5);
            assert_eq!(body["no_traffic"], false);
        }
        let response = client
            .post(&url)
            .bearer_auth(&token)
            .json(&json!({
                "run_id":manual,"triggered_by_user_id":Uuid::new_v4(),
                "org_id":org.id,"workspace_id":workspace,"spec_excerpt":"openapi: 3.0.0",
                "endpoints":[{"method":"GET","path":"/audit"}]
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
        let audits: Vec<(Option<Uuid>, serde_json::Value)> = sqlx::query_as(
            "SELECT user_id,metadata FROM audit_logs WHERE org_id=$1 AND event_type='ai_usage' ORDER BY created_at,id"
        ).bind(org.id).fetch_all(db.pool()).await.unwrap();
        assert_eq!(audits.len(), 3, "rejected attribution must not create a usage audit");
        assert_eq!(
            audits.iter().map(|r| r.0).collect::<Vec<_>>(),
            vec![Some(owner.id), None, None]
        );
        for (_, metadata) in audits {
            assert_eq!(metadata["provider"], "platform");
            assert_eq!(metadata["prompt_tokens"], 3);
            assert_eq!(metadata["completion_tokens"], 2);
            assert_eq!(metadata["total_tokens"], 5);
        }
        let billed: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(ai_tokens_used),0)::bigint FROM usage_counters WHERE org_id=$1",
        )
        .bind(org.id)
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(billed, 15, "all accepted calls must bill only the originating org");
        server.abort();
        let _ = server.await;
    }
}
