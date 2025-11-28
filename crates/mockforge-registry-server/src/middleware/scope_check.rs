//! Scope checking utilities for API tokens
//!
//! Helpers to verify that API tokens have required scopes for specific operations

use axum::http::StatusCode;
use uuid::Uuid;

use crate::{
    middleware::api_token_auth::TokenAuthResult,
    models::{ApiToken, TokenScope},
};

/// Check if the current request has an API token with the required scope
///
/// Returns the token if it has the scope, None otherwise
pub fn check_scope(
    request_extensions: &axum::http::Extensions,
    required_scope: TokenScope,
) -> Result<Option<ApiToken>, StatusCode> {
    // Get API token from extensions (set by api_token_auth_middleware)
    let token = request_extensions
        .get::<ApiToken>()
        .cloned();

    match token {
        Some(t) => {
            if t.has_scope(&required_scope) {
                Ok(Some(t))
            } else {
                Err(StatusCode::FORBIDDEN)
            }
        }
        None => {
            // No API token, might be JWT auth (which has all scopes)
            // Return None to indicate no scope check needed
            Ok(None)
        }
    }
}

/// Require a specific scope for API token requests
///
/// If the request uses an API token, it must have the required scope.
/// JWT requests are allowed (they have implicit all scopes).
pub fn require_scope(
    request_extensions: &axum::http::Extensions,
    required_scope: TokenScope,
) -> Result<(), StatusCode> {
    // Check if this is an API token request
    let is_api_token = request_extensions
        .get::<String>()
        .map(|s| s.starts_with("auth_type:api_token"))
        .unwrap_or(false);

    if !is_api_token {
        // JWT request, allow (implicit all scopes)
        return Ok(());
    }

    // API token request, check scope
    check_scope(request_extensions, required_scope)?;
    Ok(())
}

/// Get the authenticated user's org_id from request extensions
///
/// Works for both JWT and API token auth
pub fn get_org_id_from_extensions(
    request_extensions: &axum::http::Extensions,
) -> Option<Uuid> {
    // Try to get org_id from API token first
    if let Some(org_id_str) = request_extensions
        .iter()
        .find_map(|(_, ext)| {
            ext.downcast_ref::<String>()
                .and_then(|s| {
                    if s.starts_with("org_id:") {
                        Uuid::parse_str(&s[7..]).ok()
                    } else {
                        None
                    }
                })
        })
    {
        return Some(org_id_str);
    }

    None
}
