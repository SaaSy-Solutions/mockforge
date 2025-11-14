//! Optional authentication extractor
//!
//! Allows extracting user context from public routes when authentication is present
//! but doesn't fail if authentication is missing

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use uuid::Uuid;

use crate::auth::verify_token;

/// Optional authenticated user ID
/// Returns None if no valid auth token is present
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<Uuid>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get user_id from request extensions (set by auth_middleware or api_token_auth_middleware)
        if let Some(user_id_str) = parts.extensions.get::<String>() {
            if let Ok(user_id) = Uuid::parse_str(user_id_str) {
                return Ok(OptionalAuthUser(Some(user_id)));
            }
        }

        // Try to extract from Authorization header directly (for optional auth on public routes)
        if let Some(auth_header) = parts.headers.get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..];
                    let secret = std::env::var("JWT_SECRET").ok();
                    if let Some(secret) = secret {
                        if let Ok(claims) = verify_token(token, &secret) {
                            if let Ok(user_id) = Uuid::parse_str(&claims.sub) {
                                return Ok(OptionalAuthUser(Some(user_id)));
                            }
                        }
                    }
                }
            }
        }

        // No authentication present - that's okay for optional auth
        Ok(OptionalAuthUser(None))
    }
}
