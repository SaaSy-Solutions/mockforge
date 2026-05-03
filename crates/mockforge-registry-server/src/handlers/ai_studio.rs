//! Cloud AI Studio handlers.
//!
//! Endpoints under `/api/v1/ai-studio/*`. Every handler runs the same
//! pipeline:
//!
//!   resolve_org_context ─▶ load_byok_config ─▶ pick_provider
//!     ─▶ check_ai_quota ─▶ build LlmCall ─▶ call_llm
//!     ─▶ record_ai_usage ─▶ return content + UsageMeta
//!
//! That pipeline lives in `run_completion`. Public handlers just
//! build a `PromptInputs` and shape the response.
//!
//! See `docs/cloud/CLOUD_AI_STUDIO_DESIGN.md` for the full design.

use axum::{extract::State, http::HeaderMap, Json};
use mockforge_registry_core::models::{BYOKConfig, Plan};
use serde::{Deserialize, Serialize};

use crate::{
    ai::{
        call_llm, check_ai_quota, pick_provider, record_ai_usage, LlmCall, Provider,
        ProviderSelection,
    },
    error::{ApiError, ApiResult},
    handlers::settings::decrypt_api_key,
    middleware::{resolve_org_context, AuthUser},
    AppState,
};

const DEFAULT_TEMPERATURE: f64 = 0.7;
const DEFAULT_MAX_TOKENS: u32 = 1024;
const DEFAULT_SYSTEM_PROMPT: &str = "You are a helpful AI assistant integrated into MockForge.";

/// Common usage metadata embedded in every AI Studio response so the UI
/// can render the BYOK/platform badge and the quota meter without extra
/// round-trips.
#[derive(Debug, Serialize)]
pub struct UsageMeta {
    /// Which key paid for this call.
    pub provider: &'static str,
    /// Tokens used by this single call (prompt + completion).
    pub tokens_used: u64,
    /// Updated monthly counter, for the UI's quota meter.
    pub tokens_used_this_period: i64,
    /// Monthly platform-token limit. `-1` means unlimited.
    pub tokens_limit: i64,
}

/// Internal: what each handler hands to `run_completion`.
struct PromptInputs {
    system: String,
    user: String,
    /// Optional model override; falls back to BYOK / platform default.
    model: Option<String>,
    temperature: f64,
    max_tokens: u32,
}

// --- /chat ------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    /// User prompt; required.
    pub prompt: String,
    #[serde(default)]
    pub system: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub content: String,
    #[serde(flatten)]
    pub meta: UsageMeta,
}

/// `POST /api/v1/ai-studio/chat`
pub async fn chat(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<ChatRequest>,
) -> ApiResult<Json<ChatResponse>> {
    if request.prompt.trim().is_empty() {
        return Err(ApiError::InvalidRequest("prompt must not be empty".into()));
    }

    let inputs = PromptInputs {
        system: request.system.unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.into()),
        user: request.prompt,
        model: request.model,
        temperature: request.temperature.unwrap_or(DEFAULT_TEMPERATURE),
        max_tokens: request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
    };

    let (content, meta) = run_completion(&state, user_id, &headers, inputs).await?;
    Ok(Json(ChatResponse { content, meta }))
}

// --- /generate-openapi ------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct GenerateOpenApiRequest {
    /// Natural-language description of the API to mock.
    pub description: String,
    /// Optional title for the generated spec.
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GenerateOpenApiResponse {
    /// Raw text returned by the LLM (useful for debugging).
    pub content: String,
    /// Best-effort parsed OpenAPI 3 document. `None` if the model
    /// response wasn't valid JSON; the UI can fall back to `content`.
    pub spec: Option<serde_json::Value>,
    #[serde(flatten)]
    pub meta: UsageMeta,
}

