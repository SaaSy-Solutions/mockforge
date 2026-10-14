//! Actual router behavior with optional feature combinations and minimal builds.

use mockforge_core::{config::RouteConfig, SharedOverrides};
use mockforge_http::{build_router_with_chains_and_multi_tenant, HealthManager};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};

struct Server(tokio::task::JoinHandle<std::io::Result<()>>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test]
async fn optional_routes_follow_features_and_basic_mocking_stays_available() {
    let route: RouteConfig = serde_json::from_value(json!({
        "path":"/feature-fixture", "method":"get",
        "response":{"status":200,"body":{"marker":"basic-route"}}
    }))
    .unwrap();
    let health = Arc::new(HealthManager::new());
    health.set_ready().await;
    let app = build_router_with_chains_and_multi_tenant(
        None,
        None,
        None,
        None,
        Some(vec![route]),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        Some(health),
        None,
        None,
        None,
        SharedOverrides::load(Vec::new()).await,
    )
    .await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let _server = Server(tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await
    }));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let response = client.get(format!("http://{address}/feature-fixture")).send().await.unwrap();
    assert!(response.status().is_success());
    assert_eq!(response.json::<Value>().await.unwrap()["marker"], "basic-route");
    assert!(client
        .get(format!("http://{address}/health"))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    for (path, enabled) in [
        ("/__mockforge/api/route-chaos", cfg!(feature = "route-chaos")),
        ("/__mockforge/api/state-machines", cfg!(feature = "scenario-engine")),
        ("/api/world-state/graph", cfg!(feature = "world-state")),
        ("/api/performance/status", cfg!(feature = "performance-profiling")),
    ] {
        let response = client.get(format!("http://{address}{path}")).send().await.unwrap();
        assert_eq!(response.status().as_u16(), if enabled { 200 } else { 404 }, "{path}");
    }
}
