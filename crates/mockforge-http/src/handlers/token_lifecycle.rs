//! Token lifecycle test scenario handlers
//!
//! This module provides API endpoints for testing token lifecycle scenarios:
//! - Token revocation
//! - Key rotation
//! - Clock skew
//! - Prebuilt test scenarios

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::token_lifecycle::{
    extract_token_id, ClockSkewState, KeyRotationState, RevokedToken, TokenLifecycleManager,
    TokenRevocationStore,
};

/// State for token lifecycle handlers
#[derive(Clone)]
pub struct TokenLifecycleState {
    /// Token lifecycle manager
    pub manager: Arc<TokenLifecycleManager>,
}

/// Revoke token request
#[derive(Debug, Deserialize)]
pub struct RevokeTokenRequest {
    /// Token to revoke
    pub token: Option<String>,
    /// Token ID (jti claim)
    pub token_id: Option<String>,
    /// User ID (sub claim)
    pub user_id: Option<String>,
    /// Reason for revocation
    pub reason: String,
}

/// Revoke user tokens request
#[derive(Debug, Deserialize)]
pub struct RevokeUserTokensRequest {
    /// User ID
    pub user_id: String,
    /// Reason for revocation
    pub reason: String,
}

/// Key rotation request
#[derive(Debug, Deserialize)]
pub struct RotateKeyRequest {
    /// New key ID
    pub new_key_id: String,
    /// Grace period in seconds
    pub grace_period_seconds: Option<i64>,
}

/// Clock skew request
#[derive(Debug, Deserialize)]
pub struct ClockSkewRequest {
    /// Skew in seconds (positive = server ahead, negative = server behind)
    pub skew_seconds: i64,
    /// Duration in seconds (0 = permanent)
    pub duration_seconds: Option<u64>,
}

/// Force refresh failure request
#[derive(Debug, Deserialize)]
pub struct ForceRefreshFailureRequest {
    /// User ID
    pub user_id: String,
    /// Failure type
    pub failure_type: String,
}

/// Revoke mid-session request
#[derive(Debug, Deserialize)]
pub struct RevokeMidSessionRequest {
    /// User ID
    pub user_id: String,
    /// Delay in seconds before revocation
    pub delay_seconds: u64,
}

/// Revoke token endpoint
pub async fn revoke_token(
    State(state): State<TokenLifecycleState>,
    Json(request): Json<RevokeTokenRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let token_id = if let Some(token) = request.token {
        extract_token_id(&token)
    } else if let Some(tid) = request.token_id {
        tid
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };

    state
        .manager
        .revocation
        .revoke_token(
            token_id.clone(),
            request.user_id,
            request.reason,
            None, // expires_at not provided
        )
        .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "token_id": token_id,
        "message": "Token revoked successfully"
    })))
}

/// Revoke all tokens for a user
pub async fn revoke_user_tokens(
    State(state): State<TokenLifecycleState>,
    Json(request): Json<RevokeUserTokensRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state
        .manager
        .revocation
        .revoke_user_tokens(request.user_id.clone(), request.reason)
        .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": request.user_id,
        "message": "All user tokens revoked successfully"
    })))
}

/// Get token revocation status
pub async fn get_token_status(
    State(state): State<TokenLifecycleState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let token_id = if let Some(token) = params.get("token") {
        extract_token_id(token)
    } else if let Some(tid) = params.get("token_id") {
        tid.clone()
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };

    if let Some(revoked) = state.manager.revocation.get_revocation_status(&token_id).await {
        Ok(Json(serde_json::json!({
            "revoked": true,
            "revoked_at": revoked.revoked_at,
            "reason": revoked.reason
        })))
    } else {
        Ok(Json(serde_json::json!({
            "revoked": false
        })))
    }
}

/// Rotate keys
pub async fn rotate_keys(
    State(state): State<TokenLifecycleState>,
    Json(request): Json<RotateKeyRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Update the key rotation state
    // Note: Actual key rotation in OIDC state would require access to OidcState
    // which is managed separately. This endpoint manages the rotation lifecycle
    // and the OIDC state should be updated separately via configuration.
    state
        .manager
        .key_rotation
        .rotate_key(request.new_key_id.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "new_key_id": request.new_key_id,
        "message": "Key rotation initiated. Update OIDC configuration to use the new key."
    })))
}

/// Get active keys
pub async fn get_active_keys(
    State(state): State<TokenLifecycleState>,
) -> Json<serde_json::Value> {
    let keys = state.manager.key_rotation.get_active_keys().await;
    Json(serde_json::json!({
        "keys": keys.iter().map(|k| serde_json::json!({
            "kid": k.kid,
            "created_at": k.created_at,
            "inactive_at": k.inactive_at,
            "is_primary": k.is_primary
        })).collect::<Vec<_>>()
    }))
}

/// Set clock skew
pub async fn set_clock_skew(
    State(state): State<TokenLifecycleState>,
    Json(request): Json<ClockSkewRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state.manager.clock_skew.set_skew(request.skew_seconds).await;

    // If duration is specified, schedule reset
    if let Some(duration) = request.duration_seconds {
        let state_clone = state.clone();
        let skew_value = request.skew_seconds;
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;
            state_clone.manager.clock_skew.set_skew(0).await;
        });
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "skew_seconds": request.skew_seconds,
        "message": "Clock skew set successfully"
    })))
}

/// Get clock skew
pub async fn get_clock_skew(
    State(state): State<TokenLifecycleState>,
) -> Json<serde_json::Value> {
    let skew = state.manager.clock_skew.get_skew().await;
    let adjusted_time = state.manager.clock_skew.get_adjusted_time().await;
    let server_time = chrono::Utc::now().timestamp();

    Json(serde_json::json!({
        "skew_seconds": skew,
        "server_time": server_time,
        "adjusted_time": adjusted_time
    }))
}

/// Force refresh token failure (test scenario)
pub async fn force_refresh_failure(
    State(state): State<TokenLifecycleState>,
    Json(request): Json<ForceRefreshFailureRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Revoke all tokens for the user to simulate refresh failure
    let reason = format!("test_scenario:{}", request.failure_type);
    state
        .manager
        .revocation
        .revoke_user_tokens(request.user_id.clone(), reason)
        .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": request.user_id,
        "failure_type": request.failure_type,
        "message": "Refresh token failure simulated"
    })))
}

/// Revoke token mid-session (test scenario)
pub async fn revoke_mid_session(
    State(state): State<TokenLifecycleState>,
    Json(request): Json<RevokeMidSessionRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let state_clone = state.clone();
    let user_id = request.user_id.clone();
    let delay = request.delay_seconds;

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
        state_clone
            .manager
            .revocation
            .revoke_user_tokens(user_id, "mid_session_revocation".to_string())
            .await;
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": request.user_id,
        "delay_seconds": request.delay_seconds,
        "message": format!("Token will be revoked in {} seconds", request.delay_seconds)
    })))
}

/// Create token lifecycle router
pub fn token_lifecycle_router(state: TokenLifecycleState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/tokens/revoke", post(revoke_token))
        .route("/tokens/revoke/user", post(revoke_user_tokens))
        .route("/tokens/status", get(get_token_status))
        .route("/keys/rotate", post(rotate_keys))
        .route("/keys/active", get(get_active_keys))
        .route("/clock/skew", post(set_clock_skew))
        .route("/clock/skew", get(get_clock_skew))
        .route("/test/force-refresh-failure", post(force_refresh_failure))
        .route("/test/revoke-mid-session", post(revoke_mid_session))
        .with_state(state)
}
