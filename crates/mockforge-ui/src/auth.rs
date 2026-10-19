//! Authentication and JWT token management
//!
//! This module provides authentication endpoints and JWT token generation/validation
//! for the Admin UI.
//!
//! # Features
//! - JWT token generation and validation
//! - Password hashing with bcrypt
//! - Rate limiting for login attempts
//! - In-memory user store (can be replaced with database)
//!
//! # Database Integration
//! See `auth/database.rs` for database-backed user store implementation.

use axum::{extract::State, http::StatusCode, response::Json};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};
use tokio::sync::RwLock;

use crate::handlers::AdminState;
use crate::models::ApiResponse;
use crate::rbac::UserContext;
use mockforge_collab::models::UserRole;

mod initial_users;
mod password_policy;
pub use password_policy::{PasswordPolicy, PasswordValidationError};

const MIN_JWT_SECRET_LEN: usize = 32;

fn is_truthy_env(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref().map(str::to_ascii_lowercase).as_deref(),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}

fn is_development_environment() -> bool {
    if cfg!(test) {
        return true;
    }

    matches!(
        std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "production".to_string())
            .to_ascii_lowercase()
            .as_str(),
        "development" | "dev" | "local"
    )
}

fn is_dev_auth_enabled() -> bool {
    cfg!(test) || is_truthy_env("MOCKFORGE_ENABLE_DEV_AUTH")
}

fn get_jwt_secret_bytes() -> Result<Vec<u8>, jsonwebtoken::errors::Error> {
    if cfg!(test) {
        return Ok(b"test-jwt-secret-which-is-long-enough".to_vec());
    }

    if let Ok(secret) = std::env::var("JWT_SECRET") {
        if secret.len() < MIN_JWT_SECRET_LEN {
            tracing::error!(
                "JWT_SECRET is too short ({} chars). Minimum required is {} chars.",
                secret.len(),
                MIN_JWT_SECRET_LEN
            );
            return Err(jsonwebtoken::errors::Error::from(
                jsonwebtoken::errors::ErrorKind::InvalidToken,
            ));
        }
        return Ok(secret.into_bytes());
    }

    if is_development_environment() && is_dev_auth_enabled() {
        // No constant fallback: a secret in the source lets anyone forge an
        // admin token (#1016).
        if let Ok(dev_secret) = std::env::var("MOCKFORGE_DEV_JWT_SECRET") {
            tracing::warn!(
                "Using MOCKFORGE_DEV_JWT_SECRET. Set JWT_SECRET for production-like testing."
            );
            return Ok(dev_secret.into_bytes());
        }
    }

    // Fallback: a random per-process secret so `--admin` works out of the
    // box. It used to be a constant in this file, which let anyone forge an
    // admin token (#1019). Tokens now stop working when the server restarts.
    Ok(process_jwt_secret().to_vec())
}

/// A random secret generated once per process
fn process_jwt_secret() -> &'static [u8; 32] {
    static SECRET: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
    SECRET.get_or_init(|| {
        use rand::RngCore;
        let mut secret = [0u8; 32];
        rand::rng().fill_bytes(&mut secret);
        tracing::info!(
            "Using a per-process random JWT secret for the admin UI. Set JWT_SECRET to \
             keep sessions across restarts."
        );
        secret
    })
}

pub fn validate_auth_config_on_startup() -> Result<(), String> {
    if !is_development_environment() && !is_truthy_env("MOCKFORGE_ALLOW_INMEMORY_AUTH") {
        // Auto-allow in-memory auth for mock server usage, but warn
        tracing::info!(
            "Admin UI using in-memory auth (default for mock servers). \
             Set MOCKFORGE_ALLOW_INMEMORY_AUTH=true to suppress, or configure a \
             real auth backend for production use."
        );
    }

    get_jwt_secret_bytes()
        .map(|_| ())
        .map_err(|_| "JWT_SECRET is missing or invalid for current environment".to_string())
}

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// User role
    pub role: String,
    /// Email (optional)
    pub email: Option<String>,
    /// Issued at
    pub iat: i64,
    /// Expiration time
    pub exp: i64,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: UserInfo,
    pub expires_in: i64,
}

