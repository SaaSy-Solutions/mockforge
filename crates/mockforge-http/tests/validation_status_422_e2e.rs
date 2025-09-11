use axum::Router;
use mockforge_http::build_router;
use mockforge_core::openapi_routes::{ValidationMode, ValidationOptions};
use std::net::SocketAddr;

#[tokio::test]
async fn enforce_uses_422_when_flag_set() {
    // Spec requiring a query param integer, send invalid to trigger validation
    let spec = serde_json::json!({
        "openapi":"3.0.0",
        "info": {"title":"E2E","version":"1"},
        "paths": {"/needsInt": {"get": {
            "parameters": [{"name":"n","in":"query","required":true,"schema":{"type":"integer"}}],
            "responses": {"200":{"description":"ok"}}
        }}}
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spec.json");
    tokio::fs::write(&path, serde_json::to_vec(&spec).unwrap()).await.unwrap();

    // Turn on 422
    std::env::set_var("MOCKFORGE_VALIDATION_STATUS", "422");

    let opts = Some(ValidationOptions { request_mode: ValidationMode::Enforce, aggregate_errors: true, validate_responses: false, overrides: std::collections::HashMap::new(), admin_skip_prefixes: vec!["/__mockforge".into()], response_template_expand: false, validation_status: None });
    let app: Router = build_router(Some(path.to_string_lossy().to_string()), opts).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    let url = format!("http://{}/needsInt?n=abc", addr);
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);

    drop(server);
}

