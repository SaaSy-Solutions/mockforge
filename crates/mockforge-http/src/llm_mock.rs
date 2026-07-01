//! Mock LLM endpoint (#912, #915).
//!
//! Serves OpenAI-compatible (`POST /v1/chat/completions`, `GET /v1/models`)
//! and Anthropic-compatible (`POST /v1/messages`) endpoints so an agent
//! (Cursor, Claude Code, ChatGPT clients, custom agents) can point its base
//! URL at MockForge and receive correctly-shaped completions with realistic
//! envelopes (ids, `usage` token counts, `finish_reason` / `stop_reason`) and
//! SSE streaming when the caller sets `stream: true`.
//!
//! Four modes ([`LlmMockMode`]), all opt-in via `--llm-mock-mode`; the default
//! stays a pure offline mock:
//! - `mock` (default): canned/templated text, never calls out. Deterministic.
//! - `proxy`: forward every request to a configured OpenAI/Anthropic-compatible
//!   upstream and return the real response (a man-in-the-middle for agent<->LLM
//!   traffic; combine with `--latency`/`--failures` for chaos on real traffic).
//! - `record`: on a cassette miss, forward to upstream and save the response;
//!   on a hit, replay from the cassette. Real content, deterministic after
//!   warm-up.
//! - `replay`: serve only from the cassette (fully offline); a miss falls back
//!   to the canned reply.
//!
//! Any request the caller sends is already in the upstream's wire shape, so
//! upstream calls forward the model + messages verbatim (always non-streaming);
//! streaming clients get the resolved text re-chunked locally.
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
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// How the mock LLM endpoint sources its reply text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LlmMockMode {
    /// Canned/templated text only; never calls out. Deterministic + offline.
    #[default]
    Mock,
    /// Always forward to the configured upstream and return the real response.
    Proxy,
    /// Cassette miss forwards to upstream and records; hit replays from cassette.
    Record,
    /// Cassette only (offline); a miss falls back to the canned reply.
    Replay,
}

impl std::str::FromStr for LlmMockMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "mock" | "canned" | "off" => Ok(Self::Mock),
            "proxy" | "passthrough" => Ok(Self::Proxy),
            "record" => Ok(Self::Record),
            "replay" => Ok(Self::Replay),
            _ => Err(format!("unknown --llm-mock-mode '{s}' (mock|proxy|record|replay)")),
        }
    }
}

impl std::fmt::Display for LlmMockMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Mock => "mock",
            Self::Proxy => "proxy",
            Self::Record => "record",
            Self::Replay => "replay",
        })
    }
}

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
    /// Reply sourcing mode. Defaults to [`LlmMockMode::Mock`].
    pub mode: LlmMockMode,
    /// Base URL of the OpenAI/Anthropic-compatible upstream (no trailing path),
    /// e.g. `https://api.openai.com` or `http://localhost:11434`. Required for
    /// `proxy` and `record`.
    pub upstream_base_url: Option<String>,
    /// API key forwarded to the upstream (Bearer for OpenAI, `x-api-key` for
    /// Anthropic). When None, no auth header is sent (fine for local upstreams).
    pub upstream_api_key: Option<String>,
    /// Cassette file for `record` / `replay`. Loaded at startup; `record`
    /// rewrites it as new prompts are captured.
    pub cassette_path: Option<PathBuf>,
}

impl Default for LlmMockConfig {
    fn default() -> Self {
        Self {
            canned_reply: "This is a mock response from MockForge's LLM endpoint.".to_string(),
            default_model: "mockforge-mock-1".to_string(),
            echo_prompt: true,
            stream_chunk_delay_ms: 0,
            mode: LlmMockMode::Mock,
            upstream_base_url: None,
            upstream_api_key: None,
            cassette_path: None,
        }
    }
}

/// One recorded completion, keyed by a hash of (endpoint, model, messages).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct CassetteEntry {
    text: String,
    prompt_tokens: u32,
    completion_tokens: u32,
}

/// In-memory cassette backed by a JSON file on disk.
#[derive(Default)]
struct Cassette {
    entries: HashMap<String, CassetteEntry>,
    path: Option<PathBuf>,
}

impl Cassette {
    fn load(path: Option<PathBuf>) -> Self {
        let entries = path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice::<HashMap<String, CassetteEntry>>(&b).ok())
            .unwrap_or_default();
        Self { entries, path }
    }

    fn save(&self) {
        if let Some(ref p) = self.path {
            if let Ok(json) = serde_json::to_string_pretty(&self.entries) {
                let _ = std::fs::write(p, json);
            }
        }
    }
}

