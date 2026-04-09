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
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use mockforge_registry_core::error::StoreResult;
use mockforge_registry_core::models::organization::Plan;
use mockforge_registry_core::models::TokenScope;
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
        .route("/api/admin/registry/users", post(create_user))
        .route("/api/admin/registry/users/email/{email}", get(find_user_by_email))
        .route("/api/admin/registry/users/username/{username}", get(find_user_by_username))
        .route("/api/admin/registry/users/{id}/verify", post(mark_user_verified))
        .route("/api/admin/registry/orgs", post(create_org))
        .route("/api/admin/registry/orgs/slug/{slug}", get(find_org_by_slug))
        .route("/api/admin/registry/orgs/{org_id}/tokens", post(create_api_token))
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

// ---------------------------------------------------------------------------
// Write endpoints
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct CreateUserReq {
    username: String,
    email: String,
    /// Already-hashed password (caller is expected to have run bcrypt).
    /// The OSS admin bootstrap flow lives in mockforge_registry_core::auth.
    password_hash: String,
}

async fn create_user(
    State(state): State<CoreAppState>,
    Json(req): Json<CreateUserReq>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if req.username.trim().is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "username must not be empty".into()));
    }
    if !req.email.contains('@') {
        return Err(ApiError(StatusCode::BAD_REQUEST, "email looks invalid".into()));
    }
    if req.password_hash.len() < 20 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "password_hash looks too short — are you sending plaintext?".into(),
        ));
    }

    let user = state.store.create_user(&req.username, &req.email, &req.password_hash).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": user.id,
            "username": user.username,
            "email": user.email,
            "is_verified": user.is_verified,
            "is_admin": user.is_admin,
            "created_at": user.created_at,
        })),
    ))
}

async fn mark_user_verified(
    State(state): State<CoreAppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = Uuid::parse_str(&id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad uuid: {}", e)))?;
    state.store.mark_user_verified(user_id).await?;
    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, format!("user {} not found", id)))?;
    Ok(Json(json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "is_verified": user.is_verified,
    })))
}

#[derive(Debug, Deserialize)]
struct CreateOrgReq {
    name: String,
    slug: String,
    owner_id: String,
    #[serde(default)]
    plan: Option<String>,
}

async fn create_org(
    State(state): State<CoreAppState>,
    Json(req): Json<CreateOrgReq>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if req.name.trim().is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "name must not be empty".into()));
    }
    if req.slug.trim().is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "slug must not be empty".into()));
    }
    let owner_id = Uuid::parse_str(&req.owner_id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad owner_id uuid: {}", e)))?;
    let plan = match req.plan.as_deref() {
        Some("pro") => Plan::Pro,
        Some("team") => Plan::Team,
        None | Some("free") => Plan::Free,
        Some(other) => {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                format!("unknown plan '{}' (expected free/pro/team)", other),
            ));
        }
    };

    let org = state.store.create_organization(&req.name, &req.slug, owner_id, plan).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": org.id,
            "name": org.name,
            "slug": org.slug,
            "owner_id": org.owner_id,
            "plan": org.plan,
            "created_at": org.created_at,
        })),
    ))
}

#[derive(Debug, Deserialize)]
struct CreateApiTokenReq {
    name: String,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    scopes: Vec<String>,
}

async fn create_api_token(
    State(state): State<CoreAppState>,
    Path(org_id): Path<String>,
    Json(req): Json<CreateApiTokenReq>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if req.name.trim().is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "name must not be empty".into()));
    }
    let org_id = Uuid::parse_str(&org_id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad org_id uuid: {}", e)))?;
    let user_id = req
        .user_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad user_id uuid: {}", e)))?;

    let mut scope_enums = Vec::with_capacity(req.scopes.len());
    for s in &req.scopes {
        match TokenScope::from_string(s) {
            Some(scope) => scope_enums.push(scope),
            None => {
                return Err(ApiError(StatusCode::BAD_REQUEST, format!("unknown scope '{}'", s)));
            }
        }
    }

    let (plaintext, token) = state
        .store
        .create_api_token(org_id, user_id, &req.name, &scope_enums, None)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            // Only returned on create — clients MUST save it now.
            "token": plaintext,
            "id": token.id,
            "org_id": token.org_id,
            "user_id": token.user_id,
            "name": token.name,
            "token_prefix": token.token_prefix,
            "scopes": token.scopes,
            "created_at": token.created_at,
        })),
    ))
}

