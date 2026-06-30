//! Mock LLM endpoint (#912, #79 round 50 follow-up).
//!
//! Serves OpenAI-compatible (`POST /v1/chat/completions`, `GET /v1/models`)
//! and Anthropic-compatible (`POST /v1/messages`) endpoints so an agent
//! (Cursor, Claude Code, ChatGPT clients, custom agents) can point its base
//! URL at MockForge and receive correctly-shaped, deterministic completions
//! for scale / offline / failure testing. This is a MOCK: it never calls a
//! real model, it returns canned/templated text with realistic envelopes
//! (ids, `usage` token counts, `finish_reason` / `stop_reason`) and supports
//! SSE streaming when the caller sets `stream: true`.
//!
//! Mounted by `mockforge serve --llm-mock`.

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream::{self, Stream};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::time::Duration;

/// Runtime configuration for the mock LLM endpoint.
#[derive(Clone, Debug)]
pub struct LlmMockConfig {
    /// Canned assistant reply used when no per-request override applies.
    pub canned_reply: String,
    /// Model id echoed back in responses when the request omits one.
    pub default_model: String,
    /// When true, prepend a short echo of the user's last message to the
    /// reply so callers can confirm round-trip wiring.
    pub echo_prompt: bool,
    /// Per-chunk delay for streaming responses (milliseconds). 0 = no delay.
    pub stream_chunk_delay_ms: u64,
}

impl Default for LlmMockConfig {
    fn default() -> Self {
        Self {
            canned_reply: "This is a mock response from MockForge's LLM endpoint.".to_string(),
            default_model: "mockforge-mock-1".to_string(),
            echo_prompt: true,
            stream_chunk_delay_ms: 0,
        }
    }
}

/// Build the axum router exposing the mock LLM endpoints.
pub fn router(config: LlmMockConfig) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
        .route("/v1/messages", post(anthropic_messages))
        .with_state(config)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Approximate token count: whitespace-delimited words. Good enough for a
/// mock; real tokenizers differ but callers testing wiring only need a
/// plausible, monotonic-with-length number.
fn approx_tokens(text: &str) -> u32 {
    text.split_whitespace().count().max(1) as u32
}

/// Extract the last user message text from a list of OpenAI/Anthropic-shaped
/// messages. `content` may be a plain string or an array of content blocks
/// (`[{"type":"text","text":"..."}]`); both are handled.
fn last_user_text(messages: &[Value]) -> String {
    for m in messages.iter().rev() {
        if m.get("role").and_then(|r| r.as_str()) == Some("user") {
            return content_to_text(m.get("content"));
        }
    }
    // Fall back to the last message of any role.
    messages.last().map(|m| content_to_text(m.get("content"))).unwrap_or_default()
}

fn content_to_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

/// Produce the deterministic reply text for a request.
fn build_reply(config: &LlmMockConfig, messages: &[Value]) -> String {
    if config.echo_prompt {
        let prompt = last_user_text(messages);
        if !prompt.is_empty() {
            let trimmed: String = prompt.chars().take(120).collect();
            return format!("{} (you said: \"{}\")", config.canned_reply, trimmed);
        }
    }
    config.canned_reply.clone()
}

/// Deterministic id derived from the reply + a prefix, so repeated identical
/// requests yield stable ids (useful for snapshot tests) without pulling in a
/// random/uuid dependency.
fn stable_id(prefix: &str, seed: &str) -> String {
    let mut hash: u64 = 1469598103934665603; // FNV-1a offset
    for b in seed.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("{prefix}{hash:016x}")
}

/// Split a reply into streaming "tokens" (words, keeping trailing spaces) for
/// chunked SSE delivery.
fn stream_chunks(reply: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, word) in reply.split_whitespace().enumerate() {
        if i == 0 {
            out.push(word.to_string());
        } else {
            out.push(format!(" {word}"));
        }
    }
    if out.is_empty() {
        out.push(reply.to_string());
    }
    out
}

// ---------------------------------------------------------------------------
// OpenAI: GET /v1/models
// ---------------------------------------------------------------------------

async fn list_models(State(config): State<LlmMockConfig>) -> Json<Value> {
    Json(json!({
        "object": "list",
        "data": [{
            "id": config.default_model,
            "object": "model",
            "created": 0,
            "owned_by": "mockforge",
        }],
    }))
}

