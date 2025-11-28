//! Settings handlers
//!
//! Provides endpoints for managing user and organization settings, including BYOK configuration

use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    cache::{Cache, org_setting_cache_key, ttl},
    error::{ApiError, ApiResult},
    middleware::{AuthUser, resolve_org_context},
    models::{AuditEventType, OrgSetting, SuspiciousActivityType, UserSetting, BYOKConfig, record_audit_event, record_suspicious_activity},
    AppState,
};

/// Get BYOK configuration for the current organization
pub async fn get_byok_config(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<BYOKConfig>> {
    let pool = state.db.pool();

    // Resolve org context (BYOK is org-level)
    let org_ctx = resolve_org_context(&state, user_id, &headers, None).await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Try cache first
    let cache_key = org_setting_cache_key(&org_ctx.org_id, "byok");
    let config = if let Some(redis) = &state.redis {
        let cache = Cache::new(redis.clone());
        cache.get_or_set(
            &cache_key,
            ttl::SETTINGS,
            || async {
                let setting = OrgSetting::get(pool, org_ctx.org_id, "byok")
                    .await
                    .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;

                if let Some(setting) = setting {
                    let config: BYOKConfig = serde_json::from_value(setting.setting_value)
                        .map_err(|e| anyhow::anyhow!("Invalid BYOK configuration: {}", e))?;
                    Ok(config)
                } else {
                    Ok(BYOKConfig {
                        provider: "openai".to_string(),
                        api_key: String::new(),
                        base_url: None,
                        enabled: false,
                    })
                }
            },
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Cache error: {}", e)))?
    } else {
        // No Redis - fallback to database
        let setting = OrgSetting::get(pool, org_ctx.org_id, "byok")
            .await
            .map_err(|e| ApiError::Database(e))?;

        if let Some(setting) = setting {
            let config: BYOKConfig = serde_json::from_value(setting.setting_value)
                .map_err(|_| ApiError::Internal("Invalid BYOK configuration".to_string()))?;
            config
        } else {
            BYOKConfig {
                provider: "openai".to_string(),
                api_key: String::new(),
                base_url: None,
                enabled: false,
            }
        }
    };

    Ok(Json(config))
}

/// Update BYOK configuration for the current organization
pub async fn update_byok_config(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(config): Json<BYOKConfig>,
) -> ApiResult<Json<BYOKConfig>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None).await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Validate config
    if config.enabled && config.api_key.trim().is_empty() {
        return Err(ApiError::InvalidRequest(
            "API key is required when BYOK is enabled".to_string(),
        ));
    }

    if config.provider == "custom" && config.base_url.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
        return Err(ApiError::InvalidRequest(
            "Base URL is required for custom provider".to_string(),
        ));
    }

    // Encrypt API key before storing (in production, use proper encryption)
    // For now, we'll store it as-is, but in production you should encrypt it
    let config_value = serde_json::to_value(&config)
        .map_err(|_| ApiError::Internal("Failed to serialize config".to_string()))?;

    // Store setting
    OrgSetting::set(pool, org_ctx.org_id, "byok", config_value)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Invalidate cache
    if let Some(redis) = &state.redis {
        let cache = Cache::new(redis.clone());
        let cache_key = org_setting_cache_key(&org_ctx.org_id, "byok");
        let _ = cache.delete(&cache_key).await;
    }

    // Record audit log
    let ip_address = headers.get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent")
        .and_then(|h| h.to_str().ok());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::ByokConfigUpdated,
        format!("BYOK configuration updated for provider: {}", config.provider),
        Some(serde_json::json!({
            "provider": config.provider,
            "enabled": config.enabled,
            "has_base_url": config.base_url.is_some(),
        })),
        ip_address,
        user_agent,
    )
    .await;

    Ok(Json(config))
}

/// Delete BYOK configuration
pub async fn delete_byok_config(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None).await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Record audit log before deletion
    let ip_address = headers.get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent")
        .and_then(|h| h.to_str().ok());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::ByokConfigDeleted,
        "BYOK configuration deleted".to_string(),
        None,
        ip_address,
        user_agent,
    )
    .await;

    // Delete setting
    OrgSetting::delete(pool, org_ctx.org_id, "byok")
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Invalidate cache
    if let Some(redis) = &state.redis {
        let cache = Cache::new(redis.clone());
        let cache_key = org_setting_cache_key(&org_ctx.org_id, "byok");
        let _ = cache.delete(&cache_key).await;
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "BYOK configuration deleted"
    })))
}
