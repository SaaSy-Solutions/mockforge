//! Pillar usage analytics handlers
//!
//! Provides pillar usage analytics endpoints for workspaces and organizations,
//! tracking usage of MockForge pillars (Reality, Contracts, DevX, Cloud, AI).

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    AppState,
};
use mockforge_analytics::{Pillar, PillarUsageEvent, PillarUsageMetrics};

/// Get pillar usage metrics for a workspace
///
/// GET /api/v1/workspaces/{workspace_id}/analytics/pillars
pub async fn get_workspace_pillar_metrics(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(params): Query<PillarMetricsQuery>,
) -> ApiResult<Json<PillarMetricsResponse>> {
    // Resolve org context and verify access
    let _org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Parse duration (default to 7 days)
    let time_range_str = params.time_range.unwrap_or_else(|| "7d".to_string());
    let duration_seconds = parse_duration(&time_range_str).ok_or_else(|| {
        ApiError::InvalidRequest(
            "Invalid time_range format. Use '1d', '7d', '30d', '90d', or 'all'".to_string(),
        )
    })?;

    // Get analytics database and query metrics
    let metrics = if let Some(analytics_db) = &state.analytics_db {
        analytics_db
            .get_workspace_pillar_metrics(&workspace_id.to_string(), duration_seconds)
            .await
            .map_err(|e| {
                ApiError::Internal(anyhow::Error::msg(format!(
                    "Failed to query pillar metrics: {}",
                    e
                )))
            })?
    } else {
        // Return empty metrics if analytics database is not available
        PillarUsageMetrics {
            workspace_id: Some(workspace_id.to_string()),
            org_id: None,
            time_range: time_range_str.clone(),
            reality: None,
            contracts: None,
            devx: None,
            cloud: None,
            ai: None,
        }
    };

    Ok(Json(PillarMetricsResponse {
        workspace_id: Some(workspace_id.to_string()),
        org_id: None,
        time_range: time_range_str,
        metrics,
    }))
}

/// Get pillar usage metrics for an organization
///
/// GET /api/v1/organizations/{org_id}/analytics/pillars
pub async fn get_org_pillar_metrics(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
    Query(params): Query<PillarMetricsQuery>,
) -> ApiResult<Json<PillarMetricsResponse>> {
    // Resolve org context and verify access
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Verify the resolved org matches the requested org_id
    if org_ctx.org_id != org_id {
        return Err(ApiError::PermissionDenied);
    }

    // Verify user has permission (owner or admin)
    use crate::models::{OrgMember, OrgRole};
    let is_owner = org_ctx.org.owner_id == user_id;
    let is_admin = if !is_owner {
        if let Ok(Some(member)) = OrgMember::find(state.db.pool(), org_ctx.org_id, user_id).await {
            matches!(member.role(), OrgRole::Admin | OrgRole::Owner)
        } else {
            false
        }
    } else {
        false
    };

    if !is_owner && !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Parse duration (default to 30 days)
    let time_range_str = params.time_range.unwrap_or_else(|| "30d".to_string());
    let duration_seconds = parse_duration(&time_range_str).ok_or_else(|| {
        ApiError::InvalidRequest(
            "Invalid time_range format. Use '1d', '7d', '30d', '90d', or 'all'".to_string(),
        )
    })?;

    // Get analytics database and query metrics
    let metrics = if let Some(analytics_db) = &state.analytics_db {
        analytics_db
            .get_org_pillar_metrics(&org_id.to_string(), duration_seconds)
            .await
            .map_err(|e| {
                ApiError::Internal(anyhow::Error::msg(format!(
                    "Failed to query pillar metrics: {}",
                    e
                )))
            })?
    } else {
        // Return empty metrics if analytics database is not available
        PillarUsageMetrics {
            workspace_id: None,
            org_id: Some(org_id.to_string()),
            time_range: time_range_str.clone(),
            reality: None,
            contracts: None,
            devx: None,
            cloud: None,
            ai: None,
        }
    };

    Ok(Json(PillarMetricsResponse {
        workspace_id: None,
        org_id: Some(org_id.to_string()),
        time_range: time_range_str,
        metrics,
    }))
}

/// Record a pillar usage event
///
/// POST /api/v1/analytics/pillars/events
pub async fn record_pillar_event(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<RecordPillarEventRequest>,
) -> ApiResult<Json<RecordPillarEventResponse>> {
    // Resolve org context
    let _org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Record event if analytics database is available
    if let Some(analytics_db) = &state.analytics_db {
        let event = PillarUsageEvent {
            workspace_id: request.workspace_id,
            org_id: request.org_id,
            pillar: request.pillar,
            metric_name: request.metric_name,
            metric_value: request.metric_value,
            timestamp: Utc::now(),
        };

        analytics_db.record_pillar_usage(&event).await.map_err(|e| {
            ApiError::Internal(anyhow::Error::msg(format!("Failed to record pillar event: {}", e)))
        })?;
    }

    Ok(Json(RecordPillarEventResponse {
        success: true,
        message: "Pillar usage event recorded".to_string(),
    }))
}

// Request/Response types

#[derive(Debug, Deserialize)]
pub struct PillarMetricsQuery {
    /// Time range: "1d", "7d", "30d", "90d", or "all"
    pub time_range: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PillarMetricsResponse {
    pub workspace_id: Option<String>,
    pub org_id: Option<String>,
    pub time_range: String,
    pub metrics: PillarUsageMetrics,
}

#[derive(Debug, Deserialize)]
pub struct RecordPillarEventRequest {
    pub workspace_id: Option<String>,
    pub org_id: Option<String>,
    pub pillar: Pillar,
    pub metric_name: String,
    pub metric_value: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct RecordPillarEventResponse {
    pub success: bool,
    pub message: String,
}

/// Parse duration string to seconds
///
/// Supports: "1d", "7d", "30d", "90d", "all" (returns max i64 for "all")
fn parse_duration(s: &str) -> Option<i64> {
    match s.to_lowercase().as_str() {
        "1d" => Some(86400),     // 1 day
        "7d" => Some(604800),    // 7 days
        "30d" => Some(2592000),  // 30 days
        "90d" => Some(7776000),  // 90 days
        "all" => Some(i64::MAX), // All time
        _ => None,
    }
}
