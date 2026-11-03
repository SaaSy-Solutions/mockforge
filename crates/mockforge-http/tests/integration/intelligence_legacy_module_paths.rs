//! Downstream consumers retain the old module paths and shared route behavior.
use axum::Router;
use mockforge_core::consistency::ConsistencyEngine;
use mockforge_http::handlers::{consistency, fidelity, xray};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};

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
        let engine = Arc::new(ConsistencyEngine::new());
        let router = Router::new()
            .merge(consistency::consistency_router(consistency::ConsistencyState {
                engine: engine.clone(),
            }))
            .merge(xray::xray_router(xray::XRayState::new(engine)))
            .merge(fidelity::fidelity_router(fidelity::FidelityState::new()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Self {
            base: format!("http://{address}"),
            client: reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            task,
        }
    }

    async fn get(&self, path: &str) -> Value {
        let response = self.client.get(format!("{}{path}", self.base)).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        response.json().await.unwrap()
    }

    async fn post(&self, path: &str, body: Value) -> Value {
        let response = self
            .client
            .post(format!("{}{path}", self.base))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        response.json().await.unwrap()
    }
}

#[tokio::test]
async fn legacy_consistency_and_xray_consumers_share_the_real_engine() {
    let server = Server::start().await;
    let result = server
        .post(
            "/api/v1/consistency/scenario?workspace=legacy-consumer",
            json!({"scenario_id": "owned-scenario"}),
        )
        .await;
    assert_eq!(result["success"], true);
    let state = server.get("/api/v1/consistency/state?workspace=legacy-consumer").await;
    assert_eq!(state["active_scenario"], "owned-scenario");
    let summary = server.get("/api/v1/xray/state/summary?workspace=legacy-consumer").await;
    assert_eq!(summary["workspace_id"], "legacy-consumer");
    assert_eq!(summary["scenario"], state["active_scenario"]);
}

#[tokio::test]
async fn legacy_fidelity_consumer_calculates_and_reads_persisted_scores() {
    let server = Server::start().await;
    let schema = json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"]});
    let calculated = server
        .post(
            "/api/v1/workspace/legacy-consumer/fidelity",
            json!({"mock_schema":schema,"real_schema":schema,"mock_samples":[{"id":"fixture"}],"real_samples":[{"id":"fixture"}]}),
        )
        .await;
    assert_eq!(calculated["success"], true);
    let stored = server.get("/api/v1/workspace/legacy-consumer/fidelity").await;
    assert_eq!(stored["workspace_id"], "legacy-consumer");
    assert_eq!(stored["score"]["overall"], calculated["score"]["overall"]);
    let absent = server
        .client
        .get(format!("{}/api/v1/workspace/missing-workspace/fidelity", server.base))
        .send()
        .await
        .unwrap();
    assert_eq!(absent.status(), reqwest::StatusCode::NOT_FOUND);
}
