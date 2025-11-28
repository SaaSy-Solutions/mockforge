//! Authentication handlers

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{create_token, hash_password, verify_password},
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

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: String,
    pub username: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<Json<AuthResponse>> {
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

    // Generate JWT token
    let token = create_token(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| ApiError::Internal(e))?;

    Ok(Json(AuthResponse {
        token,
        user_id: user.id.to_string(),
        username: user.username,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<AuthResponse>> {
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
        let code = request.two_factor_code
            .ok_or_else(|| ApiError::InvalidRequest("2FA code is required".to_string()))?;

        // Get secret
        let secret = user.two_factor_secret
            .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("2FA enabled but no secret found")))?;

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
                    if verify_backup_code(&code, hashed_code)
                        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Backup code verification error: {}", e)))?
                    {
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

    // Generate JWT token
    let token = create_token(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| ApiError::Internal(e))?;

    Ok(Json(AuthResponse {
        token,
        user_id: user.id.to_string(),
        username: user.username,
    }))
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub token: String,
}

pub async fn refresh_token(
    State(state): State<AppState>,
    Json(request): Json<RefreshTokenRequest>,
) -> ApiResult<Json<AuthResponse>> {
    use crate::auth::verify_token;

    // Verify the existing token
    let claims = verify_token(&request.token, &state.config.jwt_secret)
        .map_err(|_| ApiError::InvalidRequest("Invalid token".to_string()))?;

    let pool = state.db.pool();

    // Parse user ID from claims
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiError::InvalidRequest("Invalid user ID".to_string()))?;

    // Find user
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Generate new JWT token
    let token = create_token(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| ApiError::Internal(e))?;

    Ok(Json(AuthResponse {
        token,
        user_id: user.id.to_string(),
        username: user.username,
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
                message: "If an account with that email exists, a password reset link has been sent.".to_string(),
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
    sqlx::query("UPDATE verification_tokens SET expires_at = NOW() + INTERVAL '1 hour' WHERE id = $1")
        .bind(reset_token.id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Send password reset email (non-blocking)
    let email_service = EmailService::from_env();
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
        message: "If an account with that email exists, a password reset link has been sent.".to_string(),
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
        return Err(ApiError::InvalidRequest("Reset token has expired or already been used".to_string()));
    }

    // Get user
    let user = User::find_by_id(pool, reset_token.user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Hash new password
    let password_hash = hash_password(&request.new_password)
        .map_err(|e| ApiError::Internal(e))?;

    // Update user password
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
        .bind(&password_hash)
        .bind(user.id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Mark token as used
    VerificationToken::mark_as_used(pool, reset_token.id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    tracing::info!("Password reset completed: user_id={}, email={}", user.id, user.email);

    Ok(Json(PasswordResetConfirmResponse {
        success: true,
        message: "Password has been reset successfully. You can now log in with your new password.".to_string(),
    }))
}
