use axum::Router;
use mockforge_http::build_router;
use mockforge_core::openapi_routes::{ValidationMode, ValidationOptions};

#[tokio::test]
async fn returns_202_when_present() {
    let spec = serde_json::json!({
        "openapi":"3.0.0",
        "info": {"title":"E2E","version":"1"},
        "paths": {"/accept": {"get": {
            "responses": {"202": {"description":"Accepted","content":{"application/json":{"schema":{"type":"object"}}}}}
        }}}
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spec.json");
    tokio::fs::write(&path, serde_json::to_vec(&spec).unwrap()).await.unwrap();
    let app: Router = build_router(Some(path.to_string_lossy().to_string()), Some(ValidationOptions::default())).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::new();
    let url = format!("http://{}/accept", addr);
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::ACCEPTED);
    drop(server);
}

#[tokio::test]
async fn returns_204_with_empty_body() {
    let spec = serde_json::json!({
        "openapi":"3.0.0",
        "info": {"title":"E2E","version":"1"},
        "paths": {"/nocontent": {"get": {
            "responses": {"204": {"description":"No Content"}}
        }}}
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spec.json");
    tokio::fs::write(&path, serde_json::to_vec(&spec).unwrap()).await.unwrap();
    let app: Router = build_router(Some(path.to_string_lossy().to_string()), Some(ValidationOptions::default())).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::new();
    let url = format!("http://{}/nocontent", addr);
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::NO_CONTENT);
    assert!(res.text().await.unwrap().is_empty());
    drop(server);
}

