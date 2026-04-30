//! Review handlers

use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    models::AuditEventType,
    AppState,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page", alias = "per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    0
}

fn default_per_page() -> i64 {
    20
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewWithUser {
    pub id: String,
    pub plugin_id: String,
    pub version: String,
    pub rating: i16,
    pub title: Option<String>,
    pub comment: String,
    pub helpful_count: i32,
    pub unhelpful_count: i32,
    pub verified: bool,
    pub created_at: String,
    pub updated_at: String,
    pub user: UserInfo,
    pub user_name: String,
    /// Plugin author's response to this review, set via the
    /// `/respond` endpoint. `None` until the author posts one.
    pub author_response: Option<AuthorResponseDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorResponseDto {
    pub text: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewsResponse {
    pub reviews: Vec<ReviewWithUser>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub stats: ReviewStats,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewStats {
    pub average_rating: f64,
    pub total_reviews: i64,
    pub rating_distribution: std::collections::HashMap<i16, i64>,
}

pub async fn get_reviews(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<ReviewQuery>,
) -> ApiResult<Json<ReviewsResponse>> {
    // Get plugin
    let plugin = state
        .store
        .find_plugin_by_name(&name)
        .await?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    // Get reviews with pagination
    let offset = query.page * query.per_page;
    let reviews = state.store.get_plugin_reviews(plugin.id, query.per_page, offset).await?;

    // Get total count
    let total = state.store.count_plugin_reviews(plugin.id).await?;

    // Get users for reviews
    let mut reviews_with_users = Vec::new();
    for review in reviews {
        let user = state
            .store
            .get_user_public_info(review.user_id)
            .await?
            .unwrap_or_else(|| (review.user_id.to_string(), "unknown".to_string()));

        let author_response = match (review.author_response_text, review.author_response_at) {
            (Some(text), Some(at)) => Some(AuthorResponseDto {
                text,
                created_at: at.to_rfc3339(),
            }),
            _ => None,
        };
        reviews_with_users.push(ReviewWithUser {
            id: review.id.to_string(),
            plugin_id: review.plugin_id.to_string(),
            version: review.version,
            rating: review.rating,
            title: review.title,
            comment: review.comment,
            helpful_count: review.helpful_count,
            unhelpful_count: review.unhelpful_count,
            verified: review.verified,
            created_at: review.created_at.to_rfc3339(),
            updated_at: review.updated_at.to_rfc3339(),
            user: UserInfo {
                id: user.0,
                username: user.1.clone(),
            },
            user_name: user.1,
            author_response,
        });
    }

    // Calculate stats
    let (average_rating, total_reviews) = state.store.get_plugin_review_stats(plugin.id).await?;
    let rating_distribution = state.store.get_plugin_review_distribution(plugin.id).await?;

    let stats = ReviewStats {
        average_rating,
        total_reviews,
        rating_distribution,
    };

    Ok(Json(ReviewsResponse {
        reviews: reviews_with_users,
        total,
        page: query.page,
        per_page: query.per_page,
        stats,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SubmitReviewRequest {
    pub version: String,
    pub rating: i16,
    pub title: Option<String>,
    pub comment: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitReviewResponse {
    pub success: bool,
    pub review_id: String,
    pub message: String,
}

pub async fn submit_review(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Extension(user_id): Extension<String>, // From auth middleware
    Json(request): Json<SubmitReviewRequest>,
) -> ApiResult<Json<SubmitReviewResponse>> {
    // Validate rating
    if request.rating < 1 || request.rating > 5 {
        return Err(ApiError::InvalidRequest("Rating must be between 1 and 5".to_string()));
    }

    // Validate comment length
    if request.comment.len() < 10 {
        return Err(ApiError::InvalidRequest("Comment must be at least 10 characters".to_string()));
    }

    if request.comment.len() > 5000 {
        return Err(ApiError::InvalidRequest(
            "Comment must be less than 5000 characters".to_string(),
        ));
    }

    // Validate title if provided
    if let Some(ref title) = request.title {
        if title.len() > 100 {
            return Err(ApiError::InvalidRequest(
                "Title must be less than 100 characters".to_string(),
            ));
        }
    }

    // Get plugin
    let plugin = state
        .store
        .find_plugin_by_name(&name)
        .await?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    // Parse user_id
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid user ID".to_string()))?;

    // Check if user already reviewed this plugin
    if state.store.find_existing_plugin_review(plugin.id, user_uuid).await?.is_some() {
        return Err(ApiError::InvalidRequest(
            "You have already reviewed this plugin. Please edit your existing review.".to_string(),
        ));
    }

    // Create review
    let review = state
        .store
        .create_plugin_review(
            plugin.id,
            user_uuid,
            &request.version,
            request.rating,
            request.title.as_deref(),
            &request.comment,
        )
        .await?;

    // Update plugin rating stats
    let (avg, count) = state.store.get_plugin_review_stats(plugin.id).await?;
    state.store.update_plugin_rating_stats(plugin.id, avg, count as i32).await?;

    Ok(Json(SubmitReviewResponse {
        success: true,
        review_id: review.id.to_string(),
        message: "Review submitted successfully".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    pub helpful: bool,
}

pub async fn vote_review(
    State(state): State<AppState>,
    Path((plugin_name, review_id)): Path<(String, String)>,
    Extension(_user_id): Extension<String>,
    Json(request): Json<VoteRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Parse review_id
    let review_uuid = Uuid::parse_str(&review_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid review ID".to_string()))?;

    // Get plugin
    let plugin = state
        .store
        .find_plugin_by_name(&plugin_name)
        .await?
        .ok_or_else(|| ApiError::PluginNotFound(plugin_name.clone()))?;

    state
        .store
        .increment_plugin_review_vote(plugin.id, review_uuid, request.helpful)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Vote recorded"
    })))
}

#[derive(Debug, Deserialize)]
pub struct AuthorResponseRequest {
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorResponseResponse {
    pub success: bool,
    pub message: String,
    pub author_response: Option<AuthorResponseDto>,
}

/// Post (or replace) the plugin author's response to a single review.
/// Author check uses `plugins.author_id` rather than ownership of the
/// review itself — only the plugin owner can respond. Replaces any
/// existing response so authors can correct typos without a separate
/// edit endpoint.
pub async fn respond_to_review(
    State(state): State<AppState>,
    Path((plugin_name, review_id)): Path<(String, String)>,
    Extension(user_id): Extension<String>,
    Json(request): Json<AuthorResponseRequest>,
) -> ApiResult<Json<AuthorResponseResponse>> {
    let trimmed = request.text.trim();
    if trimmed.is_empty() {
        return Err(ApiError::InvalidRequest(
            "Response text cannot be empty — use DELETE to clear instead".to_string(),
        ));
    }
    if trimmed.len() > 5000 {
        return Err(ApiError::InvalidRequest(
            "Response text must be 5000 characters or fewer".to_string(),
        ));
    }

    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid user ID".to_string()))?;
    let review_uuid = Uuid::parse_str(&review_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid review ID".to_string()))?;

    let plugin = state
        .store
        .find_plugin_by_name(&plugin_name)
        .await?
        .ok_or_else(|| ApiError::PluginNotFound(plugin_name.clone()))?;

    if plugin.author_id != user_uuid {
        return Err(ApiError::PermissionDenied);
    }

    // Verify the review exists and belongs to this plugin so a typo in
    // the path returns 404 (not 500 from the UPDATE silently succeeding
    // on no rows).
    state
        .store
        .find_review_in_plugin(plugin.id, review_uuid)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Review not found for plugin".to_string()))?;

    state.store.set_review_author_response(review_uuid, Some(trimmed)).await?;

    let now = chrono::Utc::now();
    let response_dto = AuthorResponseDto {
        text: trimmed.to_string(),
        created_at: now.to_rfc3339(),
    };

    state
        .store
        .record_audit_event(
            Uuid::nil(),
            Some(user_uuid),
            AuditEventType::PluginReviewResponsePosted,
            format!("Author responded to review on plugin '{}'", plugin_name),
            Some(serde_json::json!({
                "plugin_name": plugin_name,
                "review_id": review_id,
            })),
            None,
            None,
        )
        .await;

    Ok(Json(AuthorResponseResponse {
        success: true,
        message: "Response posted".to_string(),
        author_response: Some(response_dto),
    }))
}

/// Clear an existing author response. Same author check as `respond`.
pub async fn clear_review_response(
    State(state): State<AppState>,
    Path((plugin_name, review_id)): Path<(String, String)>,
    Extension(user_id): Extension<String>,
) -> ApiResult<Json<AuthorResponseResponse>> {
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid user ID".to_string()))?;
    let review_uuid = Uuid::parse_str(&review_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid review ID".to_string()))?;

    let plugin = state
        .store
        .find_plugin_by_name(&plugin_name)
        .await?
        .ok_or_else(|| ApiError::PluginNotFound(plugin_name.clone()))?;

    if plugin.author_id != user_uuid {
        return Err(ApiError::PermissionDenied);
    }

    state
        .store
        .find_review_in_plugin(plugin.id, review_uuid)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Review not found for plugin".to_string()))?;

    state.store.set_review_author_response(review_uuid, None).await?;

    Ok(Json(AuthorResponseResponse {
        success: true,
        message: "Response cleared".to_string(),
        author_response: None,
    }))
}
