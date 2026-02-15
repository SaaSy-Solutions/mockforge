//! Organization settings handlers
//!
//! Provides endpoints for managing organization settings, usage stats, and billing info

use anyhow;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    models::{
        record_audit_event, AuditEventType, BYOKConfig, OrgAiSettings, OrgMember, OrgRole,
        OrgSetting, Organization, Subscription, User,
    },
    AppState,
};

/// Get organization settings
pub async fn get_organization_settings(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
) -> ApiResult<Json<OrganizationSettingsResponse>> {
    let pool = state.db.pool();

    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user has access
    if org.owner_id != user_id {
        let member = OrgMember::find(pool, org_id, user_id)
            .await
            .map_err(|e| ApiError::Database(e))?;
        if member.is_none() {
            return Err(ApiError::InvalidRequest(
                "You don't have access to this organization".to_string(),
            ));
        }
    }

    // Get BYOK settings (if any exist)
    let byok_setting =
        OrgSetting::get(pool, org_id, "byok").await.map_err(|e| ApiError::Database(e))?;

    let (byok_enabled, byok_provider) = if let Some(setting) = &byok_setting {
        let config: Result<BYOKConfig, _> = serde_json::from_value(setting.setting_value.clone());
        if let Ok(config) = config {
            (config.enabled, Some(config.provider))
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    Ok(Json(OrganizationSettingsResponse {
        org_id: org.id,
        org_name: org.name.clone(),
        org_slug: org.slug.clone(),
        plan: org.plan().to_string(),
        limits: org.limits_json.clone(),
        byok_enabled,
        byok_provider,
        created_at: org.created_at,
        updated_at: org.updated_at,
    }))
}

/// Update organization settings
pub async fn update_organization_settings(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    Json(request): Json<UpdateOrganizationSettingsRequest>,
) -> ApiResult<Json<OrganizationSettingsResponse>> {
    let pool = state.db.pool();

    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user is owner or admin
    let is_owner = org.owner_id == user_id;
    let is_admin = if !is_owner {
        if let Ok(Some(member)) = OrgMember::find(pool, org_id, user_id).await {
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

    // Update settings if provided
    if let Some(byok_api_key) = &request.byok_api_key {
        // Create or update BYOK config
        let provider = request.byok_provider.as_deref().unwrap_or("openai");
        let byok_config = BYOKConfig {
            provider: provider.to_string(),
            api_key: byok_api_key.clone(),
            base_url: None,
            enabled: true,
        };

        let config_value = serde_json::to_value(&byok_config).map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Failed to serialize BYOK config: {}", e))
        })?;

        OrgSetting::set(pool, org_id, "byok", config_value)
            .await
            .map_err(|e| ApiError::Database(e))?;
    }

    // Get updated organization and settings
    let updated_org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let byok_setting =
        OrgSetting::get(pool, org_id, "byok").await.map_err(|e| ApiError::Database(e))?;

    let (byok_enabled, byok_provider) = if let Some(setting) = &byok_setting {
        let config: Result<BYOKConfig, _> = serde_json::from_value(setting.setting_value.clone());
        if let Ok(config) = config {
            (config.enabled, Some(config.provider))
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    // Record audit event
    record_audit_event(
        pool,
        org_id,
        Some(user_id),
        AuditEventType::ByokConfigUpdated,
        "Organization BYOK settings updated".to_string(),
        Some(serde_json::json!({
            "byok_enabled": byok_enabled,
            "byok_provider": byok_provider,
        })),
        None,
        None,
    )
    .await;

    Ok(Json(OrganizationSettingsResponse {
        org_id: updated_org.id,
        org_name: updated_org.name.clone(),
        org_slug: updated_org.slug.clone(),
        plan: updated_org.plan().to_string(),
        limits: updated_org.limits_json.clone(),
        byok_enabled,
        byok_provider,
        created_at: updated_org.created_at,
        updated_at: updated_org.updated_at,
    }))
}

/// Get organization usage statistics
pub async fn get_organization_usage(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
) -> ApiResult<Json<OrganizationUsageResponse>> {
    let pool = state.db.pool();

    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user has access
    if org.owner_id != user_id {
        let member = OrgMember::find(pool, org_id, user_id)
            .await
            .map_err(|e| ApiError::Database(e))?;
        if member.is_none() {
            return Err(ApiError::InvalidRequest(
                "You don't have access to this organization".to_string(),
            ));
        }
    }

    // Get usage statistics
    let total_requests: (Option<i64>,) = sqlx::query_as(
        "SELECT COALESCE(SUM(requests), 0)::BIGINT FROM usage_counters WHERE org_id = $1",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let total_storage_gb: (Option<f64>,) = sqlx::query_as(
        "SELECT COALESCE(SUM(storage_bytes), 0)::FLOAT8 / 1073741824.0 FROM usage_counters WHERE org_id = $1",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let total_ai_tokens: (Option<i64>,) = sqlx::query_as(
        "SELECT COALESCE(SUM(ai_tokens_used), 0)::BIGINT FROM usage_counters WHERE org_id = $1",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Get feature usage counts
    let hosted_mocks_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM hosted_mocks WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    let plugins_published: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM plugins WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    let api_tokens_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM api_tokens WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    Ok(Json(OrganizationUsageResponse {
        org_id: org.id,
        total_requests: total_requests.0.unwrap_or(0),
        total_storage_gb: total_storage_gb.0.unwrap_or(0.0),
        total_ai_tokens: total_ai_tokens.0.unwrap_or(0),
        hosted_mocks_count: hosted_mocks_count.0,
        plugins_published: plugins_published.0,
        api_tokens_count: api_tokens_count.0,
    }))
}

/// Get organization billing information
pub async fn get_organization_billing(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
) -> ApiResult<Json<OrganizationBillingResponse>> {
    let pool = state.db.pool();

    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user is owner (billing info is sensitive)
    if org.owner_id != user_id {
        return Err(ApiError::PermissionDenied);
    }

    // Get subscription info
    let subscription = Subscription::find_by_org(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(OrganizationBillingResponse {
        org_id: org.id,
        plan: org.plan().to_string(),
        stripe_customer_id: org.stripe_customer_id.clone(),
        subscription: subscription.map(|s| SubscriptionInfo {
            id: s.id,
            status: s.status().to_string(),
            current_period_start: Some(s.current_period_start.date_naive()),
            current_period_end: Some(s.current_period_end.date_naive()),
            cancel_at_period_end: s.cancel_at_period_end,
        }),
    }))
}

#[derive(Debug, Serialize)]
pub struct OrganizationSettingsResponse {
    pub org_id: Uuid,
    pub org_name: String,
    pub org_slug: String,
    pub plan: String,
    pub limits: serde_json::Value,
    pub byok_enabled: bool,
    pub byok_provider: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrganizationSettingsRequest {
    pub byok_api_key: Option<String>,
    pub byok_provider: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrganizationUsageResponse {
    pub org_id: Uuid,
    pub total_requests: i64,
    pub total_storage_gb: f64,
    pub total_ai_tokens: i64,
    pub hosted_mocks_count: i64,
    pub plugins_published: i64,
    pub api_tokens_count: i64,
}

#[derive(Debug, Serialize)]
pub struct OrganizationBillingResponse {
    pub org_id: Uuid,
    pub plan: String,
    pub stripe_customer_id: Option<String>,
    pub subscription: Option<SubscriptionInfo>,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionInfo {
    pub id: Uuid,
    pub status: String,
    pub current_period_start: Option<chrono::NaiveDate>,
    pub current_period_end: Option<chrono::NaiveDate>,
    pub cancel_at_period_end: bool,
}

/// Get organization AI settings
pub async fn get_organization_ai_settings(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
) -> ApiResult<Json<OrgAiSettings>> {
    let pool = state.db.pool();

    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user has access
    if org.owner_id != user_id {
        let member = OrgMember::find(pool, org_id, user_id)
            .await
            .map_err(|e| ApiError::Database(e))?;
        if member.is_none() {
            return Err(ApiError::InvalidRequest(
                "You don't have access to this organization".to_string(),
            ));
        }
    }

    // Get AI settings
    let ai_setting = OrgSetting::get(pool, org_id, "ai_settings")
        .await
        .map_err(|e| ApiError::Database(e))?;

    let ai_settings = if let Some(setting) = &ai_setting {
        serde_json::from_value(setting.setting_value.clone())
            .unwrap_or_else(|_| OrgAiSettings::default())
    } else {
        OrgAiSettings::default()
    };

    Ok(Json(ai_settings))
}

/// Update organization AI settings
pub async fn update_organization_ai_settings(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    Json(request): Json<OrgAiSettings>,
) -> ApiResult<Json<OrgAiSettings>> {
    let pool = state.db.pool();

    // Get organization
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify user is owner or admin
    let is_owner = org.owner_id == user_id;
    let is_admin = if !is_owner {
        if let Ok(Some(member)) = OrgMember::find(pool, org_id, user_id).await {
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

    // Save AI settings
    let config_value = serde_json::to_value(&request).map_err(|e| {
        ApiError::Internal(anyhow::anyhow!("Failed to serialize AI settings: {}", e))
    })?;

    OrgSetting::set(pool, org_id, "ai_settings", config_value)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Record audit event
    record_audit_event(
        pool,
        org_id,
        Some(user_id),
        AuditEventType::SettingsUpdated,
        "Organization AI settings updated".to_string(),
        None,
        None,
        None,
    )
    .await;

    Ok(Json(request))
}
