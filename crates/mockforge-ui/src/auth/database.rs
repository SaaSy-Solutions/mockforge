//! Database-backed user store implementation
//!
//! This module provides a database-backed user store that replaces
//! the in-memory implementation for production use.

use crate::auth::password_policy::{PasswordPolicy, PasswordValidationError};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Utc};
use mockforge_collab::models::UserRole;
#[cfg(feature = "database-auth")]
use sqlx::{AnyPool, Row};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// User struct for database-backed store
#[derive(Debug, Clone)]
pub struct DatabaseUser {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub role: UserRole,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
}

/// Rate limiting for login attempts (in-memory for performance)
#[derive(Debug, Clone)]
pub struct RateLimiter {
    attempts: Arc<RwLock<std::collections::HashMap<String, Vec<std::time::Instant>>>>,
    max_attempts: usize,
    window_seconds: u64,
}

impl RateLimiter {
    pub fn new(max_attempts: usize, window_seconds: u64) -> Self {
        Self {
            attempts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            max_attempts,
            window_seconds,
        }
    }

    pub async fn check_rate_limit(&self, username: &str) -> Result<(), String> {
        let mut attempts = self.attempts.write().await;
        let now = std::time::Instant::now();
        let window = std::time::Duration::from_secs(self.window_seconds);

        // Clean up old attempts
        let user_attempts = attempts.entry(username.to_string()).or_insert_with(Vec::new);
        user_attempts.retain(|&time| now.duration_since(time) < window);

        // Check if limit exceeded
        if user_attempts.len() >= self.max_attempts {
            return Err(format!(
                "Too many login attempts. Please try again in {} seconds.",
                self.window_seconds
            ));
        }

        // Record this attempt
        user_attempts.push(now);
        Ok(())
    }

    pub async fn reset_rate_limit(&self, username: &str) {
        let mut attempts = self.attempts.write().await;
        attempts.remove(username);
    }
}

/// Account lockout configuration
#[derive(Debug, Clone)]
pub struct AccountLockout {
    max_attempts: usize,
    lockout_duration_seconds: u64,
}

impl AccountLockout {
    pub fn new(max_attempts: usize, lockout_duration_seconds: u64) -> Self {
        Self {
            max_attempts,
            lockout_duration_seconds,
        }
    }
}

/// Database-backed user store
///
/// This implementation uses SQLite or PostgreSQL for persistent user storage.
#[derive(Clone)]
pub struct DatabaseUserStore {
    #[cfg(feature = "database-auth")]
    db: AnyPool,
    rate_limiter: RateLimiter,
    account_lockout: AccountLockout,
    password_policy: PasswordPolicy,
}

impl DatabaseUserStore {
    /// Create a new database-backed user store
    ///
    /// # Arguments
    /// * `database_url` - Database connection string (SQLite or PostgreSQL)
    ///
    /// # Example
    /// ```rust,no_run
    /// let store = DatabaseUserStore::new("sqlite://mockforge.db").await?;
    /// ```
    #[cfg(feature = "database-auth")]
    pub async fn new(database_url: &str) -> Result<Self, String> {
        // Connect to database using AnyPool (supports both SQLite and PostgreSQL)
        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(|e| format!("Failed to connect to database: {}", e))?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| format!("Failed to run migrations: {}", e))?;

