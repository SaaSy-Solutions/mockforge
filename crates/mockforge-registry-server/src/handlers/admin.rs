//! Admin handlers

use axum::{
    extract::{Path, State},
    Extension, Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    models::Plugin,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct VerifyPluginRequest {
    pub verified: bool,
}

#[derive(Debug, Serialize)]
pub struct VerifyPluginResponse {
    pub success: bool,
    pub plugin_name: String,
    pub verified: bool,
    pub verified_at: Option<String>,
    pub message: String,
}

pub async fn verify_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Extension(user_id): Extension<String>,
    Json(request): Json<VerifyPluginRequest>,
) -> ApiResult<Json<VerifyPluginResponse>> {
    let pool = state.db.pool();

    // Parse user_id
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid user ID".to_string()))?;

    // Check if user is admin
    let user = sqlx::query_as::<_, (bool,)>(
        "SELECT is_admin FROM users WHERE id = $1"
    )
    .bind(user_uuid)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    if !user.0 {
        return Err(ApiError::PermissionDenied);
    }

    // Get plugin
    let plugin = Plugin::find_by_name(pool, &name)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    // Update verification status
    let verified_at = if request.verified {
        Some(Utc::now())
    } else {
        None
    };

    sqlx::query(
        "UPDATE plugins SET verified_at = $1 WHERE id = $2"
    )
    .bind(verified_at)
    .bind(plugin.id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let message = if request.verified {
        format!("Plugin '{}' has been verified", name)
    } else {
        format!("Plugin '{}' verification has been removed", name)
    };

    Ok(Json(VerifyPluginResponse {
        success: true,
        plugin_name: name,
        verified: request.verified,
        verified_at: verified_at.map(|dt| dt.to_rfc3339()),
        message,
    }))
}

#[derive(Debug, Serialize)]
pub struct PluginWithBadges {
    pub name: String,
    pub version: String,
    pub badges: Vec<String>,
}

pub async fn get_plugin_badges(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<PluginWithBadges>> {
    let pool = state.db.pool();

    // Get plugin
    let plugin = Plugin::find_by_name(pool, &name)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    let mut badges = Vec::new();

    // Check for "Official" badge (created by admin user)
    let admin_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    if plugin.author_id == admin_id {
        badges.push("official".to_string());
    }

    // Check for "Verified" badge
    if plugin.verified_at.is_some() {
        badges.push("verified".to_string());
    }

    // Check for "Popular" badge (1000+ downloads)
    if plugin.downloads_total >= 1000 {
        badges.push("popular".to_string());
    }

    // Check for "Highly Rated" badge (4.5+ stars with 10+ reviews)
    if plugin.rating_avg >= rust_decimal::Decimal::new(45, 1) && plugin.rating_count >= 10 {
        badges.push("highly-rated".to_string());
    }

    // Check for "Maintained" badge (updated within last 90 days)
    let ninety_days_ago = Utc::now() - chrono::Duration::days(90);
    if plugin.updated_at > ninety_days_ago {
        badges.push("maintained".to_string());
    }

    // Check for "Trending" badge (check downloads in last week)
    // For MVP, we'll use a simple heuristic
    if plugin.downloads_total > 100 {
        badges.push("trending".to_string());
    }

    Ok(Json(PluginWithBadges {
        name: plugin.name,
        version: plugin.current_version,
        badges,
    }))
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_plugins: i64,
    pub total_downloads: i64,
    pub total_users: i64,
    pub verified_plugins: i64,
    pub total_reviews: i64,
    pub average_rating: f64,
}

pub async fn get_admin_stats(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> ApiResult<Json<StatsResponse>> {
    let pool = state.db.pool();

    // Parse user_id
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid user ID".to_string()))?;

    // Check if user is admin
    let user = sqlx::query_as::<_, (bool,)>(
        "SELECT is_admin FROM users WHERE id = $1"
    )
    .bind(user_uuid)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    if !user.0 {
        return Err(ApiError::PermissionDenied);
    }

    // Get stats
    let plugin_stats = sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT COUNT(*), SUM(downloads_total), COUNT(*) FILTER (WHERE verified_at IS NOT NULL) FROM plugins"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let user_count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM users"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let review_stats = sqlx::query_as::<_, (i64, f64)>(
        "SELECT COUNT(*), COALESCE(AVG(rating), 0.0)::float8 FROM reviews"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    Ok(Json(StatsResponse {
        total_plugins: plugin_stats.0,
        total_downloads: plugin_stats.1,
        verified_plugins: plugin_stats.2,
        total_users: user_count.0,
        total_reviews: review_stats.0,
        average_rating: review_stats.1,
    }))
}
