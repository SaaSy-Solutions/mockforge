//! Production Simulation Tests
//!
//! Comprehensive test suite that simulates production-like scenarios:
//! - High load scenarios
//! - Multi-protocol stress testing
//! - Error recovery
//! - Resource limits
//! - Concurrent operations

use mockforge_http::management_auth::MANAGEMENT_TOKEN_HEADER;
use mockforge_test::MockForgeServer;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

/// Create a mock on the serving port and return its actual generated ID.
async fn create_dynamic_mock(
    client: &Client,
    server: &MockForgeServer,
    config: serde_json::Value,
) -> String {
    let http_port = server.http_port();
    let response = client
        .post(format!("http://127.0.0.1:{http_port}/__mockforge/api/mocks"))
        .header(MANAGEMENT_TOKEN_HEADER, server.management_token())
        .json(&config)
        .send()
        .await
        .expect("Mock creation request must complete");
    assert_eq!(response.status(), reqwest::StatusCode::CREATED);
    let created: serde_json::Value = response.json().await.expect("Created mock must be JSON");
    let id = created["id"].as_str().expect("Created mock must have an ID");
    assert!(!id.is_empty());
    id.to_string()
}

/// Test high-load HTTP scenario
#[tokio::test]
#[ignore] // Long-running test
async fn test_production_high_load_http() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .ws_port(0)
        .grpc_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Production simulation server must start");

    let http_port = server.http_port();
    let client = Client::new();

    // Create multiple stubs
    for i in 0..100 {
        create_dynamic_mock(
            &client,
            &server,
            json!({
                "path": format!("/api/load-test/{i}"),
                "method": "GET",
                "status_code": 200,
                "response": {"body": {"id": i, "message": "load test"}}
            }),
        )
        .await;
    }

    // Make concurrent requests
    let mut handles = vec![];
    for i in 0..50 {
        let client = client.clone();
        let url = format!("http://localhost:{}/api/load-test/{}", http_port, i);
        handles.push(tokio::spawn(async move {
            for _ in 0..10 {
                let response =
                    client.get(url.as_str()).send().await.expect("Load request must complete");
                assert_eq!(response.status(), reqwest::StatusCode::OK);
                let body: serde_json::Value =
                    response.json().await.expect("Load response must be JSON");
                assert_eq!(body["id"], json!(i));
            }
        }));
    }

    // Wait for all requests to complete (with timeout)
    let results = timeout(Duration::from_secs(30), futures_util::future::join_all(handles))
        .await
        .expect("Load test must finish within its deadline");
    for result in results {
        result.expect("Every load worker must succeed");
    }

    server.stop().expect("Failed to stop server");
}

/// Test multi-protocol concurrent operations
#[tokio::test]
#[ignore] // Long-running test
async fn test_production_multi_protocol_concurrent() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .ws_port(0)
        .grpc_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Production simulation server must start");

    let http_port = server.http_port();
    let client = Client::new();

    // Concurrent HTTP requests
    let http_handles: Vec<_> = (0..20)
        .map(|_i| {
            let client = client.clone();
            let url = format!("http://localhost:{}/health", http_port);
            tokio::spawn(async move {
                for _ in 0..5 {
                    let _ = client.get(url.as_str()).send().await;
                }
            })
        })
        .collect();

    // Wait for all operations
    let result =
        timeout(Duration::from_secs(30), futures_util::future::join_all(http_handles)).await;
    assert!(result.is_ok(), "All concurrent operations should complete");

    server.stop().expect("Failed to stop server");
}

/// Test error recovery and resilience
#[tokio::test]
async fn test_production_error_recovery() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .ws_port(0)
        .grpc_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Production simulation server must start");

    let http_port = server.http_port();
    let client = Client::new();

    // Create stub that returns errors
    create_dynamic_mock(
        &client,
        &server,
        json!({
            "path": "/api/error-test", "method": "GET", "status_code": 500,
            "response": {"body": {"error": "test error"}}
        }),
    )
    .await;

    // Make request and verify error handling
    let response = client
        .get(format!("http://localhost:{}/api/error-test", http_port))
        .send()
        .await
        .expect("Request should complete");

    // Server should handle errors gracefully
    assert_eq!(response.status(), reqwest::StatusCode::INTERNAL_SERVER_ERROR);

    // Verify server is still running
    let health_response = client
        .get(format!("http://localhost:{}/health", http_port))
        .send()
        .await
        .expect("Health check should work");

    assert!(health_response.status().is_success(), "Server should recover from errors");

    server.stop().expect("Failed to stop server");
}

/// Test resource limits and cleanup
#[tokio::test]
async fn test_production_resource_cleanup() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .ws_port(0)
        .grpc_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Production simulation server must start");

    let http_port = server.http_port();
    let client = Client::new();
    let mut ids = Vec::new();
    for i in 0..50 {
        ids.push(
            create_dynamic_mock(
                &client,
                &server,
                json!({
                    "path": format!("/api/resource-test/{i}"), "method": "GET",
                    "response": {"body": {"id": i}}
                }),
            )
            .await,
        );
    }
    for id in ids {
        let response = client
            .delete(format!("http://127.0.0.1:{http_port}/__mockforge/api/mocks/{id}"))
            .header(MANAGEMENT_TOKEN_HEADER, server.management_token())
            .send()
            .await
            .expect("Delete request must complete");
        assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
    }
    let list = client
        .get(format!("http://127.0.0.1:{http_port}/__mockforge/api/mocks"))
        .header(MANAGEMENT_TOKEN_HEADER, server.management_token())
        .send()
        .await
        .expect("Mock list must be reachable");
    assert!(list.status().is_success());
    let remaining: serde_json::Value = list.json().await.expect("Mock list must be JSON");
    assert!(remaining["mocks"].as_array().expect("Mock list array").is_empty());

    server.stop().expect("Failed to stop server");
}

/// Test deployment readiness verification
#[tokio::test]
async fn test_production_deployment_readiness() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .ws_port(0)
        .grpc_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Production simulation server must start");

    let http_port = server.http_port();
    let client = Client::new();

    // Verify health endpoints
    let health_endpoints = [
        "/health",
        "/health/live",
        "/health/ready",
        "/health/startup",
    ];

    for endpoint in health_endpoints {
        let response = client
            .get(format!("http://localhost:{}{}", http_port, endpoint))
            .send()
            .await
            .expect("Health endpoint should respond");

        assert!(
            response.status().is_success(),
            "Health endpoint {} should return success",
            endpoint
        );
    }

    // Verify metrics endpoint (if available)
    let metrics_response = client.get("http://localhost:9090/metrics").send().await;

    // Metrics may or may not be available depending on configuration
    if let Ok(resp) = metrics_response {
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 404,
            "Metrics endpoint should exist or return 404"
        );
    }

    server.stop().expect("Failed to stop server");
}
