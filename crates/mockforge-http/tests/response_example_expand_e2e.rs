use axum::Router;
use mockforge_core::openapi_routes::{ValidationMode, ValidationOptions};
use mockforge_http::build_router;
use std::net::SocketAddr;

#[tokio::test]
async fn media_example_token_expansion_toggle() {
    // Spec with media example containing tokens
    let spec = serde_json::json!({
        "openapi":"3.0.0",
        "info": {"title":"E2E","version":"1"},
        "paths": {"/token": {"get": {
            "responses": {"200":{
                "description":"ok",
                "content": {"application/json": {"example": {"id":"{{uuid}}","ts":"{{now}}","price":"{{rand.float}}"}}}
            }}
        }}}
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spec.json");
    tokio::fs::write(&path, serde_json::to_vec(&spec).unwrap()).await.unwrap();

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

    // Bind server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { 
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap() 
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/token", addr);

    // Enable expansion
    std::env::set_var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND", "true");
    let resp = client.get(&url).send().await.unwrap();
    assert!(resp.status().is_success());
    let j: serde_json::Value = resp.json().await.unwrap();
    let id = j.get("id").and_then(|v| v.as_str()).expect("id present");
    let ts = j.get("ts").and_then(|v| v.as_str()).expect("ts present");
    assert!(!id.contains("{{"));
    assert!(!ts.contains("{{"));
    uuid::Uuid::parse_str(id).expect("uuid expanded");

    // Disable expansion; tokens should be literal
    std::env::set_var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND", "false");
    let resp = client.get(&url).send().await.unwrap();
    assert!(resp.status().is_success());
    let j: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(j.get("id").and_then(|v| v.as_str()).unwrap(), "{{uuid}}");

    drop(server);
}
