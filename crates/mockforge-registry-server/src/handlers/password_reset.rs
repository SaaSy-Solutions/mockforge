//! Password reset handlers

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::hash_password,
    email::EmailService,
    error::{ApiError, ApiResult},
    models::AuditEventType,
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
    // Find user by email
    let user = match state.store.find_user_by_email(&request.email).await? {
        Some(user) => user,
        None => {
            // Don't reveal if email exists or not (security best practice)
            return Ok(Json(PasswordResetRequestResponse {
                success: true,
                message:
                    "If an account with that email exists, a password reset link has been sent."
                        .to_string(),
            }));
        }
    };

    // Create password reset token (reusing VerificationToken model)
    // Token expires in 1 hour instead of the default 24
    let reset_token = state.store.create_verification_token(user.id).await?;
    state.store.set_verification_token_expiry_hours(reset_token.id, 1).await?;

    // Send password reset email (non-blocking)
    let email_service = match EmailService::from_env() {
        Ok(service) => service,
        Err(e) => {
            tracing::warn!("Failed to create email service: {}", e);
            // Still return success to avoid leaking information about email existence
            return Ok(Json(PasswordResetRequestResponse {
                success: true,
                message: "If your email is registered, you'll receive a password reset link."
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
#[axum::debug_handler]
pub async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(request): Json<PasswordResetConfirmRequest>,
) -> ApiResult<Json<PasswordResetConfirmResponse>> {
    // Validate password
    if request.new_password.len() < 8 {
        return Err(ApiError::InvalidRequest("Password must be at least 8 characters".to_string()));
    }

    // Find token
    let reset_token = state
        .store
        .find_verification_token_by_token(&request.token)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Invalid or expired reset token".to_string()))?;

    // Check if token is valid (not expired and not used)
    if !reset_token.is_valid() {
        return Err(ApiError::InvalidRequest(
            "Reset token has expired or already been used".to_string(),
        ));
    }

    // Get user
    let user = state
        .store
        .find_user_by_id(reset_token.user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Hash new password
    let password_hash = hash_password(&request.new_password).map_err(ApiError::Internal)?;

    // Update user password
    state.store.update_user_password_hash(user.id, &password_hash).await?;

    // Mark token as used
    state.store.mark_verification_token_used(reset_token.id).await?;

    tracing::info!("Password reset completed: user_id={}, email={}", user.id, user.email);

    // Record audit event for password change
    state
        .store
        .record_audit_event(
            Uuid::nil(), // System-level operation (no org context)
            Some(user.id),
            AuditEventType::PasswordChanged,
            format!("Password reset completed for user {}", user.email),
            None,
            None,
            None,
        )
        .await;

    Ok(Json(PasswordResetConfirmResponse {
        success: true,
        message: "Password has been reset successfully. You can now log in with your new password."
            .to_string(),
    }))
}
