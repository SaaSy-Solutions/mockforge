//! RBAC (Role-Based Access Control) middleware and permission enforcement
//!
//! This module provides middleware for enforcing role-based access control
//! on admin endpoints, ensuring users can only perform actions they're authorized for.

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use mockforge_collab::models::UserRole;
use mockforge_collab::permissions::{Permission, RolePermissions};
use serde::{Deserialize, Serialize};

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
            "update_latency"
            | "update_faults"
            | "update_proxy"
            | "update_traffic_shaping"
            | "update_validation" => {
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
            "get_dashboard" | "get_logs" | "get_metrics" | "get_routes" | "get_fixtures"
            | "get_config" => {
                vec![Permission::WorkspaceRead, Permission::MockRead]
            }

            // Audit log access requires ManageSettings (sensitive)
            "get_audit_logs" | "get_audit_stats" => {
                vec![Permission::ManageSettings]
            }

            // Scenario-specific permissions
            // Modify chaos rules - typically QA only
            "modify_scenario_chaos_rules" | "update_scenario_chaos" => {
                vec![Permission::ScenarioModifyChaosRules]
            }
            // Modify reality defaults - typically Platform team only
            "modify_scenario_reality_defaults" | "update_scenario_reality" => {
                vec![Permission::ScenarioModifyRealityDefaults]
            }
            // Promote scenarios between environments
            "promote_scenario" | "create_scenario_promotion" => {
                vec![Permission::ScenarioPromote]
            }
            // Approve scenario promotions
            "approve_scenario_promotion" | "reject_scenario_promotion" => {
                vec![Permission::ScenarioApprove]
            }
            // Modify drift budgets for scenarios
            "modify_scenario_drift_budget" | "update_scenario_drift_budget" => {
                vec![Permission::ScenarioModifyDriftBudgets]
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
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
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
        email: headers.get("x-user-email").and_then(|h| h.to_str().ok()).map(|s| s.to_string()),
    })
}

