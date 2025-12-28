//! Authentication handlers

use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{
        create_token_pair, hash_password, verify_password, verify_refresh_token,
        REFRESH_TOKEN_EXPIRY_DAYS,
    },
    error::{ApiError, ApiResult},
    models::User,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub two_factor_code: Option<String>, // Optional 2FA code (required if 2FA is enabled)
}

/// Legacy auth response (for backwards compatibility)
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: String,
    pub username: String,
}

/// New auth response with both access and refresh tokens
#[derive(Debug, Serialize)]
pub struct AuthResponseV2 {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: i64,
    pub refresh_token_expires_at: i64,
    pub user_id: String,
    pub username: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<Json<AuthResponseV2>> {
    let pool = state.db.pool();

    // Validate input
    if request.username.len() < 3 {
        return Err(ApiError::InvalidRequest("Username must be at least 3 characters".to_string()));
    }

    if request.password.len() < 8 {
        return Err(ApiError::InvalidRequest("Password must be at least 8 characters".to_string()));
    }

    // Check if user already exists
    if User::find_by_email(pool, &request.email)
        .await
        .map_err(|e| ApiError::Database(e))?
        .is_some()
    {
        return Err(ApiError::InvalidRequest("Email already registered".to_string()));
    }

    if User::find_by_username(pool, &request.username)
        .await
        .map_err(|e| ApiError::Database(e))?
        .is_some()
    {
        return Err(ApiError::InvalidRequest("Username already taken".to_string()));
    }

    // Hash password
    let password_hash = hash_password(&request.password).map_err(|e| ApiError::Internal(e))?;

    // Create user
    let user = User::create(pool, &request.username, &request.email, &password_hash)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Generate token pair (access + refresh)
    let (token_pair, jti) = create_token_pair(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| ApiError::Internal(e))?;

    // Store refresh token JTI in database for revocation support
    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(REFRESH_TOKEN_EXPIRY_DAYS))
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Failed to calculate token expiry")))?;

    state.db.store_refresh_token_jti(&jti, user.id, expires_at).await.map_err(|e| {
        tracing::warn!("Failed to store refresh token JTI: {}", e);
        ApiError::Internal(e)
    })?;

    Ok(Json(AuthResponseV2 {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        access_token_expires_at: token_pair.access_token_expires_at,
        refresh_token_expires_at: token_pair.refresh_token_expires_at,
        user_id: user.id.to_string(),
        username: user.username,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<AuthResponseV2>> {
    let pool = state.db.pool();

    // Find user
    let user = User::find_by_email(pool, &request.email)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Invalid email or password".to_string()))?;

    // Verify password
    let valid = verify_password(&request.password, &user.password_hash)
        .map_err(|e| ApiError::Internal(e))?;

    if !valid {
        return Err(ApiError::InvalidRequest("Invalid email or password".to_string()));
    }

    // Check if 2FA is enabled
    if user.two_factor_enabled {
        // Require 2FA code
        let code = request
            .two_factor_code
            .ok_or_else(|| ApiError::InvalidRequest("2FA code is required".to_string()))?;

        // Get secret
        let secret = user.two_factor_secret.ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!("2FA enabled but no secret found"))
        })?;

        // Verify TOTP code
        use crate::two_factor::verify_totp_code;
        let totp_valid = verify_totp_code(&secret, &code, Some(1))
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("TOTP verification error: {}", e)))?;

        if !totp_valid {
            // Try backup codes
            let mut backup_valid = false;
            if let Some(backup_codes) = &user.two_factor_backup_codes {
                use crate::two_factor::verify_backup_code;
                for (index, hashed_code) in backup_codes.iter().enumerate() {
                    if verify_backup_code(&code, hashed_code).map_err(|e| {
                        ApiError::Internal(anyhow::anyhow!("Backup code verification error: {}", e))
                    })? {
                        // Remove used backup code
                        User::remove_backup_code(pool, user.id, index)
                            .await
                            .map_err(|e| ApiError::Database(e))?;
                        backup_valid = true;
                        break;
                    }
                }
            }

            if !backup_valid {
                return Err(ApiError::InvalidRequest("Invalid 2FA code".to_string()));
            }
        }

        // Update 2FA verified timestamp
        User::update_2fa_verified(pool, user.id)
            .await
            .map_err(|e| ApiError::Database(e))?;
    }

    // Generate token pair (access + refresh)
    let (token_pair, jti) = create_token_pair(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| ApiError::Internal(e))?;

    // Store refresh token JTI in database for revocation support
    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(REFRESH_TOKEN_EXPIRY_DAYS))
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Failed to calculate token expiry")))?;

    state.db.store_refresh_token_jti(&jti, user.id, expires_at).await.map_err(|e| {
        tracing::warn!("Failed to store refresh token JTI: {}", e);
        ApiError::Internal(e)
    })?;

    Ok(Json(AuthResponseV2 {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        access_token_expires_at: token_pair.access_token_expires_at,
        refresh_token_expires_at: token_pair.refresh_token_expires_at,
        user_id: user.id.to_string(),
        username: user.username,
    }))
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Response for refresh token endpoint
#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: i64,
    pub refresh_token_expires_at: i64,
}

