//! Runtime route-scoped chaos API.
//!
//! Static `route_configs` from the YAML config get baked into per-route
//! handlers at startup. This module provides the *runtime* counterpart:
//! a shared, mutable rule set that operators can update via HTTP without
//! restarting the deployment. A middleware layer consults the live state
//! on every request.
//!
//! ## Why additive (rather than replacing the static path)
//!
//! Each static route is registered as its own Axum handler that captures
//! a clone of the initial injector — refactoring all of them to read a
//! shared `RwLock` would touch a lot of unrelated code. Instead this
//! middleware runs *in front of* the static handlers: if a runtime rule
//! injects a fault, we short-circuit; otherwise we fall through. Routes
//! that are configured both statically and at runtime have the runtime
//! rule win — that's the surprise-minimising default for an operator
//! who's just turned on a fault via API.
//!
//! ## Endpoints
//!
//! - `GET    /__mockforge/api/route-chaos`           — list rules
//! - `PUT    /__mockforge/api/route-chaos`           — replace all rules
//! - `POST   /__mockforge/api/route-chaos/route`     — add or upsert one rule
//! - `DELETE /__mockforge/api/route-chaos/route`     — remove one rule by method+path

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header::CONTENT_TYPE, HeaderName, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use mockforge_core::config::RouteConfig;
use mockforge_route_chaos::RouteChaosInjector;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tracing::warn;

/// Shared, mutable set of route-chaos rules. Cheap to clone (Arc).
#[derive(Clone)]
pub struct RuntimeRouteChaosState {
    inner: Arc<Inner>,
}

struct Inner {
    /// Source-of-truth list of rules.
    routes: RwLock<Vec<RouteConfig>>,
    /// Rebuilt every time `routes` mutates. `None` when the rule set is
    /// empty — saves the matcher walk on the hot path.
    injector: RwLock<Option<RouteChaosInjector>>,
}

impl RuntimeRouteChaosState {
    /// Construct from an initial rule set (typically empty on a hosted
    /// mock — runtime rules supplement the static-config path).
    pub fn new(initial: Vec<RouteConfig>) -> Self {
        let injector = build_injector(&initial);
        Self {
            inner: Arc::new(Inner {
                routes: RwLock::new(initial),
                injector: RwLock::new(injector),
            }),
        }
    }

    /// Snapshot of the current rule set.
    pub fn list(&self) -> Vec<RouteConfig> {
        self.inner.routes.read().expect("route-chaos state poisoned").clone()
    }

    /// Replace the entire rule set atomically.
    pub fn replace_all(&self, new_routes: Vec<RouteConfig>) -> Result<(), String> {
        let new_injector = if new_routes.is_empty() {
            None
        } else {
            Some(RouteChaosInjector::new(new_routes.clone()).map_err(|e| e.to_string())?)
        };
        let mut routes = self.inner.routes.write().expect("route-chaos state poisoned");
        let mut inj = self.inner.injector.write().expect("route-chaos state poisoned");
        *routes = new_routes;
        *inj = new_injector;
        Ok(())
    }

    /// Add or upsert a single rule. Matching is on (method, path) — same
    /// (method, path) replaces the existing rule.
    pub fn upsert(&self, route: RouteConfig) -> Result<(), String> {
        let mut routes = self.inner.routes.write().expect("route-chaos state poisoned");
        if let Some(existing) = routes
            .iter_mut()
            .find(|r| r.method.eq_ignore_ascii_case(&route.method) && r.path == route.path)
        {
            *existing = route;
        } else {
            routes.push(route);
        }
        let snapshot = routes.clone();
        drop(routes);
        let new_injector = build_injector(&snapshot);
        let mut inj = self.inner.injector.write().expect("route-chaos state poisoned");
        *inj = new_injector;
        Ok(())
    }

    /// Remove a rule by (method, path). Returns whether something was
    /// removed.
    pub fn remove(&self, method: &str, path: &str) -> bool {
        let mut routes = self.inner.routes.write().expect("route-chaos state poisoned");
        let before = routes.len();
        routes.retain(|r| !(r.method.eq_ignore_ascii_case(method) && r.path == path));
        let removed = routes.len() != before;
        let snapshot = routes.clone();
        drop(routes);
        if removed {
            let new_injector = build_injector(&snapshot);
            let mut inj = self.inner.injector.write().expect("route-chaos state poisoned");
            *inj = new_injector;
        }
        removed
    }

    /// Snapshot of the current injector. Returns None when the rule set
    /// is empty — the middleware fast-paths off this.
    fn current_injector(&self) -> Option<RouteChaosInjector> {
        self.inner.injector.read().expect("route-chaos state poisoned").clone()
    }
}

fn build_injector(routes: &[RouteConfig]) -> Option<RouteChaosInjector> {
    if routes.is_empty() {
        return None;
    }
    match RouteChaosInjector::new(routes.to_vec()) {
        Ok(inj) => Some(inj),
        Err(e) => {
            warn!(error = %e, "Failed to build runtime route-chaos injector; rules ignored");
            None
        }
    }
}