/// Router state: config plus the shared cassette and an HTTP client for
/// upstream calls. Cheap to clone (Arc + reqwest::Client are handle types).
#[derive(Clone)]
pub struct LlmMockState {
    config: LlmMockConfig,
    cassette: Arc<Mutex<Cassette>>,
    http: reqwest::Client,
}

impl LlmMockState {
    /// Build state from config, loading any existing cassette from disk.
    pub fn new(config: LlmMockConfig) -> Self {
        let cassette = Cassette::load(config.cassette_path.clone());
        Self {
            config,
            cassette: Arc::new(Mutex::new(cassette)),
            http: reqwest::Client::new(),
        }
    }
}

/// Build the axum router exposing the mock LLM endpoints.
pub fn router(config: LlmMockConfig) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
        .route("/v1/messages", post(anthropic_messages))
        .with_state(LlmMockState::new(config))
}

/// Which upstream wire dialect a request/response uses.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialect {
    OpenAi,
    Anthropic,
}

/// The resolved reply text plus token counts, and where it came from.
struct Resolved {
    text: String,
    prompt_tokens: u32,
    completion_tokens: u32,
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

async fn list_models(State(state): State<LlmMockState>) -> Json<Value> {
    Json(json!({
        "object": "list",
        "data": [{
            "id": state.config.default_model,
            "object": "model",
            "created": 0,
            "owned_by": "mockforge",
        }],
    }))
}

// ---------------------------------------------------------------------------
// OpenAI: POST /v1/chat/completions
// ---------------------------------------------------------------------------

async fn chat_completions(State(state): State<LlmMockState>, Json(body): Json<Value>) -> Response {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(&state.config.default_model)
        .to_string();
    let messages: Vec<Value> =
        body.get("messages").and_then(|m| m.as_array()).cloned().unwrap_or_default();
    let stream = body.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    let r = resolve_reply(&state, Dialect::OpenAi, &model, &messages).await;
    let id = stable_id("chatcmpl-", &r.text);

    if stream {
        return openai_stream(state.config.stream_chunk_delay_ms, id, model, r.text)
            .into_response();
    }

    Json(json!({
        "id": id,
        "object": "chat.completion",
        "created": 0,
        "model": model,
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": r.text },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": r.prompt_tokens,
            "completion_tokens": r.completion_tokens,
            "total_tokens": r.prompt_tokens + r.completion_tokens,
        },
    }))
    .into_response()
}

fn openai_stream(
    delay_ms: u64,
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

    sse_response(events, delay_ms)
}

// ---------------------------------------------------------------------------
// Anthropic: POST /v1/messages
// ---------------------------------------------------------------------------

async fn anthropic_messages(
    State(state): State<LlmMockState>,
    Json(body): Json<Value>,
) -> Response {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(&state.config.default_model)
        .to_string();
    let messages: Vec<Value> =
        body.get("messages").and_then(|m| m.as_array()).cloned().unwrap_or_default();
    let stream = body.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    let r = resolve_reply(&state, Dialect::Anthropic, &model, &messages).await;
    let id = stable_id("msg_", &r.text);

    if stream {
        return anthropic_stream(
            state.config.stream_chunk_delay_ms,
            id,
            model,
            r.text,
            r.prompt_tokens,
            r.completion_tokens,
        )
        .into_response();
    }

    Json(json!({
        "id": id,
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": [{ "type": "text", "text": r.text }],
        "stop_reason": "end_turn",
        "stop_sequence": Value::Null,
        "usage": { "input_tokens": r.prompt_tokens, "output_tokens": r.completion_tokens },
    }))
    .into_response()
}

fn anthropic_stream(
    delay_ms: u64,
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

    sse_response(events, delay_ms)
}

// ---------------------------------------------------------------------------
// Reply resolution: mock / proxy / record / replay
// ---------------------------------------------------------------------------

/// Stable cassette key for a request: FNV hash of dialect + model + canonical
/// messages JSON. serde_json's default `Map` sorts keys, so the key is stable
/// across runs for identical inputs.
fn cassette_key(dialect: Dialect, model: &str, messages: &[Value]) -> String {
    let d = match dialect {
        Dialect::OpenAi => "openai",
        Dialect::Anthropic => "anthropic",
    };
    let msgs = serde_json::to_string(messages).unwrap_or_default();
    stable_id("", &format!("{d}\n{model}\n{msgs}"))
}