/// User information
#[derive(Debug, Serialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: String,
    pub email: Option<String>,
}

/// Refresh token request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// In-memory user store (in production, use database)
#[derive(Debug, Clone)]
pub struct UserStore {
    users: Arc<RwLock<HashMap<String, User>>>,
    rate_limiter: RateLimiter,
    account_lockout: AccountLockout,
    #[allow(dead_code)]
    password_policy: PasswordPolicy,
}

#[derive(Debug, Clone)]
pub(crate) struct User {
    id: String,
    username: String,
    password_hash: String, // Bcrypt hashed password
    role: UserRole,
    email: Option<String>,
}

/// Rate limiting for login attempts
#[derive(Debug, Clone)]
struct RateLimiter {
    attempts: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    max_attempts: usize,
    window_seconds: u64,
}

/// Account lockout configuration
#[derive(Debug, Clone)]
struct AccountLockout {
    /// Failed login attempts per account
    #[allow(clippy::type_complexity)]
    failed_attempts: Arc<RwLock<HashMap<String, (usize, Option<Instant>)>>>,
    /// Maximum failed attempts before lockout
    max_failed_attempts: usize,
    /// Lockout duration in seconds
    lockout_duration_seconds: u64,
}

impl AccountLockout {
    fn new(max_failed_attempts: usize, lockout_duration_seconds: u64) -> Self {
        Self {
            failed_attempts: Arc::new(RwLock::new(HashMap::new())),
            max_failed_attempts,
            lockout_duration_seconds,
        }
    }

    /// Check if account is locked
    async fn is_locked(&self, username: &str) -> bool {
        let attempts = self.failed_attempts.read().await;
        if let Some((count, locked_until)) = attempts.get(username) {
            if *count >= self.max_failed_attempts {
                if let Some(until) = locked_until {
                    return until > &Instant::now();
                }
            }
        }
        false
    }

    /// Record a failed login attempt
    async fn record_failure(&self, username: &str) {
        let mut attempts = self.failed_attempts.write().await;
        let entry = attempts.entry(username.to_string()).or_insert((0, None));
        entry.0 += 1;

        if entry.0 >= self.max_failed_attempts {
            entry.1 = Some(Instant::now() + StdDuration::from_secs(self.lockout_duration_seconds));
            tracing::warn!("Account locked: {} ({} failed attempts)", username, entry.0);
        }
    }

    /// Reset failed attempts on successful login
    async fn reset(&self, username: &str) {
        let mut attempts = self.failed_attempts.write().await;
        attempts.remove(username);
    }

    /// Get remaining lockout time in seconds
    #[allow(clippy::collapsible_match)]
    async fn remaining_lockout_time(&self, username: &str) -> Option<u64> {
        let attempts = self.failed_attempts.read().await;
        if let Some((_, locked_until)) = attempts.get(username) {
            if let Some(until) = locked_until {
                let now = Instant::now();
                if until > &now {
                    return Some(until.duration_since(now).as_secs());
                }
            }
        }
        None
    }
}

impl RateLimiter {
    fn new(max_attempts: usize, window_seconds: u64) -> Self {
        Self {
            attempts: Arc::new(RwLock::new(HashMap::new())),
            max_attempts,
            window_seconds,
        }
    }

    async fn check_rate_limit(&self, key: &str) -> Result<(), String> {
        let mut attempts = self.attempts.write().await;
        let now = Instant::now();
        let window = StdDuration::from_secs(self.window_seconds);

        // Clean up old attempts
        if let Some(attempt_times) = attempts.get_mut(key) {
            attempt_times.retain(|&time| now.duration_since(time) < window);

            if attempt_times.len() >= self.max_attempts {
                return Err(format!(
                    "Too many login attempts. Please try again in {} seconds.",
                    self.window_seconds
                ));
            }
        }

        // Record this attempt
        attempts.entry(key.to_string()).or_insert_with(Vec::new).push(now);

        Ok(())
    }

