//! RBAC (Role-Based Access Control) middleware and permission enforcement
//!
//! This module provides middleware for enforcing role-based access control
//! on admin endpoints, ensuring users can only perform actions they're authorized for.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use mockforge_collab::permissions::{Permission, PermissionChecker, RolePermissions};
use mockforge_collab::models::UserRole;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User context extracted from request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// User ID
    pub user_id: String,
    /// Username
    pub username: String,
    /// User role
    pub role: UserRole,
    /// User email (optional)
    pub email: Option<String>,
}

/// Admin action to permission mapping
pub struct AdminActionPermissions;

impl AdminActionPermissions {
    /// Map admin action to required permissions
    /// Returns a list of permissions (user must have at least one if multiple)
    pub fn get_required_permissions(action: &str) -> Vec<Permission> {
        match action {
            // Configuration changes require ManageSettings
            "update_latency" | "update_faults" | "update_proxy" | "update_traffic_shaping" | "update_validation" => {
                vec![Permission::ManageSettings]
            }

            // Server management requires ManageSettings (admin only)
            "restart_servers" | "shutdown_servers" => {
                vec![Permission::ManageSettings]
            }

            // Log management requires ManageSettings
            "clear_logs" | "export_logs" => {
                vec![Permission::ManageSettings]
            }

            // Fixture management requires MockUpdate/MockDelete
            "create_fixture" => {
                vec![Permission::MockCreate]
            }
            "update_fixture" | "rename_fixture" | "move_fixture" => {
                vec![Permission::MockUpdate]
            }
            "delete_fixture" | "delete_fixtures_bulk" => {
                vec![Permission::MockDelete]
            }

            // Route management requires MockUpdate
            "enable_route" | "disable_route" | "create_route" | "update_route" | "delete_route" => {
                vec![Permission::MockUpdate]
            }

            // Service management requires ManageSettings
            "enable_service" | "disable_service" | "update_service_config" => {
                vec![Permission::ManageSettings]
            }

            // User management requires ChangeRoles
            "create_user" | "update_user" | "delete_user" | "change_role" => {
                vec![Permission::ChangeRoles]
            }

            // Permission management requires ChangeRoles
            "grant_permission" | "revoke_permission" => {
                vec![Permission::ChangeRoles]
            }

            // API key management requires ManageSettings
            "create_api_key" | "delete_api_key" | "rotate_api_key" => {
                vec![Permission::ManageSettings]
            }

            // Security operations require ManageSettings
            "update_security_policy" => {
                vec![Permission::ManageSettings]
            }

            // Read operations require appropriate read permissions
            "get_dashboard" | "get_logs" | "get_metrics" | "get_routes" | "get_fixtures" | "get_config" => {
                vec![Permission::WorkspaceRead, Permission::MockRead]
            }

            // Audit log access requires ManageSettings (sensitive)
            "get_audit_logs" | "get_audit_stats" => {
                vec![Permission::ManageSettings]
            }

            // Default: require ManageSettings for unknown actions
            _ => {
                vec![Permission::ManageSettings]
            }
        }
    }
}

/// Extract user context from request headers
/// Currently supports:
/// - Authorization: Bearer <token> (JWT with user info)
/// - X-User-Id, X-Username, X-User-Role headers (for development/testing)
pub fn extract_user_context(headers: &HeaderMap) -> Option<UserContext> {
    // Try to extract from Authorization header (JWT)
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                if let Some(user) = parse_jwt_token(token) {
                    return Some(user);
                }
            }
        }
    }

    // Fallback: Extract from custom headers (for development/testing)
    let user_id = headers.get("x-user-id")?.to_str().ok()?.to_string();
    let username = headers.get("x-username")?.to_str().ok()?.to_string();
    let role_str = headers.get("x-user-role")?.to_str().ok()?;
    let role = parse_role(role_str)?;

    Some(UserContext {
        user_id,
        username,
        role,
        email: headers.get("x-user-email")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string()),
    })
}

