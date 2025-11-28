//! HTTP/REST E2E tests
//!
//! End-to-end tests for HTTP/REST protocol functionality

use mockforge_test::{MockForgeServer, ServerConfig};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

/// Helper to assert response status
fn assert_status(response: &reqwest::Response, expected: u16) {
    assert_eq!(
        response.status().as_u16(),
        expected,
        "Expected status {}, got {}",
        expected,
        response.status()
    );
}

/// Helper to assert JSON response
async fn assert_json_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, Box<dyn std::error::Error>> {
    assert!(response.headers().get("content-type").is_some());
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        content_type.contains("application/json"),
        "Expected JSON response, got {}",
        content_type
    );
    Ok(response.json().await?)
}

#[tokio::test]
async fn test_http_basic_get() {
    // Start server with HTTP config
    let server = MockForgeServer::builder()
        .http_port(0) // Auto-assign
        .admin_port(0) // Auto-assign
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    let http_port = server.http_port();
    let admin_port = 9080; // Default admin port

    // Create a stub via Admin API
    let client = Client::new();
    let stub_response = client
        .post(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .json(&json!({
            "path": "/api/users",
            "method": "GET",
            "response": {
                "status": 200,
                "body": [
                    {"id": 1, "name": "Alice"},
                    {"id": 2, "name": "Bob"}
                ]
            }
        }))
        .send()
        .await
        .expect("Failed to create stub");

    assert_status(&stub_response, 201);

    // Make GET request
    let response = client
        .get(&format!("http://localhost:{}/api/users", http_port))
        .send()
        .await
        .expect("Failed to make GET request");

    assert_status(&response, 200);
    let body: serde_json::Value = assert_json_response(response).await.expect("Failed to parse JSON");
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 2);

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_http_post_with_validation() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    let http_port = server.http_port();
    let admin_port = 9080;

    // Create POST stub
    let client = Client::new();
    let stub_response = client
        .post(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .json(&json!({
            "path": "/api/users",
            "method": "POST",
            "response": {
                "status": 201,
                "body": {
                    "id": 123,
                    "name": "Alice",
                    "email": "alice@example.com"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create stub");

    assert_status(&stub_response, 201);

    // Make POST request
    let response = client
        .post(&format!("http://localhost:{}/api/users", http_port))
        .json(&json!({
            "name": "Alice",
            "email": "alice@example.com"
        }))
        .send()
        .await
        .expect("Failed to make POST request");

    assert_status(&response, 201);
    let body: serde_json::Value = assert_json_response(response).await.expect("Failed to parse JSON");
    assert_eq!(body["id"], 123);
    assert_eq!(body["name"], "Alice");

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_http_dynamic_stub_creation() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    let http_port = server.http_port();
    let admin_port = 9080;
    let client = Client::new();

    // Create stub via Admin API
    let stub_response = client
        .post(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .json(&json!({
            "path": "/api/test",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {"message": "test"}
            }
        }))
        .send()
        .await
        .expect("Failed to create stub");

    assert_status(&stub_response, 201);

    // Verify stub works
    let test_response = client
        .get(&format!("http://localhost:{}/api/test", http_port))
        .send()
        .await
        .expect("Failed to test stub");

    assert_status(&test_response, 200);
    let body: serde_json::Value = assert_json_response(test_response).await.expect("Failed to parse JSON");
    assert_eq!(body["message"], "test");

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_http_stub_update() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    let http_port = server.http_port();
    let admin_port = 9080;
    let client = Client::new();

    // Create initial stub
    let create_response = client
        .post(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .json(&json!({
            "path": "/api/update-test",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {"version": 1}
            }
        }))
        .send()
        .await
        .expect("Failed to create stub");

    assert_status(&create_response, 201);
    let stub_data: serde_json::Value = assert_json_response(create_response).await.expect("Failed to parse JSON");
    let stub_id = stub_data["id"].as_str().expect("No stub ID returned");

    // Update stub
    let update_response = client
        .put(&format!("http://localhost:{}/__mockforge/api/mocks/{}", admin_port, stub_id))
        .json(&json!({
            "path": "/api/update-test",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {"version": 2}
            }
        }))
        .send()
        .await
        .expect("Failed to update stub");

    assert_status(&update_response, 200);

    // Verify updated stub
    let test_response = client
        .get(&format!("http://localhost:{}/api/update-test", http_port))
        .send()
        .await
        .expect("Failed to test updated stub");

    assert_status(&test_response, 200);
    let body: serde_json::Value = assert_json_response(test_response).await.expect("Failed to parse JSON");
    assert_eq!(body["version"], 2);

    server.stop().expect("Failed to stop server");
}

#[tokio::test]
async fn test_http_stub_deletion() {
    let server = MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .health_timeout(Duration::from_secs(30))
        .build()
        .await
        .expect("Failed to start test server");

    let http_port = server.http_port();
    let admin_port = 9080;
    let client = Client::new();

    // Create stub
    let create_response = client
        .post(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .json(&json!({
            "path": "/api/delete-test",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {"message": "exists"}
            }
        }))
        .send()
        .await
        .expect("Failed to create stub");

    assert_status(&create_response, 201);
    let stub_data: serde_json::Value = assert_json_response(create_response).await.expect("Failed to parse JSON");
    let stub_id = stub_data["id"].as_str().expect("No stub ID returned");

    // Verify stub exists
    let test_response = client
        .get(&format!("http://localhost:{}/api/delete-test", http_port))
        .send()
        .await
        .expect("Failed to test stub");

    assert_status(&test_response, 200);

    // Delete stub
    let delete_response = client
        .delete(&format!("http://localhost:{}/__mockforge/api/mocks/{}", admin_port, stub_id))
        .send()
        .await
        .expect("Failed to delete stub");

    assert_status(&delete_response, 204);

    // Verify stub is gone (should return 404)
    let test_response = client
        .get(&format!("http://localhost:{}/api/delete-test", http_port))
        .send()
        .await
        .expect("Failed to test deleted stub");

    assert_status(&test_response, 404);

    server.stop().expect("Failed to stop server");
}
