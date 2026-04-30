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
    models::{AuditEventType, Plugin},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct VerifyPluginRequest {
    pub verified: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakedownPluginRequest {
    /// Optional reason shown on the admin detail view. Stored on the
    /// plugin row so admins reviewing past moderation see why.
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TakedownPluginResponse {
    pub success: bool,
    pub plugin_name: String,
    pub taken_down: bool,
    pub taken_down_at: Option<String>,
    pub reason: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
    let user = sqlx::query_as::<_, (bool,)>("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_uuid)
        .fetch_one(pool)
        .await
        .map_err(ApiError::Database)?;

    if !user.0 {
        return Err(ApiError::PermissionDenied);
    }

    // Get plugin
    let plugin = Plugin::find_by_name(pool, &name)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    // Update verification status
    let verified_at = if request.verified {
        Some(Utc::now())
    } else {
        None
    };

    sqlx::query("UPDATE plugins SET verified_at = $1 WHERE id = $2")
        .bind(verified_at)
        .bind(plugin.id)
        .execute(pool)
        .await
        .map_err(ApiError::Database)?;

    let message = if request.verified {
        format!("Plugin '{}' has been verified", name)
    } else {
        format!("Plugin '{}' verification has been removed", name)
    };

    // Record audit event for admin verification action
    state
        .store
        .record_audit_event(
            Uuid::nil(),
            Some(user_uuid),
            AuditEventType::AdminImpersonation, // Reusing admin action type for verification
            message.clone(),
            Some(serde_json::json!({
                "plugin_name": name,
                "verified": request.verified,
                "action": "verify_plugin",
            })),
            None,
            None,
        )
        .await;

    Ok(Json(VerifyPluginResponse {
        success: true,
        plugin_name: name,
        verified: request.verified,
        verified_at: verified_at.map(|dt| dt.to_rfc3339()),
        message,
    }))
}

/// Soft-delete a plugin from the public catalog. Reversible via
/// `restore_plugin` — installed copies keep working because we only flip
/// flags; we don't drop rows.
pub async fn takedown_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Extension(user_id): Extension<String>,
    Json(request): Json<TakedownPluginRequest>,
) -> ApiResult<Json<TakedownPluginResponse>> {
    let pool = state.db.pool();

    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid user ID".to_string()))?;

    let user = sqlx::query_as::<_, (bool,)>("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_uuid)
        .fetch_one(pool)
        .await
        .map_err(ApiError::Database)?;
    if !user.0 {
        return Err(ApiError::PermissionDenied);
    }

    let plugin = Plugin::find_by_name(pool, &name)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    let reason = request.reason.as_deref().map(str::trim).filter(|s| !s.is_empty());
    state.store.take_down_plugin(plugin.id, reason).await?;

    let message = format!("Plugin '{}' has been taken down", name);
    state
        .store
        .record_audit_event(
            Uuid::nil(),
            Some(user_uuid),
            AuditEventType::PluginTakenDown,
            message.clone(),
            Some(serde_json::json!({
                "plugin_name": name,
                "reason": reason,
            })),
            None,
            None,
        )
        .await;

    Ok(Json(TakedownPluginResponse {
        success: true,
        plugin_name: name,
        taken_down: true,
        taken_down_at: Some(Utc::now().to_rfc3339()),
        reason: reason.map(str::to_string),
        message,
    }))
}

/// Reverse a takedown — clears both the timestamp and the stored reason.
pub async fn restore_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Extension(user_id): Extension<String>,
) -> ApiResult<Json<TakedownPluginResponse>> {
    let pool = state.db.pool();

    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid user ID".to_string()))?;

    let user = sqlx::query_as::<_, (bool,)>("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_uuid)
        .fetch_one(pool)
        .await
        .map_err(ApiError::Database)?;
    if !user.0 {
        return Err(ApiError::PermissionDenied);
    }

    let plugin = Plugin::find_by_name(pool, &name)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    state.store.restore_plugin(plugin.id).await?;

    let message = format!("Plugin '{}' has been restored", name);
    state
        .store
        .record_audit_event(
            Uuid::nil(),
            Some(user_uuid),
            AuditEventType::PluginRestored,
            message.clone(),
            Some(serde_json::json!({ "plugin_name": name })),
            None,
            None,
        )
        .await;

    Ok(Json(TakedownPluginResponse {
        success: true,
        plugin_name: name,
        taken_down: false,
        taken_down_at: None,
        reason: None,
        message,
    }))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TakenDownPluginEntry {
    pub name: String,
    pub description: String,
    pub category: String,
    pub current_version: String,
    pub author: TakenDownAuthorInfo,
    pub taken_down_at: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TakenDownAuthorInfo {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTakenDownResponse {
    pub plugins: Vec<TakenDownPluginEntry>,
    pub total: usize,
}

/// Admin moderation: list every plugin that's currently taken-down.
/// `Plugin::search` filters these out, so this is the only programmatic
/// path for the moderation UI to find them after the post-takedown
/// snackbar window closes.
pub async fn list_taken_down_plugins(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> ApiResult<Json<ListTakenDownResponse>> {
    let pool = state.db.pool();

    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| ApiError::InvalidRequest("Invalid user ID".to_string()))?;

    let user = sqlx::query_as::<_, (bool,)>("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_uuid)
        .fetch_one(pool)
        .await
        .map_err(ApiError::Database)?;
    if !user.0 {
        return Err(ApiError::PermissionDenied);
    }

    let plugins = state.store.list_taken_down_plugins().await?;
    let mut entries = Vec::with_capacity(plugins.len());
    for plugin in plugins {
        let author = state
            .store
            .find_user_by_id(plugin.author_id)
            .await?
            .unwrap_or_else(|| crate::models::User::placeholder(plugin.author_id));
        entries.push(TakenDownPluginEntry {
            name: plugin.name,
            description: plugin.description,
            category: plugin.category,
            current_version: plugin.current_version,
            author: TakenDownAuthorInfo {
                id: author.id.to_string(),
                username: author.username,
                email: Some(author.email),
            },
            // taken_down_at is guaranteed Some by the SQL filter, but
            // keep a defensive fallback so a clock-skew or column
            // regression can't crash the page.
            taken_down_at: plugin.taken_down_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
            reason: plugin.taken_down_reason,
        });
    }

    let total = entries.len();
    Ok(Json(ListTakenDownResponse {
        plugins: entries,
        total,
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
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    let mut badges = Vec::new();

    // Check for "Official" badge (created by admin user)
    // ADMIN_USER_ID: UUID of the admin user for official plugins
    // Default: "00000000-0000-0000-0000-000000000001"
    let admin_id = std::env::var("ADMIN_USER_ID")
        .ok()
        .and_then(|s| Uuid::parse_str(&s).ok())
        .unwrap_or_else(|| {
            Uuid::parse_str("00000000-0000-0000-0000-000000000001")
                .expect("default admin UUID is valid")
        });
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
    if plugin.rating_avg >= 4.5 && plugin.rating_count >= 10 {
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

    // "Signed" badge — set when the plugin's current version was
    // published with a verified Ed25519 SBOM attestation. The scanner
    // also surfaces this inside security findings, but the badge here
    // makes it visible at the marketplace card level so users can spot
    // signed plugins without opening the security tab.
    let signed: Option<bool> = sqlx::query_scalar(
        r#"
        SELECT (sbom_signed_key_id IS NOT NULL) AS signed
        FROM plugin_versions
        WHERE plugin_id = $1 AND version = $2
        LIMIT 1
        "#,
    )
    .bind(plugin.id)
    .bind(&plugin.current_version)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?;
    if matches!(signed, Some(true)) {
        badges.push("signed".to_string());
    }

    Ok(Json(PluginWithBadges {
        name: plugin.name,
        version: plugin.current_version,
        badges,
    }))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
    let user = sqlx::query_as::<_, (bool,)>("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_uuid)
        .fetch_one(pool)
        .await
        .map_err(ApiError::Database)?;

    if !user.0 {
        return Err(ApiError::PermissionDenied);
    }

    // Get stats
    let plugin_stats = sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT COUNT(*), SUM(downloads_total), COUNT(*) FILTER (WHERE verified_at IS NOT NULL) FROM plugins"
    )
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)?;

    let user_count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .map_err(ApiError::Database)?;

    let review_stats = sqlx::query_as::<_, (i64, f64)>(
        "SELECT COUNT(*), COALESCE(AVG(rating), 0.0)::float8 FROM reviews",
    )
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(StatsResponse {
        total_plugins: plugin_stats.0,
        total_downloads: plugin_stats.1,
        verified_plugins: plugin_stats.2,
        total_users: user_count.0,
        total_reviews: review_stats.0,
        average_rating: review_stats.1,
    }))
}