/// Middleware that runs *before* the static route handlers. If a runtime
/// rule injects a fault, return it; otherwise fall through.
pub async fn runtime_route_chaos_middleware(
    State(state): State<RuntimeRouteChaosState>,
    req: Request,
    next: Next,
) -> Response {
    let Some(injector) = state.current_injector() else {
        return next.run(req).await;
    };

    use mockforge_core::priority_handler::RouteChaosInjectorTrait;
    let method = req.method().clone();
    let uri = req.uri().clone();

    if let Some(fault) = injector.get_fault_response(&method, &uri) {
        let status =
            StatusCode::from_u16(fault.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::to_vec(&serde_json::json!({
            "error": fault.fault_type,
            "message": fault.error_message,
        }))
        .unwrap_or_default();
        let mut resp = Response::new(Body::from(body));
        *resp.status_mut() = status;
        resp.headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        resp.headers_mut().insert(
            HeaderName::from_static("x-mockforge-source"),
            HeaderValue::from_static("route-chaos-runtime"),
        );
        return resp;
    }

    if let Err(e) = injector.inject_latency(&method, &uri).await {
        warn!(error = %e, "Runtime route-chaos latency injection errored; continuing");
    }

    next.run(req).await
}

#[derive(Debug, Serialize)]
struct ListResponse {
    rules: Vec<RouteConfig>,
}

async fn list_handler(State(state): State<RuntimeRouteChaosState>) -> Json<ListResponse> {
    Json(ListResponse {
        rules: state.list(),
    })
}

#[derive(Debug, Deserialize)]
struct ReplaceRequest {
    rules: Vec<RouteConfig>,
}

async fn replace_handler(
    State(state): State<RuntimeRouteChaosState>,
    Json(req): Json<ReplaceRequest>,
) -> Result<Json<ListResponse>, (StatusCode, String)> {
    state.replace_all(req.rules).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(Json(ListResponse {
        rules: state.list(),
    }))
}

async fn upsert_handler(
    State(state): State<RuntimeRouteChaosState>,
    Json(route): Json<RouteConfig>,
) -> Result<Json<ListResponse>, (StatusCode, String)> {
    state.upsert(route).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(Json(ListResponse {
        rules: state.list(),
    }))
}

#[derive(Debug, Deserialize)]
struct RemoveQuery {
    method: String,
    path: String,
}

async fn remove_handler(
    State(state): State<RuntimeRouteChaosState>,
    axum::extract::Query(q): axum::extract::Query<RemoveQuery>,
) -> Response {
    let removed = state.remove(&q.method, &q.path);
    if removed {
        StatusCode::NO_CONTENT.into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

/// Build the runtime route-chaos API router. Mount under
/// `/__mockforge/api/route-chaos`.
pub fn route_chaos_api_router(state: RuntimeRouteChaosState) -> Router {
    Router::new()
        .route("/", get(list_handler).put(replace_handler))
        .route("/route", post(upsert_handler).delete(remove_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::config::RouteResponseConfig;

    fn dummy_route(method: &str, path: &str) -> RouteConfig {
        RouteConfig {
            method: method.to_string(),
            path: path.to_string(),
            request: None,
            response: RouteResponseConfig {
                status: 200,
                headers: Default::default(),
                body: None,
            },
            fault_injection: None,
            latency: None,
        }
    }

    #[test]
    fn empty_state_has_no_injector() {
        let state = RuntimeRouteChaosState::new(Vec::new());
        assert!(state.current_injector().is_none());
        assert!(state.list().is_empty());
    }

    #[test]
    fn upsert_replaces_existing_route() {
        let state = RuntimeRouteChaosState::new(Vec::new());
        let mut r = dummy_route("GET", "/users");
        r.response.status = 200;
        state.upsert(r).unwrap();
        let mut r2 = dummy_route("GET", "/users");
        r2.response.status = 503;
        state.upsert(r2).unwrap();
        let rules = state.list();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].response.status, 503);
    }

    #[test]
    fn upsert_adds_distinct_routes() {
        let state = RuntimeRouteChaosState::new(Vec::new());
        state.upsert(dummy_route("GET", "/a")).unwrap();
        state.upsert(dummy_route("POST", "/a")).unwrap();
        state.upsert(dummy_route("GET", "/b")).unwrap();
        assert_eq!(state.list().len(), 3);
    }

    #[test]
    fn remove_returns_false_when_not_found() {
        let state = RuntimeRouteChaosState::new(Vec::new());
        state.upsert(dummy_route("GET", "/a")).unwrap();
        assert!(!state.remove("GET", "/missing"));
        assert_eq!(state.list().len(), 1);
    }

    #[test]
    fn remove_strips_route_and_rebuilds() {
        let state = RuntimeRouteChaosState::new(Vec::new());
        state.upsert(dummy_route("GET", "/a")).unwrap();
        state.upsert(dummy_route("GET", "/b")).unwrap();
        assert!(state.remove("get", "/a")); // case-insensitive method
        let rules = state.list();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].path, "/b");
    }

    #[test]
    fn replace_all_swaps_atomically() {
        let state = RuntimeRouteChaosState::new(vec![dummy_route("GET", "/a")]);
        state
            .replace_all(vec![dummy_route("POST", "/x"), dummy_route("PUT", "/y")])
            .unwrap();
        let rules = state.list();
        assert_eq!(rules.len(), 2);
        assert!(rules.iter().any(|r| r.method == "POST" && r.path == "/x"));
        assert!(rules.iter().any(|r| r.method == "PUT" && r.path == "/y"));
    }
}
