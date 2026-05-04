//! Minimal LLM HTTP client for cloud AI Studio.
//!
//! Intentionally narrow: we only need single-turn chat completions on two
//! provider shapes (OpenAI-compatible, Anthropic) for v1. The broader
//! `mockforge-ai-core` crate has a richer abstraction but is currently
//! orphaned and doesn't compile, so this module avoids it.
//!
//! Provider mapping:
//! - `openai`, `openai-compatible`, `together`, `fireworks`, `openrouter`,
//!   `groq` → OpenAI Chat Completions wire format.
//! - `anthropic` → Anthropic Messages wire format.
//!
//! Endpoints default to each provider's public hostname when `base_url`
//! is `None`. Callers can override via the BYOK `base_url` field.

use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};

/// Inputs to a single chat completion call.
#[derive(Debug, Clone)]
pub struct LlmCall {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub system: String,
    pub user: String,
    pub temperature: f64,
    pub max_tokens: u32,
}

/// Output from a chat completion call.
#[derive(Debug, Clone)]
pub struct LlmResult {
    pub content: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

impl LlmResult {
    pub fn total_tokens(&self) -> u64 {
        self.prompt_tokens + self.completion_tokens
    }
}

/// Dispatch to the appropriate provider implementation.
pub async fn call_llm(call: LlmCall) -> ApiResult<LlmResult> {
    let provider = call.provider.to_lowercase();
    match provider.as_str() {
        "anthropic" | "claude" => call_anthropic(call).await,
        // Everything else uses the OpenAI Chat Completions wire format.
        // OpenRouter, Together, Fireworks, Groq, Ollama, vLLM, LiteLLM all
        // implement this shape.
        _ => call_openai_compatible(call).await,
    }
}

#[derive(Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage<'a>>,
    temperature: f64,
    max_tokens: u32,
}

#[derive(Serialize)]
struct OpenAiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: OpenAiUsage,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}

#[derive(Deserialize, Default)]
struct OpenAiUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
}

async fn call_openai_compatible(call: LlmCall) -> ApiResult<LlmResult> {
    let base = call
        .base_url
        .as_deref()
        .unwrap_or("https://api.openai.com")
        .trim_end_matches('/');
    let url = format!("{base}/v1/chat/completions");

    let body = OpenAiRequest {
        model: &call.model,
        messages: vec![
            OpenAiMessage {
                role: "system",
                content: &call.system,
            },
            OpenAiMessage {
                role: "user",
                content: &call.user,
            },
        ],
        temperature: call.temperature,
        max_tokens: call.max_tokens,
    };

    let resp = reqwest::Client::new()
        .post(&url)
        .bearer_auth(&call.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("LLM HTTP error: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(ApiError::Internal(anyhow::anyhow!("LLM provider returned {status}: {text}")));
    }

    let parsed: OpenAiResponse = resp
        .json()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("LLM response parse error: {e}")))?;

    let content = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("LLM returned no choices")))?;

    Ok(LlmResult {
        content,
        prompt_tokens: parsed.usage.prompt_tokens,
        completion_tokens: parsed.usage.completion_tokens,
    })
}

#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<AnthropicMessage<'a>>,
    temperature: f64,
}

#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Deserialize, Default)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

async fn call_anthropic(call: LlmCall) -> ApiResult<LlmResult> {
    let base = call
        .base_url
        .as_deref()
        .unwrap_or("https://api.anthropic.com")
        .trim_end_matches('/');
    let url = format!("{base}/v1/messages");

    let body = AnthropicRequest {
        model: &call.model,
        max_tokens: call.max_tokens,
        system: &call.system,
        messages: vec![AnthropicMessage {
            role: "user",
            content: &call.user,
        }],
        temperature: call.temperature,
    };

    let resp = reqwest::Client::new()
        .post(&url)
        .header("x-api-key", &call.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("LLM HTTP error: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(ApiError::Internal(anyhow::anyhow!("LLM provider returned {status}: {text}")));
    }

    let parsed: AnthropicResponse = resp
        .json()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("LLM response parse error: {e}")))?;

    let content = parsed
        .content
        .into_iter()
        .find(|b| b.block_type == "text")
        .and_then(|b| b.text)
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Anthropic returned no text block")))?;

    Ok(LlmResult {
        content,
        prompt_tokens: parsed.usage.input_tokens,
        completion_tokens: parsed.usage.output_tokens,
    })
}
