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
    /// JWT signing secret. If empty, auth endpoints (register/login)
    /// are still functional but the JWTs they issue will also verify
    /// against an empty secret — fine for tests, NOT for production.
    pub jwt_secret: String,
}

impl CoreAppState {
    /// Wrap an arbitrary [`RegistryStore`] implementation. Useful for
    /// tests that want to hand in a mock or in-memory store. Uses an
    /// empty JWT secret — call [`CoreAppState::with_jwt_secret`] or
    /// construct manually for production use.
    pub fn new(store: Arc<dyn RegistryStore>) -> Self {
        Self {
            store,
            jwt_secret: String::new(),
        }
    }

    /// Like [`CoreAppState::new`] but with a real JWT signing secret.
    pub fn with_jwt_secret(store: Arc<dyn RegistryStore>, jwt_secret: String) -> Self {
        Self { store, jwt_secret }
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

/// First-run bootstrap for the OSS admin: if the store has zero users
/// and the operator has provided `MOCKFORGE_ADMIN_USERNAME`,
/// `MOCKFORGE_ADMIN_EMAIL`, and `MOCKFORGE_ADMIN_PASSWORD` environment
/// variables, create a verified admin user so they can log in
/// immediately. Returns `Ok(true)` if an admin was created, `Ok(false)`
/// if the bootstrap was skipped (either because users already exist or
/// because the env vars weren't set).
///
/// This lets a fresh `mockforge serve --admin` run auto-provision its
/// first user without requiring a manual `curl` dance, matching the
/// UX of other self-hosted admin tools (Grafana, Prometheus Alertmanager,
/// etc.).
pub async fn bootstrap_admin_user_from_env<S: RegistryStore + ?Sized>(
    store: &S,
) -> StoreResult<bool> {
    let Ok(username) = std::env::var("MOCKFORGE_ADMIN_USERNAME") else {
        return Ok(false);
    };
    let Ok(email) = std::env::var("MOCKFORGE_ADMIN_EMAIL") else {
        return Ok(false);
    };
    let Ok(password) = std::env::var("MOCKFORGE_ADMIN_PASSWORD") else {
        return Ok(false);
    };

    if store.find_user_by_username(&username).await?.is_some()
        || store.find_user_by_email(&email).await?.is_some()
    {
        // Already provisioned — no-op.
        return Ok(false);
    }

    let hash = mockforge_registry_core::auth::hash_password(&password)
        .map_err(|e| mockforge_registry_core::error::StoreError::Hash(e.to_string()))?;
    let user = store.create_user(&username, &email, &hash).await?;
    // Mark as verified so they can log in without an email round-trip.
    store.mark_user_verified(user.id).await?;
    tracing::info!(
        "Bootstrapped admin user '{}' (email: {}) from MOCKFORGE_ADMIN_* env vars",
        username,
        email
    );
    Ok(true)
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
        // Auth — register/login issue a JWT bound to the user id
        .route("/api/admin/registry/auth/register", post(register))
        .route("/api/admin/registry/auth/login", post(login))
        .route("/api/admin/registry/auth/me", get(auth_me))
        // User management
        .route("/api/admin/registry/users", post(create_user))
        .route("/api/admin/registry/users/email/{email}", get(find_user_by_email))
        .route("/api/admin/registry/users/username/{username}", get(find_user_by_username))
        .route("/api/admin/registry/users/{id}/verify", post(mark_user_verified))
        // Org management
        .route("/api/admin/registry/orgs", post(create_org))
        .route("/api/admin/registry/orgs/slug/{slug}", get(find_org_by_slug))
        .route("/api/admin/registry/orgs/{org_id}/tokens", post(create_api_token))
        // Org members — "teams" in the task-list vocabulary. Existing
        // trait methods power everything here; this is pure wiring.
        .route("/api/admin/registry/orgs/{org_id}/members", get(list_org_members_endpoint))
        .route("/api/admin/registry/orgs/{org_id}/members", post(add_org_member_endpoint))
        .route(
            "/api/admin/registry/orgs/{org_id}/members/{user_id}",
            axum::routing::patch(update_org_member_role_endpoint)
                .delete(remove_org_member_endpoint),
        )
        // Org quota — reuses org_settings under the reserved "quota" key,
        // so no new trait method or schema change is needed.
        .route(
            "/api/admin/registry/orgs/{org_id}/quota",
            get(get_org_quota).put(set_org_quota),
        )
        // Invitation flow — uses the existing verification_tokens table
        // by storing a JSON-encoded payload in the `token` column.
        .route("/api/admin/registry/orgs/{org_id}/invitations", post(create_invitation))
        .route("/api/admin/registry/invitations/{token}", get(get_invitation))
        .route(
            "/api/admin/registry/invitations/{token}/accept",
            post(accept_invitation),
        )
        .with_state(state)
}

async fn health(State(state): State<CoreAppState>) -> Result<Json<serde_json::Value>, ApiError> {
    state.store.health_check().await?;
    Ok(Json(json!({ "status": "ok" })))
}

// --- Auth ------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RegisterReq {
    username: String,
    email: String,
    /// Plaintext — this endpoint bcrypts it server-side. Contrast with
    /// POST /users which accepts an already-hashed password so callers
    /// can integrate with external hashing strategies.
    password: String,
}

async fn register(
    State(state): State<CoreAppState>,
    Json(req): Json<RegisterReq>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if req.username.trim().is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "username must not be empty".into()));
    }
    if !req.email.contains('@') {
        return Err(ApiError(StatusCode::BAD_REQUEST, "email looks invalid".into()));
    }
    if req.password.len() < 8 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "password must be at least 8 characters".into(),
        ));
    }

    // Reject duplicate usernames/emails up front so we return a helpful
    // 409 instead of a cryptic StoreError::Database UNIQUE violation.
    if state.store.find_user_by_email(&req.email).await?.is_some() {
        return Err(ApiError(StatusCode::CONFLICT, "email already registered".into()));
    }
    if state.store.find_user_by_username(&req.username).await?.is_some() {
        return Err(ApiError(StatusCode::CONFLICT, "username already taken".into()));
    }

    let hash = mockforge_registry_core::auth::hash_password(&req.password)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("hash: {}", e)))?;
    let user = state.store.create_user(&req.username, &req.email, &hash).await?;
    let token =
        mockforge_registry_core::auth::create_access_token(&user.id.to_string(), &state.jwt_secret)
            .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("jwt: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "user": {
                "id": user.id,
                "username": user.username,
                "email": user.email,
                "is_verified": user.is_verified,
            },
            "token": token,
        })),
    ))
}

