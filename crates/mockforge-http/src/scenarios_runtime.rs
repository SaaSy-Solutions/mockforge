//! Runtime named-scenario activation API.
//!
//! Exposes the locally-installed scenario catalogue (from
//! `mockforge_scenarios::ScenarioStorage`) over HTTP and lets operators
//! activate one by name. Activation today writes the name into the
//! consistency engine's `UnifiedState::active_scenario` for the chosen
//! workspace; downstream consumers (the consistency middleware, X-Ray,
//! etc.) can already observe it. Full manifest-driven application (
//! applying personas, reality levels, fixtures from the scenario
//! bundle) is intentionally **not** done here — it would require
//! hooking into every behavioural subsystem and is tracked separately.
//!
//! ## Endpoints (mounted under `/__mockforge/api/scenarios`)
//!
//! - `GET    /                              — list installed scenarios
//! - `POST   /{name}/activate?workspace=…  — set this name as active
//! - `POST   /deactivate?workspace=…       — clear active
//! - `GET    /active?workspace=…           — current active name (204 if none)
//!
//! `workspace` query param is optional; defaults to `"default"`.

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use mockforge_core::consistency::ConsistencyEngine;
use mockforge_scenarios::ScenarioStorage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cheap-to-clone shared state.
#[derive(Clone)]
pub struct ScenarioRuntimeState {
    inner: Arc<Inner>,
}

struct Inner {
    /// Local installed-scenario index. Loaded once at startup; mutations
    /// happen through scenario install/remove which is out of scope for
    /// this PR (existing CLI commands handle it).
    storage: RwLock<ScenarioStorage>,
    /// Consistency engine — activation writes through to its
    /// `set_active_scenario` so the existing middleware can pick it up.
    engine: Arc<ConsistencyEngine>,
}

impl ScenarioRuntimeState {
    /// Construct from a (loaded) scenario storage and a consistency engine.
    pub fn new(storage: ScenarioStorage, engine: Arc<ConsistencyEngine>) -> Self {
        Self {
            inner: Arc::new(Inner {
                storage: RwLock::new(storage),
                engine,
            }),
        }
    }
}

#[derive(Debug, Serialize)]
struct ScenarioSummary {
    name: String,
    version: String,
    source: String,
    installed_at: u64,
    description: String,
}

#[derive(Debug, Serialize)]
struct ListResponse {
    scenarios: Vec<ScenarioSummary>,
}

async fn list_handler(State(state): State<ScenarioRuntimeState>) -> Json<ListResponse> {
    let storage = state.inner.storage.read().await;
    let scenarios = storage
        .list()
        .into_iter()
        .map(|s| ScenarioSummary {
            name: s.name.clone(),
            version: s.version.clone(),
            source: s.source.clone(),
            installed_at: s.installed_at,
            description: s.manifest.description.clone(),
        })
        .collect();
    Json(ListResponse { scenarios })
}

#[derive(Debug, Deserialize)]
struct WorkspaceQuery {
    #[serde(default)]
    workspace: Option<String>,
}

fn workspace_or_default(q: &WorkspaceQuery) -> String {
    q.workspace.clone().unwrap_or_else(|| "default".to_string())
}

async fn activate_handler(
    State(state): State<ScenarioRuntimeState>,
    AxumPath(name): AxumPath<String>,
    Query(q): Query<WorkspaceQuery>,
) -> Response {
    // Defensive: make sure the scenario actually exists in local
    // storage. We don't want to "activate" arbitrary strings — that
    // would let typos quietly become active without surfacing.
    let exists = {
        let storage = state.inner.storage.read().await;
        storage.get_latest(&name).is_some()
    };
    if !exists {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "scenario_not_found",
                "message": format!("No installed scenario named '{}'", name),
            })),
        )
            .into_response();
    }

    let workspace = workspace_or_default(&q);
    if let Err(e) = state.inner.engine.set_active_scenario(&workspace, name.clone()).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "activate_failed",
                "message": e.to_string(),
            })),
        )
            .into_response();
    }
    Json(serde_json::json!({
        "active": name,
        "workspace": workspace,
    }))
    .into_response()
}

async fn deactivate_handler(
    State(state): State<ScenarioRuntimeState>,
    Query(q): Query<WorkspaceQuery>,
) -> Response {
    let workspace = workspace_or_default(&q);
    // The consistency engine doesn't have a "clear scenario" method; we
    // emulate it by setting an empty string. Consumers downstream check
    // for None / empty.
    if let Err(e) = state.inner.engine.set_active_scenario(&workspace, String::new()).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "deactivate_failed",
                "message": e.to_string(),
            })),
        )
            .into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn active_handler(
    State(state): State<ScenarioRuntimeState>,
    Query(q): Query<WorkspaceQuery>,
) -> Response {
    let workspace = workspace_or_default(&q);
    let unified = state.inner.engine.get_state(&workspace).await;
    match unified.and_then(|s| s.active_scenario) {
        Some(name) if !name.is_empty() => {
            Json(serde_json::json!({ "active": name, "workspace": workspace })).into_response()
        }
        _ => StatusCode::NO_CONTENT.into_response(),
    }
}

/// Build the runtime scenario API router. Mount under
/// `/__mockforge/api/scenarios`.
pub fn scenarios_api_router(state: ScenarioRuntimeState) -> Router {
    Router::new()
        .route("/", get(list_handler))
        .route("/active", get(active_handler))
        .route("/deactivate", post(deactivate_handler))
        .route("/{name}/activate", post(activate_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_default_when_unset() {
        assert_eq!(workspace_or_default(&WorkspaceQuery { workspace: None }), "default");
    }

    #[test]
    fn workspace_uses_explicit_value() {
        assert_eq!(
            workspace_or_default(&WorkspaceQuery {
                workspace: Some("billing-team".to_string())
            }),
            "billing-team"
        );
    }
}