        Ok(Self {
            db: pool,
            rate_limiter: RateLimiter::new(5, 300), // 5 attempts per 5 minutes
            account_lockout: AccountLockout::new(5, 900), // 5 attempts, 15 minute lockout
            password_policy: PasswordPolicy::default(),
        })
    }

    /// Authenticate a user against the database
    #[cfg(feature = "database-auth")]
    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<DatabaseUser, String> {
        // Check rate limiting
        self.rate_limiter.check_rate_limit(username).await?;

        // Fetch user from database
        let row = sqlx::query(
            r#"
            SELECT id, username, password_hash, role, email, created_at, updated_at,
                   last_login_at, failed_login_attempts, locked_until
            FROM admin_users
            WHERE username = $1 AND (locked_until IS NULL OR locked_until < CURRENT_TIMESTAMP)
            "#
        )
        .bind(username)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        let row = row.ok_or_else(|| "Invalid username or password".to_string())?;

        let mut user_row = DatabaseUserRow {
            id: row.try_get("id").map_err(|e| format!("Failed to read id: {}", e))?,
            username: row.try_get("username").map_err(|e| format!("Failed to read username: {}", e))?,
            password_hash: row.try_get("password_hash").map_err(|e| format!("Failed to read password_hash: {}", e))?,
            role: row.try_get("role").map_err(|e| format!("Failed to read role: {}", e))?,
            email: row.try_get("email").ok(),
            created_at: row.try_get::<chrono::NaiveDateTime, _>("created_at")
                .map_err(|e| format!("Failed to read created_at: {}", e))?
                .and_utc(),
            updated_at: row.try_get::<chrono::NaiveDateTime, _>("updated_at")
                .map_err(|e| format!("Failed to read updated_at: {}", e))?
                .and_utc(),
            last_login_at: row.try_get::<chrono::NaiveDateTime, _>("last_login_at")
                .ok()
                .map(|dt| dt.and_utc()),
            failed_login_attempts: row.try_get("failed_login_attempts").unwrap_or(0),
            locked_until: row.try_get::<chrono::NaiveDateTime, _>("locked_until")
                .ok()
                .map(|dt| dt.and_utc()),
        };

        // Check if account is locked
        if let Some(locked_until) = user_row.locked_until {
            if locked_until > Utc::now() {
                let remaining = (locked_until - Utc::now()).num_seconds();
                return Err(format!(
                    "Account is locked due to too many failed login attempts. Please try again in {} seconds.",
                    remaining
                ));
            }
        }

        // Verify password
        let password_valid = verify(password, &user_row.password_hash)
            .map_err(|e| format!("Password verification error: {}", e))?;

        // Record login attempt
        let attempt_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO login_attempts (id, username, ip_address, user_agent, success, created_at)
            VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
            "#
        )
        .bind(&attempt_id)
        .bind(username)
        .bind(ip_address)
        .bind(user_agent)
        .bind(if password_valid { 1 } else { 0 })
        .execute(&self.db)
        .await
        .ok(); // Don't fail on logging error

        if !password_valid {
            // Increment failed login attempts
            user_row.failed_login_attempts += 1;

            // Lock account if too many failures
            if user_row.failed_login_attempts >= self.account_lockout.max_attempts as i32 {
                let locked_until = Utc::now() + chrono::Duration::seconds(self.account_lockout.lockout_duration_seconds as i64);
                sqlx::query(
                    r#"
                    UPDATE admin_users
                    SET failed_login_attempts = $1, locked_until = $2, updated_at = CURRENT_TIMESTAMP
                    WHERE id = $3
                    "#
                )
                .bind(user_row.failed_login_attempts)
                .bind(locked_until.naive_utc())
                .bind(&user_row.id)
                .execute(&self.db)
                .await
                .ok();
            } else {
                sqlx::query(
                    r#"
                    UPDATE admin_users
                    SET failed_login_attempts = $1, updated_at = CURRENT_TIMESTAMP
                    WHERE id = $2
                    "#
                )
                .bind(user_row.failed_login_attempts)
                .bind(&user_row.id)
                .execute(&self.db)
                .await
                .ok();
            }

            return Err("Invalid username or password".to_string());
        }

        // Successful login - reset failed attempts and update last login
        sqlx::query(
            r#"
            UPDATE admin_users
            SET failed_login_attempts = 0, locked_until = NULL, last_login_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            "#
        )
        .bind(&user_row.id)
        .execute(&self.db)
        .await
        .map_err(|e| format!("Failed to update user: {}", e))?;

        // Reset rate limit
        self.rate_limiter.reset_rate_limit(username).await;

        // Convert to DatabaseUser
        Ok(DatabaseUser {
            id: user_row.id,
            username: user_row.username,
            password_hash: user_row.password_hash,
            role: parse_role(&user_row.role)?,
            email: user_row.email,
            created_at: user_row.created_at,
            updated_at: user_row.updated_at,
            last_login_at: user_row.last_login_at,
            failed_login_attempts: 0,
            locked_until: None,
        })
    }

    /// Get user by ID from database
    #[cfg(feature = "database-auth")]
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<DatabaseUser>, String> {
        let row = sqlx::query(
            r#"
            SELECT id, username, password_hash, role, email, created_at, updated_at,
                   last_login_at, failed_login_attempts, locked_until
            FROM admin_users
            WHERE id = $1
            "#
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        Ok(row.map(|row| {
            DatabaseUser {
                id: row.try_get("id").unwrap_or_default(),
                username: row.try_get("username").unwrap_or_default(),
                password_hash: row.try_get("password_hash").unwrap_or_default(),
                role: parse_role(&row.try_get::<String, _>("role").unwrap_or_default()).unwrap_or(UserRole::Viewer),
                email: row.try_get("email").ok(),
                created_at: row.try_get::<chrono::NaiveDateTime, _>("created_at")
                    .unwrap_or_default()
                    .and_utc(),
                updated_at: row.try_get::<chrono::NaiveDateTime, _>("updated_at")
                    .unwrap_or_default()
                    .and_utc(),
                last_login_at: row.try_get::<chrono::NaiveDateTime, _>("last_login_at")
                    .ok()
                    .map(|dt| dt.and_utc()),
                failed_login_attempts: row.try_get("failed_login_attempts").unwrap_or(0),
                locked_until: row.try_get::<chrono::NaiveDateTime, _>("locked_until")
                    .ok()
                    .map(|dt| dt.and_utc()),
            }
        }))
    }

    /// Create a new user in the database
    #[cfg(feature = "database-auth")]
    pub async fn create_user(
        &self,
        username: String,
        password: String,
        role: UserRole,
        email: Option<String>,
    ) -> Result<DatabaseUser, String> {
        // Validate password against policy
        self.password_policy.validate(&password, Some(&username))
            .map_err(|e| e.to_string())?;

        // Check if user already exists
        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM admin_users WHERE username = $1"#
        )
        .bind(&username)
        .fetch_one(&self.db)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        if count > 0 {
            return Err("Username already exists".to_string());
        }

        // Hash password
        let password_hash = hash(&password, DEFAULT_COST)
            .map_err(|e| format!("Failed to hash password: {}", e))?;

        // Create user
        let user_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let role_str = format!("{:?}", role).to_lowercase();
        let now_naive = now.naive_utc();

        sqlx::query(
            r#"
            INSERT INTO admin_users (id, username, password_hash, role, email, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#
        )
        .bind(&user_id)
        .bind(&username)
        .bind(&password_hash)
        .bind(&role_str)
        .bind(&email)
        .bind(now_naive)
        .bind(now_naive)
        .execute(&self.db)
        .await
        .map_err(|e| format!("Failed to create user: {}", e))?;

        Ok(DatabaseUser {
            id: user_id,
            username,
            password_hash,
            role,
            email,
            created_at: now,
            updated_at: now,
            last_login_at: None,
            failed_login_attempts: 0,
            locked_until: None,
        })
    }

    /// Store a refresh token in the database
    #[cfg(feature = "database-auth")]
    pub async fn store_refresh_token(
        &self,
        user_id: &str,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), String> {
        let token_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at)
            VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)
            "#
        )
        .bind(&token_id)
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at.naive_utc())
        .execute(&self.db)
        .await
        .map_err(|e| format!("Failed to store refresh token: {}", e))?;

        Ok(())
    }

    /// Validate and revoke a refresh token
    #[cfg(feature = "database-auth")]
    pub async fn validate_refresh_token(&self, token_hash: &str) -> Result<String, String> {
        let row = sqlx::query(
            r#"
            SELECT user_id, expires_at, revoked_at
            FROM refresh_tokens
            WHERE token_hash = $1 AND revoked_at IS NULL
            "#
        )
        .bind(token_hash)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        let row = row.ok_or_else(|| "Invalid refresh token".to_string())?;

        // Check if token is expired
        let expires_at: DateTime<Utc> = row.try_get::<chrono::NaiveDateTime, _>("expires_at")
            .map_err(|_| "Invalid token expiration date".to_string())?
            .and_utc();

        if expires_at < Utc::now() {
            return Err("Refresh token has expired".to_string());
        }

        let user_id: String = row.try_get("user_id")
            .map_err(|_| "Invalid user_id".to_string())?;

        Ok(user_id)
    }

    /// Revoke a refresh token
    #[cfg(feature = "database-auth")]
    pub async fn revoke_refresh_token(&self, token_hash: &str) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked_at = CURRENT_TIMESTAMP
            WHERE token_hash = $1 AND revoked_at IS NULL
            "#
        )
        .bind(token_hash)
        .execute(&self.db)
        .await
        .map_err(|e| format!("Failed to revoke refresh token: {}", e))?;

        Ok(())
    }

    /// Revoke all refresh tokens for a user
    #[cfg(feature = "database-auth")]
    pub async fn revoke_all_refresh_tokens(&self, user_id: &str) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked_at = CURRENT_TIMESTAMP
            WHERE user_id = $1 AND revoked_at IS NULL
            "#
        )
        .bind(user_id)
        .execute(&self.db)
        .await
        .map_err(|e| format!("Failed to revoke refresh tokens: {}", e))?;

        Ok(())
    }
}

/// Database row representation
#[derive(Debug)]
struct DatabaseUserRow {
    id: String,
    username: String,
    password_hash: String,
    role: String,
    email: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_login_at: Option<DateTime<Utc>>,
    failed_login_attempts: i32,
    locked_until: Option<DateTime<Utc>>,
}

/// Parse role string to UserRole enum
fn parse_role(role_str: &str) -> Result<UserRole, String> {
    match role_str.to_lowercase().as_str() {
        "admin" => Ok(UserRole::Admin),
        "editor" => Ok(UserRole::Editor),
        "viewer" => Ok(UserRole::Viewer),
        _ => Err(format!("Invalid role: {}", role_str)),
    }
}