#[derive(Debug, Deserialize)]
struct LoginReq {
    /// Either username or email — the handler tries both, in that order.
    identifier: String,
    password: String,
}

async fn login(
    State(state): State<CoreAppState>,
    Json(req): Json<LoginReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Resolve the user by username first, then by email, before doing
    // the expensive bcrypt verify. Missing user returns the same 401 as
    // a bad password so we don't leak account-existence info.
    let user = match state.store.find_user_by_username(&req.identifier).await? {
        Some(u) => u,
        None => state
            .store
            .find_user_by_email(&req.identifier)
            .await?
            .ok_or(ApiError(StatusCode::UNAUTHORIZED, "invalid credentials".into()))?,
    };

    let ok = mockforge_registry_core::auth::verify_password(&req.password, &user.password_hash)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("verify: {}", e)))?;
    if !ok {
        return Err(ApiError(StatusCode::UNAUTHORIZED, "invalid credentials".into()));
    }

    let token =
        mockforge_registry_core::auth::create_access_token(&user.id.to_string(), &state.jwt_secret)
            .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("jwt: {}", e)))?;

    Ok(Json(json!({
        "user": {
            "id": user.id,
            "username": user.username,
            "email": user.email,
            "is_verified": user.is_verified,
            "is_admin": user.is_admin,
        },
        "token": token,
    })))
}

/// GET /api/admin/registry/auth/me — verifies the Authorization: Bearer
/// token against the configured JWT secret and returns the user.
async fn auth_me(
    State(state): State<CoreAppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let auth = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "missing Authorization header".into()))?;
    let token = auth.strip_prefix("Bearer ").ok_or(ApiError(
        StatusCode::UNAUTHORIZED,
        "expected 'Authorization: Bearer <token>'".into(),
    ))?;
    let claims = mockforge_registry_core::auth::verify_token(token, &state.jwt_secret)
        .map_err(|e| ApiError(StatusCode::UNAUTHORIZED, format!("invalid token: {}", e)))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|e| ApiError(StatusCode::UNAUTHORIZED, format!("bad subject: {}", e)))?;
    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, "user no longer exists".into()))?;
    Ok(Json(json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "is_verified": user.is_verified,
        "is_admin": user.is_admin,
        "claims_exp": claims.exp,
    })))
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

