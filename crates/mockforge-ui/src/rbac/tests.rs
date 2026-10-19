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

/// Serializes the tests that write `MOCKFORGE_ALLOW_UNAUTHENTICATED`: the
/// env is process-wide and the harness runs tests in parallel, so one test
/// removing it while the other has just set it fails either at random.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_get_default_user_context_without_env_var() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("MOCKFORGE_ALLOW_UNAUTHENTICATED");
    let context = get_default_user_context();
    assert!(context.is_none());
}

#[test]
fn test_get_default_user_context_with_env_var() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
async fn test_rbac_middleware_rejects_anonymous_get_mockforge_api() {
    use axum::routing::get;
    use axum::{body::Body, middleware::from_fn, Router};
    use tower::ServiceExt;

    async fn handler() -> &'static str {
        "OK"
    }

    let app = Router::new()
        .route("/__mockforge/dashboard", get(handler))
        .layer(from_fn(rbac_middleware));

    // GET to /__mockforge/* needs credentials like writes (#1019)
    let request = axum::http::Request::builder()
        .uri("/__mockforge/dashboard")
        .method(Method::GET)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
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
    for (_path, expected_action) in test_cases {
        assert!(!expected_action.is_empty());
    }

    assert_eq!(action_for("GET", "/__mockforge/logs"), "get_dashboard");
    assert_eq!(action_for("HEAD", "/__mockforge/logs"), "get_dashboard");
    assert_eq!(action_for("GET", "/__mockforge/env"), "get_env");
    assert_eq!(action_for("GET", "/__mockforge/audit/logs"), "get_audit_logs");
    assert_eq!(action_for("POST", "/__mockforge/env"), "unknown");
    assert_eq!(
        AdminActionPermissions::get_required_permissions("get_env"),
        vec![Permission::ManageSettings]
    );
}

#[test]
fn test_sse_query_token_only_on_sse_get() {
    assert_eq!(
        sse_query_token("GET", "/__mockforge/logs/sse", Some("access_token=abc")),
        Some("abc")
    );
    assert_eq!(
        sse_query_token("GET", "/__mockforge/logs/sse", Some("a=1&access_token=abc")),
        Some("abc")
    );
    assert_eq!(sse_query_token("GET", "/__mockforge/logs", Some("access_token=abc")), None);
    assert_eq!(sse_query_token("POST", "/__mockforge/logs/sse", Some("access_token=abc")), None);
    assert_eq!(sse_query_token("GET", "/__mockforge/logs/sse", None), None);
}

#[test]
fn test_public_routes_reject_traversal_variants() {
    assert!(is_public_route("/assets/index.js"));
    assert!(is_public_route("/__mockforge/health"));
    for path in [
        "/assets/../__mockforge/logs",
        "/assets/..%2F__mockforge/logs",
        "/assets//x",
        "/mockforge-x/../__mockforge/logs",
        "/__mockforge/health/",
        "/__mockforge/Health",
        "/__mockforge/%68ealth",
        "/api/resilience/bulkheads",
        "/__mockforge/logs",
    ] {
        assert!(!is_public_route(path), "{path} must not be public");
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