    async fn reset_rate_limit(&self, key: &str) {
        let mut attempts = self.attempts.write().await;
        attempts.remove(key);
    }
}

impl Default for UserStore {
    fn default() -> Self {
        Self::new()
    }
}

impl UserStore {
    pub fn new() -> Self {
        let users = Arc::new(RwLock::new(HashMap::new()));
        let rate_limiter = RateLimiter::new(5, 300); // 5 attempts per 5 minutes
        let account_lockout = AccountLockout::new(5, 900); // 5 attempts, 15 minute lockout
        let password_policy = PasswordPolicy::default(); // Use default policy

        let store = Self {
            users,
            rate_limiter,
            account_lockout,
            password_policy,
        };

        initial_users::initialize(&store.users);

        store
    }

    pub(crate) async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<User, String> {
        // Check if account is locked
        if self.account_lockout.is_locked(username).await {
            if let Some(remaining) = self.account_lockout.remaining_lockout_time(username).await {
                return Err(format!(
                    "Account is locked due to too many failed login attempts. Please try again in {} seconds.",
                    remaining
                ));
            }
        }

        // Check rate limiting
        self.rate_limiter.check_rate_limit(username).await?;

        let users = self.users.read().await;
        if let Some(user) = users.get(username) {
            // Verify password with bcrypt
            match verify(password, &user.password_hash) {
                Ok(true) => {
                    // Successful login - reset rate limit and lockout
                    self.rate_limiter.reset_rate_limit(username).await;
                    self.account_lockout.reset(username).await;
                    Ok(user.clone())
                }
                Ok(false) => {
                    // Wrong password - record failure
                    self.account_lockout.record_failure(username).await;
                    Err("Invalid username or password".to_string())
                }
                Err(e) => {
                    tracing::error!("Password verification error: {}", e);
                    Err("Authentication error".to_string())
                }
            }
        } else {
            // User not found - still count as failed attempt (but don't lock non-existent accounts)
            Err("Invalid username or password".to_string())
        }
    }

    /// Create a new user with password policy validation
    #[allow(dead_code)]
    pub(crate) async fn create_user(
        &self,
        username: String,
        password: String,
        role: UserRole,
        email: Option<String>,
    ) -> Result<User, String> {
        // Validate password against policy
        #[cfg(feature = "password-policy")]
        {
            self.password_policy
                .validate(&password, Some(&username))
                .map_err(|e| e.to_string())?;
        }

        // Check if user already exists
        let mut users = self.users.write().await;
        if users.contains_key(&username) {
            return Err("Username already exists".to_string());
        }

        // Hash password
        let password_hash =
            hash(&password, DEFAULT_COST).map_err(|e| format!("Failed to hash password: {}", e))?;

        // Create user
        let user = User {
            id: format!("{}-{}", username, uuid::Uuid::new_v4()),
            username: username.clone(),
            password_hash,
            role,
            email,
        };

        users.insert(username, user.clone());
        Ok(user)
    }

    pub(crate) async fn get_user_by_id(&self, user_id: &str) -> Option<User> {
        let users = self.users.read().await;
        users.values().find(|u| u.id == user_id).cloned()
    }
}

/// Global user store instance
static GLOBAL_USER_STORE: std::sync::OnceLock<Arc<UserStore>> = std::sync::OnceLock::new();

/// Global set of revoked JWT tokens (jti claim or raw token string)
/// Tokens are stored with their expiry time so we can prune expired entries.
static REVOKED_TOKENS: std::sync::OnceLock<Arc<RwLock<HashMap<String, i64>>>> =
    std::sync::OnceLock::new();

/// Get or initialize the revoked tokens store
fn get_revoked_tokens() -> Arc<RwLock<HashMap<String, i64>>> {
    REVOKED_TOKENS.get_or_init(|| Arc::new(RwLock::new(HashMap::new()))).clone()
}

