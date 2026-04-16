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

    // Parse event types if provided. Accepts a single value or a
    // comma-separated list. Unknown values are silently dropped; if the user
    // supplied a filter but every value was unknown, return an empty page
    // rather than 500-ing or returning all rows.
    let event_types = query.event_type.as_deref().map(|raw| {
        raw.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(parse_audit_event_type)
            .collect::<Vec<AuditEventType>>()
    });

    if let Some(parsed) = &event_types {
        if parsed.is_empty() {
            return Ok(Json(AuditLogListResponse {
                logs: Vec::new(),
                total: 0,
                limit: query.limit.unwrap_or(100),
                offset: query.offset.unwrap_or(0),
            }));
        }
    }

    let logs = state
        .store
        .list_audit_logs(
            org_id,
            query.limit.or(Some(100)),
            query.offset.or(Some(0)),
            event_types.clone(),
        )
        .await?;

    let total = state.store.count_audit_logs(org_id, event_types).await?;

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

fn parse_audit_event_type(s: &str) -> Option<AuditEventType> {
    match s {
        "member_added" => Some(AuditEventType::MemberAdded),
        "member_removed" => Some(AuditEventType::MemberRemoved),
        "member_role_changed" => Some(AuditEventType::MemberRoleChanged),
        "org_created" => Some(AuditEventType::OrgCreated),
        "org_updated" => Some(AuditEventType::OrgUpdated),
        "org_deleted" => Some(AuditEventType::OrgDeleted),
        "org_plan_changed" => Some(AuditEventType::OrgPlanChanged),
        "billing_checkout" => Some(AuditEventType::BillingCheckout),
        "billing_upgrade" => Some(AuditEventType::BillingUpgrade),
        "billing_downgrade" => Some(AuditEventType::BillingDowngrade),
        "billing_canceled" => Some(AuditEventType::BillingCanceled),
        "api_token_created" => Some(AuditEventType::ApiTokenCreated),
        "api_token_deleted" => Some(AuditEventType::ApiTokenDeleted),
        "api_token_rotated" => Some(AuditEventType::ApiTokenRotated),
        "settings_updated" => Some(AuditEventType::SettingsUpdated),
        "byok_config_updated" => Some(AuditEventType::ByokConfigUpdated),
        "byok_config_deleted" => Some(AuditEventType::ByokConfigDeleted),
        "deployment_created" => Some(AuditEventType::DeploymentCreated),
        "deployment_deleted" => Some(AuditEventType::DeploymentDeleted),
        "deployment_updated" => Some(AuditEventType::DeploymentUpdated),
        "plugin_published" => Some(AuditEventType::PluginPublished),
        "template_published" => Some(AuditEventType::TemplatePublished),
        "scenario_published" => Some(AuditEventType::ScenarioPublished),
        "password_changed" => Some(AuditEventType::PasswordChanged),
        "email_changed" => Some(AuditEventType::EmailChanged),
        "two_factor_enabled" => Some(AuditEventType::TwoFactorEnabled),
        "two_factor_disabled" => Some(AuditEventType::TwoFactorDisabled),
        "admin_impersonation" => Some(AuditEventType::AdminImpersonation),
        _ => None,
    }
}