/// `POST /api/v1/ai-studio/generate-openapi`
pub async fn generate_openapi(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<GenerateOpenApiRequest>,
) -> ApiResult<Json<GenerateOpenApiResponse>> {
    if request.description.trim().is_empty() {
        return Err(ApiError::InvalidRequest("description must not be empty".into()));
    }

    let title = request.title.as_deref().unwrap_or("Generated API");
    let inputs = PromptInputs {
        system: format!(
            "You are an expert API designer. Generate a complete, valid OpenAPI 3.0 \
             specification in JSON for the API described by the user. Include realistic \
             paths, request/response schemas, examples, and at least one error response \
             per endpoint. Output ONLY the JSON document, no prose, no markdown fences. \
             Use `{title}` as the spec's `info.title` unless a different title is in the \
             user's description."
        ),
        user: request.description,
        model: request.model,
        // Lower temperature for structured output: we want valid JSON, not creativity.
        temperature: 0.2,
        // OpenAPI specs can be large; raise the cap.
        max_tokens: 4096,
    };

    let (content, meta) = run_completion(&state, user_id, &headers, inputs).await?;
    let spec = extract_json_payload(&content);
    Ok(Json(GenerateOpenApiResponse {
        content,
        spec,
        meta,
    }))
}

// --- /explain-rule ----------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ExplainRuleRequest {
    /// Identifier (e.g. rule name or path) — used in the prompt for context.
    pub rule_id: String,
    /// Rule definition (JSON). Anything serializable; the prompt embeds it as-is.
    pub definition: serde_json::Value,
    /// Optional extra context (e.g. surrounding workspace name).
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExplainRuleResponse {
    /// Plain-language explanation of what the rule does and when it fires.
    pub explanation: String,
    #[serde(flatten)]
    pub meta: UsageMeta,
}

/// `POST /api/v1/ai-studio/explain-rule`
pub async fn explain_rule(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<ExplainRuleRequest>,
) -> ApiResult<Json<ExplainRuleResponse>> {
    if request.rule_id.trim().is_empty() {
        return Err(ApiError::InvalidRequest("rule_id must not be empty".into()));
    }
    let definition_str = serde_json::to_string_pretty(&request.definition).map_err(|e| {
        ApiError::InvalidRequest(format!("definition must be serializable JSON: {e}"))
    })?;

    let context_blurb = request
        .context
        .as_ref()
        .map(|c| format!("\n\nContext: {c}"))
        .unwrap_or_default();

    let inputs = PromptInputs {
        system: "You are a senior engineer explaining MockForge mock rules to a junior \
                 teammate. Be specific: when does this rule fire, what does it return, \
                 and what edge cases does it cover? Keep it under 200 words and avoid \
                 marketing language."
            .into(),
        user: format!(
            "Rule id: {id}\n\nDefinition:\n```json\n{def}\n```{ctx}",
            id = request.rule_id,
            def = definition_str,
            ctx = context_blurb,
        ),
        model: request.model,
        temperature: 0.4,
        max_tokens: 800,
    };

    let (explanation, meta) = run_completion(&state, user_id, &headers, inputs).await?;
    Ok(Json(ExplainRuleResponse { explanation, meta }))
}

// --- shared pipeline --------------------------------------------------------

/// Runs the full provider-routing + quota + LLM-call + metering pipeline
/// for one prompt. Returns the model's raw text plus the usage metadata
/// every AI Studio response embeds.
async fn run_completion(
    state: &AppState,
    user_id: uuid::Uuid,
    headers: &HeaderMap,
    prompt: PromptInputs,
) -> ApiResult<(String, UsageMeta)> {
    // 1. Auth + plan info.
    let org_ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;

    // 2. BYOK lookup.
    let byok = load_byok_config(state, org_ctx.org_id).await?;

    // 3. Provider routing (pure).
    let is_paid_plan = matches!(org_ctx.org.plan(), Plan::Pro | Plan::Team);
    let provider = pick_provider(is_paid_plan, byok);

    // 4. Pre-call quota check.
    let quota = check_ai_quota(state, &org_ctx.org, provider.selection()).await?;
    if !quota.allowed {
        return Err(quota.into_error());
    }

    // 5. Build LLM call.
    let selection = provider.selection();
    let llm_call = build_llm_call(&provider, prompt)?;

    // 6. Call.
    let result = call_llm(llm_call).await?;

    // 7. Meter (Platform only; BYOK skips the platform quota).
    let total_tokens = result.total_tokens();
    record_ai_usage(state, org_ctx.org_id, selection, total_tokens as i64).await?;

    // 8. Build response metadata.
    let billed_now = if matches!(selection, ProviderSelection::Platform) {
        total_tokens as i64
    } else {
        0
    };

    let meta = UsageMeta {
        provider: match selection {
            ProviderSelection::Byok => "byok",
            ProviderSelection::Platform => "platform",
            ProviderSelection::Disabled => "disabled", // unreachable: quota check above
        },
        tokens_used: total_tokens,
        tokens_used_this_period: quota.used + billed_now,
        tokens_limit: quota.limit,
    };

    Ok((result.content, meta))
}

