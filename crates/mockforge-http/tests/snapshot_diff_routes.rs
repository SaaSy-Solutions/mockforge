//! Real TCP coverage of the management projection and snapshot comparison.

use axum::Router;
use mockforge_http::{
    handlers::snapshot_diff::{snapshot_diff_router, MockSnapshot, SnapshotDiff},
    management::{ManagementState, MockConfig},
};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::time::Duration;

struct Server {
    base: String,
    client: Client,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Server {
    async fn start(state: ManagementState) -> Self {
        let router: Router = snapshot_diff_router(state.clone()).with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Self {
            base: format!("http://{address}"),
            client: Client::builder().timeout(Duration::from_secs(5)).build().unwrap(),
            task,
        }
    }

    async fn snapshot(&self, metadata: Value) -> MockSnapshot {
        let response = self
            .client
            .post(format!("{}/snapshots", self.base))
            .json(&metadata)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        response.json().await.unwrap()
    }
}

fn mock(id: &str, path: &str, status: Option<u16>, body: Value, header: &str) -> MockConfig {
    serde_json::from_value(json!({
        "id":id,"method":"GET","path":path,"status_code":status,
        "response":{"body":body,"headers":{"X-Fixture":header}}
    }))
    .unwrap()
}

#[tokio::test]
async fn saved_snapshots_keep_projected_data_and_compare_all_existing_difference_types() {
    let state = ManagementState::new(None, None, 0);
    let server = Server::start(state.clone()).await;
    let environment = uuid::Uuid::new_v4().to_string();
    let shared = mock("shared", "/shared", None, json!({"version":1}), "left");
    *state.mocks.write().await = vec![
        shared.clone(),
        mock("left", "/left", None, json!({}), "left"),
    ];
    let left = server.snapshot(json!({"environment_id":environment,"persona_id":"persona-a", "scenario_id":"scenario-a", "reality_level":0.4,"metadata":{"fixture":true}})).await;
    assert_eq!(left.mocks[0].status_code, 200);
    assert_eq!(left.mocks[0].config, serde_json::to_value(&shared).unwrap());
    assert_eq!(left.metadata["fixture"], true);
    let retrieved = server
        .client
        .get(format!("{}/snapshots/{}", server.base, left.id))
        .send()
        .await
        .unwrap();
    assert_eq!(retrieved.status(), StatusCode::OK);
    let saved: MockSnapshot = retrieved.json().await.unwrap();
    assert_eq!(saved.mocks[0].response_body, json!({"version":1}));

    *state.mocks.write().await = vec![
        mock("shared", "/shared", Some(503), json!({"version":2}), "right"),
        mock("right", "/right", None, json!({}), "right"),
    ];
    let right = server.snapshot(json!({"environment_id":environment,"persona_id":"persona-a","scenario_id":"scenario-a"})).await;
    let response = server
        .client
        .post(format!("{}/snapshots/compare", server.base))
        .json(&json!({"left_snapshot_id":left.id,"right_snapshot_id":right.id}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let diff: SnapshotDiff = response.json().await.unwrap();
    assert_eq!(diff.summary.left_total, 2);
    assert_eq!(diff.summary.right_total, 2);
    assert_eq!(diff.summary.only_in_left, 1);
    assert_eq!(diff.summary.only_in_right, 1);
    assert_eq!(diff.summary.differences_count, 5);
    assert_eq!(diff.summary.mocks_with_differences, 3);
    let types: std::collections::HashSet<_> = diff
        .differences
        .iter()
        .map(|d| serde_json::to_value(&d.diff_type).unwrap().as_str().unwrap().to_owned())
        .collect();
    assert_eq!(types.len(), 5);
    for name in [
        "missing_in_left",
        "missing_in_right",
        "status_code_mismatch",
        "body_mismatch",
        "headers_mismatch",
    ] {
        assert!(types.contains(name));
    }

    let listed = server
        .client
        .get(format!("{}/snapshots", server.base))
        .query(&[
            ("environment_id", environment.as_str()),
            ("persona_id", "persona-a"),
            ("scenario_id", "scenario-a"),
        ])
        .send()
        .await
        .unwrap()
        .json::<Vec<MockSnapshot>>()
        .await
        .unwrap();
    assert_eq!(listed.len(), 2);
    let empty = server
        .client
        .get(format!("{}/snapshots", server.base))
        .query(&[
            ("environment_id", environment.as_str()),
            ("persona_id", "wrong"),
        ])
        .send()
        .await
        .unwrap()
        .json::<Vec<MockSnapshot>>()
        .await
        .unwrap();
    assert!(empty.is_empty());
    let missing = server
        .client
        .get(format!("{}/snapshots/{}", server.base, uuid::Uuid::new_v4()))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn live_comparison_uses_current_management_state_and_unknown_ids_fail() {
    let state = ManagementState::new(None, None, 0);
    let server = Server::start(state.clone()).await;
    *state.mocks.write().await = vec![mock(
        "live",
        "/live",
        Some(201),
        json!({"live":true}),
        "live",
    )];
    let response = server.client.post(format!("{}/snapshots/compare", server.base)).json(&json!({"left_environment_id":"left","right_environment_id":"right","left_persona_id":"p-left","right_scenario_id":"s-right","right_reality_level":0.8})).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let diff: SnapshotDiff = response.json().await.unwrap();
    assert_eq!(diff.summary.differences_count, 0);
    assert_eq!(diff.left.mocks.len(), 1);
    assert_eq!(diff.right.mocks[0].response_body, json!({"live":true}));
    assert_eq!(diff.left.environment_id.as_deref(), Some("left"));
    assert_eq!(diff.right.environment_id.as_deref(), Some("right"));
    assert_eq!(diff.left.persona_id.as_deref(), Some("p-left"));
    assert_eq!(diff.right.scenario_id.as_deref(), Some("s-right"));
    assert_eq!(diff.right.reality_level, Some(0.8));
    let unknown = server
        .client
        .post(format!("{}/snapshots/compare", server.base))
        .json(&json!({"left_snapshot_id":uuid::Uuid::new_v4().to_string()}))
        .send()
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
}
