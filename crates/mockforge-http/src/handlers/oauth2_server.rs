//! OAuth2 server endpoints
//!
//! This module provides OAuth2 authorization server endpoints that integrate
//! with OIDC, token lifecycle, consent, and risk simulation.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Json, Redirect},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::oidc::{generate_oidc_token, OidcState, TenantContext};
use crate::auth::token_lifecycle::{extract_token_id, TokenLifecycleManager};
use chrono::Utc;
use hex;
use rand::Rng;
use serde_json::json;
use uuid;

/// OAuth2 server state
#[derive(Clone)]
pub struct OAuth2ServerState {
    /// OIDC state for token generation
    pub oidc_state: Arc<RwLock<Option<OidcState>>>,
    /// Token lifecycle manager
    pub lifecycle_manager: Arc<TokenLifecycleManager>,
    /// Authorization codes (code -> authorization info)
    pub auth_codes: Arc<RwLock<HashMap<String, AuthorizationCodeInfo>>>,
}

/// Authorization code information
#[derive(Debug, Clone)]
pub struct AuthorizationCodeInfo {
    /// Client ID
    pub client_id: String,
    /// Redirect URI
    pub redirect_uri: String,
    /// Scopes requested
    pub scopes: Vec<String>,
    /// User ID (subject)
    pub user_id: String,
    /// State parameter (CSRF protection)
    pub state: Option<String>,
    /// Expiration time
    pub expires_at: i64,
    /// Tenant context
    pub tenant_context: Option<TenantContext>,
}

/// OAuth2 authorization request parameters
#[derive(Debug, Deserialize)]
pub struct AuthorizationRequest {
    /// Client ID
    pub client_id: String,
    /// Response type (code, token, id_token)
    pub response_type: String,
    /// Redirect URI
    pub redirect_uri: String,
    /// Scopes (space-separated)
    pub scope: Option<String>,
    /// State parameter (CSRF protection)
    pub state: Option<String>,
    /// Nonce (for OpenID Connect)
    pub nonce: Option<String>,
}

/// OAuth2 token request
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    /// Grant type
    pub grant_type: String,
    /// Authorization code (for authorization_code grant)
    pub code: Option<String>,
    /// Redirect URI (must match authorization request)
    pub redirect_uri: Option<String>,
    /// Client ID
    pub client_id: Option<String>,
    /// Client secret
    pub client_secret: Option<String>,
    /// Scope (for client_credentials grant)
    pub scope: Option<String>,
    /// Nonce (for OpenID Connect)
    pub nonce: Option<String>,
}

/// OAuth2 token response
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    /// Access token
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Expires in (seconds)
    pub expires_in: i64,
    /// Refresh token (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Scope (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// ID token (for OpenID Connect)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
}

/// OAuth2 authorization endpoint
pub async fn authorize(
    State(state): State<OAuth2ServerState>,
    Query(params): Query<AuthorizationRequest>,
) -> Result<Redirect, StatusCode> {
    // Validate response_type
    if params.response_type != "code" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check if consent is required (simplified - in production, check user consent)
    // For now, auto-approve and generate authorization code

    // Generate authorization code before any await points (ThreadRng is not Send)
    let auth_code = {
        let mut rng = rand::thread_rng();
        let code_bytes: [u8; 32] = rng.gen();
        hex::encode(code_bytes)
    };

    // Parse scopes
    let scopes = params
        .scope
        .as_ref()
        .map(|s| s.split(' ').map(|s| s.to_string()).collect())
        .unwrap_or_else(Vec::new);

    // Store authorization code (expires in 10 minutes)
    let code_info = AuthorizationCodeInfo {
        client_id: params.client_id.clone(),
        redirect_uri: params.redirect_uri.clone(),
        scopes,
        // For mock server, use default user ID
        // In production, extract from authenticated session
        user_id: "user-default".to_string(),
        state: params.state.clone(),
        expires_at: Utc::now().timestamp() + 600, // 10 minutes
        // Tenant context can be extracted from request headers or session
        tenant_context: None,
    };

    {
        let mut codes = state.auth_codes.write().await;
        codes.insert(auth_code.clone(), code_info);
    }

    // Build redirect URL with authorization code
    let mut redirect_url =
        url::Url::parse(&params.redirect_uri).map_err(|_| StatusCode::BAD_REQUEST)?;
    redirect_url.query_pairs_mut().append_pair("code", &auth_code);
    if let Some(state) = params.state {
        redirect_url.query_pairs_mut().append_pair("state", &state);
    }

    Ok(Redirect::to(redirect_url.as_str()))
}

