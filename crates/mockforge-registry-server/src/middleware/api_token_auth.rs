//! API Token authentication middleware
//!
//! This middleware authenticates requests using Personal Access Tokens (PATs).
//! Tokens are passed in the Authorization header as "Bearer mfx_..." or "Token mfx_..."

use axum::http::StatusCode;
use uuid::Uuid;

use crate::{models::ApiToken, AppState};

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
