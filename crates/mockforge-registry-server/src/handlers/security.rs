//! Security and suspicious activity handlers
//!
//! Provides endpoints for detecting and managing suspicious activities

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{SuspiciousActivity, SuspiciousActivityType, User},
    AppState,
};

#[derive(Debug, Serialize)]
pub struct SuspiciousActivityResponse {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub activity_type: String,
    pub severity: String,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub resolved: bool,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SuspiciousActivityQuery {
    pub severity: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SuspiciousActivityListResponse {
    pub activities: Vec<SuspiciousActivityResponse>,
    pub total: i64,
}

/// Get suspicious activities for an organization (admin only)
pub async fn get_suspicious_activities(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Query(query): Query<SuspiciousActivityQuery>,
) -> ApiResult<Json<SuspiciousActivityListResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get activities for this org
    let activities = SuspiciousActivity::get_unresolved(
        pool,
        Some(org_ctx.org_id),
        None,
        query.severity.as_deref(),
        query.limit.or(Some(100)),
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Get total count
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM suspicious_activities WHERE org_id = $1 AND resolved = FALSE",
    )
    .bind(org_ctx.org_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let activity_responses: Vec<SuspiciousActivityResponse> = activities
        .into_iter()
        .map(|a| SuspiciousActivityResponse {
            id: a.id,
            org_id: a.org_id,
            user_id: a.user_id,
            activity_type: format!("{:?}", a.activity_type),
            severity: a.severity,
            description: a.description,
            metadata: a.metadata,
            ip_address: a.ip_address,
            user_agent: a.user_agent,
            resolved: a.resolved,
            resolved_at: a.resolved_at,
            created_at: a.created_at,
        })
        .collect();

    Ok(Json(SuspiciousActivityListResponse {
        activities: activity_responses,
        total: total.0,
    }))
}

/// Mark suspicious activity as resolved
pub async fn resolve_suspicious_activity(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(activity_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db.pool();

    // Mark as resolved
    SuspiciousActivity::resolve(pool, activity_id, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Suspicious activity marked as resolved"
    })))
}