pub async fn refresh_token(
    State(state): State<AppState>,
    Json(request): Json<RefreshTokenRequest>,
) -> ApiResult<Json<RefreshTokenResponse>> {
    // Verify the refresh token (not just any token)
    let (claims, old_jti) = verify_refresh_token(&request.refresh_token, &state.config.jwt_secret)
        .map_err(|e| {
            tracing::debug!("Refresh token validation failed: {}", e);
            ApiError::InvalidRequest("Invalid or expired refresh token".to_string())
        })?;

    // Check if the JTI has been revoked in the database
    let is_revoked = state.db.is_token_revoked(&old_jti).await.map_err(|e| {
        tracing::warn!("Failed to check token revocation status: {}", e);
        ApiError::Internal(e)
    })?;

    if is_revoked {
        tracing::warn!("Attempt to use revoked refresh token: jti={}", old_jti);
        return Err(ApiError::InvalidRequest("Refresh token has been revoked".to_string()));
    }

    let pool = state.db.pool();

    // Parse user ID from claims
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiError::InvalidRequest("Invalid user ID".to_string()))?;

    // Find user to ensure they still exist and are active
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Revoke old refresh token JTI (token rotation for security)
    state.db.revoke_token(&old_jti, "refresh").await.map_err(|e| {
        tracing::warn!("Failed to revoke old refresh token: {}", e);
        ApiError::Internal(e)
    })?;

    // Generate new token pair
    let (token_pair, new_jti) = create_token_pair(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| ApiError::Internal(e))?;

    // Store new refresh token JTI in database
    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(REFRESH_TOKEN_EXPIRY_DAYS))
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Failed to calculate token expiry")))?;

    state
        .db
        .store_refresh_token_jti(&new_jti, user.id, expires_at)
        .await
        .map_err(|e| {
            tracing::warn!("Failed to store new refresh token JTI: {}", e);
            ApiError::Internal(e)
        })?;

    Ok(Json(RefreshTokenResponse {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        access_token_expires_at: token_pair.access_token_expires_at,
        refresh_token_expires_at: token_pair.refresh_token_expires_at,
    }))
}

// Password reset handlers (moved here to avoid axum version conflicts)
use crate::email::EmailService;
use crate::models::VerificationToken;

#[derive(Debug, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct PasswordResetRequestResponse {
    pub success: bool,
    pub message: String,
}

/// Request password reset (sends email with reset token)
pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(request): Json<PasswordResetRequest>,
) -> ApiResult<Json<PasswordResetRequestResponse>> {
    let pool = state.db.pool();

    // Find user by email
    let user = match User::find_by_email(pool, &request.email).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            // Don't reveal if email exists or not (security best practice)
            // Return success even if user doesn't exist
            return Ok(Json(PasswordResetRequestResponse {
                success: true,
                message:
                    "If an account with that email exists, a password reset link has been sent."
                        .to_string(),
            }));
        }
        Err(e) => return Err(ApiError::Database(e)),
    };

    // Create password reset token (reusing VerificationToken model)
    // Token expires in 1 hour
    let reset_token = VerificationToken::create(pool, user.id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Update token expiration to 1 hour (instead of default 24 hours)
    sqlx::query(
        "UPDATE verification_tokens SET expires_at = NOW() + INTERVAL '1 hour' WHERE id = $1",
    )
    .bind(reset_token.id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Send password reset email (non-blocking)
    let email_service = match EmailService::from_env() {
        Ok(service) => service,
        Err(e) => {
            tracing::warn!("Failed to create email service: {}", e);
            return Ok(Json(PasswordResetRequestResponse {
                success: true,
                message:
                    "If an account with that email exists, a password reset link has been sent."
                        .to_string(),
            }));
        }
    };
    let reset_email = EmailService::generate_password_reset_email(
        &user.username,
        &user.email,
        &reset_token.token,
    );

    tokio::spawn(async move {
        if let Err(e) = email_service.send(reset_email).await {
            tracing::warn!("Failed to send password reset email: {}", e);
        }
    });

    tracing::info!("Password reset requested: user_id={}, email={}", user.id, user.email);

    Ok(Json(PasswordResetRequestResponse {
        success: true,
        message: "If an account with that email exists, a password reset link has been sent."
            .to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetConfirmRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct PasswordResetConfirmResponse {
    pub success: bool,
    pub message: String,
}

/// Confirm password reset (with token and new password)
pub async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(request): Json<PasswordResetConfirmRequest>,
) -> ApiResult<Json<PasswordResetConfirmResponse>> {
    let pool = state.db.pool();

    // Validate password
    if request.new_password.len() < 8 {
        return Err(ApiError::InvalidRequest("Password must be at least 8 characters".to_string()));
    }

    // Find token
    let reset_token = VerificationToken::find_by_token(pool, &request.token)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Invalid or expired reset token".to_string()))?;

    // Check if token is valid (not expired and not used)
    if !reset_token.is_valid() {
        return Err(ApiError::InvalidRequest(
            "Reset token has expired or already been used".to_string(),
        ));
    }

    // Get user
    let user = User::find_by_id(pool, reset_token.user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Hash new password
    let password_hash = hash_password(&request.new_password).map_err(|e| ApiError::Internal(e))?;

    // Update user password
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
        .bind(&password_hash)
        .bind(user.id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Revoke all existing refresh tokens for security (password changed)
    let revoked_count =
        state.db.revoke_all_user_tokens(user.id, "password_reset").await.map_err(|e| {
            tracing::warn!("Failed to revoke user tokens on password reset: {}", e);
            ApiError::Internal(e)
        })?;

    tracing::info!(
        "Revoked {} refresh tokens for user {} on password reset",
        revoked_count,
        user.id
    );

    // Mark token as used
    VerificationToken::mark_as_used(pool, reset_token.id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    tracing::info!("Password reset completed: user_id={}, email={}", user.id, user.email);

    Ok(Json(PasswordResetConfirmResponse {
        success: true,
        message: "Password has been reset successfully. You can now log in with your new password."
            .to_string(),
    }))
}
