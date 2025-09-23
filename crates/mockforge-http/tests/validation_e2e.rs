use axum::Router;
use mockforge_core::openapi_routes::{ValidationMode, ValidationOptions};
use mockforge_http::build_router;
use std::net::SocketAddr;

#[tokio::test]
async fn toggling_validation_mode_runtime() {
    // Write a temporary OpenAPI spec to disk
    let spec = serde_json::json!({
        "openapi":"3.0.0",
        "info": {"title":"E2E","version":"1"},
        "paths": {"/e2e": {"post": {
            "parameters": [{"name":"q","in":"query","required":true,"schema":{"type":"integer"}}],
            "responses": {"200":{"description":"ok"}}
        }}}
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spec.json");
    tokio::fs::write(&path, serde_json::to_vec(&spec).unwrap()).await.unwrap();

    // Build app with enforce mode
    let opts = Some(ValidationOptions {
        request_mode: ValidationMode::Enforce,
        aggregate_errors: true,
        validate_responses: false,
        overrides: std::collections::HashMap::new(),
        admin_skip_prefixes: vec!["/__mockforge".into()],
        response_template_expand: false,
        validation_status: None,
    });
    let app: Router = build_router(Some(path.to_string_lossy().to_string()), opts, None).await;

    // Bind on random port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    // Invalid request should 400 under enforce
    let client = reqwest::Client::new();
    let url = format!("http://{}/e2e", addr);
    let res = client.post(&url).send().await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);

    // Test with per-route warn override by setting environment variable
    std::env::set_var("MOCKFORGE_VALIDATION_OVERRIDES_JSON", r#"{"POST /e2e": "warn"}"#);

    // Rebuild router with the updated overrides
    let mut overrides = std::collections::HashMap::new();
    overrides.insert("POST /e2e".to_string(), ValidationMode::Warn);
    let updated_opts = Some(ValidationOptions {
        request_mode: ValidationMode::Enforce,
        aggregate_errors: true,
        validate_responses: false,
        overrides,
        admin_skip_prefixes: vec!["/__mockforge".into()],
        response_template_expand: false,
        validation_status: None,
    });
    let updated_app: Router = build_router(Some(path.to_string_lossy().to_string()), updated_opts, None).await;

    // Bind on new random port for updated router
    let updated_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let updated_addr: SocketAddr = updated_listener.local_addr().unwrap();
    let updated_server = tokio::spawn(async move {
        axum::serve(updated_listener, updated_app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    // Now invalid request should pass due to per-route warn override
    let updated_url = format!("http://{}/e2e", updated_addr);
    let res = client.post(&updated_url).send().await.unwrap();
    assert!(res.status().is_success());

    // Cleanup updated server
    drop(updated_server);

    // Cleanup server
    drop(server);
}