/// Revoke a token so it can no longer be used
pub async fn revoke_token(token: &str) {
    // Decode without validation to get expiry (we want to revoke even expired tokens)
    let exp = validate_token(token)
        .map(|c| c.exp)
        .unwrap_or_else(|_| Utc::now().timestamp() + 7 * 24 * 60 * 60);
    let store = get_revoked_tokens();
    let mut revoked = store.write().await;
    revoked.insert(token.to_string(), exp);
    // Prune expired entries to prevent unbounded growth
    let now = Utc::now().timestamp();
    revoked.retain(|_, &mut exp_time| exp_time > now);
}

/// Check if a token has been revoked
pub async fn is_token_revoked(token: &str) -> bool {
    let store = get_revoked_tokens();
    let revoked = store.read().await;
    revoked.contains_key(token)
}

/// Initialize the global user store
pub fn init_global_user_store() -> Arc<UserStore> {
    if let Err(e) = validate_auth_config_on_startup() {
        panic!("Authentication startup validation failed: {}", e);
    }
    GLOBAL_USER_STORE.get_or_init(|| Arc::new(UserStore::new())).clone()
}

/// Get the global user store
pub fn get_global_user_store() -> Option<Arc<UserStore>> {
    GLOBAL_USER_STORE.get().cloned()
}

/// Generate JWT token
pub(crate) fn generate_token(
    user: &User,
    expires_in_seconds: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let exp = now + Duration::seconds(expires_in_seconds);
    let secret = get_jwt_secret_bytes()?;

    let claims = Claims {
        sub: user.id.clone(),
        username: user.username.clone(),
        role: format!("{:?}", user.role).to_lowercase(),
        email: user.email.clone(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
    };

    encode(&Header::default(), &claims, &EncodingKey::from_secret(&secret))
}

/// Generate refresh token
pub(crate) fn generate_refresh_token(user: &User) -> Result<String, jsonwebtoken::errors::Error> {
    // Refresh tokens expire in 7 days
    generate_token(user, 7 * 24 * 60 * 60)
}

/// Validate JWT token
pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = get_jwt_secret_bytes()?;
    let token_data =
        decode::<Claims>(token, &DecodingKey::from_secret(&secret), &Validation::default())?;

    Ok(token_data.claims)
}

