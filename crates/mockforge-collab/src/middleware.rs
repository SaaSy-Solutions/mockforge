//! Middleware for authentication and authorization

use crate::auth::AuthService;
use crate::error::CollabError;
use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use uuid::Uuid;

/// Extension for authenticated user
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub username: String,
}

/// Implement extractor for AuthUser
#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthUser>()
            .cloned()
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Not authenticated".to_string()))
    }
}

/// JWT authentication middleware
pub async fn auth_middleware(
    State(auth): State<Arc<AuthService>>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?;

    // Check Bearer prefix
    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "Invalid Authorization header format".to_string())
    })?;

    // Verify token
    let claims = auth
        .verify_token(token)
        .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Invalid token: {}", e)))?;

    // Parse user ID
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Invalid user ID in token".to_string()))?;

    // Add user to request extensions
    request.extensions_mut().insert(AuthUser {
        user_id,
        username: claims.username,
    });

    Ok(next.run(request).await)
}

/// Extract authenticated user from request
pub fn extract_auth_user(request: &Request) -> Result<&AuthUser, CollabError> {
    request
        .extensions()
        .get::<AuthUser>()
        .ok_or_else(|| CollabError::AuthenticationFailed("Not authenticated".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_user_creation() {
        let user = AuthUser {
            user_id: Uuid::new_v4(),
            username: "testuser".to_string(),
        };

        assert_eq!(user.username, "testuser");
    }
}
