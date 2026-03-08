//! Waitlist / beta signup handlers

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::{ApiError, ApiResult},
    models::waitlist::WaitlistSubscriber,
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct SubscribeRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    /// Where the signup came from (e.g. "landing_page", "pricing_page", "blog")
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubscribeResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct UnsubscribeQuery {
    pub token: Uuid,
}

/// POST /api/v1/waitlist/subscribe — public, rate-limited
pub async fn subscribe(
    State(state): State<AppState>,
    Json(request): Json<SubscribeRequest>,
) -> ApiResult<Json<SubscribeResponse>> {
    request.validate().map_err(|e| ApiError::InvalidRequest(e.to_string()))?;

    let email = request.email.trim().to_lowercase();
    let source = request.source.as_deref().unwrap_or("landing_page");

    WaitlistSubscriber::subscribe(state.db.pool(), &email, source)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(email = %email, source = %source, "Waitlist subscriber added");

    Ok(Json(SubscribeResponse {
        success: true,
        message: "You're on the list! We'll notify you when we launch.".to_string(),
    }))
}

/// GET /api/v1/waitlist/unsubscribe?token=<uuid> — public
pub async fn unsubscribe(
    State(state): State<AppState>,
    Query(query): Query<UnsubscribeQuery>,
) -> ApiResult<Json<SubscribeResponse>> {
    let removed = WaitlistSubscriber::unsubscribe_by_token(state.db.pool(), query.token)
        .await
        .map_err(ApiError::Database)?;

    if removed {
        Ok(Json(SubscribeResponse {
            success: true,
            message: "You've been unsubscribed.".to_string(),
        }))
    } else {
        Ok(Json(SubscribeResponse {
            success: false,
            message: "Link is invalid or you're already unsubscribed.".to_string(),
        }))
    }
}
