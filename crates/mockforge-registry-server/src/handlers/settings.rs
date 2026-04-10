//! Settings handlers
//!
//! Provides endpoints for managing user and organization settings, including BYOK configuration

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;

use crate::{
    cache::{org_setting_cache_key, ttl, Cache},
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{AuditEventType, BYOKConfig},
    AppState,
};

/// Encryption key salt for BYOK API keys (stable across restarts)
const BYOK_KEY_SALT: &[u8] = b"mockforge-byok-api-key-encryption";

/// Derive an encryption key for BYOK secrets from the BYOK_ENCRYPTION_KEY
/// environment variable (falls back to a derived key from JWT_SECRET).
fn get_byok_encryption_key() -> Result<mockforge_core::encryption::EncryptionKey, ApiError> {
    let secret = std::env::var("MOCKFORGE_BYOK_ENCRYPTION_KEY")
        .or_else(|_| std::env::var("JWT_SECRET"))
        .or_else(|_| std::env::var("MOCKFORGE_JWT_SECRET"))
        .unwrap_or_else(|_| "mockforge-default-key-change-me".to_string());

    mockforge_core::encryption::EncryptionKey::from_password_pbkdf2(
        &secret,
        Some(BYOK_KEY_SALT),
        mockforge_core::encryption::EncryptionAlgorithm::Aes256Gcm,
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to derive encryption key: {}", e)))
}

/// Encrypt a BYOK API key before storing
pub(crate) fn encrypt_api_key(api_key: &str) -> Result<String, ApiError> {
    if api_key.is_empty() {
        return Ok(String::new());
    }
    let key = get_byok_encryption_key()?;
    key.encrypt(api_key, None)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to encrypt API key: {}", e)))
}

/// Decrypt a stored BYOK API key
pub(crate) fn decrypt_api_key(encrypted: &str) -> Result<String, ApiError> {
    if encrypted.is_empty() {
        return Ok(String::new());
    }
    let key = get_byok_encryption_key()?;
    key.decrypt(encrypted, None)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to decrypt API key: {}", e)))
}

/// Query parameters for BYOK config retrieval
#[derive(Debug, Deserialize)]
pub struct BYOKQueryParams {
    /// Set to true to reveal the full API key (default: masked)
    #[serde(default)]
    pub reveal: bool,
}

/// Mask an API key, showing only the first 4 and last 4 characters
fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    format!("{}...{}", &key[..4], &key[key.len() - 4..])
}

/// Get BYOK configuration for the current organization
pub async fn get_byok_config(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Query(params): Query<BYOKQueryParams>,
) -> ApiResult<Json<BYOKConfig>> {
    // Resolve org context (BYOK is org-level)
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Try cache first
    let cache_key = org_setting_cache_key(&org_ctx.org_id, "byok");
    let store = state.store.clone();
    let mut config = if let Some(redis) = &state.redis {
        let cache = Cache::new(redis.clone());
        cache
            .get_or_set(&cache_key, ttl::SETTINGS, || {
                let store = store.clone();
                let org_id = org_ctx.org_id;
                async move {
                    let setting = store
                        .get_org_setting(org_id, "byok")
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
                            model: None,
                            enabled: false,
                        })
                    }
                }
            })
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("Cache error: {}", e)))?
    } else {
        // No Redis - fallback to database
        let setting = state.store.get_org_setting(org_ctx.org_id, "byok").await?;

        if let Some(setting) = setting {
            let config: BYOKConfig = serde_json::from_value(setting.setting_value)
                .map_err(|_| ApiError::Internal(anyhow::anyhow!("Invalid BYOK configuration")))?;
            config
        } else {
            BYOKConfig {
                provider: "openai".to_string(),
                api_key: String::new(),
                base_url: None,
                model: None,
                enabled: false,
            }
        }
    };

    // Decrypt the stored API key before returning
    let decrypted = decrypt_api_key(&config.api_key)?;
    config.api_key = if params.reveal {
        decrypted
    } else {
        mask_api_key(&decrypted)
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
    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Validate config
    if config.enabled && config.api_key.trim().is_empty() {
        return Err(ApiError::InvalidRequest(
            "API key is required when BYOK is enabled".to_string(),
        ));
    }

    if config.provider == "custom"
        && config.base_url.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
    {
        return Err(ApiError::InvalidRequest(
            "Base URL is required for custom provider".to_string(),
        ));
    }

    // Encrypt API key before storing
    let mut stored_config = config.clone();
    stored_config.api_key = encrypt_api_key(&config.api_key)?;
    let config_value = serde_json::to_value(&stored_config)
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to serialize config")))?;

    // Store setting
    state.store.set_org_setting(org_ctx.org_id, "byok", config_value).await?;

    // Invalidate cache
    if let Some(redis) = &state.redis {
        let cache = Cache::new(redis.clone());
        let cache_key = org_setting_cache_key(&org_ctx.org_id, "byok");
        let _ = cache.delete(&cache_key).await;
    }

    // Record audit log
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
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
    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Record audit log before deletion
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
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
    state.store.delete_org_setting(org_ctx.org_id, "byok").await?;

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

/// Request body for testing a BYOK connection
#[derive(Debug, Deserialize)]
pub struct TestBYOKRequest {
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

/// Test a BYOK API key by making a lightweight request to the provider
pub async fn test_byok_connection(
    AuthUser(_user_id): AuthUser,
    Json(request): Json<TestBYOKRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to create HTTP client: {}", e)))?;

    let (url, auth_header) = match request.provider.as_str() {
        "openai" => (
            "https://api.openai.com/v1/models".to_string(),
            format!("Bearer {}", request.api_key),
        ),
        "anthropic" => {
            ("https://api.anthropic.com/v1/messages".to_string(), request.api_key.clone())
        }
        "together" => (
            "https://api.together.xyz/v1/models".to_string(),
            format!("Bearer {}", request.api_key),
        ),
        "fireworks" => (
            "https://api.fireworks.ai/inference/v1/models".to_string(),
            format!("Bearer {}", request.api_key),
        ),
        "custom" => {
            let base = request
                .base_url
                .as_deref()
                .ok_or_else(|| {
                    ApiError::InvalidRequest("Base URL is required for custom provider".to_string())
                })?
                .trim_end_matches('/');
            (format!("{}/models", base), format!("Bearer {}", request.api_key))
        }
        _ => {
            return Err(ApiError::InvalidRequest(format!(
                "Unknown provider: {}",
                request.provider
            )));
        }
    };

    // Anthropic uses a different auth header and needs a POST with minimal body
    let response = if request.provider == "anthropic" {
        client
            .post(&url)
            .header("x-api-key", &request.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(r#"{"model":"claude-haiku-4-5-20251001","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#)
            .send()
            .await
    } else {
        client.get(&url).header("Authorization", &auth_header).send().await
    };

    match response {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": "Connection successful",
                    "provider": request.provider,
                })))
            } else {
                let body = resp.text().await.unwrap_or_default();
                Ok(Json(serde_json::json!({
                    "success": false,
                    "message": format!("Provider returned HTTP {}", status.as_u16()),
                    "details": body,
                })))
            }
        }
        Err(e) => Ok(Json(serde_json::json!({
            "success": false,
            "message": format!("Connection failed: {}", e),
        }))),
    }
}
