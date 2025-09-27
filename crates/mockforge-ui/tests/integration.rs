/**
 * Integration tests for backend API endpoints
 * Tests the Rust backend API responses and error handling
 */
use axum::{body::Body, http::Request};
use mockforge_ui::create_admin_router;
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_dashboard_endpoint_integration() {
    let app = create_admin_router(None, None, None, None, true);

    let response = app
        .oneshot(Request::builder().uri("/__mockforge/dashboard").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    // Parse the JSON response
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    // Verify response structure
    assert_eq!(json_response["success"], true);
    assert!(json_response["data"].is_object());

    let data = &json_response["data"];

    // Check required fields exist
    assert!(data["system"].is_object());
    assert!(data["servers"].is_array());
    assert!(data["routes"].is_array());
    assert!(data["recent_logs"].is_array());

    // Check system info structure
    let system = &data["system"];
    assert!(system["version"].is_string());
    assert!(system["uptime_seconds"].is_number());
    assert!(system["memory_usage_mb"].is_number());
    assert!(system["cpu_usage_percent"].is_number());
    assert!(system["active_threads"].is_number());
    assert!(system["total_routes"].is_number());

    // Verify total_routes is the expected value (9 static + 25 API routes = 34)
    assert_eq!(system["total_routes"], 34);
}

#[tokio::test]
async fn test_logs_endpoint_with_filters() {
    let app = create_admin_router(None, None, None, None, true);

    // Test with method filter
    let response = app
        .oneshot(
            Request::builder()
                .uri("/__mockforge/logs?method=GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert_eq!(json_response["success"], true);
    assert!(json_response["data"].is_array());
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let app = create_admin_router(None, None, None, None, true);

    let response = app
        .oneshot(Request::builder().uri("/__mockforge/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert_eq!(json_response["success"], true);
    assert!(json_response["data"].is_object());

    let data = &json_response["data"];
    assert!(data["requests_by_endpoint"].is_object());
    assert!(data["response_time_percentiles"].is_object());
    assert!(data["error_rate_by_endpoint"].is_object());
}

#[tokio::test]
async fn test_configuration_endpoints() {
    let app = create_admin_router(None, None, None, None, true);

    // Test GET config
    let response = app
        .oneshot(Request::builder().uri("/__mockforge/config").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert_eq!(json_response["success"], true);
    assert!(json_response["data"].is_object());

    let data = &json_response["data"];
    assert!(data["latency"].is_object());
    assert!(data["faults"].is_object());
    assert!(data["proxy"].is_object());
    assert!(data["validation"].is_object());
}

#[tokio::test]
async fn test_latency_configuration_update() {
    let app = create_admin_router(None, None, None, None, true);

    let update_payload = json!({
        "config_type": "latency",
        "data": {
            "base_ms": 100,
            "jitter_ms": 50,
            "tag_overrides": {
                "auth": 200
            }
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/__mockforge/config/latency")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&update_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should succeed (200) or be a client error (400), but not a server error (500)
    assert!(!response.status().is_server_error());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    // Should have success field
    assert!(json_response["success"].is_boolean());
}

#[tokio::test]
async fn test_fault_injection_update() {
    let app = create_admin_router(None, None, None, None, true);

    let update_payload = json!({
        "config_type": "faults",
        "data": {
            "enabled": true,
            "failure_rate": 0.1,
            "status_codes": [500, 502, 503]
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/__mockforge/config/faults")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&update_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(!response.status().is_server_error());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert!(json_response["success"].is_boolean());
}

#[tokio::test]
async fn test_proxy_configuration_update() {
    let app = create_admin_router(None, None, None, None, true);

    let update_payload = json!({
        "config_type": "proxy",
        "data": {
            "enabled": true,
            "upstream_url": "http://api.example.com",
            "timeout_seconds": 60
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/__mockforge/config/proxy")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&update_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(!response.status().is_server_error());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert!(json_response["success"].is_boolean());
}

#[tokio::test]
async fn test_validation_settings_update() {
    let app = create_admin_router(None, None, None, None, true);

    let update_payload = json!({
        "mode": "warn",
        "aggregate_errors": false,
        "validate_responses": true,
        "overrides": {
            "GET /health": "off"
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/__mockforge/validation")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&update_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(!response.status().is_server_error());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert!(json_response["success"].is_boolean());
}

#[tokio::test]
async fn test_fixtures_endpoint() {
    let app = create_admin_router(None, None, None, None, true);

    let response = app
        .oneshot(Request::builder().uri("/__mockforge/fixtures").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert_eq!(json_response["success"], true);
    assert!(json_response["data"].is_array());
}

#[tokio::test]
async fn test_environment_variables_endpoint() {
    let app = create_admin_router(None, None, None, None, true);

    let response = app
        .oneshot(Request::builder().uri("/__mockforge/env").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert_eq!(json_response["success"], true);
    assert!(json_response["data"].is_object());
}

#[tokio::test]
async fn test_server_restart_endpoint() {
    let app = create_admin_router(None, None, None, None, true);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/__mockforge/servers/restart")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason": "Integration test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should not be a server error
    assert!(!response.status().is_server_error());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert!(json_response["success"].is_boolean());
}

#[tokio::test]
async fn test_logs_clear_endpoint() {
    let app = create_admin_router(None, None, None, None, true);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/__mockforge/logs/clear")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should not be a server error
    assert!(!response.status().is_server_error());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert!(json_response["success"].is_boolean());
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_admin_router(None, None, None, None, true);

    let response = app
        .oneshot(Request::builder().uri("/__mockforge/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let json_response: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    // The health endpoint returns a direct HealthCheck response, not wrapped in ApiResponse
    assert!(json_response["status"].is_string());
    assert!(json_response["services"].is_object());
    assert!(json_response["last_check"].is_string());
    assert!(json_response["issues"].is_array());
}

#[tokio::test]
async fn test_error_responses() {
    let app = create_admin_router(None, None, None, None, true);

    // Test invalid JSON in POST request
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/__mockforge/config/latency")
                .header("content-type", "application/json")
                .body(Body::from("invalid json"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle gracefully (400 error, not 500)
    assert!(response.status().is_client_error() || response.status().is_success());

    // Test non-existent endpoint
    let app2 = create_admin_router(None, None, None, None, true);
    let response = app2
        .oneshot(Request::builder().uri("/__mockforge/nonexistent").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should return 404, not 500
    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn test_cors_headers() {
    let app = create_admin_router(None, None, None, None, true);

    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/__mockforge/dashboard")
                .header("origin", "http://localhost:3000")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should allow CORS
    assert!(response.status().is_success());
}
