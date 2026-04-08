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
    // Find token
    let verification_token = state
        .store
        .find_verification_token_by_token(&params.token)
        .await?
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
    let user = state
        .store
        .find_user_by_id(verification_token.user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Mark user as verified
    state.store.mark_user_verified(verification_token.user_id).await?;

    // Mark token as used
    state.store.mark_verification_token_used(verification_token.id).await?;

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
    // Get user
    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Check if already verified
    if user.is_verified {
        return Ok(Json(ResendVerificationResponse {
            success: true,
            message: "Email address is already verified".to_string(),
        }));
    }

    // Create new verification token
    let verification_token = state.store.create_verification_token(user_id).await?;

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
