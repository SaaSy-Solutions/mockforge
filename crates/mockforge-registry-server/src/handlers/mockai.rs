//! Cloud MockAI handlers — rule explanations, learning from examples,
//! traffic-based OpenAPI generation.
//!
//! Replaces the local-only `/__mockforge/api/mockai/*` surface when the
//! UI is in cloud mode. The same `run_completion` pipeline as the rest
//! of AI Studio handles BYOK routing + quota metering.
//!
//! Routes:
//!   GET  /api/v1/workspaces/{workspace_id}/mockai/rule-explanations
//!   GET  /api/v1/workspaces/{workspace_id}/mockai/rule-explanations/{rule_id}
//!   POST /api/v1/workspaces/{workspace_id}/mockai/learn
//!   POST /api/v1/organizations/{org_id}/mockai/generate-openapi-from-traffic
//!
//! Cloud-enablement task #353.

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use mockforge_registry_core::models::{
    mockai_rule_explanation::UpsertMockaiRuleExplanation, MockaiRuleExplanation,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::ai_studio::{extract_json_payload, run_completion, PromptInputs, UsageMeta},
    middleware::AuthUser,
    AppState,
};

// --- list / get -------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListExplanationsQuery {
    #[serde(default)]
    pub rule_type: Option<String>,
    #[serde(default)]
    pub min_confidence: Option<f32>,
}

#[derive(Debug, Serialize)]
pub struct ListExplanationsResponse {
    pub explanations: Vec<MockaiRuleExplanation>,
    pub total: usize,
}

