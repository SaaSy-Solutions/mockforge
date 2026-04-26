//! Audit log handlers
//!
//! Provides endpoints for organization admins to view audit logs

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    models::{AuditEventType, OrgRole},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub event_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Option<Uuid>,
    pub event_type: String,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogListResponse {
    pub logs: Vec<AuditLogResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Get audit logs for an organization (admin only)
pub async fn get_audit_logs(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    Query(query): Query<AuditLogQuery>,
) -> ApiResult<Json<AuditLogListResponse>> {
    // Verify organization exists
    let org = state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user is admin or owner of the organization
    let is_owner = org.owner_id == user_id;
    let is_admin = if !is_owner {
        let member = state.store.find_org_member(org_id, user_id).await?;
        member.map(|m| m.role() == OrgRole::Admin).unwrap_or(false)
    } else {
        true
    };

    if !is_owner && !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Parse event types if provided. Accepts a single value or a comma-separated list
    // (e.g. `?event_type=byok_config_updated,byok_config_deleted`). Unknown values are
    // silently ignored so a stale client can't 500 the endpoint.
    let event_types: Vec<AuditEventType> = query
        .event_type
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .filter_map(AuditEventType::from_str)
                .collect()
        })
        .unwrap_or_default();

    // Get audit logs
    let logs = state
        .store
        .list_audit_logs(org_id, query.limit.or(Some(100)), query.offset.or(Some(0)), &event_types)
        .await?;

    // Get total count
    let total = state.store.count_audit_logs(org_id, &event_types).await?;

    // Convert to response format
    let log_responses: Vec<AuditLogResponse> = logs
        .into_iter()
        .map(|log| AuditLogResponse {
            id: log.id,
            org_id: log.org_id,
            user_id: log.user_id,
            event_type: format!("{:?}", log.event_type),
            description: log.description,
            metadata: log.metadata,
            ip_address: log.ip_address,
            user_agent: log.user_agent,
            created_at: log.created_at,
        })
        .collect();

    Ok(Json(AuditLogListResponse {
        logs: log_responses,
        total,
        limit: query.limit.unwrap_or(100),
        offset: query.offset.unwrap_or(0),
    }))
}