// ---------------------------------------------------------------------------
// Org member management (Phase 3a)
// ---------------------------------------------------------------------------

async fn list_org_members_endpoint(
    State(state): State<CoreAppState>,
    Path(org_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let org_id = Uuid::parse_str(&org_id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad org_id: {}", e)))?;
    let members = state.store.list_org_members(org_id).await?;
    let list: Vec<serde_json::Value> = members
        .into_iter()
        .map(|m| {
            json!({
                "id": m.id,
                "org_id": m.org_id,
                "user_id": m.user_id,
                "role": m.role,
                "created_at": m.created_at,
                "updated_at": m.updated_at,
            })
        })
        .collect();
    Ok(Json(json!({ "members": list })))
}

#[derive(Debug, Deserialize)]
struct AddOrgMemberReq {
    user_id: String,
    #[serde(default)]
    role: Option<String>,
}

fn parse_role(s: &str) -> Result<mockforge_registry_core::models::organization::OrgRole, ApiError> {
    use mockforge_registry_core::models::organization::OrgRole;
    match s {
        "owner" => Ok(OrgRole::Owner),
        "admin" => Ok(OrgRole::Admin),
        "member" => Ok(OrgRole::Member),
        other => Err(ApiError(
            StatusCode::BAD_REQUEST,
            format!("unknown role '{}' (expected owner/admin/member)", other),
        )),
    }
}

async fn add_org_member_endpoint(
    State(state): State<CoreAppState>,
    Path(org_id): Path<String>,
    Json(req): Json<AddOrgMemberReq>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let org_id = Uuid::parse_str(&org_id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad org_id: {}", e)))?;
    let user_id = Uuid::parse_str(&req.user_id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad user_id: {}", e)))?;
    let role = parse_role(req.role.as_deref().unwrap_or("member"))?;

    let member = state.store.create_org_member(org_id, user_id, role).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": member.id,
            "org_id": member.org_id,
            "user_id": member.user_id,
            "role": member.role,
            "created_at": member.created_at,
        })),
    ))
}

#[derive(Debug, Deserialize)]
struct UpdateOrgMemberRoleReq {
    role: String,
}

async fn update_org_member_role_endpoint(
    State(state): State<CoreAppState>,
    Path((org_id, user_id)): Path<(String, String)>,
    Json(req): Json<UpdateOrgMemberRoleReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let org_id = Uuid::parse_str(&org_id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad org_id: {}", e)))?;
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad user_id: {}", e)))?;
    let role = parse_role(&req.role)?;

    state.store.update_org_member_role(org_id, user_id, role).await?;
    let member = state
        .store
        .find_org_member(org_id, user_id)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, "member not found".into()))?;
    Ok(Json(json!({
        "id": member.id,
        "org_id": member.org_id,
        "user_id": member.user_id,
        "role": member.role,
        "updated_at": member.updated_at,
    })))
}

