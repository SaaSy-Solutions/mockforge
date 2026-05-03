//! Cloud AI Studio handlers.
//!
//! Currently exposes:
//! - `POST /api/v1/ai-studio/chat`  — single-turn chat completion routed
//!   through provider selection + quota enforcement.
//!
//! Future routes (`generate-openapi`, `learn`, `rules`, `voice/transcribe`)
//! will land here as additional handlers; they all share the same
//! `pick_provider` + `check_ai_quota` + `record_ai_usage` flow.
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

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    /// System prompt sent to the LLM.
    #[serde(default)]
    pub system: Option<String>,
    /// User prompt. Required.
    pub prompt: String,
    /// Optional override of the model name (subject to plan + provider support).
    #[serde(default)]
    pub model: Option<String>,
    /// 0.0–2.0, defaults to 0.7 if omitted.
    #[serde(default)]
    pub temperature: Option<f64>,
    /// Defaults to 1024 if omitted.
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    /// Generated text.
    pub content: String,
    /// Which key paid for this call.
    pub provider: &'static str,
    /// Tokens used (prompt + completion).
    pub tokens_used: u64,
    /// Updated monthly usage counter, for the UI's quota meter.
    pub tokens_used_this_period: i64,
    /// Monthly limit (-1 = unlimited).
    pub tokens_limit: i64,
}

const DEFAULT_TEMPERATURE: f64 = 0.7;
const DEFAULT_MAX_TOKENS: u32 = 1024;
const DEFAULT_SYSTEM_PROMPT: &str = "You are a helpful AI assistant integrated into MockForge.";

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

    // 1. Resolve org context (auth + plan info).
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;

    // 2. Look up BYOK config (if any).
    let byok = load_byok_config(&state, org_ctx.org_id).await?;

    // 3. Pick provider (pure decision).
    let is_paid_plan = matches!(org_ctx.org.plan(), Plan::Pro | Plan::Team);
    let provider = pick_provider(is_paid_plan, byok);

    // 4. Quota check.
    let quota = check_ai_quota(&state, &org_ctx.org, provider.selection()).await?;
    if !quota.allowed {
        return Err(quota.into_error());
    }

    // 5. Build LLM call from the chosen provider.
    let llm_call = build_llm_call(&provider, request)?;
    let selection = provider.selection();

    // 6. Run the call.
    let result = call_llm(llm_call).await?;

    // 7. Record token usage (only billed for Platform requests; BYOK skips).
    let total_tokens = result.total_tokens();
    record_ai_usage(&state, org_ctx.org_id, selection, total_tokens as i64).await?;

    // 8. Return response with the updated counter.
    let billed_now = if matches!(selection, ProviderSelection::Platform) {
        total_tokens as i64
    } else {
        0
    };

    Ok(Json(ChatResponse {
        content: result.content,
        provider: match selection {
            ProviderSelection::Byok => "byok",
            ProviderSelection::Platform => "platform",
            ProviderSelection::Disabled => "disabled", // unreachable: blocked by quota check
        },
        tokens_used: total_tokens,
        tokens_used_this_period: quota.used + billed_now,
        tokens_limit: quota.limit,
    }))
}

/// Read the org's BYOK config, returning `None` if missing or disabled.
/// Decryption of the API key happens later in `build_llm_call` only when
/// we actually need to use it.
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

/// Translate the provider decision + request into an `LlmCall`.
fn build_llm_call(provider: &Provider, request: ChatRequest) -> ApiResult<LlmCall> {
    let system = request.system.unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.into());
    let temperature = request.temperature.unwrap_or(DEFAULT_TEMPERATURE);
    let max_tokens = request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);

    match provider {
        Provider::Disabled => Err(ApiError::ResourceLimitExceeded(
            "AI is not available — add a BYOK key or upgrade your plan".into(),
        )),
        Provider::Byok(cfg) => {
            let api_key = decrypt_api_key(&cfg.api_key)?;
            Ok(LlmCall {
                provider: cfg.provider.clone(),
                model: request
                    .model
                    .or_else(|| cfg.model.clone())
                    .unwrap_or_else(|| "gpt-4o-mini".into()),
                api_key,
                base_url: cfg.base_url.clone(),
                system,
                user: request.prompt,
                temperature,
                max_tokens,
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
                model: request.model.unwrap_or(default_model),
                api_key,
                base_url: endpoint,
                system,
                user: request.prompt,
                temperature,
                max_tokens,
            })
        }
    }
}
