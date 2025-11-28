//! Template review handlers

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{AuthUser, resolve_org_context},
    models::{Template, TemplateReview, TemplateVersion, User},
    AppState,
};

/// Get reviews for a template
pub async fn get_template_reviews(
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
    Query(params): Query<ReviewQueryParams>,
) -> ApiResult<Json<TemplateReviewsResponse>> {
    let pool = state.db.pool();

    let template = Template::find_by_name_version(pool, &name, &version)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::TemplateNotFound(format!("{}@{}", name, version)))?;

    let limit = params.per_page.unwrap_or(20) as i64;
    let offset = (params.page.unwrap_or(0) * limit as usize) as i64;

    let reviews = TemplateReview::get_by_template(pool, template.id, limit, offset)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let total = TemplateReview::count_by_template(pool, template.id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let mut review_responses = Vec::new();
    for review in reviews {
        let reviewer = User::find_by_id(pool, review.reviewer_id)
            .await
            .map_err(|e| ApiError::Database(e))?
            .unwrap_or_else(|| User {
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

        review_responses.push(TemplateReviewResponse {
            id: review.id.to_string(),
            reviewer: reviewer.username,
            reviewer_email: Some(reviewer.email),
            rating: review.rating as u8,
            title: review.title,
            comment: review.comment,
            created_at: review.created_at.to_rfc3339(),
            helpful_count: review.helpful_count as u32,
            verified_use: review.verified_use,
        });
    }

    Ok(Json(TemplateReviewsResponse {
        reviews: review_responses,
        total: total as usize,
        page: params.page.unwrap_or(0),
        per_page: params.per_page.unwrap_or(20),
    }))
}

/// Submit a review for a template
pub async fn submit_template_review(
    State(state): State<AppState>,
    AuthUser(reviewer_id): AuthUser,
    headers: HeaderMap,
    Path((name, version)): Path<(String, String)>,
    Json(request): Json<SubmitTemplateReviewRequest>,
) -> ApiResult<Json<SubmitReviewResponse>> {
    let pool = state.db.pool();

    // Validate rating
    if request.rating < 1 || request.rating > 5 {
        return Err(ApiError::InvalidRequest("Rating must be between 1 and 5".to_string()));
    }

    // Validate comment
    if request.comment.len() < 10 {
        return Err(ApiError::InvalidRequest("Comment must be at least 10 characters".to_string()));
    }

    let template = Template::find_by_name_version(pool, &name, &version)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::TemplateNotFound(format!("{}@{}", name, version)))?;

    // Check if user already reviewed
    let existing = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM template_reviews WHERE template_id = $1 AND reviewer_id = $2",
    )
    .bind(template.id)
    .bind(reviewer_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    if existing.is_some() {
        return Err(ApiError::InvalidRequest(
            "You have already reviewed this template".to_string(),
        ));
    }

    // Create review
    let review = TemplateReview::create(
        pool,
        template.id,
        reviewer_id,
        request.rating as i32,
        request.title.as_deref(),
        &request.comment,
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Update template stats
    TemplateReview::update_template_stats(pool, template.id)
        .await
        .map_err(|e| ApiError::Database(e))?;

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
pub struct SubmitTemplateReviewRequest {
    pub rating: u8,
    pub title: Option<String>,
    pub comment: String,
}

#[derive(Debug, Serialize)]
pub struct TemplateReviewsResponse {
    pub reviews: Vec<TemplateReviewResponse>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

#[derive(Debug, Serialize)]
pub struct TemplateReviewResponse {
    pub id: String,
    pub reviewer: String,
    pub reviewer_email: Option<String>,
    pub rating: u8,
    pub title: Option<String>,
    pub comment: String,
    pub created_at: String,
    pub helpful_count: u32,
    pub verified_use: bool,
}

#[derive(Debug, Serialize)]
pub struct SubmitReviewResponse {
    pub success: bool,
    pub review_id: String,
    pub message: String,
}
