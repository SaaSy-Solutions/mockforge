//! End-to-end integration tests for tunnel server with request forwarding

#[cfg(feature = "server")]
use mockforge_tunnel::server::start_test_server;
use reqwest::StatusCode;
#[cfg(feature = "server")]
use std::time::Duration;
#[cfg(feature = "server")]
use tokio::time::sleep;

/// Simple test HTTP server that returns known responses
#[cfg(feature = "server")]
async fn start_test_local_server(port: u16) -> tokio::task::JoinHandle<()> {
    use axum::{response::Json, routing::get, Router};
    use serde_json::json;

    let app = Router::new()
        .route("/", get(|| async { Json(json!({"message": "Hello from local server"})) }))
        .route("/test", get(|| async { Json(json!({"status": "ok", "endpoint": "test"})) }))
        .route("/api/data", get(|| async { Json(json!({"data": [1, 2, 3], "count": 3})) }));

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .expect("Failed to bind local test server");
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("Local test server failed");
    })
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_end_to_end_path_based_tunnel() {
    // Start tunnel server on random port
    let tunnel_server_addr = start_test_server(0).await.unwrap();
    let tunnel_server_url = format!("http://{}", tunnel_server_addr);

    // Start local test server
    let _local_server = start_test_local_server(3001).await;
    sleep(Duration::from_millis(200)).await; // Give server time to start

    // Create HTTP client
    let client = reqwest::Client::new();

    // Verify local server is responding
    let local_check = client.get("http://localhost:3001/").send().await;
    if local_check.is_err() {
        eprintln!("⚠️  Local server not ready yet, waiting longer...");
        sleep(Duration::from_millis(300)).await;
    }
    let tunnel_resp = client
        .post(format!("{}/api/tunnels", tunnel_server_url))
        .json(&serde_json::json!({
            "local_url": "http://localhost:3001",
            "subdomain": "test-e2e"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(tunnel_resp.status(), StatusCode::OK);
    let tunnel: mockforge_tunnel::TunnelStatus = tunnel_resp.json().await.unwrap();
    assert!(tunnel.active);
    assert!(!tunnel.tunnel_id.is_empty());

    // Wait a bit for tunnel to be ready
    sleep(Duration::from_millis(100)).await;

    // Make request through tunnel using path-based routing
    let tunnel_url = format!("{}/tunnel/{}/test", tunnel_server_url, tunnel.tunnel_id);
    eprintln!("Making request to tunnel URL: {}", tunnel_url);
    let response = client.get(&tunnel_url).send().await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Expected 200 OK but got {} when requesting {}",
        response.status(),
        tunnel_url
    );
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["endpoint"], "test");

    // Test root path (note: trailing slash might be handled differently)
    let root_url = format!("{}/tunnel/{}/", tunnel_server_url, tunnel.tunnel_id);
    eprintln!("Making request to root tunnel URL: {}", root_url);
    let response = client.get(&root_url).send().await.unwrap();

    eprintln!("Root path response status: {:?}", response.status());
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Expected 200 OK but got {} when requesting root path {}",
        response.status(),
        root_url
    );
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["message"], "Hello from local server");

    // Verify tunnel stats were updated
    let status_resp = client
        .get(format!("{}/api/tunnels/{}", tunnel_server_url, tunnel.tunnel_id))
        .send()
        .await
        .unwrap();
    let status: mockforge_tunnel::TunnelStatus = status_resp.json().await.unwrap();
    assert!(status.request_count >= 2); // At least 2 requests made
    assert!(status.bytes_transferred > 0);
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_tunnel_not_found() {
    let tunnel_server_addr = start_test_server(0).await.unwrap();
    let tunnel_server_url = format!("http://{}", tunnel_server_addr);

    let client = reqwest::Client::new();

    // Try to access non-existent tunnel
    let response = client
        .get(format!("{}/tunnel/invalid-id/test", tunnel_server_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_tunnel_post_request() {
    let tunnel_server_addr = start_test_server(0).await.unwrap();
    let tunnel_server_url = format!("http://{}", tunnel_server_addr);

    // Start local test server with POST endpoint on random port
    let local_listener =
        tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind");
    let local_port = local_listener.local_addr().unwrap().port();
    let local_url = format!("http://127.0.0.1:{}", local_port);

    let _local_server = tokio::spawn(async move {
        use axum::{extract::Json, response::Json as RespJson, routing::post, Router};
        use serde_json::json;

        let app = Router::new().route(
            "/api/echo",
            post(|payload: Json<serde_json::Value>| async move {
                RespJson(json!({
                    "received": payload.0,
                    "echo": true
                }))
            }),
        );

        axum::serve(local_listener, app).await.expect("Server failed");
    });

    sleep(Duration::from_millis(200)).await;

    // Create tunnel
    let client = reqwest::Client::new();
    let tunnel_resp = client
        .post(format!("{}/api/tunnels", tunnel_server_url))
        .json(&serde_json::json!({
            "local_url": local_url
        }))
        .send()
        .await
        .unwrap();

    let tunnel: mockforge_tunnel::TunnelStatus = tunnel_resp.json().await.unwrap();

    // Make POST request through tunnel
    let response = client
        .post(format!("{}/tunnel/{}/api/echo", tunnel_server_url, tunnel.tunnel_id))
        .json(&serde_json::json!({"message": "test"}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["echo"], true);
    assert_eq!(body["received"]["message"], "test");
}
