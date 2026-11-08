// Cloud-only HTTP and startup boundary checks.
#![cfg(feature = "cloud-validation")]

use axum::{routing::get, Json, Router};
use mockforge_tunnel::{
    cloud_validation::RegistryTunnelStore,
    server::{create_tunnel_server_router, TunnelStore, TunnelStoreTrait, TunnelStoreWrapper},
    TunnelConfig,
};
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::test]
async fn cloud_binary_refuses_missing_or_invalid_configuration() {
    for (registry, token, expected) in [
        (None, None, "REGISTRY_URL required"),
        (Some("http://127.0.0.1:1"), Some(" "), "MOCKFORGE_INTERNAL_API_TOKEN required"),
        (Some("file:///tmp/registry"), Some("fixture"), "REGISTRY_URL must be"),
    ] {
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_tunnel-server"));
        command
            .env_remove("REGISTRY_URL")
            .env_remove("MOCKFORGE_INTERNAL_API_TOKEN")
            .env("TUNNEL_USE_IN_MEMORY_STORAGE", "1")
            .env("TUNNEL_SERVER_BIND", "127.0.0.1")
            .env("TUNNEL_SERVER_PORT", "0")
            .kill_on_drop(true);
        if let Some(value) = registry {
            command.env("REGISTRY_URL", value);
        }
        if let Some(value) = token {
            command.env("MOCKFORGE_INTERNAL_API_TOKEN", value);
        }
        let output = tokio::time::timeout(std::time::Duration::from_secs(5), command.output())
            .await
            .expect("invalid cloud configuration started a server")
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
}

async fn registry() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = Router::new().route(
        "/api/v1/internal/tunnel-reservations/by-subdomain/{name}",
        get(|| async { Json(serde_json::json!({"id":"00000000-0000-4000-8000-000000000001", "org_id":"00000000-0000-4000-8000-000000000002", "subdomain":"reserved", "status": "reserved"})) }),
    );
    let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (format!("http://{address}"), task)
}

#[tokio::test]
async fn reservations_never_create_connected_destinations() {
    let (url, task) = registry().await;
    let inner = Arc::new(TunnelStore::new());
    let cloud = RegistryTunnelStore::new(inner.clone(), url, "fixture-token".to_string());
    for _ in 0..2 {
        assert!(cloud.get_tunnel_by_subdomain("reserved").await.is_err());
    }
    assert!(inner.list_tunnels().await.is_empty());
    task.abort();
}

#[tokio::test]
async fn legacy_local_cache_cannot_authorize_cloud_forwarding() {
    let (url, task) = registry().await;
    let inner = Arc::new(TunnelStore::new());
    let legacy = inner
        .create_tunnel(&TunnelConfig {
            subdomain: Some("reserved".to_string()),
            local_url: "http://127.0.0.1:1".to_string(),
            ..Default::default()
        })
        .await
        .unwrap();
    let cloud = RegistryTunnelStore::new(inner.clone(), url, "fixture-token".to_string());
    assert!(cloud.get_tunnel_by_subdomain("reserved").await.is_err());
    assert!(cloud.get_tunnel_by_id(&legacy.tunnel_id).await.is_err());
    assert!(cloud.get_tunnel(&legacy.tunnel_id).await.is_err());
    // Refusal preserves the original state instead of silently deleting it.
    assert!(inner.get_tunnel(&legacy.tunnel_id).await.is_ok());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = create_tunnel_server_router().with_state(TunnelStoreWrapper::new(Arc::new(cloud)));
    let relay = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let http = reqwest::Client::new();
    for path in [
        "/owned-canary".to_string(),
        format!("/tunnel/{}", legacy.tunnel_id),
        format!("/tunnel/{}/", legacy.tunnel_id),
        format!("/tunnel/{}/owned-canary", legacy.tunnel_id),
    ] {
        let response = http
            .get(format!("http://{address}{path}"))
            .header("host", "reserved.tunnels.mockforge.dev")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    }
    relay.abort();
    task.abort();
}

#[tokio::test]
async fn cloud_management_http_refuses_anonymous_destination_registration_and_disclosure() {
    let cloud = RegistryTunnelStore::new(
        Arc::new(TunnelStore::new()),
        "http://127.0.0.1:1".to_string(),
        "fixture-token".to_string(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = create_tunnel_server_router().with_state(TunnelStoreWrapper::new(Arc::new(cloud)));
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let http = reqwest::Client::new();
    for (method, path) in [
        (reqwest::Method::POST, "/api/tunnels"),
        (reqwest::Method::GET, "/api/tunnels"),
        (reqwest::Method::GET, "/api/tunnels/unowned"),
        (reqwest::Method::DELETE, "/api/tunnels/unowned"),
    ] {
        let response = http
            .request(method, format!("http://{address}{path}"))
            .json(&serde_json::json!({"local_url": "http://127.0.0.1:1"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NOT_IMPLEMENTED);
    }
    assert_eq!(
        http.get(format!("http://{address}/health")).send().await.unwrap().status(),
        reqwest::StatusCode::OK
    );
    task.abort();
}
