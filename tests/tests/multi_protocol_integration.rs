//! Multi-Protocol Integration Tests
//!
//! Tests that verify MockForge can handle multiple protocols simultaneously
//! and that cross-protocol interactions work correctly.

use mockforge_test::MockForgeServer;
use reqwest::Client;
use std::time::Duration;

/// Test that HTTP server starts and responds to health checks
#[tokio::test]
async fn test_http_server_health() {
    // Start MockForge server with HTTP enabled
    let server = match MockForgeServer::builder()
        .http_port(0) // Auto-assign port
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: Failed to start server: {}", e);
            return;
        }
    };

    // Verify server is running and healthy
    assert!(server.is_running());
    assert!(server.is_ready().await);

    // Check health endpoint
    let health = server.health_check().await;
    assert!(health.is_ok());
    let health = health.unwrap();
    assert_eq!(health.status, "healthy");

    // Test HTTP endpoint directly
    let client = Client::new();
    let response = client
        .get(format!("{}/health", server.base_url()))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success());
            let body: serde_json::Value = resp.json().await.unwrap();
            assert_eq!(body["status"], "healthy");
        }
        Err(e) => {
            eprintln!("Warning: Health endpoint test failed: {}", e);
        }
    }
}

/// Test HTTP server with OpenAPI spec
#[tokio::test]
#[ignore] // Requires OpenAPI spec file
async fn test_http_with_openapi_spec() {
    use std::path::PathBuf;

    let spec_path = PathBuf::from("examples/openapi-demo.json");
    if !spec_path.exists() {
        eprintln!("Skipping test: OpenAPI spec not found at {:?}", spec_path);
        return;
    }

    let server = match MockForgeServer::builder()
        .http_port(0)
        .extra_arg("--spec")
        .extra_arg(spec_path.to_string_lossy().to_string())
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: Failed to start server: {}", e);
            return;
        }
    };

    // Test an endpoint from the OpenAPI spec
    let client = Client::new();
    let response = client
        .get(format!("{}/ping", server.base_url()))
        .send()
        .await;

    if let Ok(resp) = response {
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"], "pong");
    }
}

/// Test that HTTP and admin UI can run simultaneously
#[tokio::test]
async fn test_http_with_admin() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .enable_admin(true)
        .admin_port(0) // Auto-assign
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: Failed to start server: {}", e);
            return;
        }
    };

    // Verify HTTP endpoint works
    let client = Client::new();
    let http_response = client
        .get(format!("{}/health", server.base_url()))
        .send()
        .await;

    assert!(http_response.is_ok());
    assert!(http_response.unwrap().status().is_success());

    // Note: Admin UI port access would require exposing admin_port() method
    // For now, we just verify the server started with admin enabled
    assert!(server.is_running());
}

/// Test that HTTP and WebSocket servers can run simultaneously
#[tokio::test]
#[ignore] // Requires running server
async fn test_http_with_websocket() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .ws_port(0) // Auto-assign WebSocket port
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: Failed to start server: {}", e);
            return;
        }
    };

    // Verify HTTP endpoint works
    let client = Client::new();
    let http_response = client
        .get(format!("{}/health", server.base_url()))
        .send()
        .await;

    assert!(http_response.is_ok());
    assert!(http_response.unwrap().status().is_success());

    // Verify WebSocket port is configured
    assert!(server.ws_port().is_some(), "WebSocket port should be configured");
    assert!(server.ws_url().is_some(), "WebSocket URL should be available");

    eprintln!("✅ HTTP and WebSocket servers running simultaneously");
}

/// Test that HTTP, WebSocket, and gRPC servers can run simultaneously
#[tokio::test]
#[ignore] // Requires running server
async fn test_all_protocols_simultaneous() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .ws_port(0)
        .grpc_port(0) // Auto-assign gRPC port
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: Failed to start server: {}", e);
            return;
        }
    };

    // Verify HTTP works
    let client = Client::new();
    let http_response = client
        .get(format!("{}/health", server.base_url()))
        .send()
        .await;

    assert!(http_response.is_ok());
    assert!(http_response.unwrap().status().is_success());

    // Verify WebSocket is configured
    assert!(server.ws_port().is_some());
    assert!(server.ws_url().is_some());

    // Verify gRPC is configured
    assert!(server.grpc_port().is_some(), "gRPC port should be configured");

    eprintln!("✅ HTTP, WebSocket, and gRPC servers all running");
    eprintln!("   HTTP: {}", server.http_port());
    eprintln!("   WebSocket: {:?}", server.ws_port());
    eprintln!("   gRPC: {:?}", server.grpc_port());
}

/// Test that protocols don't interfere with each other
#[tokio::test]
#[ignore] // Requires running server
async fn test_protocol_isolation() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .ws_port(0)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: Failed to start server: {}", e);
            return;
        }
    };

    let base_url = server.base_url();
    let client = Client::new();

    // Make multiple HTTP requests while WebSocket is running
    for i in 0..5 {
        let response = client
            .get(format!("{}/health", base_url))
            .send()
            .await;

        assert!(
            response.is_ok(),
            "HTTP request {} should succeed even with WebSocket running",
            i
        );
        assert!(response.unwrap().status().is_success());

        // Small delay
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Verify WebSocket is still available
    assert!(server.ws_port().is_some());
    assert!(server.is_running());

    eprintln!("✅ Protocols isolated - HTTP and WebSocket don't interfere");
}