/// Build the canned reply + approximate token counts (the default path and the
/// fallback for every mode when the upstream/cassette can't serve).
fn canned_resolved(cfg: &LlmMockConfig, messages: &[Value]) -> Resolved {
    let text = build_reply(cfg, messages);
    let prompt_text = messages
        .iter()
        .map(|m| content_to_text(m.get("content")))
        .collect::<Vec<_>>()
        .join(" ");
    Resolved {
        prompt_tokens: approx_tokens(&prompt_text),
        completion_tokens: approx_tokens(&text),
        text,
    }
}

fn cassette_lookup(state: &LlmMockState, key: &str) -> Option<Resolved> {
    let e = state.cassette.lock().ok()?.entries.get(key).cloned()?;
    Some(Resolved {
        text: e.text,
        prompt_tokens: e.prompt_tokens,
        completion_tokens: e.completion_tokens,
    })
}

/// Resolve the reply text + token counts per the configured mode. Upstream and
/// cassette failures degrade to the canned reply so the mock never hard-fails a
/// caller (it logs the reason instead).
async fn resolve_reply(
    state: &LlmMockState,
    dialect: Dialect,
    model: &str,
    messages: &[Value],
) -> Resolved {
    let cfg = &state.config;
    match cfg.mode {
        LlmMockMode::Mock => canned_resolved(cfg, messages),
        LlmMockMode::Replay => {
            let key = cassette_key(dialect, model, messages);
            cassette_lookup(state, &key).unwrap_or_else(|| {
                tracing::warn!(target: "mockforge::llm_mock", "replay cassette miss (key {key}); serving canned reply");
                canned_resolved(cfg, messages)
            })
        }
        LlmMockMode::Record => {
            let key = cassette_key(dialect, model, messages);
            if let Some(hit) = cassette_lookup(state, &key) {
                return hit;
            }
            match call_upstream(state, dialect, model, messages).await {
                Ok(r) => {
                    if let Ok(mut c) = state.cassette.lock() {
                        c.entries.insert(
                            key,
                            CassetteEntry {
                                text: r.text.clone(),
                                prompt_tokens: r.prompt_tokens,
                                completion_tokens: r.completion_tokens,
                            },
                        );
                        c.save();
                    }
                    r
                }
                Err(e) => {
                    tracing::error!(target: "mockforge::llm_mock", "record: upstream call failed ({e}); serving canned reply");
                    canned_resolved(cfg, messages)
                }
            }
        }
        LlmMockMode::Proxy => match call_upstream(state, dialect, model, messages).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(target: "mockforge::llm_mock", "proxy: upstream call failed ({e}); serving canned reply");
                canned_resolved(cfg, messages)
            }
        },
    }
}