/// Read the org's BYOK config, returning `None` if missing or disabled.
async fn load_byok_config(state: &AppState, org_id: uuid::Uuid) -> ApiResult<Option<BYOKConfig>> {
    let setting = state.store.get_org_setting(org_id, "byok").await?;
    let Some(setting) = setting else {
        return Ok(None);
    };

    let cfg: BYOKConfig = match serde_json::from_value(setting.setting_value) {
        Ok(c) => c,
        Err(_) => return Ok(None), // tolerate legacy/malformed rows by treating as no-BYOK
    };

    if !cfg.enabled || cfg.api_key.is_empty() {
        return Ok(None);
    }
    Ok(Some(cfg))
}

/// Translate the provider decision + prompt into an `LlmCall`.
fn build_llm_call(provider: &Provider, prompt: PromptInputs) -> ApiResult<LlmCall> {
    match provider {
        Provider::Disabled => Err(ApiError::ResourceLimitExceeded(
            "AI is not available — add a BYOK key or upgrade your plan".into(),
        )),
        Provider::Byok(cfg) => {
            let api_key = decrypt_api_key(&cfg.api_key)?;
            Ok(LlmCall {
                provider: cfg.provider.clone(),
                model: prompt
                    .model
                    .or_else(|| cfg.model.clone())
                    .unwrap_or_else(|| "gpt-4o-mini".into()),
                api_key,
                base_url: cfg.base_url.clone(),
                system: prompt.system,
                user: prompt.user,
                temperature: prompt.temperature,
                max_tokens: prompt.max_tokens,
            })
        }
        Provider::Platform => {
            let api_key = std::env::var("MOCKFORGE_PLATFORM_LLM_API_KEY").map_err(|_| {
                ApiError::Internal(anyhow::anyhow!(
                    "Platform LLM not configured: MOCKFORGE_PLATFORM_LLM_API_KEY missing"
                ))
            })?;
            let provider_name = std::env::var("MOCKFORGE_PLATFORM_LLM_PROVIDER")
                .unwrap_or_else(|_| "openai".into());
            let default_model = std::env::var("MOCKFORGE_PLATFORM_LLM_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini".into());
            let endpoint = std::env::var("MOCKFORGE_PLATFORM_LLM_ENDPOINT").ok();

            Ok(LlmCall {
                provider: provider_name,
                model: prompt.model.unwrap_or(default_model),
                api_key,
                base_url: endpoint,
                system: prompt.system,
                user: prompt.user,
                temperature: prompt.temperature,
                max_tokens: prompt.max_tokens,
            })
        }
    }
}

/// Best-effort JSON extraction. Returns `None` if `text` doesn't look like
/// JSON. Tolerates a single ```json fence wrapper because models love
/// adding them despite system-prompt instructions.
fn extract_json_payload(text: &str) -> Option<serde_json::Value> {
    let trimmed = text.trim();
    let stripped = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|s| s.trim_start())
        .unwrap_or(trimmed);
    let stripped = stripped.strip_suffix("```").map(str::trim_end).unwrap_or(stripped);

    serde_json::from_str(stripped).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_handles_plain() {
        let v = extract_json_payload(r#"{"openapi": "3.0.0"}"#).unwrap();
        assert_eq!(v["openapi"], "3.0.0");
    }

    #[test]
    fn extract_json_handles_fenced_block() {
        let v = extract_json_payload("```json\n{\"openapi\": \"3.0.0\"}\n```").unwrap();
        assert_eq!(v["openapi"], "3.0.0");
    }

    #[test]
    fn extract_json_handles_unfenced_with_whitespace() {
        let v = extract_json_payload("\n  {\"x\": 1}  \n").unwrap();
        assert_eq!(v["x"], 1);
    }

    #[test]
    fn extract_json_returns_none_for_prose() {
        assert!(extract_json_payload("Sure, here's the spec…").is_none());
    }
}