/// Tiny helper for tests that need to POST JSON bodies.
#[cfg(test)]
fn json_post(uri: &str, body: serde_json::Value) -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body.to_string()))
        .unwrap()
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

    #[tokio::test]
    async fn test_create_user_endpoint() {
        let (router, _, _) = test_router_with_seed().await;
        let resp = router
            .oneshot(json_post(
                "/api/admin/registry/users",
                json!({
                    "username": "brand-new",
                    "email": "new@example.com",
                    "password_hash": "bcrypt-hash-placeholder-long-enough"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = body_json(resp).await;
        assert_eq!(body["username"], "brand-new");
        assert_eq!(body["email"], "new@example.com");
        assert_eq!(body["is_verified"], false);
    }

    #[tokio::test]
    async fn test_create_user_validates_empty_username() {
        let (router, _, _) = test_router_with_seed().await;
        let resp = router
            .oneshot(json_post(
                "/api/admin/registry/users",
                json!({
                    "username": "",
                    "email": "ok@example.com",
                    "password_hash": "bcrypt-hash-placeholder-long-enough"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_user_rejects_short_password_hash() {
        let (router, _, _) = test_router_with_seed().await;
        let resp = router
            .oneshot(json_post(
                "/api/admin/registry/users",
                json!({
                    "username": "x",
                    "email": "x@example.com",
                    "password_hash": "plaintext"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_mark_user_verified_endpoint() {
        let (router, user_id, _) = test_router_with_seed().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/admin/registry/users/{}/verify", user_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["is_verified"], true);
        assert_eq!(body["id"], user_id.to_string());
    }

    #[tokio::test]
    async fn test_create_org_endpoint() {
        let (router, user_id, _) = test_router_with_seed().await;
        let resp = router
            .oneshot(json_post(
                "/api/admin/registry/orgs",
                json!({
                    "name": "Second Org",
                    "slug": "second-org",
                    "owner_id": user_id.to_string(),
                    "plan": "pro"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = body_json(resp).await;
        assert_eq!(body["slug"], "second-org");
        assert_eq!(body["plan"], "pro");
        assert_eq!(body["owner_id"], user_id.to_string());
    }

    #[tokio::test]
    async fn test_create_org_rejects_unknown_plan() {
        let (router, user_id, _) = test_router_with_seed().await;
        let resp = router
            .oneshot(json_post(
                "/api/admin/registry/orgs",
                json!({
                    "name": "X",
                    "slug": "x",
                    "owner_id": user_id.to_string(),
                    "plan": "enterprise"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_api_token_endpoint() {
        let (router, user_id, org_id) = test_router_with_seed().await;
        let resp = router
            .oneshot(json_post(
                &format!("/api/admin/registry/orgs/{}/tokens", org_id),
                json!({
                    "name": "ci-token",
                    "user_id": user_id.to_string(),
                    "scopes": ["read:packages", "publish:packages"]
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = body_json(resp).await;
        assert!(body["token"].as_str().unwrap().starts_with("mfx_"));
        assert_eq!(body["name"], "ci-token");
        assert_eq!(body["scopes"].as_array().unwrap().len(), 2);
        assert!(body["token_prefix"].as_str().unwrap().starts_with("mfx_"));
    }

    #[tokio::test]
    async fn test_create_api_token_rejects_unknown_scope() {
        let (router, user_id, org_id) = test_router_with_seed().await;
        let resp = router
            .oneshot(json_post(
                &format!("/api/admin/registry/orgs/{}/tokens", org_id),
                json!({
                    "name": "ci-token",
                    "user_id": user_id.to_string(),
                    "scopes": ["bogus:scope"]
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
