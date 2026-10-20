//! RBAC (Role-Based Access Control) middleware and permission enforcement
//!
//! This module provides middleware for enforcing role-based access control
//! on admin endpoints, ensuring users can only perform actions they're authorized for.

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use mockforge_collab::models::UserRole;
use mockforge_collab::permissions::{Permission, RolePermissions};
use serde::{Deserialize, Serialize};

use crate::admin_access::AdminAccessPolicy;

mod identity;
use identity::parse_jwt_token;
pub use identity::{extract_user_context, get_default_user_context};
#[cfg(test)]
use identity::{parse_jwt_payload, parse_role};

fn is_truthy_env(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref().map(str::to_ascii_lowercase).as_deref(),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}

fn is_development_environment() -> bool {
    if cfg!(test) {
        return true;
    }

    matches!(
        std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "production".to_string())
            .to_ascii_lowercase()
            .as_str(),
        "development" | "dev" | "local"
    )
}

fn is_dev_auth_enabled() -> bool {
    cfg!(test) || is_truthy_env("MOCKFORGE_ENABLE_DEV_AUTH")
}

fn allow_unauthenticated_dev_access() -> bool {
    is_development_environment()
        && is_dev_auth_enabled()
        && (is_truthy_env("MOCKFORGE_ALLOW_UNAUTHENTICATED")
            || is_truthy_env("MOCKFORGE_ALLOW_UNAUTHENTICATED_DEV"))
}

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
            "get_audit_logs" | "get_audit_stats" | "get_env" => {
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

/// Routes served to anyone: the SPA shell and its static assets, the login
/// endpoints, the admin health and liveness probes, and the registry-admin sub-router,
/// which carries its own JWT layer
/// (`crates/mockforge-ui/src/registry_admin.rs`).
///
/// Matching is exact on the raw path. Anything not listed here needs
/// credentials, so path variants (trailing or doubled slashes,
/// percent-encoding, case) fall through to the authenticated branch.
pub(crate) fn is_public_route(path: &str) -> bool {
    // Dot segments, doubled slashes and percent-escapes never name a public
    // file; refuse them rather than reason about how a handler resolves them.
    if path.contains("..") || path.contains("//") || path.contains('%') {
        return false;
    }
    path == "/"
        || path.starts_with("/assets/")
        || path == "/__mockforge/auth/login"
        || path == "/__mockforge/auth/refresh"
        || path == "/__mockforge/auth/logout"
        || path == "/__mockforge/health"
        // Liveness answers status, version and uptime only, for probes.
        || path == "/health/live"
        || path == "/livez"
        || path.starts_with("/mockforge-")
        || path == "/manifest.json"
        || path == "/sw.js"
        || path == "/api-docs"
        || path.starts_with("/api/admin/registry/")
}

/// The JWT in `?access_token=` on an SSE stream. `EventSource` cannot send
/// an `Authorization` header, so the admin UI passes its token this way on
/// `GET .../sse` only.
fn sse_query_token<'a>(method: &str, path: &str, query: Option<&'a str>) -> Option<&'a str> {
    if method != "GET" || !path.ends_with("/sse") {
        return None;
    }
    query?.split('&').find_map(|pair| pair.strip_prefix("access_token="))
}

/// Who is making `request`, under `policy`, or `None` when anonymous
fn authenticate(request: &Request, policy: &AdminAccessPolicy) -> Option<UserContext> {
    let headers = request.headers();
    if policy.management_token_matches(headers) {
        return Some(UserContext {
            user_id: "management-token".to_string(),
            username: "management-token".to_string(),
            role: UserRole::Admin,
            email: None,
        });
    }
    if !policy.accepts_jwt() {
        return None;
    }
    if let Some(user) = extract_user_context(headers) {
        return Some(user);
    }
    let uri = request.uri();
    sse_query_token(request.method().as_str(), uri.path(), uri.query()).and_then(parse_jwt_token)
}

/// Map a request to the admin action whose permissions it needs
pub(crate) fn action_for(method: &str, path: &str) -> &'static str {
    match (method, path) {
        (_, p) if p.contains("/config/latency") => "update_latency",
        (_, p) if p.contains("/config/faults") => "update_faults",
        (_, p) if p.contains("/config/proxy") => "update_proxy",
        (_, p) if p.contains("/config/traffic-shaping") => "update_traffic_shaping",
        ("DELETE", p) if p.contains("/logs") => "clear_logs",
        ("POST", p) if p.contains("/restart") => "restart_servers",
        ("DELETE", p) if p.contains("/fixtures") => "delete_fixture",
        ("POST", p) if p.contains("/fixtures") && p.contains("/rename") => "rename_fixture",
        ("POST", p) if p.contains("/fixtures") && p.contains("/move") => "move_fixture",
        ("GET" | "HEAD", p) if p.contains("/audit/logs") => "get_audit_logs",
        ("GET" | "HEAD", p) if p.contains("/audit/stats") => "get_audit_stats",
        // Environment variables are operator settings, not mock data.
        ("GET" | "HEAD", p) if p.starts_with("/__mockforge/env") => "get_env",
        // Admin reads (dashboard, logs, recorder, chaos, ...) need a read
        // permission, so viewers keep the dashboard they had before (#1019).
        ("GET" | "HEAD", p) if p.starts_with("/__mockforge/") => "get_dashboard",
        ("GET" | "HEAD", _) => "read", // Other reads
        _ => "unknown",
    }
}

/// RBAC middleware to enforce permissions on admin endpoints.
///
/// Every route that is not in [`is_public_route`] needs credentials, reads
/// included (#1019). The only anonymous access is the loopback developer
/// mode decided at bind time (see [`crate::admin_access`]).
pub async fn rbac_middleware(mut request: Request, next: Next) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_string();
    let method = request.method().as_str().to_string();

    if is_public_route(&path) {
        return Ok(next.run(request).await);
    }

    let policy = request.extensions().get::<AdminAccessPolicy>().cloned().unwrap_or_default();

    let user_context = match authenticate(&request, &policy) {
        Some(ctx) => ctx,
        None if policy.allows_anonymous_loopback_reads() => {
            if policy.allows_anonymous_read(request.method(), request.uri(), request.headers()) {
                return Ok(next.run(request).await);
            }
            // Loopback developer mode: the explicit dev escape hatch.
            match get_default_user_context() {
                Some(ctx) => ctx,
                None => return Err(StatusCode::UNAUTHORIZED),
            }
        }
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let action_name = action_for(&method, &path);

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
pub fn get_user_context_from_state<T>(_state: &T) -> Option<UserContext>
where
    T: std::any::Any,
{
    // This is a placeholder - in practice, user context would be stored in request extensions
    None
}

#[cfg(test)]
mod tests;