async fn remove_org_member_endpoint(
    State(state): State<CoreAppState>,
    Path((org_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let org_id = Uuid::parse_str(&org_id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad org_id: {}", e)))?;
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad user_id: {}", e)))?;
    state.store.delete_org_member(org_id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Org quota (Phase 3b) — reuses org_settings under a reserved key, so no
// schema changes or new trait methods required.
// ---------------------------------------------------------------------------

const QUOTA_SETTING_KEY: &str = "quota";

async fn get_org_quota(
    State(state): State<CoreAppState>,
    Path(org_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let org_id = Uuid::parse_str(&org_id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad org_id: {}", e)))?;
    let setting = state.store.get_org_setting(org_id, QUOTA_SETTING_KEY).await?;
    let value = setting.map(|s| s.setting_value).unwrap_or_else(|| json!({}));
    Ok(Json(json!({ "org_id": org_id, "quota": value })))
}

async fn set_org_quota(
    State(state): State<CoreAppState>,
    Path(org_id): Path<String>,
    Json(quota): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !quota.is_object() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "quota body must be a JSON object".into()));
    }
    let org_id = Uuid::parse_str(&org_id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad org_id: {}", e)))?;
    let updated = state.store.set_org_setting(org_id, QUOTA_SETTING_KEY, quota).await?;
    Ok(Json(json!({
        "org_id": org_id,
        "quota": updated.setting_value,
        "updated_at": updated.updated_at,
    })))
}

// ---------------------------------------------------------------------------
// Invitation flow (Phase 3d) — purely registry-admin-local. Uses existing
// verification_tokens by stuffing a JSON payload into the `token` column:
//
//   {"kind":"invite","org_id":"<uuid>","email":"...","role":"member",
//    "nonce":"<32 random bytes, base64url>"}
//
// The nonce makes the token value unique and unguessable. Accept expects
// the caller to echo the exact token string back, which we look up via
// find_verification_token_by_token, parse the payload, and consume.
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize, Deserialize)]
struct InvitePayload {
    kind: String, // always "invite"
    org_id: Uuid,
    email: String,
    role: String,
    nonce: String,
}

#[derive(Debug, Deserialize)]
struct CreateInvitationReq {
    email: String,
    #[serde(default)]
    role: Option<String>,
}

async fn create_invitation(
    State(state): State<CoreAppState>,
    Path(org_id): Path<String>,
    Json(req): Json<CreateInvitationReq>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if !req.email.contains('@') {
        return Err(ApiError(StatusCode::BAD_REQUEST, "email looks invalid".into()));
    }
    let org_id = Uuid::parse_str(&org_id)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("bad org_id: {}", e)))?;
    let role = req.role.as_deref().unwrap_or("member").to_string();
    parse_role(&role)?; // validate before we persist anything

    // Ensure the org actually exists so the invite can't reference a
    // ghost org that's been deleted.
    state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, "org not found".into()))?;

    // Generate the nonce + payload. The verification_token row stores
    // the JSON-encoded payload directly as the `token` column — we
    // look it up verbatim on accept.
    use base64::{engine::general_purpose, Engine as _};
    use rand::RngCore;
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    let nonce = general_purpose::URL_SAFE_NO_PAD.encode(buf);
    let payload = InvitePayload {
        kind: "invite".into(),
        org_id,
        email: req.email.clone(),
        role,
        nonce,
    };
    let payload_str = serde_json::to_string(&payload)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("encode: {}", e)))?;

    // Piggyback on create_verification_token by first creating a
    // throwaway row for a synthetic user_id (the inviter is unknown
    // at this point). We then overwrite its `token` column with the
    // JSON payload via a raw UPDATE since the trait doesn't expose a
    // "set token value" method. This keeps invitations purely
    // store-side without a new trait method.
    //
    // To avoid needing raw SQL and a new trait method, we instead use
    // `set_org_setting` to stash invitations under a per-token key.
    let setting_key = format!("invite:{}", payload.nonce);
    state
        .store
        .set_org_setting(org_id, &setting_key, serde_json::to_value(&payload).unwrap())
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "token": payload_str,
            "org_id": org_id,
            "email": payload.email,
            "role": payload.role,
        })),
    ))
}

fn decode_invite_token(token: &str) -> Result<InvitePayload, ApiError> {
    let payload: InvitePayload = serde_json::from_str(token)
        .map_err(|_| ApiError(StatusCode::NOT_FOUND, "invalid invitation token".into()))?;
    if payload.kind != "invite" {
        return Err(ApiError(StatusCode::NOT_FOUND, "token is not an invitation".into()));
    }
    Ok(payload)
}

async fn get_invitation(
    State(state): State<CoreAppState>,
    Path(token): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let payload = decode_invite_token(&token)?;
    let setting_key = format!("invite:{}", payload.nonce);
    let setting =
        state
            .store
            .get_org_setting(payload.org_id, &setting_key)
            .await?
            .ok_or(ApiError(
                StatusCode::NOT_FOUND,
                "invitation not found or already accepted".into(),
            ))?;
    let stored: InvitePayload = serde_json::from_value(setting.setting_value)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("decode: {}", e)))?;
    // Defense in depth: verify the client-supplied token matches what we
    // stored (ie. they didn't forge a fresh JSON with a guessed nonce).
    if stored.nonce != payload.nonce
        || stored.email != payload.email
        || stored.org_id != payload.org_id
    {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "invitation not found or already accepted".into(),
        ));
    }
    Ok(Json(json!({
        "org_id": stored.org_id,
        "email": stored.email,
        "role": stored.role,
    })))
}

