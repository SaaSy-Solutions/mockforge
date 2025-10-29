//! Integration tests for MockForge SDK

use mockforge_sdk::MockServer;
use serde_json::json;

#[tokio::test]
async fn test_basic_server_start_stop() {
    let mut server = MockServer::new()
        .port(0) // Use random port
        .start()
        .await
        .expect("Failed to start server");

    assert!(server.is_running());
    assert!(server.port() > 0);

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_stub_get_request() {
    let mut server = MockServer::new().port(0).start().await.expect("Failed to start server");

    // Add a stub
    server
        .stub_response(
            "GET",
            "/api/users/123",
            json!({
                "id": 123,
                "name": "John Doe",
                "email": "john@example.com"
            }),
        )
        .await
        .expect("Failed to add stub");

    // Make a request
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/users/123", server.url()))
        .send()
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["id"], 123);
    assert_eq!(body["name"], "John Doe");

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_stub_post_request() {
    let mut server = MockServer::new().port(0).start().await.expect("Failed to start server");

    // Add a stub for POST
    server
        .stub_response(
            "POST",
            "/api/users",
            json!({
                "id": 456,
                "status": "created"
            }),
        )
        .await
        .expect("Failed to add stub");

    // Make a POST request
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/users", server.url()))
        .json(&json!({"name": "Jane Doe"}))
        .send()
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["id"], 456);
    assert_eq!(body["status"], "created");

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_multiple_stubs() {
    let mut server = MockServer::new().port(0).start().await.expect("Failed to start server");

    // Add multiple stubs
    server
        .stub_response("GET", "/api/users/1", json!({"id": 1, "name": "User 1"}))
        .await
        .expect("Failed to add stub");

    server
        .stub_response("GET", "/api/users/2", json!({"id": 2, "name": "User 2"}))
        .await
        .expect("Failed to add stub");

    server
        .stub_response("GET", "/api/products", json!({"products": []}))
        .await
        .expect("Failed to add stub");

    // Test each stub
    let client = reqwest::Client::new();

    let resp1 = client
        .get(format!("{}/api/users/1", server.url()))
        .send()
        .await
        .expect("Failed to make request");
    assert_eq!(resp1.status(), 200);

    let resp2 = client
        .get(format!("{}/api/users/2", server.url()))
        .send()
        .await
        .expect("Failed to make request");
    assert_eq!(resp2.status(), 200);

    let resp3 = client
        .get(format!("{}/api/products", server.url()))
        .send()
        .await
        .expect("Failed to make request");
    assert_eq!(resp3.status(), 200);

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_clear_stubs() {
    let mut server = MockServer::new().port(0).start().await.expect("Failed to start server");

    // Add a stub
    server
        .stub_response("GET", "/api/test", json!({"message": "hello"}))
        .await
        .expect("Failed to add stub");

    // Clear all stubs
    server.clear_stubs().await.expect("Failed to clear stubs");

    server.stop().await.expect("Failed to stop server");
}