/// Login endpoint
pub async fn login(
    State(_state): State<AdminState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, StatusCode> {
    let user_store = get_global_user_store().ok_or_else(|| {
        tracing::error!("User store not initialized");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Authenticate user
    let user =
        user_store
            .authenticate(&request.username, &request.password)
            .await
            .map_err(|e| {
                tracing::warn!("Authentication failed for user {}: {}", request.username, e);
                // Return appropriate status code based on error
                if e.contains("Too many") {
                    StatusCode::TOO_MANY_REQUESTS
                } else {
                    StatusCode::UNAUTHORIZED
                }
            })?;

    // Generate tokens
    let access_token = generate_token(&user, 24 * 60 * 60) // 24 hours
        .map_err(|e| {
            tracing::error!("Failed to generate access token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let refresh_token = generate_refresh_token(&user).map_err(|e| {
        tracing::error!("Failed to generate refresh token: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let user_info = UserInfo {
        id: user.id,
        username: user.username,
        role: format!("{:?}", user.role).to_lowercase(),
        email: user.email,
    };

    Ok(Json(ApiResponse::success(LoginResponse {
        token: access_token,
        refresh_token,
        user: user_info,
        expires_in: 24 * 60 * 60,
    })))
}

/// Refresh token endpoint
pub async fn refresh_token(
    State(_state): State<AdminState>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, StatusCode> {
    // Check if the refresh token has been revoked
    if is_token_revoked(&request.refresh_token).await {
        tracing::warn!("Attempt to use revoked refresh token");
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Validate refresh token
    let claims = validate_token(&request.refresh_token).map_err(|_| {
        tracing::warn!("Invalid refresh token");
        StatusCode::UNAUTHORIZED
    })?;

    let user_store = get_global_user_store().ok_or_else(|| {
        tracing::error!("User store not initialized");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get user
    let user = user_store.get_user_by_id(&claims.sub).await.ok_or_else(|| {
        tracing::warn!("User not found: {}", claims.sub);
        StatusCode::UNAUTHORIZED
    })?;

    // Generate new tokens
    let access_token = generate_token(&user, 24 * 60 * 60) // 24 hours
        .map_err(|e| {
            tracing::error!("Failed to generate access token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let refresh_token = generate_refresh_token(&user).map_err(|e| {
        tracing::error!("Failed to generate refresh token: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let user_info = UserInfo {
        id: user.id,
        username: user.username,
        role: format!("{:?}", user.role).to_lowercase(),
        email: user.email,
    };

    Ok(Json(ApiResponse::success(LoginResponse {
        token: access_token,
        refresh_token,
        user: user_info,
        expires_in: 24 * 60 * 60,
    })))
}

/// Get current user endpoint
pub async fn get_current_user(
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<UserInfo>>, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header.strip_prefix("Bearer ").ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = validate_token(token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let role = claims_to_user_context(&claims).role;

    Ok(Json(ApiResponse::success(UserInfo {
        id: claims.sub,
        username: claims.username,
        role: format!("{:?}", role).to_lowercase(),
        email: claims.email,
    })))
}

/// Logout endpoint — revokes the bearer token server-side
pub async fn logout(
    headers: axum::http::HeaderMap,
    State(_state): State<AdminState>,
) -> Json<ApiResponse<String>> {
    // Extract and revoke the access token from the Authorization header
    if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            revoke_token(token).await;
            tracing::info!("Token revoked on logout");
        }
    }
    Json(ApiResponse::success("Logged out successfully".to_string()))
}

/// Convert Claims to UserContext
pub fn claims_to_user_context(claims: &Claims) -> UserContext {
    let role = match claims.role.as_str() {
        "admin" => UserRole::Admin,
        "editor" => UserRole::Editor,
        "viewer" => UserRole::Viewer,
        _ => UserRole::Viewer,
    };

    UserContext {
        user_id: claims.sub.clone(),
        username: claims.username.clone(),
        role,
        email: claims.email.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo logins that used to be built in, and must never return
    const PUBLISHED_DEMO_LOGINS: [(&str, &str); 3] = [
        ("admin", "admin123"),
        ("viewer", "viewer123"),
        ("editor", "editor123"),
    ];
    const ADMIN_PW: &str = "Fixture-P4ss-One!";
    const EDITOR_PW: &str = "Fixture-P4ss-Two!";
    const VIEWER_PW: &str = "Fixture-P4ss-Six!";

    /// A store with one user per role, created the way an operator would
    async fn fixture_store() -> UserStore {
        let store = UserStore::new();
        for (username, password, role) in [
            ("admin", ADMIN_PW, UserRole::Admin),
            ("editor", EDITOR_PW, UserRole::Editor),
            ("viewer", VIEWER_PW, UserRole::Viewer),
        ] {
            store
                .create_user(username.to_string(), password.to_string(), role, None)
                .await
                .expect("fixture user is new and meets the password policy");
        }
        store
    }

    #[test]
    fn process_jwt_secret_is_random_and_stable() {
        // The fallback used to be a constant in this file (#1019).
        let secret = process_jwt_secret();
        assert_ne!(&secret[..], b"mockforge-auto-jwt-secret-for-admin-ui-1234".as_slice());
        assert_ne!(secret, &[0u8; 32]);
        assert_eq!(secret, process_jwt_secret());
    }

    #[tokio::test]
    async fn new_store_has_no_built_in_users() {
        // The test process sets no MOCKFORGE_ADMIN_PASSWORD, so nothing is
        // seeded and no hashing thread is started (#1016).
        for store in [UserStore::new(), UserStore::default()] {
            assert!(store.users.read().await.is_empty());
            for (username, password) in PUBLISHED_DEMO_LOGINS {
                let result = store.authenticate(username, password).await;
                assert_eq!(result.unwrap_err(), "Invalid username or password");
            }
        }
    }

    #[test]
    fn source_contains_no_demo_passwords() {
        // Guard against the demo users coming back in a later edit.
        let source = include_str!("auth.rs");
        let initial_users = include_str!("auth/initial_users.rs");
        let production = format!(
            "{}\n{}",
            &source[..source.find("\nmod tests {").expect("tests module")],
            &initial_users[..initial_users.find("\nmod tests {").expect("startup policy tests")]
        );
        for (_, password) in PUBLISHED_DEMO_LOGINS {
            assert!(!production.contains(&format!("\"{password}\"")), "{password}");
        }
        assert!(!production.contains("mockforge-dev-only-secret-change-me-12345"));
    }

    #[tokio::test]
    async fn test_authenticate_success() {
        let store = fixture_store().await;

        let result = store.authenticate("admin", ADMIN_PW).await;
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.username, "admin");
        assert!(matches!(user.role, UserRole::Admin));
    }

    #[tokio::test]
    async fn test_authenticate_wrong_password() {
        let store = fixture_store().await;

        let result = store.authenticate("admin", "wrongpassword").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid username or password");
    }

    #[tokio::test]
    async fn test_authenticate_nonexistent_user() {
        let store = fixture_store().await;

        let result = store.authenticate("nonexistent", "password").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid username or password");
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let store = fixture_store().await;

        // Try to login with wrong password many times
        for _ in 0..5 {
            let _ = store.authenticate("admin", "wrongpassword").await;
        }

        // Next attempt should be rate limited or account locked
        let result = store.authenticate("admin", "wrongpassword").await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error.contains("Too many") || error.contains("locked"),
            "Expected rate limit or lockout error, got: {}",
            error
        );
    }

    #[tokio::test]
    async fn test_account_lockout_after_failures() {
        let store = fixture_store().await;

        // Try to login with wrong password 5 times
        for _ in 0..5 {
            let _ = store.authenticate("editor", "wrongpassword").await;
        }

        // Account should now be locked
        let result = store.authenticate("editor", EDITOR_PW).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error.contains("locked") || error.contains("Too many"),
            "Expected lockout error, got: {}",
            error
        );
    }

    #[tokio::test]
    async fn test_account_lockout_reset_on_success() {
        let store = fixture_store().await;

        // Try wrong password a few times
        for _ in 0..2 {
            let _ = store.authenticate("viewer", "wrongpassword").await;
        }

        // Successful login should reset counter
        let result = store.authenticate("viewer", VIEWER_PW).await;
        assert!(result.is_ok());

        // Should be able to attempt again without hitting lockout
        for _ in 0..2 {
            let _ = store.authenticate("viewer", "wrongpassword").await;
        }

        // Not yet locked (reset worked)
        let result = store.authenticate("viewer", "wrongpassword").await;
        assert!(result.is_err());
        assert!(!result.unwrap_err().contains("locked"));
    }

    #[tokio::test]
    async fn test_create_user_success() {
        let store = fixture_store().await;

        let result = store
            .create_user(
                "newuser".to_string(),
                "NewP@ssw0rd123".to_string(),
                UserRole::Editor,
                Some("newuser@example.com".to_string()),
            )
            .await;

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.username, "newuser");
        assert!(matches!(user.role, UserRole::Editor));
        assert_eq!(user.email, Some("newuser@example.com".to_string()));

        // Verify we can authenticate with the new user
        let auth_result = store.authenticate("newuser", "NewP@ssw0rd123").await;
        assert!(auth_result.is_ok());
    }

    #[tokio::test]
    async fn test_create_user_duplicate_username() {
        let store = fixture_store().await;

        let result = store
            .create_user("admin".to_string(), "NewP@ssw0rd123".to_string(), UserRole::Editor, None)
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Username already exists");
    }

    #[tokio::test]
    async fn test_get_user_by_id() {
        let store = fixture_store().await;

        // Authenticate to get user ID
        let auth_result = store.authenticate("admin", ADMIN_PW).await.unwrap();
        let user_id = auth_result.id.clone();

        // Get user by ID
        let result = store.get_user_by_id(&user_id).await;
        assert!(result.is_some());

        let user = result.unwrap();
        assert_eq!(user.id, user_id);
        assert_eq!(user.username, "admin");
    }

    #[tokio::test]
    async fn test_get_user_by_id_not_found() {
        let store = fixture_store().await;

        let result = store.get_user_by_id("nonexistent-id").await;
        assert!(result.is_none());
    }

    #[test]
    fn test_generate_token_success() {
        let user = User {
            id: "test-id".to_string(),
            username: "testuser".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Editor,
            email: Some("test@example.com".to_string()),
        };

        let result = generate_token(&user, 3600);
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_generate_refresh_token() {
        let user = User {
            id: "test-id".to_string(),
            username: "testuser".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Editor,
            email: None,
        };

        let result = generate_refresh_token(&user);
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_validate_token_success() {
        let user = User {
            id: "test-id".to_string(),
            username: "testuser".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Viewer,
            email: Some("test@example.com".to_string()),
        };

        let token = generate_token(&user, 3600).unwrap();
        let result = validate_token(&token);

        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.sub, "test-id");
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.role, "viewer");
        assert_eq!(claims.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_validate_token_invalid() {
        let result = validate_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_token_expired() {
        let user = User {
            id: "test-id".to_string(),
            username: "testuser".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Editor,
            email: None,
        };

        // Generate token that's already expired (2 minutes ago to exceed default leeway of 60s)
        let token = generate_token(&user, -120).unwrap();
        let result = validate_token(&token);

        // Should fail validation due to expiration
        assert!(result.is_err());
    }

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: "user123".to_string(),
            username: "testuser".to_string(),
            role: "admin".to_string(),
            email: Some("test@example.com".to_string()),
            iat: 1234567890,
            exp: 1234567890 + 3600,
        };

        let serialized = serde_json::to_string(&claims).unwrap();
        let deserialized: Claims = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.sub, claims.sub);
        assert_eq!(deserialized.username, claims.username);
        assert_eq!(deserialized.role, claims.role);
        assert_eq!(deserialized.email, claims.email);
    }

    #[test]
    fn test_claims_to_user_context() {
        let claims = Claims {
            sub: "user123".to_string(),
            username: "testuser".to_string(),
            role: "editor".to_string(),
            email: Some("test@example.com".to_string()),
            iat: 1234567890,
            exp: 1234567890 + 3600,
        };

        let context = claims_to_user_context(&claims);
        assert_eq!(context.user_id, "user123");
        assert_eq!(context.username, "testuser");
        assert_eq!(context.role, UserRole::Editor);
        assert_eq!(context.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_claims_to_user_context_unknown_role_defaults_to_viewer() {
        let claims = Claims {
            sub: "user123".to_string(),
            username: "testuser".to_string(),
            role: "unknown".to_string(),
            email: None,
            iat: 1234567890,
            exp: 1234567890 + 3600,
        };

        let context = claims_to_user_context(&claims);
        assert_eq!(context.role, UserRole::Viewer);
    }

    #[test]
    fn test_login_request_deserialization() {
        let json = r#"{"username": "testuser", "password": "testpass"}"#;
        let request: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.username, "testuser");
        assert_eq!(request.password, "testpass");
    }

    #[test]
    fn test_refresh_token_request_deserialization() {
        let json = r#"{"refresh_token": "token123"}"#;
        let request: RefreshTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.refresh_token, "token123");
    }

    #[test]
    fn test_user_info_serialization() {
        let user_info = UserInfo {
            id: "user123".to_string(),
            username: "testuser".to_string(),
            role: "admin".to_string(),
            email: Some("test@example.com".to_string()),
        };

        let serialized = serde_json::to_string(&user_info).unwrap();
        assert!(serialized.contains("user123"));
        assert!(serialized.contains("testuser"));
        assert!(serialized.contains("admin"));
    }

    #[test]
    fn test_login_response_serialization() {
        let user_info = UserInfo {
            id: "user123".to_string(),
            username: "testuser".to_string(),
            role: "editor".to_string(),
            email: None,
        };

        let response = LoginResponse {
            token: "access.token.here".to_string(),
            refresh_token: "refresh.token.here".to_string(),
            user: user_info,
            expires_in: 3600,
        };

        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains("access.token.here"));
        assert!(serialized.contains("refresh.token.here"));
        assert!(serialized.contains("3600"));
    }

    #[tokio::test]
    async fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(5, 60);
        let result = limiter.check_rate_limit("test-key").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_exceeds_limit() {
        let limiter = RateLimiter::new(3, 60);

        // First 3 attempts should succeed
        for _ in 0..3 {
            assert!(limiter.check_rate_limit("test-key").await.is_ok());
        }

        // 4th attempt should fail
        let result = limiter.check_rate_limit("test-key").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Too many"));
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(3, 60);

        // Use up the limit
        for _ in 0..3 {
            limiter.check_rate_limit("test-key").await.ok();
        }

        // Reset
        limiter.reset_rate_limit("test-key").await;

        // Should be able to make requests again
        let result = limiter.check_rate_limit("test-key").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_different_keys() {
        let limiter = RateLimiter::new(2, 60);

        // Use up limit for key1
        for _ in 0..2 {
            limiter.check_rate_limit("key1").await.ok();
        }

        // key2 should still work
        let result = limiter.check_rate_limit("key2").await;
        assert!(result.is_ok());

        // key1 should be limited
        let result = limiter.check_rate_limit("key1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_account_lockout_creation() {
        let lockout = AccountLockout::new(3, 900);
        let is_locked = lockout.is_locked("test-user").await;
        assert!(!is_locked);
    }

    #[tokio::test]
    async fn test_account_lockout_record_failure() {
        let lockout = AccountLockout::new(3, 900);

        for _ in 0..2 {
            lockout.record_failure("test-user").await;
        }

        let is_locked = lockout.is_locked("test-user").await;
        assert!(!is_locked, "Should not be locked after 2 failures");

        lockout.record_failure("test-user").await;
        let is_locked = lockout.is_locked("test-user").await;
        assert!(is_locked, "Should be locked after 3 failures");
    }

    #[tokio::test]
    async fn test_account_lockout_reset() {
        let lockout = AccountLockout::new(2, 900);

        // Lock the account
        for _ in 0..2 {
            lockout.record_failure("test-user").await;
        }

        assert!(lockout.is_locked("test-user").await);

        // Reset
        lockout.reset("test-user").await;

        assert!(!lockout.is_locked("test-user").await);
    }

    #[tokio::test]
    async fn test_account_lockout_remaining_time() {
        let lockout = AccountLockout::new(2, 5); // 5 second lockout

        // Lock the account
        for _ in 0..2 {
            lockout.record_failure("test-user").await;
        }

        let remaining = lockout.remaining_lockout_time("test-user").await;
        assert!(remaining.is_some());
        let time = remaining.unwrap();
        assert!(time > 0 && time <= 5);
    }

    #[tokio::test]
    async fn test_global_user_store_initialization() {
        let store1 = init_global_user_store();
        let store2 = get_global_user_store();

        assert!(store2.is_some());

        // Both should be the same instance
        let store2 = store2.unwrap();
        assert!(Arc::ptr_eq(&store1, &store2));
    }

    #[tokio::test]
    async fn test_all_fixture_users_exist() {
        let store = fixture_store().await;

        let result = store.authenticate("admin", ADMIN_PW).await;
        assert!(matches!(result.unwrap().role, UserRole::Admin));
        let result = store.authenticate("viewer", VIEWER_PW).await;
        assert!(matches!(result.unwrap().role, UserRole::Viewer));
        let result = store.authenticate("editor", EDITOR_PW).await;
        assert!(matches!(result.unwrap().role, UserRole::Editor));
    }

    #[tokio::test]
    async fn test_concurrent_authentication_attempts() {
        let store = Arc::new(fixture_store().await);

        let mut handles = vec![];

        // Spawn multiple concurrent authentication attempts
        for i in 0..10 {
            let store_clone = store.clone();
            let handle = tokio::spawn(async move {
                if i % 2 == 0 {
                    store_clone.authenticate("admin", ADMIN_PW).await
                } else {
                    store_clone.authenticate("viewer", VIEWER_PW).await
                }
            });
            handles.push(handle);
        }

        // All should succeed
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }
}