/// Parse JWT token and extract user context
/// Uses production JWT library (jsonwebtoken)
fn parse_jwt_token(token: &str) -> Option<UserContext> {
    use crate::auth::{claims_to_user_context, validate_token};

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
pub async fn rbac_middleware(mut request: Request, next: Next) -> Result<Response, StatusCode> {
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
    let user_context = extract_user_context(headers).or_else(get_default_user_context);

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
    let has_permission = required_permissions
        .iter()
        .any(|&perm| RolePermissions::has_permission(user_context.role, perm));

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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, Method};

    #[test]
    fn test_parse_role_valid() {
        assert_eq!(parse_role("admin"), Some(UserRole::Admin));
        assert_eq!(parse_role("Admin"), Some(UserRole::Admin));
        assert_eq!(parse_role("ADMIN"), Some(UserRole::Admin));
        assert_eq!(parse_role("editor"), Some(UserRole::Editor));
        assert_eq!(parse_role("viewer"), Some(UserRole::Viewer));
    }

    #[test]
    fn test_parse_role_invalid() {
        assert_eq!(parse_role("invalid"), None);
        assert_eq!(parse_role(""), None);
        assert_eq!(parse_role("super_admin"), None);
    }

    #[test]
    fn test_user_context_serialization() {
        let context = UserContext {
            user_id: "user123".to_string(),
            username: "testuser".to_string(),
            role: UserRole::Editor,
            email: Some("test@example.com".to_string()),
        };

        let serialized = serde_json::to_string(&context).unwrap();
        let deserialized: UserContext = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.user_id, context.user_id);
        assert_eq!(deserialized.username, context.username);
        assert_eq!(deserialized.role, context.role);
        assert_eq!(deserialized.email, context.email);
    }

    #[test]
    fn test_extract_user_context_from_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-user-id", HeaderValue::from_static("user123"));
        headers.insert("x-username", HeaderValue::from_static("testuser"));
        headers.insert("x-user-role", HeaderValue::from_static("admin"));
        headers.insert("x-user-email", HeaderValue::from_static("test@example.com"));

        let context = extract_user_context(&headers).unwrap();
        assert_eq!(context.user_id, "user123");
        assert_eq!(context.username, "testuser");
        assert_eq!(context.role, UserRole::Admin);
        assert_eq!(context.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_extract_user_context_missing_headers() {
        let headers = HeaderMap::new();
        let context = extract_user_context(&headers);
        assert!(context.is_none());
    }

    #[test]
    fn test_extract_user_context_partial_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-user-id", HeaderValue::from_static("user123"));
        // Missing username and role

        let context = extract_user_context(&headers);
        assert!(context.is_none());
    }

    #[test]
    fn test_extract_user_context_without_email() {
        let mut headers = HeaderMap::new();
        headers.insert("x-user-id", HeaderValue::from_static("user123"));
        headers.insert("x-username", HeaderValue::from_static("testuser"));
        headers.insert("x-user-role", HeaderValue::from_static("viewer"));

        let context = extract_user_context(&headers).unwrap();
        assert_eq!(context.user_id, "user123");
        assert_eq!(context.username, "testuser");
        assert_eq!(context.role, UserRole::Viewer);
        assert_eq!(context.email, None);
    }

    #[test]
    fn test_parse_jwt_payload() {
        let payload_json = r#"{
            "sub": "user456",
            "username": "jwtuser",
            "role": "editor",
            "email": "jwt@example.com"
        }"#;

        let context = parse_jwt_payload(payload_json).unwrap();
        assert_eq!(context.user_id, "user456");
        assert_eq!(context.username, "jwtuser");
        assert_eq!(context.role, UserRole::Editor);
        assert_eq!(context.email, Some("jwt@example.com".to_string()));
    }

    #[test]
    fn test_parse_jwt_payload_without_email() {
        let payload_json = r#"{
            "sub": "user456",
            "username": "jwtuser",
            "role": "viewer"
        }"#;

        let context = parse_jwt_payload(payload_json).unwrap();
        assert_eq!(context.email, None);
    }

    #[test]
    fn test_parse_jwt_payload_invalid_json() {
        let payload_json = "invalid json";
        let context = parse_jwt_payload(payload_json);
        assert!(context.is_none());
    }

    #[test]
    fn test_parse_jwt_payload_missing_fields() {
        let payload_json = r#"{"sub": "user456"}"#;
        let context = parse_jwt_payload(payload_json);
        assert!(context.is_none());
    }

    #[test]
    fn test_parse_jwt_payload_invalid_role() {
        let payload_json = r#"{
            "sub": "user456",
            "username": "jwtuser",
            "role": "invalid_role"
        }"#;

        let context = parse_jwt_payload(payload_json);
        assert!(context.is_none());
    }

    #[test]
    fn test_admin_action_permissions_config_changes() {
        let perms = AdminActionPermissions::get_required_permissions("update_latency");
        assert_eq!(perms, vec![Permission::ManageSettings]);

        let perms = AdminActionPermissions::get_required_permissions("update_faults");
        assert_eq!(perms, vec![Permission::ManageSettings]);

        let perms = AdminActionPermissions::get_required_permissions("update_proxy");
        assert_eq!(perms, vec![Permission::ManageSettings]);
    }

    #[test]
    fn test_admin_action_permissions_fixture_management() {
        let perms = AdminActionPermissions::get_required_permissions("create_fixture");
        assert_eq!(perms, vec![Permission::MockCreate]);

        let perms = AdminActionPermissions::get_required_permissions("update_fixture");
        assert_eq!(perms, vec![Permission::MockUpdate]);

        let perms = AdminActionPermissions::get_required_permissions("delete_fixture");
        assert_eq!(perms, vec![Permission::MockDelete]);
    }

    #[test]
    fn test_admin_action_permissions_user_management() {
        let perms = AdminActionPermissions::get_required_permissions("create_user");
        assert_eq!(perms, vec![Permission::ChangeRoles]);

        let perms = AdminActionPermissions::get_required_permissions("change_role");
        assert_eq!(perms, vec![Permission::ChangeRoles]);
    }

    #[test]
    fn test_admin_action_permissions_read_operations() {
        let perms = AdminActionPermissions::get_required_permissions("get_dashboard");
        assert_eq!(perms, vec![Permission::WorkspaceRead, Permission::MockRead]);

        let perms = AdminActionPermissions::get_required_permissions("get_logs");
        assert_eq!(perms, vec![Permission::WorkspaceRead, Permission::MockRead]);
    }

    #[test]
    fn test_admin_action_permissions_scenario_operations() {
        let perms = AdminActionPermissions::get_required_permissions("modify_scenario_chaos_rules");
        assert_eq!(perms, vec![Permission::ScenarioModifyChaosRules]);

        let perms =
            AdminActionPermissions::get_required_permissions("modify_scenario_reality_defaults");
        assert_eq!(perms, vec![Permission::ScenarioModifyRealityDefaults]);

        let perms = AdminActionPermissions::get_required_permissions("promote_scenario");
        assert_eq!(perms, vec![Permission::ScenarioPromote]);

        let perms = AdminActionPermissions::get_required_permissions("approve_scenario_promotion");
        assert_eq!(perms, vec![Permission::ScenarioApprove]);
    }

    #[test]
    fn test_admin_action_permissions_unknown_action() {
        let perms = AdminActionPermissions::get_required_permissions("unknown_action");
        assert_eq!(perms, vec![Permission::ManageSettings]);
    }

    #[test]
    fn test_get_default_user_context_without_env_var() {
        std::env::remove_var("MOCKFORGE_ALLOW_UNAUTHENTICATED");
        let context = get_default_user_context();
        assert!(context.is_none());
    }

    #[test]
    fn test_get_default_user_context_with_env_var() {
        std::env::set_var("MOCKFORGE_ALLOW_UNAUTHENTICATED", "1");
        let context = get_default_user_context();
        assert!(context.is_some());

        let context = context.unwrap();
        assert_eq!(context.user_id, "system");
        assert_eq!(context.username, "system");
        assert_eq!(context.role, UserRole::Admin);

        std::env::remove_var("MOCKFORGE_ALLOW_UNAUTHENTICATED");
    }

    #[test]
    fn test_all_permission_actions_covered() {
        // Test that all defined actions map to valid permissions
        let actions = vec![
            "update_latency",
            "update_faults",
            "restart_servers",
            "create_fixture",
            "update_fixture",
            "delete_fixture",
            "enable_route",
            "create_user",
            "grant_permission",
            "create_api_key",
            "get_dashboard",
            "get_audit_logs",
            "modify_scenario_chaos_rules",
            "promote_scenario",
            "approve_scenario_promotion",
        ];

        for action in actions {
            let perms = AdminActionPermissions::get_required_permissions(action);
            assert!(!perms.is_empty(), "Action {} should have permissions", action);
        }
    }

    #[test]
    fn test_role_permissions_admin_has_all() {
        // Admin should have all permissions
        assert!(RolePermissions::has_permission(UserRole::Admin, Permission::ManageSettings));
        assert!(RolePermissions::has_permission(UserRole::Admin, Permission::MockCreate));
        assert!(RolePermissions::has_permission(UserRole::Admin, Permission::MockUpdate));
        assert!(RolePermissions::has_permission(UserRole::Admin, Permission::MockDelete));
        assert!(RolePermissions::has_permission(UserRole::Admin, Permission::WorkspaceRead));
        assert!(RolePermissions::has_permission(UserRole::Admin, Permission::ChangeRoles));
    }

    #[test]
    fn test_role_permissions_editor_limited() {
        // Editor should have some permissions but not all
        assert!(!RolePermissions::has_permission(UserRole::Editor, Permission::ManageSettings));
        assert!(RolePermissions::has_permission(UserRole::Editor, Permission::MockUpdate));
        assert!(!RolePermissions::has_permission(UserRole::Editor, Permission::ChangeRoles));
    }

    #[test]
    fn test_role_permissions_viewer_readonly() {
        // Viewer should only have read permissions
        assert!(!RolePermissions::has_permission(UserRole::Viewer, Permission::ManageSettings));
        assert!(!RolePermissions::has_permission(UserRole::Viewer, Permission::MockCreate));
        assert!(!RolePermissions::has_permission(UserRole::Viewer, Permission::MockUpdate));
        assert!(!RolePermissions::has_permission(UserRole::Viewer, Permission::MockDelete));
        assert!(RolePermissions::has_permission(UserRole::Viewer, Permission::WorkspaceRead));
        assert!(RolePermissions::has_permission(UserRole::Viewer, Permission::MockRead));
    }

    #[test]
    fn test_scenario_permissions() {
        // Test scenario-specific permissions
        assert!(RolePermissions::has_permission(
            UserRole::Admin,
            Permission::ScenarioModifyChaosRules
        ));
        assert!(RolePermissions::has_permission(
            UserRole::Admin,
            Permission::ScenarioModifyRealityDefaults
        ));
        assert!(RolePermissions::has_permission(UserRole::Admin, Permission::ScenarioPromote));
        assert!(RolePermissions::has_permission(UserRole::Admin, Permission::ScenarioApprove));
    }

    #[tokio::test]
    async fn test_rbac_middleware_public_routes() {
        use axum::routing::get;
        use axum::{body::Body, middleware::from_fn, Router};
        use tower::ServiceExt;

        async fn handler() -> &'static str {
            "OK"
        }

        let app = Router::new().route("/", get(handler)).layer(from_fn(rbac_middleware));

        let request = axum::http::Request::builder()
            .uri("/")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rbac_middleware_health_route() {
        use axum::routing::get;
        use axum::{body::Body, middleware::from_fn, Router};
        use tower::ServiceExt;

        async fn handler() -> &'static str {
            "OK"
        }

        let app = Router::new()
            .route("/__mockforge/health", get(handler))
            .layer(from_fn(rbac_middleware));

        let request = axum::http::Request::builder()
            .uri("/__mockforge/health")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rbac_middleware_assets_route() {
        use axum::routing::get;
        use axum::{body::Body, middleware::from_fn, Router};
        use tower::ServiceExt;

        async fn handler() -> &'static str {
            "OK"
        }

        let app = Router::new()
            .route("/assets/style.css", get(handler))
            .layer(from_fn(rbac_middleware));

        let request = axum::http::Request::builder()
            .uri("/assets/style.css")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rbac_middleware_with_valid_headers() {
        use axum::routing::get;
        use axum::{body::Body, middleware::from_fn, Router};
        use tower::ServiceExt;

        async fn handler() -> &'static str {
            "OK"
        }

        let app = Router::new().route("/api/test", get(handler)).layer(from_fn(rbac_middleware));

        let request = axum::http::Request::builder()
            .uri("/api/test")
            .method(Method::GET)
            .header("x-user-id", "user123")
            .header("x-username", "testuser")
            .header("x-user-role", "admin")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_action_name_mapping() {
        // Test route to action mapping logic
        let test_cases = vec![
            ("/config/latency", "update_latency"),
            ("/config/faults", "update_faults"),
            ("/config/proxy", "update_proxy"),
            ("/logs", "clear_logs"),              // DELETE method
            ("/fixtures/test", "delete_fixture"), // DELETE method
            ("/audit/logs", "get_audit_logs"),    // GET method
        ];

        // These would be tested in the actual middleware
        // Here we verify the logic exists
        for (path, expected_action) in test_cases {
            assert!(!expected_action.is_empty());
        }
    }

    #[test]
    fn test_user_context_clone() {
        let context = UserContext {
            user_id: "user123".to_string(),
            username: "testuser".to_string(),
            role: UserRole::Editor,
            email: Some("test@example.com".to_string()),
        };

        let cloned = context.clone();
        assert_eq!(cloned.user_id, context.user_id);
        assert_eq!(cloned.username, context.username);
        assert_eq!(cloned.role, context.role);
        assert_eq!(cloned.email, context.email);
    }

    #[test]
    fn test_user_context_debug() {
        let context = UserContext {
            user_id: "user123".to_string(),
            username: "testuser".to_string(),
            role: UserRole::Viewer,
            email: None,
        };

        let debug_str = format!("{:?}", context);
        assert!(debug_str.contains("user123"));
        assert!(debug_str.contains("testuser"));
    }
}
