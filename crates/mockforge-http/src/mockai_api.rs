//! Standalone MockAI HTTP API.
//!
//! MockAI exists in the codebase but until now was only invoked from
//! the OpenAPI route generator — i.e., a hosted mock with a spec got
//! AI-augmented responses on its existing routes. There was no way to
//! reach MockAI directly without a spec, which made it impossible to
//! prototype an "AI persona" mock or hand-craft a one-off intelligent
//! response without a full OpenAPI document.
//!
//! This module exposes the engine over HTTP. Same functional contract
//! the internal callers use: a Request goes in (method/path/body/etc)
//! and a Response comes out (status_code/body/headers).
//!
//! ## Endpoints
//!
//! - `POST /__mockforge/api/mockai/generate` — generate one response
//! - `GET  /__mockforge/api/mockai/status`   — config / availability probe
//!
//! ## Auth / cost
//!
//! MockAI typically calls an LLM provider, so an API key needs to be
//! configured at server start. The endpoint returns 503 with a clear
//! reason if the engine isn't available — same surface the OpenAPI
//! handler treats absence with.

use axum::extract::{Json as AxumJson, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use mockforge_core::intelligent_behavior::{
    IntelligentBehaviorConfig, MockAI, Request as MockAiRequest, StatefulAiContext,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Cheap-to-clone shared state. Holds the existing MockAI instance built
/// at server startup; the API handler reads through the RwLock so a
/// future hot-reload of the model config (e.g., re-uploading a spec)
/// doesn't require a router rebuild.
#[derive(Clone)]
pub struct MockAiApiState {
    /// `None` when MockAI isn't configured for this deployment (no
    /// API key, no model, etc.). Endpoint surfaces 503 in that case.
    pub mockai: Option<Arc<RwLock<MockAI>>>,
}

impl MockAiApiState {
    /// Construct from the same handle the router builder is given.
    pub fn new(mockai: Option<Arc<RwLock<MockAI>>>) -> Self {
        Self { mockai }
    }
}

/// JSON body accepted by `POST /__mockforge/api/mockai/generate`.
#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    /// HTTP method to associate with the synthesized request. Defaults
    /// to GET — useful when the caller just wants "give me a believable
    /// response shape for this resource."
    #[serde(default = "default_method")]
    pub method: String,
    /// Resource path. Required.
    pub path: String,
    /// Optional JSON body (only meaningful for POST/PUT/PATCH).
    #[serde(default)]
    pub body: Option<serde_json::Value>,
    /// Query params.
    #[serde(default)]
    pub query_params: HashMap<String, String>,
    /// Headers to forward to MockAI.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Optional caller-supplied session id so subsequent calls share
    /// memory. A fresh UUID is generated when omitted.
    #[serde(default)]
    pub session_id: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

/// JSON body returned from `POST /__mockforge/api/mockai/generate`.
#[derive(Debug, Serialize)]
pub struct GenerateResponseBody {
    /// HTTP status code MockAI chose for the synthesised response.
    pub status_code: u16,
    /// Response payload.
    pub body: serde_json::Value,
    /// Response headers MockAI emitted.
    pub headers: HashMap<String, String>,
    /// Session id this call belongs to (echo of request, or freshly minted).
    pub session_id: String,
}

async fn status_handler(State(state): State<MockAiApiState>) -> Response {
    let available = state.mockai.is_some();
    Json(serde_json::json!({
        "available": available,
        "reason": if available {
            "MockAI is configured and ready"
        } else {
            "MockAI is not configured (missing API key or no model attached)"
        },
    }))
    .into_response()
}

async fn generate_handler(
    State(state): State<MockAiApiState>,
    AxumJson(req): AxumJson<GenerateRequest>,
) -> Response {
    let Some(mockai) = state.mockai.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "mockai_unavailable",
                "message": "MockAI is not configured. Set a provider API key (OPENAI_API_KEY) and redeploy.",
            })),
        )
            .into_response();
    };

    if req.path.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "missing_path",
                "message": "`path` is required",
            })),
        )
            .into_response();
    }

    let session_id = req.session_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    // The MockAI instance carries its own provider config from server
    // start; we just need a context wrapper to track session state.
    let context = StatefulAiContext::new(session_id.clone(), IntelligentBehaviorConfig::default());

    let mockai_request = MockAiRequest {
        method: req.method,
        path: req.path,
        body: req.body,
        query_params: req.query_params,
        headers: req.headers,
    };

    let guard = mockai.read().await;
    let result = guard.generate_response(&mockai_request, &context).await;
    drop(guard);

    match result {
        Ok(resp) => Json(GenerateResponseBody {
            status_code: resp.status_code,
            body: resp.body,
            headers: resp.headers,
            session_id,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "mockai_generate_failed",
                "message": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// Build the MockAI standalone API router. Mount under
/// `/__mockforge/api/mockai`.
pub fn mockai_api_router(state: MockAiApiState) -> Router {
    Router::new()
        .route("/status", get(status_handler))
        .route("/generate", post(generate_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn status_reports_unavailable_when_no_mockai() {
        let state = MockAiApiState::new(None);
        let resp = status_handler(State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"available\":false"));
    }

    #[tokio::test]
    async fn generate_returns_503_when_no_mockai() {
        let state = MockAiApiState::new(None);
        let req = GenerateRequest {
            method: "GET".into(),
            path: "/users/42".into(),
            body: None,
            query_params: HashMap::new(),
            headers: HashMap::new(),
            session_id: None,
        };
        let resp = generate_handler(State(state), AxumJson(req)).await;
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn default_method_is_get() {
        assert_eq!(default_method(), "GET");
    }
}
