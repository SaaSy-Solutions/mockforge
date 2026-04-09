//! Registry admin integration for the OSS admin UI.
//!
//! This module wires the shared [`mockforge_registry_core::store::SqliteRegistryStore`]
//! into `mockforge-ui` so the embedded admin server can manage users,
//! organizations, API tokens, and audit logs against a local SQLite
//! database — reusing the same `RegistryStore` trait and query paths that
//! power the multi-tenant SaaS `mockforge-registry-server` binary.
//!
//! This is the Phase 5a entry point (task #16). Follow-up work will add
//! the axum routes that call into the store; for now the module exposes
//! `init_sqlite_registry_store(db_url)` plus a shared [`CoreAppState`]
//! struct so any future handler can take `State<CoreAppState>` and reach
//! the store through a stable `Arc<dyn RegistryStore>` dispatch.

#![cfg(feature = "registry-admin")]

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;

use mockforge_registry_core::error::StoreResult;
use mockforge_registry_core::store::{RegistryStore, SqliteRegistryStore};

/// Minimal app state for the registry-admin subsystem.
///
/// Kept intentionally small — just the backend-agnostic `Arc<dyn
/// RegistryStore>`. The UI's main `AppState` (in `routes.rs`) can hold
/// one of these inside an `Option` and only construct it when the user
/// opts into the OSS admin backend.
#[derive(Clone)]
pub struct CoreAppState {
    pub store: Arc<dyn RegistryStore>,
}

impl CoreAppState {
    /// Wrap an arbitrary [`RegistryStore`] implementation. Useful for
    /// tests that want to hand in a mock or in-memory store.
    pub fn new(store: Arc<dyn RegistryStore>) -> Self {
        Self { store }
    }
}

/// Bootstrap a SQLite-backed registry store from a connection URL,
/// running the bundled OSS migrations. Returns the concrete store so
/// callers can also reach the pool if they need to run raw SQL during
/// setup; most callers should wrap it in a [`CoreAppState`] via
/// [`CoreAppState::new`] + `Arc::new`.
///
/// Example URLs:
///   * `sqlite::memory:`               — in-process, discarded on exit
///   * `sqlite://./mockforge.db`       — on-disk file in the cwd
///   * `sqlite:///var/lib/mockforge.db` — absolute path
pub async fn init_sqlite_registry_store(database_url: &str) -> StoreResult<SqliteRegistryStore> {
    SqliteRegistryStore::connect(database_url).await
}

// ---------------------------------------------------------------------------
// Axum router — read-only registry admin endpoints.
//
// Phase 5b exposes a minimal surface against the existing trait methods:
//
//   * GET /api/admin/registry/health
//   * GET /api/admin/registry/users/email/:email
//   * GET /api/admin/registry/users/username/:username
//   * GET /api/admin/registry/orgs/slug/:slug
//
// The router is returned as a standalone `Router<()>` with its own state
// baked in, so the existing giant `create_admin_router(...)` signature in
// routes.rs doesn't need to grow another argument — callers just do
// `.merge(registry_admin::router(state))` to plug it in.
// ---------------------------------------------------------------------------

/// Small wrapper that converts a [`StoreError`] into a JSON error body
/// with the appropriate HTTP status, so handlers can use `?` freely.
struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let ApiError(status, msg) = self;
        (
            status,
            Json(json!({
                "error": msg,
                "status": status.as_u16(),
            })),
        )
            .into_response()
    }
}

impl From<mockforge_registry_core::error::StoreError> for ApiError {
    fn from(e: mockforge_registry_core::error::StoreError) -> Self {
        use mockforge_registry_core::error::StoreError;
        match e {
            StoreError::NotFound => ApiError(StatusCode::NOT_FOUND, "not found".into()),
            StoreError::Database(err) => {
                ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("database: {}", err))
            }
            StoreError::Hash(msg) => {
                ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("internal: {}", msg))
            }
        }
    }
}

/// Build the registry-admin sub-router.
///
/// The returned router is fully state-erased (`Router<()>`) so it can be
/// `.merge()`d into any parent axum router without additional wiring.
pub fn router(state: CoreAppState) -> Router {
    Router::new()
        .route("/api/admin/registry/health", get(health))
        .route("/api/admin/registry/users/email/{email}", get(find_user_by_email))
        .route("/api/admin/registry/users/username/{username}", get(find_user_by_username))
        .route("/api/admin/registry/orgs/slug/{slug}", get(find_org_by_slug))
        .with_state(state)
}

async fn health(State(state): State<CoreAppState>) -> Result<Json<serde_json::Value>, ApiError> {
    state.store.health_check().await?;
    Ok(Json(json!({ "status": "ok" })))
}