// ---------------------------------------------------------------------------
// OpenAI: POST /v1/chat/completions
// ---------------------------------------------------------------------------

async fn chat_completions(
    State(config): State<LlmMockConfig>,
    Json(body): Json<Value>,
) -> Response {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(&config.default_model)
        .to_string();
    let messages: Vec<Value> =
        body.get("messages").and_then(|m| m.as_array()).cloned().unwrap_or_default();
    let stream = body.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    let reply = build_reply(&config, &messages);
    let prompt_text = messages
        .iter()
        .map(|m| content_to_text(m.get("content")))
        .collect::<Vec<_>>()
        .join(" ");
    let prompt_tokens = approx_tokens(&prompt_text);
    let completion_tokens = approx_tokens(&reply);
    let id = stable_id("chatcmpl-", &reply);

    if stream {
        return openai_stream(config, id, model, reply).into_response();
    }

    Json(json!({
        "id": id,
        "object": "chat.completion",
        "created": 0,
        "model": model,
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": reply },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens,
        },
    }))
    .into_response()
}

fn openai_stream(
    config: LlmMockConfig,
    id: String,
    model: String,
    reply: String,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut events: Vec<Event> = Vec::new();
    // Initial role delta.
    events.push(sse_json(&json!({
        "id": id, "object": "chat.completion.chunk", "created": 0, "model": model,
        "choices": [{ "index": 0, "delta": { "role": "assistant" }, "finish_reason": Value::Null }],
    })));
    // One content delta per token.
    for chunk in stream_chunks(&reply) {
        events.push(sse_json(&json!({
            "id": id, "object": "chat.completion.chunk", "created": 0, "model": model,
            "choices": [{ "index": 0, "delta": { "content": chunk }, "finish_reason": Value::Null }],
        })));
    }
    // Terminal chunk + [DONE].
    events.push(sse_json(&json!({
        "id": id, "object": "chat.completion.chunk", "created": 0, "model": model,
        "choices": [{ "index": 0, "delta": {}, "finish_reason": "stop" }],
    })));
    events.push(Event::default().data("[DONE]"));

    sse_response(events, config.stream_chunk_delay_ms)
}

// ---------------------------------------------------------------------------
// Anthropic: POST /v1/messages
// ---------------------------------------------------------------------------

async fn anthropic_messages(
    State(config): State<LlmMockConfig>,
    Json(body): Json<Value>,
) -> Response {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(&config.default_model)
        .to_string();
    let messages: Vec<Value> =
        body.get("messages").and_then(|m| m.as_array()).cloned().unwrap_or_default();
    let stream = body.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    let reply = build_reply(&config, &messages);
    let prompt_text = messages
        .iter()
        .map(|m| content_to_text(m.get("content")))
        .collect::<Vec<_>>()
        .join(" ");
    let input_tokens = approx_tokens(&prompt_text);
    let output_tokens = approx_tokens(&reply);
    let id = stable_id("msg_", &reply);

    if stream {
        return anthropic_stream(config, id, model, reply, input_tokens, output_tokens)
            .into_response();
    }

    Json(json!({
        "id": id,
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": [{ "type": "text", "text": reply }],
        "stop_reason": "end_turn",
        "stop_sequence": Value::Null,
        "usage": { "input_tokens": input_tokens, "output_tokens": output_tokens },
    }))
    .into_response()
}

fn anthropic_stream(
    config: LlmMockConfig,
    id: String,
    model: String,
    reply: String,
    input_tokens: u32,
    output_tokens: u32,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut events: Vec<Event> = Vec::new();
    events.push(sse_named(
        "message_start",
        &json!({
            "type": "message_start",
            "message": {
                "id": id, "type": "message", "role": "assistant", "model": model,
                "content": [], "stop_reason": Value::Null, "stop_sequence": Value::Null,
                "usage": { "input_tokens": input_tokens, "output_tokens": 0 },
            },
        }),
    ));
    events.push(sse_named(
        "content_block_start",
        &json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "text", "text": "" } }),
    ));
    for chunk in stream_chunks(&reply) {
        events.push(sse_named(
            "content_block_delta",
            &json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "text_delta", "text": chunk } }),
        ));
    }
    events.push(sse_named(
        "content_block_stop",
        &json!({ "type": "content_block_stop", "index": 0 }),
    ));
    events.push(sse_named(
        "message_delta",
        &json!({ "type": "message_delta", "delta": { "stop_reason": "end_turn", "stop_sequence": Value::Null }, "usage": { "output_tokens": output_tokens } }),
    ));
    events.push(sse_named("message_stop", &json!({ "type": "message_stop" })));

    sse_response(events, config.stream_chunk_delay_ms)
}

