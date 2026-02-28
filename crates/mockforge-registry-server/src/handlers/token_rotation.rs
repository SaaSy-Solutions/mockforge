//! API Token rotation handlers
//!
//! Provides endpoints for rotating API tokens and checking rotation status

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    email::EmailService,
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{record_audit_event, ApiToken, AuditEventType, Organization, User},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct RotateTokenRequest {
    pub new_name: Option<String>,
    pub delete_old: Option<bool>, // Default: false (keep old token)
}

#[derive(Debug, Serialize)]
pub struct RotateTokenResponse {
    pub success: bool,
    pub new_token: String, // Only shown once!
    pub new_token_id: Uuid,
    pub new_token_prefix: String,
    pub old_token_deleted: bool,
    pub message: String,
}

/// Rotate an API token (create new, optionally delete old)
pub async fn rotate_token(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(token_id): Path<Uuid>,
    Json(request): Json<RotateTokenRequest>,
) -> ApiResult<Json<RotateTokenResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify token belongs to org
    let old_token = ApiToken::find_by_id(pool, token_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Token not found".to_string()))?;

    if old_token.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Token does not belong to this organization".to_string(),
        ));
    }

    // Rotate token
    let delete_old = request.delete_old.unwrap_or(false);
    let (new_full_token, new_token, deleted_token) =
        ApiToken::rotate(pool, token_id, request.new_name.as_deref(), delete_old)
            .await
            .map_err(|e| ApiError::Database(e))?;

    // Record audit log
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::ApiTokenRotated,
        format!(
            "API token '{}' rotated{}",
            old_token.name,
            if delete_old {
                " (old token deleted)"
            } else {
                ""
            }
        ),
        Some(serde_json::json!({
            "old_token_id": token_id,
            "new_token_id": new_token.id,
            "old_token_name": old_token.name,
            "new_token_name": new_token.name,
            "delete_old": delete_old,
        })),
        ip_address,
        user_agent,
    )
    .await;

    Ok(Json(RotateTokenResponse {
        success: true,
        new_token: new_full_token, // Show full token only once!
        new_token_id: new_token.id,
        new_token_prefix: new_token.token_prefix,
        old_token_deleted: delete_old,
        message: if delete_old {
            format!("Token '{}' rotated and old token deleted", old_token.name)
        } else {
            format!("Token '{}' rotated. Old token is still active.", old_token.name)
        },
    }))
}

#[derive(Debug, Serialize)]
pub struct TokenRotationStatus {
    pub token_id: Uuid,
    pub name: String,
    pub token_prefix: String,
    pub age_days: i64,
    pub needs_rotation: bool,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct TokenRotationStatusResponse {
    pub tokens_needing_rotation: Vec<TokenRotationStatus>,
    pub rotation_threshold_days: i64,
}

#[derive(Debug, Deserialize)]
pub struct TokenRotationStatusQuery {
    pub threshold_days: Option<i64>, // Default: 90
}

/// Get tokens that need rotation
pub async fn get_tokens_needing_rotation(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Query(query): Query<TokenRotationStatusQuery>,
) -> ApiResult<Json<TokenRotationStatusResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let threshold_days = query.threshold_days.unwrap_or(90);

    // Get all tokens for org
    let all_tokens = ApiToken::find_by_org(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Filter tokens needing rotation
    let tokens_needing_rotation: Vec<TokenRotationStatus> = all_tokens
        .into_iter()
        .filter(|token| token.needs_rotation(threshold_days))
        .map(|token| {
            let age_days = token.age_days();
            TokenRotationStatus {
                token_id: token.id,
                name: token.name,
                token_prefix: token.token_prefix,
                age_days,
                needs_rotation: true,
                last_used_at: token.last_used_at,
                created_at: token.created_at,
            }
        })
        .collect();

    Ok(Json(TokenRotationStatusResponse {
        tokens_needing_rotation,
        rotation_threshold_days: threshold_days,
    }))
}

/// Background task: Send rotation reminders for tokens older than threshold
/// This should be called periodically (e.g., daily via cron or scheduled task)
pub async fn send_rotation_reminders(
    pool: &sqlx::PgPool,
    threshold_days: i64,
) -> Result<usize, anyhow::Error> {
    // Find all tokens needing rotation
    let tokens = ApiToken::find_tokens_needing_rotation(pool, None, threshold_days).await?;

    let email_service = EmailService::from_env()?;
    let mut reminders_sent = 0;

    for token in tokens {
        // Get org to find owner
        let org = Organization::find_by_id(pool, token.org_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Organization not found"))?;

        // Get user (owner or token creator)
        let user_id = token.user_id.unwrap_or(org.owner_id);
        let user = User::find_by_id(pool, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        // Build rotation URL
        let rotation_url = format!(
            "{}/settings/api-tokens/rotate/{}",
            std::env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "https://app.mockforge.dev".to_string()),
            token.id
        );

        // Send reminder email
        let email_msg = EmailService::generate_token_rotation_reminder(
            &user.username,
            &user.email,
            &token.name,
            token.age_days(),
            &rotation_url,
        );

        if let Err(e) = email_service.send(email_msg).await {
            tracing::warn!("Failed to send rotation reminder for token {}: {}", token.id, e);
        } else {
            reminders_sent += 1;
        }
    }

    Ok(reminders_sent)
}
