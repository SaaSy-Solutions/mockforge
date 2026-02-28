//! Email verification handlers

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    email::EmailService,
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    models::{User, VerificationToken},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct VerifyEmailQuery {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyEmailResponse {
    pub success: bool,
    pub message: String,
}

/// Verify email address with token
pub async fn verify_email(
    State(state): State<AppState>,
    Query(params): Query<VerifyEmailQuery>,
) -> ApiResult<Json<VerifyEmailResponse>> {
    let pool = state.db.pool();

    // Find token
    let verification_token = VerificationToken::find_by_token(pool, &params.token)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| {
            ApiError::InvalidRequest("Invalid or expired verification token".to_string())
        })?;

    // Check if token is valid
    if !verification_token.is_valid() {
        return Err(ApiError::InvalidRequest(
            "Verification token has expired or already been used".to_string(),
        ));
    }

    // Get user
    let user = User::find_by_id(pool, verification_token.user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Mark user as verified
    sqlx::query("UPDATE users SET is_verified = TRUE WHERE id = $1")
        .bind(verification_token.user_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Mark token as used
    VerificationToken::mark_as_used(pool, verification_token.id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    tracing::info!("Email verified: user_id={}, email={}", user.id, user.email);

    Ok(Json(VerifyEmailResponse {
        success: true,
        message: "Email address verified successfully!".to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub struct ResendVerificationResponse {
    pub success: bool,
    pub message: String,
}

/// Resend verification email
pub async fn resend_verification(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<ResendVerificationResponse>> {
    let pool = state.db.pool();

    // Get user
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Check if already verified
    if user.is_verified {
        return Ok(Json(ResendVerificationResponse {
            success: true,
            message: "Email address is already verified".to_string(),
        }));
    }

    // Create new verification token
    let verification_token = VerificationToken::create(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Send verification email (non-blocking)
    let verification_email = EmailService::generate_verification_email(
        &user.username,
        &user.email,
        &verification_token.token,
    );

    tokio::spawn(async move {
        match EmailService::from_env() {
            Ok(email_service) => {
                if let Err(e) = email_service.send(verification_email).await {
                    tracing::warn!("Failed to send verification email: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create email service: {}", e);
            }
        }
    });

    Ok(Json(ResendVerificationResponse {
        success: true,
        message: "Verification email sent. Please check your inbox.".to_string(),
    }))
}
