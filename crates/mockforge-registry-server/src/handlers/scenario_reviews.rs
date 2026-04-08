//! Scenario review handlers

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    models::User,
    AppState,
};

/// Get reviews for a scenario
pub async fn get_scenario_reviews(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<ReviewQueryParams>,
) -> ApiResult<Json<ScenarioReviewsResponse>> {
    let scenario = state
        .store
        .find_scenario_by_name(&name)
        .await?
        .ok_or_else(|| ApiError::ScenarioNotFound(name.clone()))?;

    let limit = params.per_page.unwrap_or(20) as i64;
    let offset = (params.page.unwrap_or(0) * limit as usize) as i64;

    let reviews = state.store.get_scenario_reviews(scenario.id, limit, offset).await?;

    let total = state.store.count_scenario_reviews(scenario.id).await?;

    let mut review_responses = Vec::new();
    for review in reviews {
        let reviewer =
            state.store.find_user_by_id(review.reviewer_id).await?.unwrap_or_else(|| User {
                id: review.reviewer_id,
                username: "unknown".to_string(),
                email: "unknown@example.com".to_string(),
                password_hash: String::new(),
                api_token: None,
                is_verified: false,
                is_admin: false,
                two_factor_enabled: false,
                two_factor_secret: None,
                two_factor_backup_codes: None,
                two_factor_verified_at: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            });

        review_responses.push(ScenarioReviewResponse {
            id: review.id.to_string(),
            reviewer: reviewer.username,
            reviewer_email: Some(reviewer.email),
            rating: review.rating as u8,
            title: review.title,
            comment: review.comment,
            created_at: review.created_at.to_rfc3339(),
            helpful_count: review.helpful_count as u32,
            verified_purchase: review.verified_purchase,
        });
    }

    Ok(Json(ScenarioReviewsResponse {
        reviews: review_responses,
        total: total as usize,
        page: params.page.unwrap_or(0),
        per_page: params.per_page.unwrap_or(20),
    }))
}

/// Submit a review for a scenario
pub async fn submit_scenario_review(
    State(state): State<AppState>,
    AuthUser(reviewer_id): AuthUser,
    _headers: HeaderMap,
    Path(name): Path<String>,
    Json(request): Json<SubmitScenarioReviewRequest>,
) -> ApiResult<Json<SubmitReviewResponse>> {
    // Validate rating
    if request.rating < 1 || request.rating > 5 {
        return Err(ApiError::InvalidRequest("Rating must be between 1 and 5".to_string()));
    }

    // Validate comment
    if request.comment.len() < 10 {
        return Err(ApiError::InvalidRequest("Comment must be at least 10 characters".to_string()));
    }

    let scenario = state
        .store
        .find_scenario_by_name(&name)
        .await?
        .ok_or_else(|| ApiError::ScenarioNotFound(name.clone()))?;

    // Check if user already reviewed
    if state
        .store
        .find_existing_scenario_review(scenario.id, reviewer_id)
        .await?
        .is_some()
    {
        return Err(ApiError::InvalidRequest(
            "You have already reviewed this scenario".to_string(),
        ));
    }

    // Create review
    let review = state
        .store
        .create_scenario_review(
            scenario.id,
            reviewer_id,
            request.rating as i32,
            request.title.as_deref(),
            &request.comment,
        )
        .await?;

    // Update scenario stats
    state.store.update_scenario_review_stats(scenario.id).await?;

    Ok(Json(SubmitReviewResponse {
        success: true,
        review_id: review.id.to_string(),
        message: "Review submitted successfully".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct ReviewQueryParams {
    #[serde(default)]
    pub page: Option<usize>,
    #[serde(default)]
    pub per_page: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitScenarioReviewRequest {
    pub rating: u8,
    pub title: Option<String>,
    pub comment: String,
}

#[derive(Debug, Serialize)]
pub struct ScenarioReviewsResponse {
    pub reviews: Vec<ScenarioReviewResponse>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

#[derive(Debug, Serialize)]
pub struct ScenarioReviewResponse {
    pub id: String,
    pub reviewer: String,
    pub reviewer_email: Option<String>,
    pub rating: u8,
    pub title: Option<String>,
    pub comment: String,
    pub created_at: String,
    pub helpful_count: u32,
    pub verified_purchase: bool,
}

#[derive(Debug, Serialize)]
pub struct SubmitReviewResponse {
    pub success: bool,
    pub review_id: String,
    pub message: String,
}
