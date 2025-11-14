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

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::handlers::AdminState;
use crate::models::ApiResponse;
use crate::rbac::UserContext;
use mockforge_collab::models::UserRole;

mod password_policy;
pub use password_policy::{PasswordPolicy, PasswordValidationError};

/// JWT secret key (in production, load from environment variable)
static JWT_SECRET: &[u8] = b"mockforge-secret-key-change-in-production";

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
        attempts
            .entry(key.to_string())
            .or_insert_with(Vec::new)
            .push(now);

        Ok(())
    }

    async fn reset_rate_limit(&self, key: &str) {
        let mut attempts = self.attempts.write().await;
        attempts.remove(key);
    }
}

impl UserStore {
    pub fn new() -> Self {
        let users = Arc::new(RwLock::new(HashMap::new()));
        let rate_limiter = RateLimiter::new(5, 300); // 5 attempts per 5 minutes
        let account_lockout = AccountLockout::new(5, 900); // 5 attempts, 15 minute lockout
        let password_policy = PasswordPolicy::default(); // Use default policy

        // Initialize with default users (in production, load from database)
        // Passwords are hashed with bcrypt
        let default_users = vec![
            // admin / admin123
            ("admin", "admin123", UserRole::Admin, "admin@mockforge.dev"),
            // viewer / viewer123
            ("viewer", "viewer123", UserRole::Viewer, "viewer@mockforge.dev"),
            // editor / editor123
            ("editor", "editor123", UserRole::Editor, "editor@mockforge.dev"),
        ];

        let store = Self {
            users,
            rate_limiter,
            account_lockout,
            password_policy,
        };

        // Hash passwords and create users asynchronously
        let store_clone = store.clone();
        tokio::spawn(async move {
            let mut users = store_clone.users.write().await;
            for (username, password, role, email) in default_users {
                // Hash password with bcrypt
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
            self.password_policy.validate(&password, Some(&username))
                .map_err(|e| e.to_string())?;
        }

        // Check if user already exists
        let mut users = self.users.write().await;
        if users.contains_key(&username) {
            return Err("Username already exists".to_string());
        }

        // Hash password
        let password_hash = hash(&password, DEFAULT_COST)
            .map_err(|e| format!("Failed to hash password: {}", e))?;

        // Create user
        let user = User {
            id: format!("{}-{}", username, uuid::Uuid::new_v4().to_string()),
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
    GLOBAL_USER_STORE.get_or_init(|| Arc::new(UserStore::new())).clone()
}

/// Get the global user store
pub fn get_global_user_store() -> Option<Arc<UserStore>> {
    GLOBAL_USER_STORE.get().cloned()
}

/// Generate JWT token
pub fn generate_token(user: &User, expires_in_seconds: i64) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let exp = now + Duration::seconds(expires_in_seconds);
    let secret = JWT_SECRET;

    let claims = Claims {
        sub: user.id.clone(),
        username: user.username.clone(),
        role: format!("{:?}", user.role).to_lowercase(),
        email: user.email.clone(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&secret),
    )
}

/// Generate refresh token
pub fn generate_refresh_token(user: &User) -> Result<String, jsonwebtoken::errors::Error> {
    // Refresh tokens expire in 7 days
    generate_token(user, 7 * 24 * 60 * 60)
}

/// Validate JWT token
pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = JWT_SECRET;
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(&secret),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

/// Login endpoint
pub async fn login(
    State(_state): State<AdminState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, StatusCode> {
    let user_store = get_global_user_store()
        .ok_or_else(|| {
            tracing::error!("User store not initialized");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Authenticate user
    let user = user_store.authenticate(&request.username, &request.password)
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

    let refresh_token = generate_refresh_token(&user)
        .map_err(|e| {
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
    let claims = validate_token(&request.refresh_token)
        .map_err(|_| {
            tracing::warn!("Invalid refresh token");
            StatusCode::UNAUTHORIZED
        })?;

    let user_store = get_global_user_store()
        .ok_or_else(|| {
            tracing::error!("User store not initialized");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get user
    let user = user_store.get_user_by_id(&claims.sub)
        .await
        .ok_or_else(|| {
            tracing::warn!("User not found: {}", claims.sub);
            StatusCode::UNAUTHORIZED
        })?;

    // Generate new tokens
    let access_token = generate_token(&user, 24 * 60 * 60) // 24 hours
        .map_err(|e| {
            tracing::error!("Failed to generate access token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let refresh_token = generate_refresh_token(&user)
        .map_err(|e| {
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
    State(_state): State<AdminState>,
) -> Result<Json<ApiResponse<UserInfo>>, StatusCode> {
    // This would extract user from request extensions (set by middleware)
    // For now, return error - should be called after authentication
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Logout endpoint (client-side token removal, but can invalidate refresh tokens)
pub async fn logout(
    State(_state): State<AdminState>,
) -> Json<ApiResponse<String>> {
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
