//! HTTP middleware

pub mod api_token_auth;
pub mod csrf;
pub mod org_context;
pub mod permission_check;
// `permissions` module now lives in mockforge-registry-core. Re-exported
// here so existing `crate::middleware::permissions::*` paths keep working.
pub use mockforge_registry_core::permissions;
pub mod rate_limit;
pub mod request_id;
pub mod scope_check;
pub mod trusted_proxy;

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::auth::verify_token;
use crate::middleware::api_token_auth::authenticate_api_token;
use crate::AppState;

pub use org_context::resolve_org_context;
pub use rate_limit::rate_limit_middleware;
pub use scope_check::{AuthType, ScopedAuth};

/// JSON error response for authentication failures
fn auth_error_response(message: &str, hint: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": message,
            "error_code": "AUTH_REQUIRED",
            "status": 401,
            "details": { "hint": hint }
        })),
    )
        .into_response()
}

/// Extractor for authenticated user ID from JWT middleware
#[derive(Debug, Clone)]
pub struct AuthUser(pub Uuid);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get user_id from request extensions (set by auth_middleware)
        let user_id_str = parts.extensions.get::<String>().ok_or_else(|| {
            auth_error_response(
                "Authentication required",
                "Include a valid Authorization header with your request",
            )
        })?;

        // Parse UUID
        let user_id = Uuid::parse_str(user_id_str).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error",
                    "error_code": "INTERNAL_ERROR",
                    "status": 500
                })),
            )
                .into_response()
        })?;

        Ok(AuthUser(user_id))
    }
}

/// Optional authenticated user extractor
/// Returns None if no authentication is present, Some(user_id) if authenticated
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<Uuid>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get user_id from request extensions (set by auth_middleware)
        if let Some(user_id_str) = parts.extensions.get::<String>() {
            if let Ok(user_id) = Uuid::parse_str(user_id_str) {
                return Ok(OptionalAuthUser(Some(user_id)));
            }
        }
        Ok(OptionalAuthUser(None))
    }
}

/// Extract and verify JWT token or API token from Authorization header
pub async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    let auth_header =
        headers.get("Authorization").and_then(|h| h.to_str().ok()).ok_or_else(|| {
            auth_error_response(
                "Authentication required",
                "Include an Authorization: Bearer <token> header",
            )
        })?;

    // Extract the token using safe strip_prefix (no panic on invalid input)
    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        auth_error_response(
            "Invalid authorization format",
            "Use format: Authorization: Bearer <token>",
        )
    })?;

    // Check if this is an API token (starts with mfx_)
    if token.starts_with("mfx_") {
        match authenticate_api_token(&state, token).await.map_err(|_| {
            auth_error_response(
                "Authentication failed",
                "API token validation error. Please try again.",
            )
        })? {
            Some(auth_result) => {
                request.extensions_mut().insert(auth_result.user_id.to_string());
                request.extensions_mut().insert(AuthType::ApiToken);
                request.extensions_mut().insert(auth_result.token);
                return Ok(next.run(request).await);
            }
            None => {
                return Err(auth_error_response(
                    "Invalid API token",
                    "The API token is invalid or has been revoked. Generate a new one at https://app.mockforge.dev/settings/tokens",
                ));
            }
        }
    }

    // JWT authentication
    let claims = verify_token(token, &state.config.jwt_secret).map_err(|_| {
        auth_error_response(
            "Invalid or expired token",
            "Your session has expired. Please run 'mockforge cloud login' to re-authenticate.",
        )
    })?;

    // Add user_id to request extensions
    request.extensions_mut().insert(claims.sub.clone());
    request.extensions_mut().insert(AuthType::Jwt);

    Ok(next.run(request).await)
}
