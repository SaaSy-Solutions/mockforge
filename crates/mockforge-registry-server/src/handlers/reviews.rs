//! Review handlers

use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    models::{Plugin, Review},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ReviewQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    0
}

fn default_per_page() -> i64 {
    20
}

#[derive(Debug, Serialize)]
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
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct ReviewsResponse {
    pub reviews: Vec<ReviewWithUser>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub stats: ReviewStats,
}

#[derive(Debug, Serialize)]
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
    let pool = state.db.pool();

    // Get plugin
    let plugin = Plugin::find_by_name(pool, &name)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    // Get reviews with pagination
    let offset = query.page * query.per_page;
    let reviews = Review::get_by_plugin(pool, plugin.id, query.per_page, offset)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Get total count
    let total = Review::count_by_plugin(pool, plugin.id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Get users for reviews
    let mut reviews_with_users = Vec::new();
    for review in reviews {
        let user = sqlx::query_as::<_, (String, String)>(
            "SELECT id::text, username FROM users WHERE id = $1"
        )
        .bind(review.user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

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
                username: user.1,
            },
        });
    }

    // Calculate stats
    let stats_query = sqlx::query_as::<_, (f64, i64)>(
        r#"
        SELECT COALESCE(AVG(rating), 0.0)::float8, COUNT(*)
        FROM reviews
        WHERE plugin_id = $1
        "#
    )
    .bind(plugin.id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let (average_rating, total_reviews) = stats_query;

    // Get rating distribution
    let distribution_rows = sqlx::query_as::<_, (i16, i64)>(
        "SELECT rating, COUNT(*) FROM reviews WHERE plugin_id = $1 GROUP BY rating"
    )
    .bind(plugin.id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let mut rating_distribution = std::collections::HashMap::new();
    for (rating, count) in distribution_rows {
        rating_distribution.insert(rating, count);
    }

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
    let pool = state.db.pool();

    // Validate rating
    if request.rating < 1 || request.rating > 5 {
        return Err(ApiError::InvalidRequest(
            "Rating must be between 1 and 5".to_string(),
        ));
    }

    // Validate comment length
    if request.comment.len() < 10 {
        return Err(ApiError::InvalidRequest(
            "Comment must be at least 10 characters".to_string(),
        ));
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
    let plugin = Plugin::find_by_name(pool, &name)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    // Parse user_id
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid user ID".to_string()))?;

    // Check if user already reviewed this plugin
    let existing = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM reviews WHERE plugin_id = $1 AND user_id = $2"
    )
    .bind(plugin.id)
    .bind(user_uuid)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    if existing.is_some() {
        return Err(ApiError::InvalidRequest(
            "You have already reviewed this plugin. Please edit your existing review.".to_string(),
        ));
    }

    // Create review
    let review = Review::create(
        pool,
        plugin.id,
        user_uuid,
        &request.version,
        request.rating,
        request.title.as_deref(),
        &request.comment,
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Update plugin rating stats
    let stats = sqlx::query_as::<_, (f64, i64)>(
        r#"
        SELECT COALESCE(AVG(rating), 0.0)::float8, COUNT(*)
        FROM reviews
        WHERE plugin_id = $1
        "#
    )
    .bind(plugin.id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    sqlx::query(
        "UPDATE plugins SET rating_avg = $1, rating_count = $2 WHERE id = $3"
    )
    .bind(stats.0)
    .bind(stats.1 as i32)
    .bind(plugin.id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

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
    let pool = state.db.pool();

    // Parse review_id
    let review_uuid = Uuid::parse_str(&review_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid review ID".to_string()))?;

    // Get plugin
    let plugin = Plugin::find_by_name(pool, &plugin_name)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::PluginNotFound(plugin_name.clone()))?;

    // Update vote count
    let field = if request.helpful {
        "helpful_count"
    } else {
        "unhelpful_count"
    };

    let query_str = format!(
        "UPDATE reviews SET {} = {} + 1 WHERE id = $1 AND plugin_id = $2",
        field, field
    );

    sqlx::query(&query_str)
        .bind(review_uuid)
        .bind(plugin.id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Vote recorded"
    })))
}
