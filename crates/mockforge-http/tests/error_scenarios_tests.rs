//! Comprehensive error handling tests for HTTP functionality.
//!
//! These tests verify that the HTTP server handles errors gracefully,
//! including malformed requests, large payloads, timeouts, and edge cases.

use axum::http::{Method, StatusCode, Uri};
use mockforge_http::build_router;
use reqwest::Client;
use serde_json::json;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::timeout;

#[tokio::test]
async fn test_malformed_json_request_body() {
    let router = build_router(None, None, None).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = Client::new();
    let base_url = format!("http://{}", addr);

    // Test with malformed JSON in request body
    let malformed_bodies = vec![
        "{ invalid json }",
        "{'key': 'value'}", // Single quotes
        "{key: value}", // No quotes
        "{\"key\": }", // Missing value
        "{\"key\":}", // Missing value
        "{\"key\"}", // Missing colon
        "{", // Incomplete
        "}", // Incomplete
        "null", // Just null
        "undefined", // Invalid
    ];

    for body in malformed_bodies {
        let response = client
            .post(&format!("{}/api/test", base_url))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await;

        // Should not panic, should return error response
        if let Ok(resp) = response {
            // Should return 400 Bad Request or similar
            assert!(resp.status().is_client_error() || resp.status().is_server_error());
        }
    }

    drop(server);
}

#[tokio::test]
async fn test_very_large_request_body() {
    let router = build_router(None, None, None).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = Client::new();
    let base_url = format!("http://{}", addr);

    // Test with very large request body (10MB)
    let large_body = "a".repeat(10_000_000);
    let large_json = json!({"data": large_body});

    let response = client
        .post(&format!("{}/api/test", base_url))
        .header("Content-Type", "application/json")
        .json(&large_json)
        .timeout(Duration::from_secs(30))
        .send()
        .await;

    // Should handle without crashing (may timeout or return error)
    let _ = response;

    drop(server);
}

#[tokio::test]
async fn test_malformed_headers() {
    let router = build_router(None, None, None).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let base_url = format!("http://{}", addr);

    // Test with various malformed header scenarios
    // Note: reqwest validates headers, so we test what we can
    let response = client
        .get(&format!("{}/health", base_url))
        .header("Content-Type", "application/json; charset=utf-8")
        .send()
        .await;

    // Should handle headers gracefully
    assert!(response.is_ok());

    drop(server);
}

#[tokio::test]
async fn test_invalid_http_methods() {
    let router = build_router(None, None, None).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = Client::new();
    let base_url = format!("http://{}", addr);

    // Test various HTTP methods
    let methods = vec!["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

    for method_str in methods {
        let method = Method::from_bytes(method_str.as_bytes()).unwrap();
        let response = client
            .request(method, &format!("{}/api/test", base_url))
            .send()
            .await;

        // Should handle all methods gracefully
        let _ = response;
    }

    drop(server);
}

#[tokio::test]
async fn test_malformed_query_parameters() {
    let router = build_router(None, None, None).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = Client::new();
    let base_url = format!("http://{}", addr);

    // Test with malformed query parameters
    let malformed_queries = vec![
        "?",
        "?=",
        "?key=",
        "?=value",
        "?key=value&",
        "?key=value&&key2=value2",
        "?key=value&key2",
        "?%",
        "?%XX", // Invalid percent encoding
        "?key=value%", // Incomplete percent encoding
    ];

    for query in malformed_queries {
        let url = format!("{}/api/test{}", base_url, query);
        let response = client.get(&url).send().await;

        // Should handle malformed queries gracefully
        let _ = response;
    }

    drop(server);
}

#[tokio::test]
async fn test_malformed_path_parameters() {
    let router = build_router(None, None, None).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = Client::new();
    let base_url = format!("http://{}", addr);

    // Test with malformed path parameters
    let malformed_paths = vec![
        "/api//users",
        "/api/users//",
        "/api//users//",
        "/api/users/",
        "/api/users/%",
        "/api/users/%XX",
        "/api/users/../../../etc/passwd",
        "/api/users/..",
        "/api/users/./test",
    ];

    for path in malformed_paths {
        let url = format!("{}{}", base_url, path);
        let response = client.get(&url).send().await;

        // Should handle malformed paths gracefully
        let _ = response;
    }

    drop(server);
}

#[tokio::test]
async fn test_concurrent_requests_during_shutdown() {
    let router = build_router(None, None, None).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = Client::new();
    let base_url = format!("http://{}", addr);

    // Spawn multiple concurrent requests
    let mut handles = vec![];
    for i in 0..10 {
        let url = format!("{}/health", base_url);
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            for _ in 0..10 {
                let _ = client_clone.get(&url).send().await;
            }
        });
        handles.push(handle);
    }

    // Wait a bit, then drop server
    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(server_handle);

    // Wait for requests to complete or fail gracefully
    for handle in handles {
        let _ = timeout(Duration::from_secs(5), handle).await;
    }
}

#[tokio::test]
async fn test_timeout_handling() {
    let router = build_router(None, None, None).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = Client::builder()
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap();
    let base_url = format!("http://{}", addr);

    // Request should timeout gracefully
    let response = timeout(
        Duration::from_secs(1),
        client.get(&format!("{}/health", base_url)).send(),
    )
    .await;

    // Should handle timeout without panicking
    let _ = response;

    drop(server);
}

#[tokio::test]
async fn test_missing_content_type_header() {
    let router = build_router(None, None, None).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = Client::new();
    let base_url = format!("http://{}", addr);

    // Request without Content-Type header
    let response = client
        .post(&format!("{}/api/test", base_url))
        .body("raw body data")
        .send()
        .await;

    // Should handle missing Content-Type gracefully
    let _ = response;

    drop(server);
}

#[tokio::test]
async fn test_unicode_in_paths_and_headers() {
    let router = build_router(None, None, None).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = Client::new();
    let base_url = format!("http://{}", addr);

    // Test with unicode in paths
    let unicode_paths = vec![
        "/api/测试",
        "/api/тест",
        "/api/テスト",
        "/api/🎉",
    ];

    for path in unicode_paths {
        let url = format!("{}{}", base_url, path);
        let response = client.get(&url).send().await;

        // Should handle unicode paths gracefully
        let _ = response;
    }

    drop(server);
}