// ---------------------------------------------------------------------------
// SSE plumbing
// ---------------------------------------------------------------------------

fn sse_json(value: &Value) -> Event {
    Event::default().data(value.to_string())
}

fn sse_named(name: &str, value: &Value) -> Event {
    Event::default().event(name).data(value.to_string())
}

/// Turn a precomputed list of events into an SSE stream, optionally spacing
/// them out by `delay_ms` so callers can observe incremental delivery.
fn sse_response(
    events: Vec<Event>,
    delay_ms: u64,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let s = stream::unfold(events.into_iter(), move |mut it| async move {
        let next = it.next()?;
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
        Some((Ok::<Event, Infallible>(next), it))
    });
    Sse::new(s).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> LlmMockConfig {
        LlmMockConfig {
            echo_prompt: false,
            ..Default::default()
        }
    }

    #[test]
    fn approx_tokens_counts_words() {
        assert_eq!(approx_tokens("one two three"), 3);
        assert_eq!(approx_tokens(""), 1); // min 1
    }

    #[test]
    fn last_user_text_handles_string_and_array_content() {
        let msgs = vec![
            json!({"role":"system","content":"be brief"}),
            json!({"role":"user","content":"hello world"}),
        ];
        assert_eq!(last_user_text(&msgs), "hello world");
        let arr = vec![
            json!({"role":"user","content":[{"type":"text","text":"a"},{"type":"text","text":"b"}]}),
        ];
        assert_eq!(last_user_text(&arr), "a b");
    }

    #[test]
    fn echo_prompt_reflects_user_message() {
        let c = LlmMockConfig {
            echo_prompt: true,
            ..Default::default()
        };
        let msgs = vec![json!({"role":"user","content":"ping"})];
        let reply = build_reply(&c, &msgs);
        assert!(reply.contains("ping"), "reply should echo the prompt: {reply}");
    }

    #[test]
    fn stable_id_is_deterministic_and_prefixed() {
        let a = stable_id("chatcmpl-", "same");
        let b = stable_id("chatcmpl-", "same");
        assert_eq!(a, b);
        assert!(a.starts_with("chatcmpl-"));
        assert_ne!(stable_id("chatcmpl-", "x"), stable_id("chatcmpl-", "y"));
    }

    #[test]
    fn stream_chunks_preserve_leading_space_after_first() {
        let chunks = stream_chunks("alpha beta gamma");
        assert_eq!(chunks, vec!["alpha", " beta", " gamma"]);
        assert_eq!(chunks.concat(), "alpha beta gamma");
    }

    #[tokio::test]
    async fn chat_completions_non_stream_shape() {
        let body = json!({"model":"gpt-x","messages":[{"role":"user","content":"hi there"}]});
        let resp = chat_completions(State(cfg()), Json(body)).await;
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["object"], "chat.completion");
        assert_eq!(v["choices"][0]["message"]["role"], "assistant");
        assert_eq!(v["choices"][0]["finish_reason"], "stop");
        assert!(v["usage"]["total_tokens"].as_u64().unwrap() >= 2);
        assert!(v["id"].as_str().unwrap().starts_with("chatcmpl-"));
    }

    #[tokio::test]
    async fn anthropic_non_stream_shape() {
        let body = json!({"model":"claude-x","messages":[{"role":"user","content":"hi"}]});
        let resp = anthropic_messages(State(cfg()), Json(body)).await;
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["type"], "message");
        assert_eq!(v["content"][0]["type"], "text");
        assert_eq!(v["stop_reason"], "end_turn");
        assert!(v["usage"]["output_tokens"].as_u64().unwrap() >= 1);
        assert!(v["id"].as_str().unwrap().starts_with("msg_"));
    }

    #[tokio::test]
    async fn models_list_shape() {
        let Json(v) = list_models(State(cfg())).await;
        assert_eq!(v["object"], "list");
        assert_eq!(v["data"][0]["owned_by"], "mockforge");
    }
}
