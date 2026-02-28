//! GDPR compliance handlers
//!
//! Implements data export and deletion endpoints for GDPR compliance

use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    models::{
        record_audit_event, ApiToken, AuditEventType, HostedMock, OrgMember, OrgSetting,
        Organization, Project, Subscription, UsageCounter, User, UserSetting,
    },
    AppState,
};

#[derive(Debug, Serialize)]
pub struct DataExportResponse {
    pub user: UserData,
    pub organizations: Vec<OrganizationData>,
    pub exported_at: String,
}

#[derive(Debug, Serialize)]
pub struct UserData {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_verified: bool,
    pub is_admin: bool,
    pub auth_provider: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub settings: Vec<SettingData>,
    pub api_tokens: Vec<ApiTokenData>,
}

#[derive(Debug, Serialize)]
pub struct OrganizationData {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub plan: String,
    pub limits: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
    pub role: String, // "owner" or "member"
    pub settings: Vec<SettingData>,
    pub projects: Vec<ProjectData>,
    pub subscriptions: Vec<SubscriptionData>,
    pub usage: Option<UsageData>,
    pub hosted_mocks: Vec<HostedMockData>,
}

#[derive(Debug, Serialize)]
pub struct SettingData {
    pub key: String,
    pub value: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ApiTokenData {
    pub id: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectData {
    pub id: String,
    pub name: String,
    pub visibility: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionData {
    pub id: String,
    pub plan: String,
    pub status: String,
    pub current_period_end: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct UsageData {
    pub requests: i64,
    pub storage_bytes: i64,
    pub ai_tokens_used: i64,
    pub period: String,
}

#[derive(Debug, Serialize)]
pub struct HostedMockData {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    pub confirm: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
    pub deleted_at: String,
}

/// Export all user data (GDPR right to data portability)
pub async fn export_data(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<DataExportResponse>> {
    let pool = state.db.pool();

    // Get user
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Get user settings
    let user_settings =
        sqlx::query_as::<_, UserSetting>("SELECT * FROM user_settings WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    // Get API tokens
    let api_tokens = sqlx::query_as::<_, ApiToken>("SELECT * FROM api_tokens WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Get organizations (owned and memberships)
    let orgs = Organization::find_by_user(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let mut org_data = Vec::new();

    for org in orgs {
        // Get role
        let membership = sqlx::query_as::<_, OrgMember>(
            "SELECT * FROM org_members WHERE org_id = $1 AND user_id = $2",
        )
        .bind(org.id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

        let role = membership.as_ref().map(|m| m.role.clone()).unwrap_or_else(|| {
            if org.owner_id == user_id {
                "owner".to_string()
            } else {
                "member".to_string()
            }
        });

        // Get org settings
        let org_settings =
            sqlx::query_as::<_, OrgSetting>("SELECT * FROM org_settings WHERE org_id = $1")
                .bind(org.id)
                .fetch_all(pool)
                .await
                .map_err(|e| ApiError::Database(e))?;

        // Get projects
        let projects = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE org_id = $1")
            .bind(org.id)
            .fetch_all(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

        // Get subscriptions
        let subscriptions =
            sqlx::query_as::<_, Subscription>("SELECT * FROM subscriptions WHERE org_id = $1")
                .bind(org.id)
                .fetch_all(pool)
                .await
                .map_err(|e| ApiError::Database(e))?;

        // Get usage
        let usage = UsageCounter::get_or_create_current(pool, org.id).await.ok();

        // Get hosted mocks
        let hosted_mocks =
            sqlx::query_as::<_, HostedMock>("SELECT * FROM hosted_mocks WHERE org_id = $1")
                .bind(org.id)
                .fetch_all(pool)
                .await
                .map_err(|e| ApiError::Database(e))?;

        org_data.push(OrganizationData {
            id: org.id.to_string(),
            name: org.name.clone(),
            slug: org.slug.clone(),
            plan: org.plan.clone(),
            limits: org.limits_json.clone(),
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
            role,
            settings: org_settings
                .into_iter()
                .map(|s| SettingData {
                    key: s.setting_key,
                    value: s.setting_value,
                    created_at: s.created_at.to_rfc3339(),
                    updated_at: s.updated_at.to_rfc3339(),
                })
                .collect(),
            projects: projects
                .into_iter()
                .map(|p| ProjectData {
                    id: p.id.to_string(),
                    name: p.name.clone(),
                    visibility: p.visibility.to_string(),
                    created_at: p.created_at.to_rfc3339(),
                    updated_at: p.updated_at.to_rfc3339(),
                })
                .collect(),
            subscriptions: subscriptions
                .into_iter()
                .map(|s| SubscriptionData {
                    id: s.id.to_string(),
                    plan: s.plan().to_string(),
                    status: s.status().to_string(),
                    current_period_end: Some(s.current_period_end.to_rfc3339()),
                    created_at: s.created_at.to_rfc3339(),
                })
                .collect(),
            usage: usage.map(|u| UsageData {
                requests: u.requests,
                storage_bytes: u.storage_bytes,
                ai_tokens_used: u.ai_tokens_used,
                period: u.period_start.format("%Y-%m").to_string(),
            }),
            hosted_mocks: hosted_mocks
                .into_iter()
                .map(|h| HostedMockData {
                    id: h.id.to_string(),
                    name: h.name.clone(),
                    slug: h.slug.clone(),
                    status: h.status().to_string(),
                    created_at: h.created_at.to_rfc3339(),
                    updated_at: h.updated_at.to_rfc3339(),
                })
                .collect(),
        });
    }

    Ok(Json(DataExportResponse {
        user: UserData {
            id: user.id.to_string(),
            username: user.username.clone(),
            email: user.email.clone(),
            is_verified: user.is_verified,
            is_admin: user.is_admin,
            auth_provider: None,
            avatar_url: None,
            created_at: user.created_at.to_rfc3339(),
            updated_at: user.updated_at.to_rfc3339(),
            settings: user_settings
                .into_iter()
                .map(|s| SettingData {
                    key: s.setting_key,
                    value: s.setting_value,
                    created_at: s.created_at.to_rfc3339(),
                    updated_at: s.updated_at.to_rfc3339(),
                })
                .collect(),
            api_tokens: api_tokens
                .into_iter()
                .map(|t| ApiTokenData {
                    id: t.id.to_string(),
                    name: t.name.clone(),
                    scopes: t.scopes.clone(),
                    created_at: t.created_at.to_rfc3339(),
                    last_used_at: t.last_used_at.map(|d| d.to_rfc3339()),
                })
                .collect(),
        },
        organizations: org_data,
        exported_at: Utc::now().to_rfc3339(),
    }))
}

/// Delete all user data (GDPR right to erasure)
///
/// This permanently deletes:
/// - User account
/// - Personal organization (if user is owner)
/// - All user settings
/// - All API tokens
/// - Organization memberships (but not orgs if user is not owner)
///
/// Note: If user owns organizations with other members, those orgs are NOT deleted.
/// The user is removed as owner and the org is transferred to the first admin or member.
pub async fn delete_data(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(request): Json<DeleteRequest>,
) -> ApiResult<Json<DeleteResponse>> {
    if !request.confirm {
        return Err(ApiError::InvalidRequest(
            "Deletion must be confirmed. Set 'confirm' to true.".to_string(),
        ));
    }

    let pool = state.db.pool();
    let mut tx = pool.begin().await.map_err(|e| ApiError::Database(e))?;

    // Get user
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    // Get organizations owned by user
    let owned_orgs =
        sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE owner_id = $1")
            .bind(user_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| ApiError::Database(e))?;

    // For each owned org, check if there are other members
    for org in &owned_orgs {
        let member_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM org_members WHERE org_id = $1 AND user_id != $2")
                .bind(org.id)
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| ApiError::Database(e))?;

        if member_count.0 > 0 {
            // Transfer ownership to first admin or member
            let new_owner = sqlx::query_as::<_, OrgMember>(
                "SELECT * FROM org_members WHERE org_id = $1 AND user_id != $2 ORDER BY CASE role WHEN 'admin' THEN 1 WHEN 'member' THEN 2 END LIMIT 1"
            )
            .bind(org.id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ApiError::Database(e))?;

            if let Some(new_owner_member) = new_owner {
                // Update org owner
                sqlx::query("UPDATE organizations SET owner_id = $1 WHERE id = $2")
                    .bind(new_owner_member.user_id)
                    .bind(org.id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| ApiError::Database(e))?;

                // Update member role to owner
                sqlx::query("UPDATE org_members SET role = 'owner' WHERE id = $1")
                    .bind(new_owner_member.id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| ApiError::Database(e))?;
            }
        } else {
            // No other members - delete the org and all its data
            // Note: This will cascade delete related data via foreign keys
            sqlx::query("DELETE FROM organizations WHERE id = $1")
                .bind(org.id)
                .execute(&mut *tx)
                .await
                .map_err(|e| ApiError::Database(e))?;
        }
    }

    // Remove user from all organization memberships
    sqlx::query("DELETE FROM org_members WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Delete user settings
    sqlx::query("DELETE FROM user_settings WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Delete API tokens
    sqlx::query("DELETE FROM api_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Delete user account
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Log deletion (for audit purposes)
    // In production, you might want to log this to a separate audit table
    tracing::info!(
        "User data deleted: user_id={}, email={}, reason={:?}",
        user_id,
        user.email,
        request.reason
    );

    tx.commit().await.map_err(|e| ApiError::Database(e))?;

    // Record audit event after commit (user is deleted, but this is compliance-required)
    record_audit_event(
        state.db.pool(),
        Uuid::nil(),
        Some(user_id),
        AuditEventType::OrgDeleted, // Reusing closest event type for data erasure
        format!("GDPR data erasure completed for user {}", user.email),
        Some(serde_json::json!({
            "action": "gdpr_data_erasure",
            "reason": request.reason,
            "orgs_affected": owned_orgs.len(),
        })),
        None,
        None,
    )
    .await;

    Ok(Json(DeleteResponse {
        success: true,
        message: "All user data has been permanently deleted.".to_string(),
        deleted_at: Utc::now().to_rfc3339(),
    }))
}
