//! Request identity extraction and explicitly gated development fallbacks.
use super::{
    allow_unauthenticated_dev_access, is_dev_auth_enabled, is_development_environment, UserContext,
    UserRole,
};
use axum::http::HeaderMap;

/// Extract user context from request headers
/// Currently supports:
/// - Authorization: Bearer <token> (JWT with user info)
/// - X-User-Id, X-Username, X-User-Role headers (for development/testing)
pub fn extract_user_context(headers: &HeaderMap) -> Option<UserContext> {
    // Try to extract from Authorization header (JWT)
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if let Some(user) = parse_jwt_token(token) {
                    return Some(user);
                }
            }
        }
    }

    // Fallback: Extract from custom headers (development/testing only)
    if is_development_environment() && is_dev_auth_enabled() {
        let user_id = headers.get("x-user-id")?.to_str().ok()?.to_string();
        let username = headers.get("x-username")?.to_str().ok()?.to_string();
        let role_str = headers.get("x-user-role")?.to_str().ok()?;
        let role = parse_role(role_str)?;

        return Some(UserContext {
            user_id,
            username,
            role,
            email: headers.get("x-user-email").and_then(|h| h.to_str().ok()).map(|s| s.to_string()),
        });
    }

    None
}

/// Parse JWT token and extract user context
/// Uses production JWT library (jsonwebtoken)
pub(super) fn parse_jwt_token(token: &str) -> Option<UserContext> {
    use crate::auth::{claims_to_user_context, validate_token};

    // Try to validate as production JWT token
    if let Ok(claims) = validate_token(token) {
        return Some(claims_to_user_context(&claims));
    }

    // Fallback: handle mock tokens from frontend (development/testing only)
    if is_development_environment() && is_dev_auth_enabled() && token.starts_with("mock.") {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() >= 3 {
            // Decode payload (base64url)
            let payload_part = parts[2];
            // Replace URL-safe characters for standard base64
            let base64_str = payload_part.replace('-', "+").replace('_', "/");
            // Add padding if needed
            let padding = (4 - (base64_str.len() % 4)) % 4;
            let padded = format!("{}{}", base64_str, "=".repeat(padding));

            // Decode base64
            use base64::{engine::general_purpose, Engine as _};
            if let Ok(decoded) = general_purpose::STANDARD.decode(&padded) {
                if let Ok(payload_str) = String::from_utf8(decoded) {
                    return parse_jwt_payload(&payload_str);
                }
            }
        }
    }

    None
}

/// Parse JWT payload JSON
pub(super) fn parse_jwt_payload(payload_str: &str) -> Option<UserContext> {
    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(payload_str) {
        let user_id = payload.get("sub")?.as_str()?.to_string();
        let username = payload.get("username")?.as_str()?.to_string();
        let role_str = payload.get("role")?.as_str()?;
        let role = parse_role(role_str)?;

        return Some(UserContext {
            user_id,
            username,
            role,
            email: payload.get("email").and_then(|v| v.as_str()).map(|s| s.to_string()),
        });
    }
    None
}

/// Parse role string to UserRole enum
pub(super) fn parse_role(role_str: &str) -> Option<UserRole> {
    match role_str.to_lowercase().as_str() {
        "admin" => Some(UserRole::Admin),
        "editor" => Some(UserRole::Editor),
        "viewer" => Some(UserRole::Viewer),
        _ => None,
    }
}

/// Default user context for unauthenticated requests (development mode)
/// In production, this should return None to enforce authentication
pub fn get_default_user_context() -> Option<UserContext> {
    // For development: allow unauthenticated access with admin role
    // In production, this should be disabled
    if allow_unauthenticated_dev_access() {
        Some(UserContext {
            user_id: "system".to_string(),
            username: "system".to_string(),
            role: UserRole::Admin,
            email: None,
        })
    } else {
        None
    }
}
