//! Scope checking utilities for API tokens
//!
//! Helpers to verify that API tokens have required scopes for specific operations

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use uuid::Uuid;

use crate::{
    error::ApiError,
    models::{ApiToken, TokenScope},
};

/// Marker type for API token authentication with scope checking
#[derive(Debug, Clone)]
pub struct ScopedAuth {
    /// The API token if present (None for JWT auth)
    pub token: Option<ApiToken>,
    /// Whether this request uses API token auth
    pub is_api_token: bool,
    /// The authenticated user ID
    pub user_id: Option<Uuid>,
}

impl<S> FromRequestParts<S> for ScopedAuth
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Check for API token in extensions
        let api_token = parts.extensions.get::<ApiToken>().cloned();

        // Check for auth type marker
        let is_api_token = parts
            .extensions
            .get::<AuthType>()
            .map(|t| matches!(t, AuthType::ApiToken))
            .unwrap_or(false);

        // Get user ID from extensions (set by auth middleware)
        let user_id = parts.extensions.get::<String>().and_then(|s| Uuid::parse_str(s).ok());

        Ok(ScopedAuth {
            token: api_token,
            is_api_token,
            user_id,
        })
    }
}

/// Marker enum for authentication type
#[derive(Debug, Clone)]
pub enum AuthType {
    Jwt,
    ApiToken,
}

impl ScopedAuth {
    /// Check if the request has the required scope
    ///
    /// For API tokens: checks if the token has the scope
    /// For JWT auth: always returns true (implicit all scopes)
    pub fn has_scope(&self, scope: &TokenScope) -> bool {
        match &self.token {
            Some(token) => token.has_scope(scope),
            None => true, // JWT auth has all scopes
        }
    }

    /// Require a specific scope, returning an error if not present
    pub fn require_scope(&self, scope: TokenScope) -> Result<(), ApiError> {
        if !self.is_api_token {
            // JWT auth has all scopes
            return Ok(());
        }

        match &self.token {
            Some(token) => {
                if token.has_scope(&scope) {
                    Ok(())
                } else {
                    Err(ApiError::InsufficientScope {
                        required: scope.to_string(),
                        scopes: token.scopes.clone(),
                    })
                }
            }
            None => {
                // No token but is_api_token is true - shouldn't happen
                Err(ApiError::InsufficientScope {
                    required: scope.to_string(),
                    scopes: vec![],
                })
            }
        }
    }

    /// Get the user ID if available
    pub fn user_id(&self) -> Option<Uuid> {
        if let Some(token) = &self.token {
            token.user_id
        } else {
            self.user_id
        }
    }
}

/// Check if the current request has an API token with the required scope
///
/// Returns the token if it has the scope, None otherwise
pub fn check_scope(
    request_extensions: &axum::http::Extensions,
    required_scope: TokenScope,
) -> Result<Option<ApiToken>, StatusCode> {
    // Get API token from extensions (set by api_token_auth_middleware)
    let token = request_extensions.get::<ApiToken>().cloned();

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

/// Get the authenticated user's org_id from request extensions
///
/// Works for both JWT and API token auth
pub fn get_org_id_from_extensions(request_extensions: &axum::http::Extensions) -> Option<Uuid> {
    // Try to get org_id from API token first
    request_extensions.get::<ApiToken>().map(|token| token.org_id)
}
