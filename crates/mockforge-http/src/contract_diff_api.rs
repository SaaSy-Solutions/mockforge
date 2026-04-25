//! Contract-diff retrieval API.
//!
//! The `contract_diff_middleware` already captures every incoming
//! request into the global `CaptureManager`. Until now there was no
//! HTTP surface to read those captures back out or to run the diff
//! analyser against the deployment's OpenAPI spec — the audit's PR-3
//! gap.
//!
//! ## Endpoints (mounted under `/__mockforge/api/contract-diff`)
//!
//! - `GET    /captures?limit=<n>`       → recent captures (default 100, capped 1000)
//! - `GET    /captures/{id}`            → single capture
//! - `POST   /analyze/{id}`             → analyse one capture against the spec
//! - `POST   /analyze`                  → analyse the most recent N captures (defaults to 50)
//! - `DELETE /captures`                 → wipe the in-memory store
//! - `GET    /statistics`               → simple counters from CaptureManager
//!
//! Analysis loads the OpenAPI spec from the path the server was started
//! with (`spec_path` carried in this module's state). A redeploy with a
//! new spec naturally takes effect on the next request.

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use mockforge_core::ai_contract_diff::{ContractDiffAnalyzer, ContractDiffConfig};
use mockforge_core::openapi::OpenApiSpec;
use mockforge_core::request_capture::get_global_capture_manager;
use serde::Deserialize;
use std::sync::Arc;

/// Shared state for the contract-diff API. Cloneable.
#[derive(Clone)]
pub struct ContractDiffApiState {
    /// Path to the OpenAPI spec the deployment was started with.
    /// `None` means analysis endpoints will return a friendly 503 —
    /// captures are still listable.
    pub spec_path: Option<String>,
}

impl ContractDiffApiState {
    /// Construct from the spec path the server was started with. Pass
    /// `None` to disable the analysis endpoints (captures still list).
    pub fn new(spec_path: Option<String>) -> Self {
        Self { spec_path }
    }
}

#[derive(Debug, Deserialize)]
struct CapturesQuery {
    #[serde(default)]
    limit: Option<usize>,
}

async fn list_captures_handler(Query(q): Query<CapturesQuery>) -> Response {
    let Some(manager) = get_global_capture_manager() else {
        return capture_manager_unavailable();
    };
    let limit = q.limit.unwrap_or(100).min(1000);
    let captures = manager.get_recent_captures(Some(limit)).await;
    let payload: Vec<serde_json::Value> = captures
        .into_iter()
        .map(|(request, metadata)| {
            serde_json::json!({
                "id": metadata.id,
                "captured_at": metadata.captured_at,
                "source": metadata.source,
                "analyzed": metadata.analyzed,
                "request": request,
            })
        })
        .collect();
    Json(serde_json::json!({
        "count": payload.len(),
        "captures": payload,
    }))
    .into_response()
}

async fn get_capture_handler(AxumPath(id): AxumPath<String>) -> Response {
    let Some(manager) = get_global_capture_manager() else {
        return capture_manager_unavailable();
    };
    match manager.get_capture(&id).await {
        Some((request, metadata)) => Json(serde_json::json!({
            "id": metadata.id,
            "captured_at": metadata.captured_at,
            "source": metadata.source,
            "analyzed": metadata.analyzed,
            "request": request,
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "capture_not_found",
                "message": format!("No capture with id '{}'", id),
            })),
        )
            .into_response(),
    }
}

async fn delete_captures_handler() -> Response {
    let Some(manager) = get_global_capture_manager() else {
        return capture_manager_unavailable();
    };
    manager.clear_captures().await;
    StatusCode::NO_CONTENT.into_response()
}

async fn statistics_handler() -> Response {
    let Some(manager) = get_global_capture_manager() else {
        return capture_manager_unavailable();
    };
    let stats = manager.get_statistics().await;
    Json(stats).into_response()
}

#[derive(Debug, Deserialize)]
struct AnalyzeAllQuery {
    #[serde(default)]
    limit: Option<usize>,
}

