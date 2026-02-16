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

fn should_seed_default_users() -> bool {
    is_development_environment()
        && is_dev_auth_enabled()
        && !is_truthy_env("MOCKFORGE_DISABLE_DEV_SEED_USERS")
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
        let dev_secret = std::env::var("MOCKFORGE_DEV_JWT_SECRET")
            .unwrap_or_else(|_| "mockforge-dev-only-secret-change-me-12345".to_string());
        tracing::warn!(
            "Using development JWT secret fallback. Set JWT_SECRET for production-like testing."
        );
        return Ok(dev_secret.into_bytes());
    }

    tracing::error!(
        "JWT_SECRET is required in production. Set JWT_SECRET with at least {} characters.",
        MIN_JWT_SECRET_LEN
    );
    Err(jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken))
}

pub fn validate_auth_config_on_startup() -> Result<(), String> {
    if !is_development_environment() && !is_truthy_env("MOCKFORGE_ALLOW_INMEMORY_AUTH") {
        return Err(
            "In-memory auth is disabled in production. Configure production auth backend or set MOCKFORGE_ALLOW_INMEMORY_AUTH=true explicitly."
                .to_string(),
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
    password_policy: PasswordPolicy,
}

#[derive(Debug, Clone)]
struct User {
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

        if should_seed_default_users() {
            // Development-only seeded users for local testing
            let default_users = vec![
                ("admin", "admin123", UserRole::Admin, "admin@mockforge.dev"),
                ("viewer", "viewer123", UserRole::Viewer, "viewer@mockforge.dev"),
                ("editor", "editor123", UserRole::Editor, "editor@mockforge.dev"),
            ];

            let store_clone = store.clone();
            tokio::spawn(async move {
                let mut users = store_clone.users.write().await;
                for (username, password, role, email) in default_users {
                    if let Ok(password_hash) = hash(password, DEFAULT_COST) {
                        let user = User {
                            id: format!("{}-001", username),
                            username: username.to_string(),
                            password_hash,
                            role,
                            email: Some(email.to_string()),
                        };
                        users.insert(username.to_string(), user);
                    } else {
                        tracing::error!("Failed to hash password for user: {}", username);
                    }
                }
            });
        }

        store
    }

    pub async fn authenticate(&self, username: &str, password: &str) -> Result<User, String> {
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
    pub async fn create_user(
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

    pub async fn get_user_by_id(&self, user_id: &str) -> Option<User> {
        let users = self.users.read().await;
        users.values().find(|u| u.id == user_id).cloned()
    }
}

/// Global user store instance
static GLOBAL_USER_STORE: std::sync::OnceLock<Arc<UserStore>> = std::sync::OnceLock::new();

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
pub fn generate_token(
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
pub fn generate_refresh_token(user: &User) -> Result<String, jsonwebtoken::errors::Error> {
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

/// Logout endpoint (client-side token removal, but can invalidate refresh tokens)
pub async fn logout(State(_state): State<AdminState>) -> Json<ApiResponse<String>> {
    // In production, invalidate refresh token in database
    // For now, just return success (client removes token)
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

    #[tokio::test]
    async fn test_user_store_creation() {
        let store = UserStore::new();
        // Wait a bit for async initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Verify default users exist
        let result = store.authenticate("admin", "admin123").await;
        assert!(result.is_ok(), "Admin user should exist");
    }

    #[tokio::test]
    async fn test_user_store_default() {
        let store = UserStore::default();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let result = store.authenticate("admin", "admin123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_authenticate_success() {
        let store = UserStore::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let result = store.authenticate("admin", "admin123").await;
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.username, "admin");
        assert!(matches!(user.role, UserRole::Admin));
    }

    #[tokio::test]
    async fn test_authenticate_wrong_password() {
        let store = UserStore::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let result = store.authenticate("admin", "wrongpassword").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid username or password");
    }

    #[tokio::test]
    async fn test_authenticate_nonexistent_user() {
        let store = UserStore::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let result = store.authenticate("nonexistent", "password").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid username or password");
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let store = UserStore::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

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
        let store = UserStore::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Try to login with wrong password 5 times
        for _ in 0..5 {
            let _ = store.authenticate("editor", "wrongpassword").await;
        }

        // Account should now be locked
        let result = store.authenticate("editor", "editor123").await;
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
        let store = UserStore::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Try wrong password a few times
        for _ in 0..2 {
            let _ = store.authenticate("viewer", "wrongpassword").await;
        }

        // Successful login should reset counter
        let result = store.authenticate("viewer", "viewer123").await;
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
        let store = UserStore::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

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
        let store = UserStore::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let result = store
            .create_user("admin".to_string(), "NewP@ssw0rd123".to_string(), UserRole::Editor, None)
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Username already exists");
    }

    #[tokio::test]
    async fn test_get_user_by_id() {
        let store = UserStore::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Authenticate to get user ID
        let auth_result = store.authenticate("admin", "admin123").await.unwrap();
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
        let store = UserStore::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

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
    async fn test_all_default_users_exist() {
        let store = UserStore::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Test admin
        let result = store.authenticate("admin", "admin123").await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap().role, UserRole::Admin));

        // Test viewer
        let result = store.authenticate("viewer", "viewer123").await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap().role, UserRole::Viewer));

        // Test editor
        let result = store.authenticate("editor", "editor123").await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap().role, UserRole::Editor));
    }

    #[tokio::test]
    async fn test_concurrent_authentication_attempts() {
        let store = Arc::new(UserStore::new());
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut handles = vec![];

        // Spawn multiple concurrent authentication attempts
        for i in 0..10 {
            let store_clone = store.clone();
            let handle = tokio::spawn(async move {
                if i % 2 == 0 {
                    store_clone.authenticate("admin", "admin123").await
                } else {
                    store_clone.authenticate("viewer", "viewer123").await
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