async fn find_user_by_email(
    State(state): State<CoreAppState>,
    Path(email): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = state
        .store
        .find_user_by_email(&email)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, format!("user '{}' not found", email)))?;
    Ok(Json(json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "is_verified": user.is_verified,
        "is_admin": user.is_admin,
        "two_factor_enabled": user.two_factor_enabled,
        "created_at": user.created_at,
        "updated_at": user.updated_at,
    })))
}

async fn find_user_by_username(
    State(state): State<CoreAppState>,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = state
        .store
        .find_user_by_username(&username)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, format!("user '{}' not found", username)))?;
    Ok(Json(json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "is_verified": user.is_verified,
        "is_admin": user.is_admin,
        "two_factor_enabled": user.two_factor_enabled,
        "created_at": user.created_at,
        "updated_at": user.updated_at,
    })))
}

async fn find_org_by_slug(
    State(state): State<CoreAppState>,
    Path(slug): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let org = state
        .store
        .find_organization_by_slug(&slug)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, format!("org '{}' not found", slug)))?;
    Ok(Json(json!({
        "id": org.id,
        "name": org.name,
        "slug": org.slug,
        "owner_id": org.owner_id,
        "plan": org.plan,
        "created_at": org.created_at,
        "updated_at": org.updated_at,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use mockforge_registry_core::models::organization::Plan;
    use tower::ServiceExt;

    /// Smoke test that proves `mockforge-ui` can actually reach into
    /// `mockforge-registry-core`: open an in-memory SQLite store, run
    /// the migrations, create a user + org, and hit the store through
    /// the `dyn RegistryStore` trait object inside `CoreAppState`.
    #[tokio::test]
    async fn test_init_sqlite_registry_store_end_to_end() {
        let store = init_sqlite_registry_store("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");

        // Exercise the concrete store first — this is what the main
        // admin server init path would do.
        let user = store
            .create_user("ui-admin", "ui-admin@example.com", "bcrypt_hash")
            .await
            .expect("create user");
        let org = store
            .create_organization("UI Org", "ui-org", user.id, Plan::Free)
            .await
            .expect("create org");
        assert_eq!(org.owner_id, user.id);

        // Wrap in CoreAppState and round-trip the lookups through the
        // Arc<dyn RegistryStore> dispatch — this is the shape the UI
        // handlers will use.
        let state = CoreAppState::new(Arc::new(store));
        let reloaded_user = state
            .store
            .find_user_by_email("ui-admin@example.com")
            .await
            .expect("find user")
            .expect("user exists");
        assert_eq!(reloaded_user.id, user.id);

        let reloaded_org = state
            .store
            .find_organization_by_slug("ui-org")
            .await
            .expect("find org")
            .expect("org exists");
        assert_eq!(reloaded_org.id, org.id);
    }

    /// Build a fully bootstrapped router + test fixtures for the HTTP tests.
    async fn test_router_with_seed() -> (Router, uuid::Uuid, uuid::Uuid) {
        let store = init_sqlite_registry_store("sqlite::memory:").await.unwrap();
        let user = store.create_user("route-admin", "route@example.com", "hash").await.unwrap();
        let org = store
            .create_organization("Route Org", "route-org", user.id, Plan::Free)
            .await
            .unwrap();
        let state = CoreAppState::new(Arc::new(store));
        (router(state), user.id, org.id)
    }

    async fn body_json(resp: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_health_endpoint_returns_ok() {
        let (router, _, _) = test_router_with_seed().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .uri("/api/admin/registry/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    async fn test_find_user_by_email_endpoint() {
        let (router, user_id, _) = test_router_with_seed().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .uri("/api/admin/registry/users/email/route@example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["email"], "route@example.com");
        assert_eq!(body["username"], "route-admin");
        assert_eq!(body["id"], user_id.to_string());
    }

    #[tokio::test]
    async fn test_find_user_by_email_missing_returns_404() {
        let (router, _, _) = test_router_with_seed().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .uri("/api/admin/registry/users/email/nobody@example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body = body_json(resp).await;
        assert!(body["error"].as_str().unwrap().contains("nobody@example.com"));
    }

    #[tokio::test]
    async fn test_find_user_by_username_endpoint() {
        let (router, user_id, _) = test_router_with_seed().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .uri("/api/admin/registry/users/username/route-admin")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["username"], "route-admin");
        assert_eq!(body["id"], user_id.to_string());
    }

    #[tokio::test]
    async fn test_find_org_by_slug_endpoint() {
        let (router, user_id, org_id) = test_router_with_seed().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .uri("/api/admin/registry/orgs/slug/route-org")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["slug"], "route-org");
        assert_eq!(body["name"], "Route Org");
        assert_eq!(body["plan"], "free");
        assert_eq!(body["id"], org_id.to_string());
        assert_eq!(body["owner_id"], user_id.to_string());
    }
}
