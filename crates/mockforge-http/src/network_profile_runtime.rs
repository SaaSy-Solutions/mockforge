//! Runtime network-profile switching API.
//!
//! Profiles like `mobile_3g` / `lossy_network` / `extremely_poor` come
//! from `mockforge_core::network_profiles::NetworkProfileCatalog`. They
//! were previously selectable only via `--network-profile` at startup,
//! which meant a hosted-mock operator couldn't probe how their consumer
//! behaves on a slow network without redeploying.
//!
//! This module exposes the catalog over HTTP and adds a middleware that
//! applies the active profile's latency on every request. Bandwidth and
//! loss simulation are not yet wired through this path — latency is the
//! 90% case for "what if my upstream gets slow" testing and is the
//! cheapest to add without per-byte accounting.
//!
//! ## Endpoints
//!
//! - `GET    /__mockforge/api/network-profiles`            — list available profiles
//! - `GET    /__mockforge/api/network-profiles/active`     — current active profile (or 204 if none)
//! - `POST   /__mockforge/api/network-profiles/{name}/activate`  — switch to profile
//! - `POST   /__mockforge/api/network-profiles/deactivate`       — clear

use axum::extract::{Path as AxumPath, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use mockforge_core::network_profiles::{NetworkProfile, NetworkProfileCatalog};
use serde::Serialize;
use std::sync::{Arc, RwLock};

/// Cheap-to-clone shared state holding the catalog (immutable) plus the
/// currently active profile (mutable).
#[derive(Clone)]
pub struct NetworkProfileRuntimeState {
    inner: Arc<Inner>,
}

struct Inner {
    catalog: NetworkProfileCatalog,
    active: RwLock<Option<NetworkProfile>>,
}

impl NetworkProfileRuntimeState {
    /// Construct from a catalog. Active profile starts unset; the
    /// middleware fast-paths off this.
    pub fn new(catalog: NetworkProfileCatalog) -> Self {
        Self {
            inner: Arc::new(Inner {
                catalog,
                active: RwLock::new(None),
            }),
        }
    }

    /// Names + descriptions of all available profiles.
    pub fn list(&self) -> Vec<(String, String)> {
        self.inner.catalog.list_profiles_with_description()
    }

    /// Active profile snapshot.
    pub fn active(&self) -> Option<NetworkProfile> {
        self.inner.active.read().expect("network-profile state poisoned").clone()
    }

    /// Switch the active profile. Returns false when the name doesn't
    /// match any catalog entry.
    pub fn activate(&self, name: &str) -> bool {
        let profile = match self.inner.catalog.get(name) {
            Some(p) => p.clone(),
            None => return false,
        };
        *self.inner.active.write().expect("network-profile state poisoned") = Some(profile);
        true
    }

    /// Clear the active profile (return to no degradation).
    pub fn deactivate(&self) {
        *self.inner.active.write().expect("network-profile state poisoned") = None;
    }
}

/// Middleware that applies the active profile's latency before passing
/// to the next layer. Reads `NetworkProfileRuntimeState::active()` per
/// request — a swap takes effect on the very next call.
pub async fn network_profile_middleware(
    State(state): State<NetworkProfileRuntimeState>,
    req: Request,
    next: Next,
) -> Response {
    if let Some(profile) = state.active() {
        let delay = profile.latency.calculate_latency(&[]);
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }
    next.run(req).await
}

#[derive(Debug, Serialize)]
struct ProfileSummary {
    name: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct ListResponse {
    profiles: Vec<ProfileSummary>,
    active: Option<String>,
}

async fn list_handler(State(state): State<NetworkProfileRuntimeState>) -> Json<ListResponse> {
    Json(ListResponse {
        profiles: state
            .list()
            .into_iter()
            .map(|(name, description)| ProfileSummary { name, description })
            .collect(),
        active: state.active().map(|p| p.name),
    })
}

async fn active_handler(State(state): State<NetworkProfileRuntimeState>) -> Response {
    match state.active() {
        Some(profile) => Json(profile).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}

async fn activate_handler(
    State(state): State<NetworkProfileRuntimeState>,
    AxumPath(name): AxumPath<String>,
) -> Response {
    if state.activate(&name) {
        Json(serde_json::json!({ "active": name })).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "profile_not_found",
                "message": format!("No network profile named '{}'", name),
            })),
        )
            .into_response()
    }
}

async fn deactivate_handler(State(state): State<NetworkProfileRuntimeState>) -> Response {
    state.deactivate();
    StatusCode::NO_CONTENT.into_response()
}

/// Build the network-profile runtime API router. Mount under
/// `/__mockforge/api/network-profiles`.
pub fn network_profile_api_router(state: NetworkProfileRuntimeState) -> Router {
    Router::new()
        .route("/", get(list_handler))
        .route("/active", get(active_handler))
        .route("/{name}/activate", post(activate_handler))
        .route("/deactivate", post(deactivate_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_active_until_activated() {
        let state = NetworkProfileRuntimeState::new(NetworkProfileCatalog::new());
        assert!(state.active().is_none());
    }

    #[test]
    fn activate_unknown_returns_false() {
        let state = NetworkProfileRuntimeState::new(NetworkProfileCatalog::new());
        assert!(!state.activate("not_a_real_profile_name"));
        assert!(state.active().is_none());
    }

    #[test]
    fn list_includes_builtin_profiles() {
        let state = NetworkProfileRuntimeState::new(NetworkProfileCatalog::new());
        let names: Vec<String> = state.list().into_iter().map(|(n, _)| n).collect();
        // Smoke test: the catalog ships with at least these.
        assert!(names.iter().any(|n| n.contains("3g") || n.contains("3G")));
        assert!(!names.is_empty());
    }

    #[test]
    fn activate_then_deactivate() {
        let state = NetworkProfileRuntimeState::new(NetworkProfileCatalog::new());
        let any_name = state.list().first().map(|(n, _)| n.clone()).expect("catalog non-empty");
        assert!(state.activate(&any_name));
        assert_eq!(state.active().map(|p| p.name), Some(any_name));
        state.deactivate();
        assert!(state.active().is_none());
    }
}