#[derive(Debug, Deserialize)]
struct AcceptInvitationReq {
    username: String,
    password: String,
}

async fn accept_invitation(
    State(state): State<CoreAppState>,
    Path(token): Path<String>,
    Json(req): Json<AcceptInvitationReq>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if req.username.trim().is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "username must not be empty".into()));
    }
    if req.password.len() < 8 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "password must be at least 8 characters".into(),
        ));
    }

    let payload = decode_invite_token(&token)?;
    let setting_key = format!("invite:{}", payload.nonce);

    // Consume: fetch, validate, then delete. Any error mid-flow leaves
    // the invitation intact so it can be retried.
    let setting =
        state
            .store
            .get_org_setting(payload.org_id, &setting_key)
            .await?
            .ok_or(ApiError(
                StatusCode::NOT_FOUND,
                "invitation not found or already accepted".into(),
            ))?;
    let stored: InvitePayload = serde_json::from_value(setting.setting_value)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("decode: {}", e)))?;
    if stored.nonce != payload.nonce || stored.email != payload.email {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "invitation not found or already accepted".into(),
        ));
    }

    // Create the user account.
    if state.store.find_user_by_username(&req.username).await?.is_some() {
        return Err(ApiError(StatusCode::CONFLICT, "username already taken".into()));
    }
    if state.store.find_user_by_email(&stored.email).await?.is_some() {
        return Err(ApiError(StatusCode::CONFLICT, "a user with this email already exists".into()));
    }
    let hash = mockforge_registry_core::auth::hash_password(&req.password)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("hash: {}", e)))?;
    let created = state.store.create_user(&req.username, &stored.email, &hash).await?;
    // Invites are pre-verified by definition (someone already owns the
    // mailbox and consented to join).
    state.store.mark_user_verified(created.id).await?;
    // Re-fetch so the response reflects the is_verified=true update.
    let user = state
        .store
        .find_user_by_id(created.id)
        .await?
        .ok_or(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "user vanished mid-accept".into()))?;

    // Add them as an org member.
    let role = parse_role(&stored.role)?;
    state.store.create_org_member(stored.org_id, user.id, role).await?;

    // Consume the invitation.
    state.store.delete_org_setting(payload.org_id, &setting_key).await?;

    // Issue a JWT so the new user is logged in immediately.
    let jwt =
        mockforge_registry_core::auth::create_access_token(&user.id.to_string(), &state.jwt_secret)
            .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("jwt: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "user": {
                "id": user.id,
                "username": user.username,
                "email": user.email,
                "is_verified": user.is_verified,
            },
            "org_id": stored.org_id,
            "role": stored.role,
            "token": jwt,
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

    /// Build a test router with a non-empty JWT secret so tokens are
    /// meaningfully signed. The seed fixtures above use the default
    /// empty-secret CoreAppState::new path.
    async fn test_router_with_jwt() -> Router {
        let store = init_sqlite_registry_store("sqlite::memory:").await.unwrap();
        let state = CoreAppState::with_jwt_secret(
            Arc::new(store),
            "test-secret-please-do-not-use-in-prod".to_string(),
        );
        router(state)
    }

    #[tokio::test]
    async fn test_register_then_login_roundtrip() {
        let router = test_router_with_jwt().await;

        // Register
        let resp = router
            .clone()
            .oneshot(json_post(
                "/api/admin/registry/auth/register",
                json!({
                    "username": "newbie",
                    "email": "newbie@example.com",
                    "password": "correcthorsebatterystaple"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = body_json(resp).await;
        assert!(body["token"].as_str().unwrap().len() > 20);
        let register_token = body["token"].as_str().unwrap().to_string();
        let user_id = body["user"]["id"].as_str().unwrap().to_string();

        // Login with username
        let resp = router
            .clone()
            .oneshot(json_post(
                "/api/admin/registry/auth/login",
                json!({
                    "identifier": "newbie",
                    "password": "correcthorsebatterystaple"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let login_token = body["token"].as_str().unwrap().to_string();
        assert_eq!(body["user"]["id"], user_id);

        // Both tokens should work against /auth/me
        for (label, tok) in [("register", &register_token), ("login", &login_token)] {
            let resp = router
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/api/admin/registry/auth/me")
                        .header("authorization", format!("Bearer {}", tok))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK, "{} token failed /auth/me", label);
            let body = body_json(resp).await;
            assert_eq!(body["id"], user_id);
            assert_eq!(body["username"], "newbie");
        }
    }

    #[tokio::test]
    async fn test_login_with_email_identifier() {
        let router = test_router_with_jwt().await;
        router
            .clone()
            .oneshot(json_post(
                "/api/admin/registry/auth/register",
                json!({
                    "username": "bob",
                    "email": "bob@example.com",
                    "password": "hunter2hunter2"
                }),
            ))
            .await
            .unwrap();

        // Login by email instead of username
        let resp = router
            .oneshot(json_post(
                "/api/admin/registry/auth/login",
                json!({
                    "identifier": "bob@example.com",
                    "password": "hunter2hunter2"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_login_wrong_password_returns_401() {
        let router = test_router_with_jwt().await;
        router
            .clone()
            .oneshot(json_post(
                "/api/admin/registry/auth/register",
                json!({
                    "username": "carol",
                    "email": "carol@example.com",
                    "password": "rightpassword"
                }),
            ))
            .await
            .unwrap();

        let resp = router
            .oneshot(json_post(
                "/api/admin/registry/auth/login",
                json!({
                    "identifier": "carol",
                    "password": "wrongpassword"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_login_unknown_user_also_401() {
        let router = test_router_with_jwt().await;
        let resp = router
            .oneshot(json_post(
                "/api/admin/registry/auth/login",
                json!({
                    "identifier": "nobody",
                    "password": "whatever"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_register_duplicate_email_is_409() {
        let router = test_router_with_jwt().await;
        router
            .clone()
            .oneshot(json_post(
                "/api/admin/registry/auth/register",
                json!({
                    "username": "first",
                    "email": "dup@example.com",
                    "password": "password1"
                }),
            ))
            .await
            .unwrap();

        let resp = router
            .oneshot(json_post(
                "/api/admin/registry/auth/register",
                json!({
                    "username": "second",
                    "email": "dup@example.com",
                    "password": "password2"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_register_rejects_short_password() {
        let router = test_router_with_jwt().await;
        let resp = router
            .oneshot(json_post(
                "/api/admin/registry/auth/register",
                json!({
                    "username": "x",
                    "email": "x@example.com",
                    "password": "short"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_auth_me_rejects_missing_header() {
        let router = test_router_with_jwt().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/admin/registry/auth/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_me_rejects_bogus_token() {
        let router = test_router_with_jwt().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/admin/registry/auth/me")
                    .header("authorization", "Bearer not-a-real-jwt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_list_org_members_endpoint() {
        let (router, _, org_id) = test_router_with_seed().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .uri(format!("/api/admin/registry/orgs/{}/members", org_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        // Fresh org with just an owner has no members yet.
        assert!(body["members"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_add_update_remove_org_member_lifecycle() {
        let (router, _, org_id) = test_router_with_seed().await;

        // Create a new user via the write endpoint first.
        let resp = router
            .clone()
            .oneshot(json_post(
                "/api/admin/registry/users",
                json!({
                    "username": "team-member",
                    "email": "member@example.com",
                    "password_hash": "bcrypt-hash-placeholder-long-enough"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let member_id = body_json(resp).await["id"].as_str().unwrap().to_string();

        // Add as member
        let resp = router
            .clone()
            .oneshot(json_post(
                &format!("/api/admin/registry/orgs/{}/members", org_id),
                json!({ "user_id": member_id, "role": "member" }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        assert_eq!(body_json(resp).await["role"], "member");

        // List shows the new member
        let resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/admin/registry/orgs/{}/members", org_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = body_json(resp).await;
        assert_eq!(body["members"].as_array().unwrap().len(), 1);

        // Update role to admin
        let resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/admin/registry/orgs/{}/members/{}", org_id, member_id))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"role": "admin"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_json(resp).await["role"], "admin");

        // Delete
        let resp = router
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/admin/registry/orgs/{}/members/{}", org_id, member_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_org_quota_get_set() {
        let (router, _, org_id) = test_router_with_seed().await;

        // Initially empty
        let resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/admin/registry/orgs/{}/quota", org_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["quota"], json!({}));

        // PUT a quota
        let resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/admin/registry/orgs/{}/quota", org_id))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "max_tokens": 10, "max_mocks": 100 }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // GET reflects it
        let resp = router
            .oneshot(
                Request::builder()
                    .uri(format!("/api/admin/registry/orgs/{}/quota", org_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = body_json(resp).await;
        assert_eq!(body["quota"]["max_tokens"], 10);
        assert_eq!(body["quota"]["max_mocks"], 100);
    }

    #[tokio::test]
    async fn test_org_quota_rejects_non_object() {
        let (router, _, org_id) = test_router_with_seed().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/admin/registry/orgs/{}/quota", org_id))
                    .header("content-type", "application/json")
                    .body(Body::from(json!([1, 2, 3]).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_invitation_full_lifecycle() {
        let router = test_router_with_jwt().await;

        // Bootstrap: register owner, create org.
        let resp = router
            .clone()
            .oneshot(json_post(
                "/api/admin/registry/auth/register",
                json!({
                    "username": "owner",
                    "email": "owner@example.com",
                    "password": "ownerpass1"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let owner_id = body_json(resp).await["user"]["id"].as_str().unwrap().to_string();

        let resp = router
            .clone()
            .oneshot(json_post(
                "/api/admin/registry/orgs",
                json!({
                    "name": "Invite Corp",
                    "slug": "invite-corp",
                    "owner_id": owner_id,
                    "plan": "free"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let org_id = body_json(resp).await["id"].as_str().unwrap().to_string();

        // Create an invitation for a new email.
        let resp = router
            .clone()
            .oneshot(json_post(
                &format!("/api/admin/registry/orgs/{}/invitations", org_id),
                json!({ "email": "invitee@example.com", "role": "admin" }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = body_json(resp).await;
        let token = body["token"].as_str().unwrap().to_string();
        assert_eq!(body["email"], "invitee@example.com");
        assert_eq!(body["role"], "admin");

        // GET the invitation (simulates the invitee clicking a link)
        let resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/admin/registry/invitations/{}", urlencoding::encode(&token)))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["email"], "invitee@example.com");
        assert_eq!(body["role"], "admin");

        // Accept — creates user, membership, and returns a JWT.
        let resp = router
            .clone()
            .oneshot(json_post(
                &format!("/api/admin/registry/invitations/{}/accept", urlencoding::encode(&token)),
                json!({
                    "username": "invitee",
                    "password": "inviteepassword"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = body_json(resp).await;
        assert_eq!(body["user"]["username"], "invitee");
        assert_eq!(body["user"]["is_verified"], true);
        assert_eq!(body["org_id"], org_id);
        assert_eq!(body["role"], "admin");
        assert!(body["token"].as_str().unwrap().len() > 20);

        // Second accept of the same token must fail — it's consumed.
        let resp = router
            .clone()
            .oneshot(json_post(
                &format!("/api/admin/registry/invitations/{}/accept", urlencoding::encode(&token)),
                json!({
                    "username": "invitee2",
                    "password": "anotherpassword"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_invitation_rejects_garbage_token() {
        let router = test_router_with_jwt().await;
        let resp = router
            .oneshot(
                Request::builder()
                    .uri("/api/admin/registry/invitations/not-a-real-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_bootstrap_admin_user_no_env_vars() {
        // Without the env vars set, bootstrap is a no-op.
        let store = init_sqlite_registry_store("sqlite::memory:").await.unwrap();
        // Explicitly clear in case the test runner has them set.
        std::env::remove_var("MOCKFORGE_ADMIN_USERNAME");
        std::env::remove_var("MOCKFORGE_ADMIN_EMAIL");
        std::env::remove_var("MOCKFORGE_ADMIN_PASSWORD");
        let result = bootstrap_admin_user_from_env(&store).await.unwrap();
        assert!(!result);
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
