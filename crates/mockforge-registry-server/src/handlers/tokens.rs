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
    middleware::{resolve_org_context, AuthUser},
    models::{AuditEventType, FeatureType, SuspiciousActivityType, TokenScope},
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
    // Resolve org context (extensions not available in handler, use None)
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Parse scopes
    let scopes: Result<Vec<TokenScope>, _> = request
        .scopes
        .iter()
        .map(|s| {
            TokenScope::from_string(s)
                .ok_or_else(|| ApiError::InvalidRequest(format!("Invalid scope: {}", s)))
        })
        .collect();

    let scopes = scopes?;

    // Check for rapid token creation (suspicious activity detection)
    let recent_tokens = state.store.list_api_tokens_by_org(org_ctx.org_id).await?;

    let tokens_last_hour = recent_tokens
        .iter()
        .filter(|t| t.created_at > chrono::Utc::now() - chrono::Duration::hours(1))
        .count();

    // If more than 5 tokens created in the last hour, flag as suspicious
    if tokens_last_hour >= 5 {
        let ip_address = headers
            .get("X-Forwarded-For")
            .or_else(|| headers.get("X-Real-IP"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim());
        let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

        state
            .store
            .record_suspicious_activity(
                Some(org_ctx.org_id),
                Some(user_id),
                SuspiciousActivityType::RapidApiTokenCreation,
                "medium",
                format!(
                    "Rapid API token creation detected: {} tokens created in the last hour",
                    tokens_last_hour + 1
                ),
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
    let (full_token, token) = state
        .store
        .create_api_token(org_ctx.org_id, Some(user_id), &request.name, &scopes, request.expires_at)
        .await?;

    // Track feature usage
    state
        .store
        .record_feature_usage(
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
            AuditEventType::ApiTokenCreated,
            format!(
                "API token '{}' created with scopes: {}",
                request.name,
                request.scopes.join(", ")
            ),
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
    // Resolve org context (extensions not available in handler, use None)
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get all tokens
    let tokens = state.store.list_api_tokens_by_org(org_ctx.org_id).await?;

    let items: Vec<TokenListItem> = tokens
        .into_iter()
        .map(|t| {
            let age_days = t.age_days();
            let needs_rotation = t.needs_rotation(90); // 90 days threshold
            TokenListItem {
                id: t.id,
                name: t.name,
                token_prefix: t.token_prefix,
                scopes: t.scopes,
                last_used_at: t.last_used_at,
                expires_at: t.expires_at,
                created_at: t.created_at,
                age_days,
                needs_rotation,
            }
        })
        .collect();

    Ok(Json(items))
}

/// Available token scope, including human-readable label and description.
#[derive(Debug, Serialize)]
pub struct TokenScopeInfo {
    pub value: String,
    pub label: String,
    pub description: String,
}

/// Return the canonical list of token scopes.
///
/// Returns static metadata derived from the `TokenScope` enum so the UI
/// doesn't have to hardcode the list. Lives behind the auth middleware along
/// with the other token routes.
pub async fn list_scopes() -> Json<Vec<TokenScopeInfo>> {
    Json(vec![
        TokenScopeInfo {
            value: TokenScope::ReadPackages.to_string(),
            label: "Read Packages".to_string(),
            description: "Read and search packages".to_string(),
        },
        TokenScopeInfo {
            value: TokenScope::PublishPackages.to_string(),
            label: "Publish Packages".to_string(),
            description: "Publish new package versions".to_string(),
        },
        TokenScopeInfo {
            value: TokenScope::DeployMocks.to_string(),
            label: "Deploy Mocks".to_string(),
            description: "Deploy hosted mock services".to_string(),
        },
        TokenScopeInfo {
            value: TokenScope::AdminOrg.to_string(),
            label: "Admin Organization".to_string(),
            description: "Full organization administration".to_string(),
        },
        TokenScopeInfo {
            value: TokenScope::ReadUsage.to_string(),
            label: "Read Usage".to_string(),
            description: "Read usage analytics and metrics".to_string(),
        },
        TokenScopeInfo {
            value: TokenScope::ManageBilling.to_string(),
            label: "Manage Billing".to_string(),
            description: "Manage billing and subscription".to_string(),
        },
    ])
}

/// Delete an API token
pub async fn delete_token(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(token_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // Resolve org context (extensions not available in handler, use None)
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify token belongs to org
    let token = state
        .store
        .find_api_token_by_id(token_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Token not found".to_string()))?;

    if token.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Token does not belong to this organization".to_string(),
        ));
    }

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
    state.store.delete_api_token(token_id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
