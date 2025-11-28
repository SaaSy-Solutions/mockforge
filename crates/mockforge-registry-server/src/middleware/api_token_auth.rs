//! API Token authentication middleware
//!
//! This middleware authenticates requests using Personal Access Tokens (PATs).
//! Tokens are passed in the Authorization header as "Bearer mfx_..." or "Token mfx_..."

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{
    models::{ApiToken, User},
    AppState,
};

/// Authentication result containing user and token info
#[derive(Debug, Clone)]
pub struct TokenAuthResult {
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub token: ApiToken,
}

/// Authenticate request using API token
/// Returns (user_id, org_id, token) if valid, None otherwise
pub async fn authenticate_api_token(
    state: &AppState,
    token: &str,
) -> Result<Option<TokenAuthResult>, StatusCode> {
    let pool = state.db.pool();

    // Verify token
    let api_token = ApiToken::verify_token(pool, token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let api_token = match api_token {
        Some(t) => t,
        None => return Ok(None),
    };

    // Check if token is expired
    if let Some(expires_at) = api_token.expires_at {
        if expires_at < chrono::Utc::now() {
            return Ok(None); // Token expired
        }
    }

    // Get user_id from token (if associated) or from org owner
    let user_id = if let Some(uid) = api_token.user_id {
        uid
    } else {
        // If token has no user_id, get org owner
        use crate::models::Organization;
        let org = Organization::find_by_id(pool, api_token.org_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::UNAUTHORIZED)?;
        org.owner_id
    };

    Ok(Some(TokenAuthResult {
        user_id,
        org_id: api_token.org_id,
        token: api_token,
    }))
}

/// API Token authentication middleware
///
/// Supports both "Bearer mfx_..." and "Token mfx_..." formats
/// Falls back to JWT if token doesn't start with "mfx_"
pub async fn api_token_auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check if Authorization header exists
    let auth_header = match headers.get("Authorization") {
        Some(h) => h.to_str().map_err(|_| StatusCode::BAD_REQUEST)?,
        None => {
            // No auth header, let JWT middleware handle it
            return Ok(next.run(request).await);
        }
    };

    // Check if it's an API token (starts with "Bearer mfx_" or "Token mfx_")
    let token = if auth_header.starts_with("Bearer mfx_") {
        &auth_header[7..] // Skip "Bearer "
    } else if auth_header.starts_with("Token mfx_") {
        &auth_header[6..] // Skip "Token "
    } else if auth_header.starts_with("mfx_") {
        auth_header // Direct token format
    } else {
        // Not an API token, let JWT middleware handle it
        return Ok(next.run(request).await);
    };

    // Authenticate API token
    match authenticate_api_token(&state, token).await {
        Ok(Some(auth_result)) => {
            // Add user_id and org_id to request extensions
            request.extensions_mut().insert(auth_result.user_id.to_string());
            request.extensions_mut().insert(format!("org_id:{}", auth_result.org_id));
            request.extensions_mut().insert(format!("auth_type:api_token"));
            request.extensions_mut().insert(auth_result.token);

            Ok(next.run(request).await)
        }
        Ok(None) => {
            // Invalid token, but let JWT middleware try
            // (in case user wants to use JWT instead)
            Ok(next.run(request).await)
        }
        Err(e) => Err(e),
    }
}