/// `GET /api/v1/workspaces/{workspace_id}/mockai/rule-explanations`
pub async fn list_rule_explanations(
    State(state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListExplanationsQuery>,
) -> ApiResult<Json<ListExplanationsResponse>> {
    let rows = MockaiRuleExplanation::list_by_workspace(
        state.db.pool(),
        workspace_id,
        query.rule_type.as_deref(),
        query.min_confidence,
    )
    .await
    .map_err(ApiError::Database)?;
    let total = rows.len();
    Ok(Json(ListExplanationsResponse {
        explanations: rows,
        total,
    }))
}

#[derive(Debug, Serialize)]
pub struct GetExplanationResponse {
    pub explanation: MockaiRuleExplanation,
}

/// `GET /api/v1/workspaces/{workspace_id}/mockai/rule-explanations/{rule_id}`
pub async fn get_rule_explanation(
    State(state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Path((workspace_id, rule_id)): Path<(Uuid, String)>,
) -> ApiResult<Json<GetExplanationResponse>> {
    let row = MockaiRuleExplanation::get_by_rule_id(state.db.pool(), workspace_id, &rule_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest(format!("Rule {rule_id} not found")))?;
    Ok(Json(GetExplanationResponse { explanation: row }))
}

// --- learn ------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LearnRequest {
    /// Example request/response pairs the LLM can pattern-match against.
    pub examples: Vec<serde_json::Value>,
    /// Optional config blob; passed through to the model verbatim so users
    /// can hint at what kind of rules they want emphasized.
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LearnResponse {
    pub success: bool,
    pub rules_generated: RulesGenerated,
    pub explanations: Vec<RuleSummary>,
    pub total_explanations: usize,
    #[serde(flatten)]
    pub meta: UsageMeta,
}

#[derive(Debug, Serialize, Default)]
pub struct RulesGenerated {
    pub consistency_rules: u32,
    pub schemas: u32,
    pub state_machines: u32,
    pub system_prompt: bool,
}

#[derive(Debug, Serialize)]
pub struct RuleSummary {
    pub rule_id: String,
    pub rule_type: String,
    pub confidence: f32,
    pub reasoning: String,
}

#[derive(Debug, Deserialize)]
struct ModelLearnOutput {
    #[serde(default)]
    rules: Vec<ModelRule>,
    #[serde(default)]
    rules_generated: Option<RulesGeneratedRaw>,
}

#[derive(Debug, Deserialize, Default)]
struct RulesGeneratedRaw {
    #[serde(default)]
    consistency_rules: u32,
    #[serde(default)]
    schemas: u32,
    #[serde(default)]
    state_machines: u32,
    #[serde(default)]
    system_prompt: bool,
}

#[derive(Debug, Deserialize)]
struct ModelRule {
    rule_id: String,
    rule_type: String,
    #[serde(default)]
    confidence: f32,
    reasoning: String,
    #[serde(default)]
    pattern_matches: serde_json::Value,
}

/// `POST /api/v1/workspaces/{workspace_id}/mockai/learn`
///
/// Submits examples to the LLM, asks it to derive rule explanations,
/// upserts the results into `cloud_mockai_rule_explanations`, and
/// returns a summary mirroring the local mockai response shape.
pub async fn learn_from_examples(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<LearnRequest>,
) -> ApiResult<Json<LearnResponse>> {
    if request.examples.is_empty() {
        return Err(ApiError::InvalidRequest("examples must not be empty".into()));
    }

    let examples_json = serde_json::to_string_pretty(&request.examples).map_err(|e| {
        ApiError::InvalidRequest(format!("examples must be serializable JSON: {e}"))
    })?;
    let config_blurb = request
        .config
        .as_ref()
        .and_then(|c| serde_json::to_string_pretty(c).ok())
        .map(|c| format!("\n\nConfig hints:\n```json\n{c}\n```"))
        .unwrap_or_default();

    let inputs = PromptInputs {
        system: "You are a MockAI rule deriver. Given example request/response pairs, output \
                 a JSON object with two fields: `rules` (array of {rule_id, rule_type, \
                 confidence (0..1), reasoning, pattern_matches}) and `rules_generated` \
                 (object with consistency_rules, schemas, state_machines, system_prompt counts). \
                 rule_type must be one of \"consistency\" | \"schema\" | \"state_machine\" | \
                 \"system_prompt\". Output ONLY the JSON, no prose, no markdown fences."
            .into(),
        user: format!("Examples:\n```json\n{examples_json}\n```{config_blurb}"),
        model: request.model,
        temperature: 0.3,
        max_tokens: 2048,
    };

    let (content, meta) = run_completion(&state, user_id, &headers, inputs).await?;
    let parsed_value = extract_json_payload(&content)
        .ok_or_else(|| ApiError::InvalidRequest("Model output was not parseable JSON".into()))?;
    let parsed: ModelLearnOutput = serde_json::from_value(parsed_value)
        .map_err(|e| ApiError::InvalidRequest(format!("Model output schema mismatch: {e}")))?;

    let pool = state.db.pool();
    let mut summaries = Vec::with_capacity(parsed.rules.len());
    for rule in &parsed.rules {
        MockaiRuleExplanation::upsert(
            pool,
            UpsertMockaiRuleExplanation {
                workspace_id,
                rule_id: &rule.rule_id,
                rule_type: &rule.rule_type,
                confidence: rule.confidence,
                source_examples: &serde_json::Value::Array(request.examples.clone()),
                reasoning: &rule.reasoning,
                pattern_matches: &rule.pattern_matches,
            },
        )
        .await
        .map_err(ApiError::Database)?;
        summaries.push(RuleSummary {
            rule_id: rule.rule_id.clone(),
            rule_type: rule.rule_type.clone(),
            confidence: rule.confidence,
            reasoning: rule.reasoning.clone(),
        });
    }

    let counts = parsed.rules_generated.unwrap_or_default();
    let total = summaries.len();
    Ok(Json(LearnResponse {
        success: true,
        rules_generated: RulesGenerated {
            consistency_rules: counts.consistency_rules,
            schemas: counts.schemas,
            state_machines: counts.state_machines,
            system_prompt: counts.system_prompt,
        },
        explanations: summaries,
        total_explanations: total,
        meta,
    }))
}

// --- generate openapi from traffic ------------------------------------------

#[derive(Debug, Deserialize)]
pub struct GenerateFromTrafficRequest {
    /// ISO-8601 lower bound on `runtime_request_logs.occurred_at`.
    #[serde(default)]
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    /// ISO-8601 upper bound on `runtime_request_logs.occurred_at`.
    #[serde(default)]
    pub until: Option<chrono::DateTime<chrono::Utc>>,
    /// Optional path-prefix filter (e.g. `/api/v1`).
    #[serde(default)]
    pub path_pattern: Option<String>,
    /// Lower bound on hits-per-(method,path); below this is dropped.
    #[serde(default)]
    pub min_confidence: Option<u32>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GenerateFromTrafficResponse {
    pub spec: Option<serde_json::Value>,
    pub content: String,
    pub metadata: TrafficGenMetadata,
    #[serde(flatten)]
    pub meta: UsageMeta,
}

#[derive(Debug, Serialize)]
pub struct TrafficGenMetadata {
    pub requests_analyzed: i64,
    pub paths_inferred: usize,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u64,
}

/// `POST /api/v1/organizations/{org_id}/mockai/generate-openapi-from-traffic`
///
/// Aggregates `runtime_request_logs` rows for all hosted mocks owned by
/// the org, builds a compact summary, and asks the LLM to synthesize an
/// OpenAPI 3.0 document. Returns the same `{spec, metadata}` shape as
/// the local endpoint plus the LLM `meta` block.
pub async fn generate_openapi_from_traffic(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<GenerateFromTrafficRequest>,
) -> ApiResult<Json<GenerateFromTrafficResponse>> {
    let started = std::time::Instant::now();
    let pool = state.db.pool();

    // Pull (method, path, hits, sample_status) tuples scoped to this org's
    // hosted mocks. We aggregate up-front so the prompt we send to the LLM
    // is bounded — without this, busy orgs could blow past the token limit.
    let path_filter = request.path_pattern.as_deref().unwrap_or("");
    let min_hits = request.min_confidence.unwrap_or(1) as i64;
    let rows: Vec<(String, String, i64, Option<i32>)> = sqlx::query_as(
        r#"
        SELECT r.method,
               r.path,
               COUNT(*)::bigint AS hits,
               MAX(r.status)::int AS sample_status
        FROM runtime_request_logs r
        JOIN hosted_mocks h ON h.id = r.deployment_id
        WHERE h.org_id = $1
          AND ($2::timestamptz IS NULL OR r.occurred_at >= $2)
          AND ($3::timestamptz IS NULL OR r.occurred_at <= $3)
          AND ($4 = '' OR r.path LIKE $4 || '%')
        GROUP BY r.method, r.path
        HAVING COUNT(*) >= $5
        ORDER BY hits DESC
        LIMIT 250
        "#,
    )
    .bind(org_id)
    .bind(request.since)
    .bind(request.until)
    .bind(path_filter)
    .bind(min_hits)
    .fetch_all(pool)
    .await
    .map_err(ApiError::Database)?;

    let requests_analyzed: i64 = rows.iter().map(|(_, _, hits, _)| *hits).sum();
    let paths_inferred = rows.len();

    if rows.is_empty() {
        return Err(ApiError::InvalidRequest(
            "No traffic recorded in the requested window — run some requests against a hosted mock first".into(),
        ));
    }

    // Build a compact \"method path -> hits / status\" summary; the LLM
    // generates an OpenAPI doc from this much faster than from raw logs.
    let traffic_summary: String = rows
        .iter()
        .map(|(method, path, hits, status)| {
            let status_str = status.map(|s| s.to_string()).unwrap_or_else(|| "?".into());
            format!("{method} {path}  (hits={hits}, sample_status={status_str})")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let inputs = PromptInputs {
        system: "You are an API archaeologist. Given a list of (method path, hits, sample_status) \
                 tuples observed against a service, infer a complete, valid OpenAPI 3.0 \
                 specification in JSON. Group by resource, infer path parameters from \
                 obvious id-shaped segments, infer request/response schemas conservatively. \
                 Output ONLY the JSON document, no prose, no markdown fences."
            .into(),
        user: format!("Observed traffic summary (top 250 unique routes):\n\n{traffic_summary}"),
        model: request.model,
        temperature: 0.2,
        max_tokens: 4096,
    };

    let (content, meta) = run_completion(&state, user_id, &headers, inputs).await?;
    let spec = extract_json_payload(&content);

    Ok(Json(GenerateFromTrafficResponse {
        spec,
        content,
        metadata: TrafficGenMetadata {
            requests_analyzed,
            paths_inferred,
            generated_at: chrono::Utc::now(),
            duration_ms: started.elapsed().as_millis() as u64,
        },
        meta,
    }))
}
