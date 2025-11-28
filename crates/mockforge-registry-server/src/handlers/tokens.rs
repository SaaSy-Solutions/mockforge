//! API Token (Personal Access Token) handlers

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{AuthUser, resolve_org_context},
    models::{ApiToken, AuditEventType, FeatureType, FeatureUsage, SuspiciousActivityType, TokenScope, record_audit_event, record_suspicious_activity},
    AppState,
};

/// Create a new API token
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct CreateTokenResponse {
    pub token: String, // Only shown once!
    pub token_id: Uuid,
    pub token_prefix: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_token(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<CreateTokenRequest>,
) -> ApiResult<Json<CreateTokenResponse>> {
    let pool = state.db.pool();

    // Resolve org context (extensions not available in handler, use None)
    let org_ctx = resolve_org_context(&state, user_id, &headers, None).await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Parse scopes
    let scopes: Result<Vec<TokenScope>, _> = request.scopes
        .iter()
        .map(|s| TokenScope::from_string(s).ok_or_else(|| ApiError::InvalidRequest(format!("Invalid scope: {}", s))))
        .collect();

    let scopes = scopes?;

    // Check for rapid token creation (suspicious activity detection)
    let recent_tokens = ApiToken::find_by_org(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let tokens_last_hour = recent_tokens.iter()
        .filter(|t| t.created_at > chrono::Utc::now() - chrono::Duration::hours(1))
        .count();

    // If more than 5 tokens created in the last hour, flag as suspicious
    if tokens_last_hour >= 5 {
        let ip_address = headers.get("X-Forwarded-For")
            .or_else(|| headers.get("X-Real-IP"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim());
        let user_agent = headers.get("User-Agent")
            .and_then(|h| h.to_str().ok());

        record_suspicious_activity(
            pool,
            Some(org_ctx.org_id),
            Some(user_id),
            SuspiciousActivityType::RapidApiTokenCreation,
            "medium",
            format!("Rapid API token creation detected: {} tokens created in the last hour", tokens_last_hour + 1),
            Some(serde_json::json!({
                "tokens_created_last_hour": tokens_last_hour + 1,
                "new_token_name": request.name,
            })),
            ip_address,
            user_agent,
        )
        .await;
    }

    // Create token
    let (full_token, token) = ApiToken::create(
        pool,
        org_ctx.org_id,
        Some(user_id),
        &request.name,
        &scopes,
        request.expires_at,
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Track feature usage
    let _ = FeatureUsage::record(
        pool,
        org_ctx.org_id,
        Some(user_id),
        FeatureType::ApiTokenCreate,
        Some(serde_json::json!({
            "token_id": token.id,
            "name": request.name,
            "scopes": request.scopes,
        })),
    )
    .await;

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
        AuditEventType::ApiTokenCreated,
        format!("API token '{}' created with scopes: {}", request.name, request.scopes.join(", ")),
        Some(serde_json::json!({
            "token_id": token.id,
            "token_name": request.name,
            "scopes": request.scopes,
        })),
        ip_address,
        user_agent,
    )
    .await;

    Ok(Json(CreateTokenResponse {
        token: full_token, // Show full token only once!
        token_id: token.id,
        token_prefix: token.token_prefix,
        name: token.name,
        scopes: token.scopes,
        expires_at: token.expires_at,
        created_at: token.created_at,
    }))
}

/// List all API tokens for an organization
#[derive(Debug, Serialize)]
pub struct TokenListItem {
    pub id: Uuid,
    pub name: String,
    pub token_prefix: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub age_days: i64,
    pub needs_rotation: bool, // True if token is older than 90 days
}

pub async fn list_tokens(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<TokenListItem>>> {
    let pool = state.db.pool();

    // Resolve org context (extensions not available in handler, use None)
    let org_ctx = resolve_org_context(&state, user_id, &headers, None).await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get all tokens
    let tokens = ApiToken::find_by_org(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let items: Vec<TokenListItem> = tokens
        .into_iter()
        .map(|t| {
            let age_days = t.age_days();
            TokenListItem {
                id: t.id,
                name: t.name,
                token_prefix: t.token_prefix,
                scopes: t.scopes,
                last_used_at: t.last_used_at,
                expires_at: t.expires_at,
                created_at: t.created_at,
                age_days,
                needs_rotation: t.needs_rotation(90), // 90 days threshold
            }
        })
        .collect();

    Ok(Json(items))
}

/// Delete an API token
pub async fn delete_token(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(token_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db.pool();

    // Resolve org context (extensions not available in handler, use None)
    let org_ctx = resolve_org_context(&state, user_id, &headers, None).await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify token belongs to org
    let token = ApiToken::find_by_id(pool, token_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Token not found".to_string()))?;

    if token.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest("Token does not belong to this organization".to_string()));
    }

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
        AuditEventType::ApiTokenDeleted,
        format!("API token '{}' deleted", token.name),
        Some(serde_json::json!({
            "token_id": token.id,
            "token_name": token.name,
            "token_prefix": token.token_prefix,
        })),
        ip_address,
        user_agent,
    )
    .await;

    // Delete token
    ApiToken::delete(pool, token_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(serde_json::json!({ "success": true })))
}
