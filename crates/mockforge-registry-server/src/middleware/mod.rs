//! HTTP middleware

pub mod rate_limit;

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::auth::verify_token;

pub use rate_limit::{rate_limit_middleware, RateLimiterState};

/// Extract and verify JWT token from Authorization header
pub async fn auth_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    // Get JWT secret from app state (need to implement this properly)
    let secret = std::env::var("JWT_SECRET").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let claims = verify_token(token, &secret).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Add user_id to request extensions
    request.extensions_mut().insert(claims.sub.clone());

    Ok(next.run(request).await)
}
