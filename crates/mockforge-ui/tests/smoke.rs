use axum::{body::Body, http::Request};
use tower::ServiceExt; // for `oneshot`

#[tokio::test]
async fn serves_root_and_assets_and_health() {
    // admin router at root
    let app = mockforge_ui::create_admin_router(
        None,
        None,
        None,
        None,
        true,
        9080,
        "http://localhost:9090".to_string(),
    );

    // /
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(res.status().is_success());

    // /assets/index.css
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/assets/index.css").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(res.status().is_success());

    // /assets/index.js
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/assets/index.js").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(res.status().is_success());

    // /__mockforge/health
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/__mockforge/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(res.status().is_success());
}

#[tokio::test]
async fn works_under_mount_prefix() {
    // router nested under /admin
    let sub = mockforge_ui::create_admin_router(
        None,
        None,
        None,
        None,
        true,
        9080,
        "http://localhost:9090".to_string(),
    );
    let app = axum::Router::new().nest("/admin", sub);

    // /admin (nested root)
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/admin").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(res.status().is_success());

    // /admin/assets/index.css
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/admin/assets/index.css").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(res.status().is_success());

    // /admin/assets/index.js
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/admin/assets/index.js").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(res.status().is_success());

    // /admin/__mockforge/health
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/admin/__mockforge/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(res.status().is_success());
}

#[tokio::test]
async fn test_api_endpoints() {
    // admin router with API enabled
    let app = mockforge_ui::create_admin_router(
        None,
        None,
        None,
        None,
        true,
        9080,
        "http://localhost:9090".to_string(),
    );

    // Test all the new API endpoints
    let endpoints = vec![
        "/__mockforge/dashboard",
        "/__mockforge/logs",
        "/__mockforge/metrics",
        "/__mockforge/config",
        "/__mockforge/fixtures",
        "/__mockforge/validation",
        "/__mockforge/env",
    ];

    for endpoint in endpoints {
        let res = app
            .clone()
            .oneshot(Request::builder().uri(endpoint).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(res.status().is_success(), "Failed on endpoint: {}", endpoint);
    }
}

#[tokio::test]
async fn test_post_endpoints() {
    // admin router with API enabled
    let app = mockforge_ui::create_admin_router(
        None,
        None,
        None,
        None,
        true,
        9080,
        "http://localhost:9090".to_string(),
    );

    // Test POST endpoints
    let post_endpoints = vec![
        ("/__mockforge/fixtures/delete", r#"{"fixture_id": "test"}"#),
        ("/__mockforge/env", r#"{"key": "TEST_KEY", "value": "test_value"}"#),
        (
            "/__mockforge/files/content",
            r#"{"file_path": "test.yaml", "file_type": "yaml"}"#,
        ),
        ("/__mockforge/files/save", r#"{"file_path": "test.yaml", "content": "test"}"#),
        (
            "/__mockforge/config/latency",
            r#"{"config_type": "latency", "data": {"base_ms": 50}}"#,
        ),
        (
            "/__mockforge/config/faults",
            r#"{"config_type": "faults", "data": {"enabled": true}}"#,
        ),
        (
            "/__mockforge/config/proxy",
            r#"{"config_type": "proxy", "data": {"enabled": true}}"#,
        ),
        (
            "/__mockforge/validation",
            r#"{"mode": "enforce", "aggregate_errors": true, "validate_responses": false}"#,
        ),
    ];

    for (endpoint, body) in post_endpoints {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(endpoint)
                    .header("Content-Type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        // These might return 200 or 400 depending on implementation, but should not be 500
        assert!(!res.status().is_server_error(), "Server error on endpoint: {}", endpoint);
    }
}