async fn analyze_one_handler(
    State(state): State<Arc<ContractDiffApiState>>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let Some(manager) = get_global_capture_manager() else {
        return capture_manager_unavailable();
    };
    let Some(spec_path) = state.spec_path.as_ref() else {
        return spec_unavailable();
    };

    let (request, _metadata) = match manager.get_capture(&id).await {
        Some(c) => c,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "capture_not_found",
                    "message": format!("No capture with id '{}'", id),
                })),
            )
                .into_response();
        }
    };

    let spec = match OpenApiSpec::from_file(spec_path).await {
        Ok(s) => s,
        Err(e) => return spec_load_failed(&e.to_string()),
    };

    let analyzer = match ContractDiffAnalyzer::new(ContractDiffConfig::default()) {
        Ok(a) => a,
        Err(e) => return analyzer_init_failed(&e.to_string()),
    };

    match analyzer.analyze(&request, &spec).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => analyzer_failed(&e.to_string()),
    }
}

async fn analyze_all_handler(
    State(state): State<Arc<ContractDiffApiState>>,
    Query(q): Query<AnalyzeAllQuery>,
) -> Response {
    let Some(manager) = get_global_capture_manager() else {
        return capture_manager_unavailable();
    };
    let Some(spec_path) = state.spec_path.as_ref() else {
        return spec_unavailable();
    };

    let limit = q.limit.unwrap_or(50).min(500);
    let captures = manager.get_recent_captures(Some(limit)).await;
    if captures.is_empty() {
        return Json(serde_json::json!({ "results": [], "analyzed": 0 })).into_response();
    }

    let spec = match OpenApiSpec::from_file(spec_path).await {
        Ok(s) => s,
        Err(e) => return spec_load_failed(&e.to_string()),
    };
    let analyzer = match ContractDiffAnalyzer::new(ContractDiffConfig::default()) {
        Ok(a) => a,
        Err(e) => return analyzer_init_failed(&e.to_string()),
    };

    let mut results = Vec::with_capacity(captures.len());
    for (request, metadata) in &captures {
        match analyzer.analyze(request, &spec).await {
            Ok(result) => {
                results.push(serde_json::json!({
                    "capture_id": metadata.id,
                    "ok": true,
                    "result": result,
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "capture_id": metadata.id,
                    "ok": false,
                    "error": e.to_string(),
                }));
            }
        }
    }

    Json(serde_json::json!({
        "analyzed": results.len(),
        "results": results,
    }))
    .into_response()
}

fn capture_manager_unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "capture_manager_not_initialised",
            "message": "Request capture is not enabled on this deployment",
        })),
    )
        .into_response()
}

fn spec_unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "no_openapi_spec",
            "message": "Analysis requires the deployment to be running with an OpenAPI spec",
        })),
    )
        .into_response()
}

fn spec_load_failed(err: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": "spec_load_failed",
            "message": err,
        })),
    )
        .into_response()
}

fn analyzer_init_failed(err: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": "analyzer_init_failed",
            "message": err,
        })),
    )
        .into_response()
}

fn analyzer_failed(err: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": "analyze_failed",
            "message": err,
        })),
    )
        .into_response()
}

/// Build the contract-diff API router. Mount under
/// `/__mockforge/api/contract-diff`.
pub fn contract_diff_api_router(state: Arc<ContractDiffApiState>) -> Router {
    Router::new()
        .route("/captures", get(list_captures_handler).delete(delete_captures_handler))
        .route("/captures/{id}", get(get_capture_handler))
        .route("/statistics", get(statistics_handler))
        .route("/analyze", post(analyze_all_handler))
        .route("/analyze/{id}", post(analyze_one_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_holds_optional_spec_path() {
        let s = ContractDiffApiState::new(None);
        assert!(s.spec_path.is_none());
        let s = ContractDiffApiState::new(Some("/tmp/spec.yaml".to_string()));
        assert_eq!(s.spec_path.as_deref(), Some("/tmp/spec.yaml"));
    }
}
