//! Dynamic stub tests

use mockforge_sdk::{DynamicStub, RequestContext};
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_dynamic_stub_creation() {
    let stub = DynamicStub::new("GET", "/api/users", |_ctx| {
        json!({
            "users": ["Alice", "Bob", "Charlie"]
        })
    });

    assert_eq!(stub.method, "GET");
    assert_eq!(stub.path, "/api/users");
    assert_eq!(stub.get_status().await, 200);
}

#[tokio::test]
async fn test_dynamic_stub_response_generation() {
    let stub = DynamicStub::new("GET", "/api/echo", |ctx| {
        json!({
            "method": ctx.method,
            "path": ctx.path,
            "params": ctx.query_params
        })
    });

    let ctx = RequestContext {
        method: "GET".to_string(),
        path: "/api/echo".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::from([
            ("name".to_string(), "test".to_string()),
            ("value".to_string(), "123".to_string()),
        ]),
        headers: HashMap::new(),
        body: None,
    };

    let response = stub.generate_response(&ctx);
    assert_eq!(response["method"], "GET");
    assert_eq!(response["path"], "/api/echo");
    assert_eq!(response["params"]["name"], "test");
    assert_eq!(response["params"]["value"], "123");
}

#[tokio::test]
async fn test_dynamic_stub_with_path_params() {
    let stub = DynamicStub::new("GET", "/api/users/{id}", |ctx| {
        let user_id = ctx.path_params.get("id").cloned().unwrap_or_default();
        json!({
            "id": user_id,
            "name": format!("User {}", user_id),
            "email": format!("user{}@example.com", user_id)
        })
    });

    let ctx = RequestContext {
        method: "GET".to_string(),
        path: "/api/users/123".to_string(),
        path_params: HashMap::from([("id".to_string(), "123".to_string())]),
        query_params: HashMap::new(),
        headers: HashMap::new(),
        body: None,
    };

    let response = stub.generate_response(&ctx);
    assert_eq!(response["id"], "123");
    assert_eq!(response["name"], "User 123");
    assert_eq!(response["email"], "user123@example.com");
}

#[tokio::test]
async fn test_dynamic_stub_modify_status() {
    let stub = DynamicStub::new("GET", "/api/test", |_ctx| json!({"status": "ok"}));

    // Initial status should be 200
    assert_eq!(stub.get_status().await, 200);

    // Modify status
    stub.set_status(404).await;
    assert_eq!(stub.get_status().await, 404);

    // Modify again
    stub.set_status(201).await;
    assert_eq!(stub.get_status().await, 201);
}

#[tokio::test]
async fn test_dynamic_stub_modify_headers() {
    let stub = DynamicStub::new("GET", "/api/test", |_ctx| json!({"data": "test"}));

    // Initially no headers
    assert_eq!(stub.get_headers().await.len(), 0);

    // Add headers
    stub.add_header("Content-Type".to_string(), "application/json".to_string())
        .await;
    stub.add_header("X-Custom".to_string(), "custom-value".to_string())
        .await;

    let headers = stub.get_headers().await;
    assert_eq!(headers.len(), 2);
    assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
    assert_eq!(headers.get("X-Custom").unwrap(), "custom-value");

    // Remove a header
    stub.remove_header("X-Custom").await;
    let headers = stub.get_headers().await;
    assert_eq!(headers.len(), 1);
    assert!(!headers.contains_key("X-Custom"));
}

#[tokio::test]
async fn test_dynamic_stub_with_latency() {
    let stub = DynamicStub::new("GET", "/api/slow", |_ctx| json!({"message": "slow"}))
        .with_latency(100);

    assert_eq!(stub.latency_ms, Some(100));
}

#[tokio::test]
async fn test_dynamic_stub_request_body_access() {
    let stub = DynamicStub::new("POST", "/api/echo", |ctx| {
        match &ctx.body {
            Some(body) => json!({
                "echoed": body,
                "message": "Body received"
            }),
            None => json!({
                "error": "No body provided"
            }),
        }
    });

    // Test with body
    let ctx_with_body = RequestContext {
        method: "POST".to_string(),
        path: "/api/echo".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        headers: HashMap::new(),
        body: Some(json!({"name": "Alice", "age": 30})),
    };

    let response = stub.generate_response(&ctx_with_body);
    assert_eq!(response["message"], "Body received");
    assert_eq!(response["echoed"]["name"], "Alice");
    assert_eq!(response["echoed"]["age"], 30);

    // Test without body
    let ctx_without_body = RequestContext {
        method: "POST".to_string(),
        path: "/api/echo".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        headers: HashMap::new(),
        body: None,
    };

    let response = stub.generate_response(&ctx_without_body);
    assert_eq!(response["error"], "No body provided");
}

#[tokio::test]
async fn test_dynamic_stub_counter() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let stub = DynamicStub::new("GET", "/api/counter", move |_ctx| {
        let count = counter_clone.fetch_add(1, Ordering::SeqCst);
        json!({
            "count": count
        })
    });

    let ctx = RequestContext {
        method: "GET".to_string(),
        path: "/api/counter".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        headers: HashMap::new(),
        body: None,
    };

    // First call
    let response1 = stub.generate_response(&ctx);
    assert_eq!(response1["count"], 0);

    // Second call
    let response2 = stub.generate_response(&ctx);
    assert_eq!(response2["count"], 1);

    // Third call
    let response3 = stub.generate_response(&ctx);
    assert_eq!(response3["count"], 2);
}