/// OAuth2 token endpoint
pub async fn token(
    State(state): State<OAuth2ServerState>,
    axum::extract::Form(request): axum::extract::Form<TokenRequest>,
) -> Result<Json<TokenResponse>, StatusCode> {
    use chrono::Utc;

    match request.grant_type.as_str() {
        "authorization_code" => handle_authorization_code_grant(state, request).await,
        "client_credentials" => handle_client_credentials_grant(state, request).await,
        "refresh_token" => handle_refresh_token_grant(state, request).await,
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// Handle authorization_code grant type
async fn handle_authorization_code_grant(
    state: OAuth2ServerState,
    request: TokenRequest,
) -> Result<Json<TokenResponse>, StatusCode> {
    let code = request.code.ok_or(StatusCode::BAD_REQUEST)?;
    let redirect_uri = request.redirect_uri.ok_or(StatusCode::BAD_REQUEST)?;

    // Look up authorization code
    let code_info = {
        let mut codes = state.auth_codes.write().await;
        codes.remove(&code).ok_or(StatusCode::BAD_REQUEST)?
    };

    // Validate redirect URI
    if code_info.redirect_uri != redirect_uri {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check expiration
    if code_info.expires_at < Utc::now().timestamp() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Generate access token using OIDC
    let oidc_state_guard = state.oidc_state.read().await;
    let oidc_state = oidc_state_guard.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build claims
    let mut additional_claims = HashMap::new();
    additional_claims.insert("scope".to_string(), json!(code_info.scopes.join(" ")));
    if let Some(nonce) = request.nonce {
        additional_claims.insert("nonce".to_string(), json!(nonce));
    }

    let access_token = generate_oidc_token(
        oidc_state,
        code_info.user_id.clone(),
        Some(additional_claims),
        Some(3600), // 1 hour expiration
        code_info.tenant_context.clone(),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Check if token is revoked (shouldn't be, but check anyway)
    let token_id = extract_token_id(&access_token);
    if state.lifecycle_manager.revocation.is_revoked(&token_id).await.is_some() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Generate refresh token (simplified)
    let refresh_token = format!("refresh_{}", uuid::Uuid::new_v4());

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: Some(refresh_token),
        scope: Some(code_info.scopes.join(" ")),
        // ID token generation for OpenID Connect can be added by calling generate_oidc_token
        // with appropriate OpenID Connect claims (sub, iss, aud, exp, iat, nonce, etc.)
        id_token: None,
    }))
}

/// Handle client_credentials grant type
async fn handle_client_credentials_grant(
    state: OAuth2ServerState,
    request: TokenRequest,
) -> Result<Json<TokenResponse>, StatusCode> {
    let client_id = request.client_id.ok_or(StatusCode::BAD_REQUEST)?;
    let _client_secret = request.client_secret.ok_or(StatusCode::BAD_REQUEST)?;

    // Validate client credentials (simplified - in production, check against database)

    // Generate access token
    let oidc_state_guard = state.oidc_state.read().await;
    let oidc_state = oidc_state_guard.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut additional_claims = HashMap::new();
    additional_claims.insert("client_id".to_string(), serde_json::json!(client_id));
    let scope_clone = request.scope.clone();
    if let Some(ref scope) = request.scope {
        additional_claims.insert("scope".to_string(), serde_json::json!(scope));
    }

    let access_token = generate_oidc_token(
        oidc_state,
        format!("client_{}", client_id),
        Some(additional_claims),
        Some(3600),
        None,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: None,
        scope: scope_clone,
        id_token: None,
    }))
}

/// Handle refresh_token grant type
async fn handle_refresh_token_grant(
    state: OAuth2ServerState,
    request: TokenRequest,
) -> Result<Json<TokenResponse>, StatusCode> {
    // For refresh token grant, we would:
    // 1. Validate the refresh token
    // 2. Check if it's revoked
    // 3. Generate a new access token
    // 4. Optionally generate a new refresh token

    // Simplified implementation - in production, validate refresh token from storage
    let client_id = request.client_id.ok_or(StatusCode::BAD_REQUEST)?;

    // Generate new access token
    let oidc_state_guard = state.oidc_state.read().await;
    let oidc_state = oidc_state_guard.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut additional_claims = HashMap::new();
    additional_claims.insert("client_id".to_string(), json!(client_id));
    let scope_clone = request.scope.clone();
    if let Some(ref scope) = request.scope {
        additional_claims.insert("scope".to_string(), json!(scope));
    }

    let access_token = generate_oidc_token(
        oidc_state,
        format!("client_{}", client_id),
        Some(additional_claims),
        Some(3600),
        None,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Generate new refresh token
    let refresh_token = format!("refresh_{}", uuid::Uuid::new_v4());

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: Some(refresh_token),
        scope: scope_clone,
        id_token: None,
    }))
}

/// Create OAuth2 server router
pub fn oauth2_server_router(state: OAuth2ServerState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/oauth2/authorize", get(authorize))
        .route("/oauth2/token", post(token))
        .with_state(state)
}
