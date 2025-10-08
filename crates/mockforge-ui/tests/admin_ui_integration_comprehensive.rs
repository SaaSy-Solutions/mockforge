/**
 * Comprehensive integration tests for Admin UI endpoints
 * Covers recently added features that were previously untested
 */
use axum::{body::Body, http::Request};
use mockforge_ui::create_admin_router;
use serde_json::json;
use tower::ServiceExt;

/// Helper to create a test router
fn create_test_router() -> axum::Router {
    create_admin_router(
        Some("127.0.0.1:3000".parse().unwrap()),
        Some("127.0.0.1:3001".parse().unwrap()),
        Some("127.0.0.1:50051".parse().unwrap()),
        Some("127.0.0.1:4000".parse().unwrap()),
        true,
        9080,
    )
}

// ============================================================================
// Health Probe Endpoints (Kubernetes Liveness/Readiness/Startup)
// ============================================================================

#[tokio::test]
async fn test_health_liveness_probe() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/health/live").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(
        response.status().is_success(),
        "Liveness probe should return success"
    );
}

#[tokio::test]
async fn test_health_readiness_probe() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(
        response.status().is_success(),
        "Readiness probe should return success"
    );
}

#[tokio::test]
async fn test_health_startup_probe() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/health/startup").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(
        response.status().is_success(),
        "Startup probe should return success"
    );
}

#[tokio::test]
async fn test_health_deep_check() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());
}

// ============================================================================
// Routes Endpoint
// ============================================================================

#[tokio::test]
async fn test_routes_endpoint() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/routes").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());
}

// ============================================================================
// Server Info Endpoint
// ============================================================================

#[tokio::test]
async fn test_server_info_endpoint() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/server-info").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());
}

// ============================================================================
// Traffic Shaping Configuration
// ============================================================================

#[tokio::test]
async fn test_traffic_shaping_config_update() {
    let app = create_test_router();

    let payload = json!({
        "config_type": "traffic_shaping",
        "data": {
            "enabled": true,
            "bandwidth_limit_kbps": 1000,
            "packet_loss_rate": 0.01
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/__mockforge/config/traffic-shaping")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should not be a server error
    assert!(!response.status().is_server_error());
}

// ============================================================================
// Restart Status Endpoint
// ============================================================================

#[tokio::test]
async fn test_restart_status_endpoint() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/restart/status").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());
}

// ============================================================================
// Plugin Management
// ============================================================================

#[tokio::test]
async fn test_plugins_list_endpoint() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/plugins").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_plugin_status_endpoint() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/plugins/status").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());
}

// ============================================================================
// Workspace Management
// ============================================================================

#[tokio::test]
async fn test_workspaces_list_endpoint() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/workspaces").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_workspace_create_endpoint() {
    let app = create_test_router();

    let payload = json!({
        "name": "Test Workspace",
        "description": "A test workspace"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/__mockforge/workspaces")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should not be a server error
    assert!(!response.status().is_server_error());
}

// ============================================================================
// Environment Management
// ============================================================================

#[tokio::test]
async fn test_environments_list_endpoint() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/workspaces/test-ws/environments").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should handle gracefully even if workspace doesn't exist
    assert!(!response.status().is_server_error());
}

// ============================================================================
// Chain Management (Proxy Endpoints)
// ============================================================================

#[tokio::test]
async fn test_chains_list_endpoint() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/chains").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Chain endpoint proxies to HTTP server, may fail if server not running
    // This is expected behavior - we're just testing that the route exists
    assert!(
        response.status().is_success() || response.status().is_server_error(),
        "Chain endpoint should exist (status: {})", response.status()
    );
}

// ============================================================================
// Validation Configuration
// ============================================================================

#[tokio::test]
async fn test_validation_get_endpoint() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/validation").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_validation_update_endpoint() {
    let app = create_test_router();

    let payload = json!({
        "mode": "enforce",
        "aggregate_errors": true,
        "validate_responses": true,
        "overrides": {}
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/__mockforge/validation")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should not be a server error
    assert!(!response.status().is_server_error());
}

// ============================================================================
// Environment Variables Management
// ============================================================================

#[tokio::test]
async fn test_env_vars_get_endpoint() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/env").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());
}

// ============================================================================
// Smoke Tests
// ============================================================================

#[tokio::test]
async fn test_smoke_tests_list_endpoint() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/smoke").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_smoke_tests_run_endpoint() {
    let app = create_test_router();
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/smoke/run").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should handle gracefully
    assert!(!response.status().is_server_error());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_spa_fallback_for_unknown_routes() {
    let app = create_test_router();

    // Test various UI routes that should fallback to SPA
    let spa_routes = vec![
        "/services",
        "/fixtures",
        "/logs",
        "/metrics",
        "/settings",
        "/unknown-route",
    ];

    for route in spa_routes {
        let response = app.clone()
            .oneshot(Request::builder().uri(route).body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Should return 200 (SPA fallback), not 404
        assert_eq!(
            response.status().as_u16(),
            200,
            "Route {} should fallback to SPA",
            route
        );
    }
}