/// Forward a request to the configured OpenAI/Anthropic-compatible upstream
/// (always non-streaming) and parse the reply text + token usage back out.
async fn call_upstream(
    state: &LlmMockState,
    dialect: Dialect,
    model: &str,
    messages: &[Value],
) -> Result<Resolved, String> {
    let base = state
        .config
        .upstream_base_url
        .as_deref()
        .ok_or("no upstream configured (set --llm-mock-upstream)")?
        .trim_end_matches('/');

    let (url, body) = match dialect {
        Dialect::OpenAi => (
            format!("{base}/v1/chat/completions"),
            json!({ "model": model, "messages": messages, "stream": false }),
        ),
        // Anthropic requires max_tokens on the request.
        Dialect::Anthropic => (
            format!("{base}/v1/messages"),
            json!({ "model": model, "messages": messages, "max_tokens": 1024, "stream": false }),
        ),
    };

    let mut req = state.http.post(&url).json(&body);
    if let Some(ref k) = state.config.upstream_api_key {
        req = match dialect {
            Dialect::OpenAi => req.header("authorization", format!("Bearer {k}")),
            Dialect::Anthropic => {
                req.header("x-api-key", k).header("anthropic-version", "2023-06-01")
            }
        };
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("{url} returned HTTP {status}"));
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;

    let (text, pt, ct) = match dialect {
        Dialect::OpenAi => (
            v.pointer("/choices/0/message/content")
                .and_then(|x| x.as_str())
                .unwrap_or_default(),
            v.pointer("/usage/prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            v.pointer("/usage/completion_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        ),
        Dialect::Anthropic => (
            v.pointer("/content/0/text").and_then(|x| x.as_str()).unwrap_or_default(),
            v.pointer("/usage/input_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            v.pointer("/usage/output_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        ),
    };
    let text = text.to_string();
    // Backfill token counts if the upstream omitted usage.
    let prompt_tokens = if pt > 0 {
        pt
    } else {
        let pj = messages
            .iter()
            .map(|m| content_to_text(m.get("content")))
            .collect::<Vec<_>>()
            .join(" ");
        approx_tokens(&pj)
    };
    let completion_tokens = if ct > 0 { ct } else { approx_tokens(&text) };
    Ok(Resolved {
        text,
        prompt_tokens,
        completion_tokens,
    })
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

    /// Router state wrapping a config, for handler tests.
    fn st() -> LlmMockState {
        LlmMockState::new(cfg())
    }

    fn user(text: &str) -> Vec<Value> {
        vec![json!({"role":"user","content":text})]
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
        let resp = chat_completions(State(st()), Json(body)).await;
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
        let resp = anthropic_messages(State(st()), Json(body)).await;
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
        let Json(v) = list_models(State(st())).await;
        assert_eq!(v["object"], "list");
        assert_eq!(v["data"][0]["owned_by"], "mockforge");
    }

    // ---- #915: modes + cassette ----

    #[test]
    fn mode_parses_and_displays() {
        for (s, m) in [
            ("mock", LlmMockMode::Mock),
            ("off", LlmMockMode::Mock),
            ("proxy", LlmMockMode::Proxy),
            ("record", LlmMockMode::Record),
            ("replay", LlmMockMode::Replay),
        ] {
            assert_eq!(s.parse::<LlmMockMode>().unwrap(), m);
        }
        assert!("bogus".parse::<LlmMockMode>().is_err());
        assert_eq!(LlmMockMode::Record.to_string(), "record");
    }

    #[test]
    fn cassette_key_is_stable_and_input_sensitive() {
        let m = user("hello");
        let k1 = cassette_key(Dialect::OpenAi, "gpt-4o", &m);
        let k2 = cassette_key(Dialect::OpenAi, "gpt-4o", &m);
        assert_eq!(k1, k2, "same input -> same key");
        assert_ne!(k1, cassette_key(Dialect::OpenAi, "gpt-4o", &user("world")));
        assert_ne!(k1, cassette_key(Dialect::OpenAi, "gpt-5", &m), "model is part of the key");
        assert_ne!(
            k1,
            cassette_key(Dialect::Anthropic, "gpt-4o", &m),
            "dialect is part of the key"
        );
    }

    #[test]
    fn cassette_roundtrips_through_disk() {
        let dir = std::env::temp_dir().join(format!("mf-cassette-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("c.json");
        let mut c = Cassette::load(Some(path.clone()));
        c.entries.insert(
            "k".into(),
            CassetteEntry {
                text: "recorded".into(),
                prompt_tokens: 3,
                completion_tokens: 1,
            },
        );
        c.save();
        let reloaded = Cassette::load(Some(path.clone()));
        assert_eq!(reloaded.entries.get("k").unwrap().text, "recorded");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn mock_mode_serves_canned_without_upstream() {
        let state = LlmMockState::new(cfg()); // mode defaults to Mock, no upstream
        let r = resolve_reply(&state, Dialect::OpenAi, "gpt-4o", &user("hi")).await;
        assert!(r.text.contains("mock response"));
    }

    #[tokio::test]
    async fn replay_hit_serves_cassette_miss_serves_canned() {
        let state = LlmMockState::new(LlmMockConfig {
            mode: LlmMockMode::Replay,
            echo_prompt: false,
            ..Default::default()
        });
        // Seed a cassette entry under the exact key the resolver will compute.
        let msgs = user("what is 2+2");
        let key = cassette_key(Dialect::OpenAi, "gpt-4o", &msgs);
        state.cassette.lock().unwrap().entries.insert(
            key,
            CassetteEntry {
                text: "four".into(),
                prompt_tokens: 4,
                completion_tokens: 1,
            },
        );
        // Hit -> cassette content.
        let hit = resolve_reply(&state, Dialect::OpenAi, "gpt-4o", &msgs).await;
        assert_eq!(hit.text, "four");
        assert_eq!(hit.completion_tokens, 1);
        // Miss -> canned fallback (offline, no upstream).
        let miss = resolve_reply(&state, Dialect::OpenAi, "gpt-4o", &user("unseen")).await;
        assert!(miss.text.contains("mock response"));
    }

    #[tokio::test]
    async fn proxy_without_upstream_degrades_to_canned() {
        // Proxy mode but no upstream configured: must not hard-fail.
        let state = LlmMockState::new(LlmMockConfig {
            mode: LlmMockMode::Proxy,
            echo_prompt: false,
            ..Default::default()
        });
        let r = resolve_reply(&state, Dialect::OpenAi, "gpt-4o", &user("hi")).await;
        assert!(r.text.contains("mock response"), "should fall back to canned, got: {}", r.text);
    }
}
