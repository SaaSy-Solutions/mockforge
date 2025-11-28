//! Password reset handlers

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    auth::hash_password,
    email::EmailService,
    error::{ApiError, ApiResult},
    models::{User, VerificationToken},
    AppState,
};

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
#[axum::debug_handler]
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
#[axum::debug_handler]
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