/// Parse JWT token and extract user context
/// Uses production JWT library (jsonwebtoken)
fn parse_jwt_token(token: &str) -> Option<UserContext> {
    use crate::auth::{validate_token, claims_to_user_context};

    // Try to validate as production JWT token
    if let Ok(claims) = validate_token(token) {
        return Some(claims_to_user_context(&claims));
    }

    // Fallback: handle mock tokens from the frontend (for backward compatibility)
    if token.starts_with("mock.") {
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
            use base64::{Engine as _, engine::general_purpose};
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
fn parse_jwt_payload(payload_str: &str) -> Option<UserContext> {
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
fn parse_role(role_str: &str) -> Option<UserRole> {
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
    if std::env::var("MOCKFORGE_ALLOW_UNAUTHENTICATED").is_ok() {
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

/// RBAC middleware to enforce permissions on admin endpoints
pub async fn rbac_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract action name from request path and HTTP method
    let path = request.uri().path();
    let method = request.method().as_str();
    let headers = request.headers();

    // Skip RBAC for public routes
    let is_public_route = path == "/"
        || path.starts_with("/assets/")
        || path.starts_with("/__mockforge/auth/")
        || path == "/__mockforge/health"
        || path.starts_with("/mockforge-")
        || path == "/manifest.json"
        || path == "/sw.js"
        || path == "/api-docs";

    if is_public_route {
        return Ok(next.run(request).await);
    }

    // Map route to action name
    let action_name = match (method, path) {
        (_, p) if p.contains("/config/latency") => "update_latency",
        (_, p) if p.contains("/config/faults") => "update_faults",
        (_, p) if p.contains("/config/proxy") => "update_proxy",
        (_, p) if p.contains("/config/traffic-shaping") => "update_traffic_shaping",
        ("DELETE", p) if p.contains("/logs") => "clear_logs",
        ("POST", p) if p.contains("/restart") => "restart_servers",
        ("DELETE", p) if p.contains("/fixtures") => "delete_fixture",
        ("POST", p) if p.contains("/fixtures") && p.contains("/rename") => "rename_fixture",
        ("POST", p) if p.contains("/fixtures") && p.contains("/move") => "move_fixture",
        ("GET", p) if p.contains("/audit/logs") => "get_audit_logs",
        ("GET", p) if p.contains("/audit/stats") => "get_audit_stats",
        ("GET", _) => "read", // Read operations
        _ => "unknown",
    };

    // Extract user context from request
    let user_context = extract_user_context(headers)
        .or_else(get_default_user_context);

    // If no user context and authentication is required, deny access
    let user_context = match user_context {
        Some(ctx) => ctx,
        None => {
            // For development: allow unauthenticated access if explicitly enabled
            // In production, this should be disabled
            if std::env::var("MOCKFORGE_ALLOW_UNAUTHENTICATED").is_ok() {
                // Use default admin context for development
                get_default_user_context().unwrap_or_else(|| UserContext {
                    user_id: "system".to_string(),
                    username: "system".to_string(),
                    role: UserRole::Admin,
                    email: None,
                })
            } else {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    };

    // Get required permissions for this action
    let required_permissions = AdminActionPermissions::get_required_permissions(action_name);

    // Check if user has at least one of the required permissions
    let has_permission = required_permissions.iter().any(|&perm| {
        RolePermissions::has_permission(user_context.role, perm)
    });

    if !has_permission {
        // Log authorization failure
        tracing::warn!(
            user_id = %user_context.user_id,
            username = %user_context.username,
            role = ?user_context.role,
            action = %action_name,
            "Authorization denied: User does not have required permissions"
        );

        return Err(StatusCode::FORBIDDEN);
    }

    // User has permission, continue with request
    // Store user context in request extensions for use in handlers
    request.extensions_mut().insert(user_context);

    Ok(next.run(request).await)
}

/// Helper to extract user context from request extensions
pub fn get_user_context_from_request(request: &Request) -> Option<UserContext> {
    request.extensions().get::<UserContext>().cloned()
}

/// Helper to get user context from axum State (if stored)
pub fn get_user_context_from_state<T>(state: &T) -> Option<UserContext>
where
    T: std::any::Any,
{
    // This is a placeholder - in practice, user context would be stored in request extensions
    None
}
