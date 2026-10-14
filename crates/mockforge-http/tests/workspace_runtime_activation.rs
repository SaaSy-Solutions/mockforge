//! Real HTTP publication controls for workspace-owned dynamic routes.
use axum::Router;
use mockforge_http::management::{dynamic_mock_fallback, management_router, ManagementState};
use serde_json::json;
use std::time::Duration;

struct Server {
    base: String,
    client: reqwest::Client,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn start() -> Self {
        let state = ManagementState::new(None, None, 0);
        let app = Router::new()
            .nest("/__mockforge/api", management_router(state.clone()))
            .fallback_service(axum::routing::any(dynamic_mock_fallback).with_state(state));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self {
            base: format!("http://{address}"),
            client: reqwest::Client::builder().timeout(Duration::from_secs(5)).build().unwrap(),
            task,
        }
    }
}
#[tokio::test]
async fn workspace_publication_replaces_only_owned_routes_and_keeps_raw_body_bytes() {
    let server = Server::start().await;
    let unrelated = json!({"id":"unrelated","method":"GET","path":"/unrelated","response":{"body":{"kept":true}}});
    assert!(server
        .client
        .post(format!("{}/__mockforge/api/mocks", server.base))
        .json(&unrelated)
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let mut own = json!({"id":"__workspace_live:fixture:req","method":"GET","path":"/workspace/fixture/live","response":{"body":" raw static response\n","headers":{"content-type":"text/plain"}},"status_code":201,"request_match":{}});
    let publish = format!("{}/__mockforge/api/workspaces/fixture/activate", server.base);
    assert!(server
        .client
        .post(&publish)
        .json(&json!([own]))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let response = server
        .client
        .get(format!("{}/workspace/fixture/live", server.base))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::CREATED);
    assert_eq!(response.bytes().await.unwrap().as_ref(), b" raw static response\n");
    let unrelated = server
        .client
        .get(format!("{}/unrelated", server.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(unrelated, json!({"kept":true}));
    own["path"] = json!("/workspace/fixture/updated");
    assert!(server
        .client
        .post(&publish)
        .json(&json!([own]))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    assert_eq!(
        server
            .client
            .get(format!("{}/workspace/fixture/live", server.base))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::NOT_FOUND
    );
    assert_eq!(
        server
            .client
            .get(format!("{}/workspace/fixture/updated", server.base))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::CREATED
    );
    let invalid =
        json!({"id":"unrelated","method":"GET","path":"/unrelated","response":{"body":"bad"}});
    assert_eq!(
        server
            .client
            .post(&publish)
            .json(&json!([invalid]))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::BAD_REQUEST
    );
    assert_eq!(
        server
            .client
            .get(format!("{}/workspace/fixture/updated", server.base))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::CREATED
    );
    assert_eq!(
        server
            .client
            .get(format!("{}/unrelated", server.base))
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap(),
        json!({"kept":true})
    );
}
