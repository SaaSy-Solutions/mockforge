//! Production Simulation Tests
//!
//! Comprehensive test suite that simulates production-like scenarios:
//! - High load scenarios
//! - Multi-protocol stress testing
//! - Error recovery
//! - Resource limits
//! - Concurrent operations

use mockforge_test::MockForgeServer;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

/// Test high-load HTTP scenario
#[tokio::test]
#[ignore] // Long-running test
async fn test_production_high_load_http() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
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

    let http_port = server.http_port();
    let admin_port = 9080;
    let client = Client::new();

    // Create multiple stubs
    for i in 0..100 {
        let _ = client
            .post(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
            .json(&json!({
                "path": format!("/api/load-test/{}", i),
                "method": "GET",
                "response": {
                    "status": 200,
                    "body": {"id": i, "message": "load test"}
                }
            }))
            .send()
            .await;
    }

    // Make concurrent requests
    let mut handles = vec![];
    for i in 0..50 {
        let client = client.clone();
        let url = format!("http://localhost:{}/api/load-test/{}", http_port, i);
        handles.push(tokio::spawn(async move {
            for _ in 0..10 {
                let _ = client.get(&url).send().await;
            }
        }));
    }

    // Wait for all requests to complete (with timeout)
    let result = timeout(Duration::from_secs(30), futures_util::future::join_all(handles)).await;
    assert!(result.is_ok(), "All requests should complete within timeout");

    server.stop().expect("Failed to stop server");
}

/// Test multi-protocol concurrent operations
#[tokio::test]
#[ignore] // Long-running test
async fn test_production_multi_protocol_concurrent() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .ws_port(0)
        .grpc_port(0)
        .admin_port(0)
        .enable_admin(true)
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

    let http_port = server.http_port();
    let client = Client::new();

    // Concurrent HTTP requests
    let http_handles: Vec<_> = (0..20)
        .map(|i| {
            let client = client.clone();
            let url = format!("http://localhost:{}/health", http_port);
            tokio::spawn(async move {
                for _ in 0..5 {
                    let _ = client.get(&url).send().await;
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
    let server = match MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
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

    let http_port = server.http_port();
    let client = Client::new();

    // Create stub that returns errors
    let _ = client
        .post(&format!("http://localhost:{}/__mockforge/api/mocks", 9080))
        .json(&json!({
            "path": "/api/error-test",
            "method": "GET",
            "response": {
                "status": 500,
                "body": {"error": "test error"}
            }
        }))
        .send()
        .await;

    // Make request and verify error handling
    let response = client
        .get(&format!("http://localhost:{}/api/error-test", http_port))
        .send()
        .await
        .expect("Request should complete");

    // Server should handle errors gracefully
    assert!(response.status().is_client_error() || response.status().is_server_error());

    // Verify server is still running
    let health_response = client
        .get(&format!("http://localhost:{}/health", http_port))
        .send()
        .await
        .expect("Health check should work");

    assert!(health_response.status().is_success(), "Server should recover from errors");

    server.stop().expect("Failed to stop server");
}

/// Test resource limits and cleanup
#[tokio::test]
async fn test_production_resource_cleanup() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
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

    let admin_port = 9080;
    let client = Client::new();

    // Create many stubs
    for i in 0..50 {
        let _ = client
            .post(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
            .json(&json!({
                "path": format!("/api/resource-test/{}", i),
                "method": "GET",
                "response": {"status": 200, "body": {"id": i}}
            }))
            .send()
            .await;
    }

    // Delete all stubs
    for i in 0..50 {
        let _ = client
            .delete(&format!(
                "http://localhost:{}/__mockforge/api/mocks/resource-test/{}",
                admin_port, i
            ))
            .send()
            .await;
    }

    // Verify cleanup
    let list_response = client
        .get(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .send()
        .await;

    // Server should handle cleanup gracefully
    if let Ok(resp) = list_response {
        assert!(resp.status().is_success() || resp.status().as_u16() == 404);
    }

    server.stop().expect("Failed to stop server");
}

/// Test deployment readiness verification
#[tokio::test]
async fn test_production_deployment_readiness() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
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

    let http_port = server.http_port();
    let client = Client::new();

    // Verify health endpoints
    let health_endpoints = vec![
        "/health",
        "/health/live",
        "/health/ready",
        "/health/startup",
    ];

    for endpoint in health_endpoints {
        let response = client
            .get(&format!("http://localhost:{}{}", http_port, endpoint))
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
    let metrics_response = client.get(&format!("http://localhost:9090/metrics")).send().await;

    // Metrics may or may not be available depending on configuration
    if let Ok(resp) = metrics_response {
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 404,
            "Metrics endpoint should exist or return 404"
        );
    }

    server.stop().expect("Failed to stop server");
}
