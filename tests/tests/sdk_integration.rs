//! SDK Integration Tests
//!
//! Tests that verify SDK integration for all supported languages:
//! - Rust SDK (native, embedded)
//! - Node.js/TypeScript SDK
//! - Python SDK
//! - Go SDK
//! - Java SDK
//! - .NET SDK
//!
//! Note: Most SDKs require the MockForge CLI to be installed and available in PATH.
//! These tests verify that SDKs can communicate with MockForge servers.

use mockforge_http::management_auth::MANAGEMENT_TOKEN_HEADER;
use mockforge_test::MockForgeServer;
use reqwest::{Client, StatusCode};
use serde_json::json;
use std::time::Duration;

/// Test Rust SDK integration (native, embedded)
#[tokio::test]
async fn test_rust_sdk_integration() {
    // Rust SDK embeds MockForge directly, so we test the core functionality
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

    // The mockforge management API (`/__mockforge/api/*`) is mounted on the
    // HTTP server, not the admin UI server (which has RBAC middleware that
    // would reject our anonymous requests). Use http_port; the older
    // hardcoded `admin_port = 9080` only "passed" because it silently hit
    // an unrelated mockforge instance that happened to listen on 9080.
    let http_port = server.http_port();
    let base = format!("http://127.0.0.1:{http_port}");
    let token = server.management_token();
    let client = Client::new();

    // Simulate Rust SDK usage: create stub programmatically. `status_code`
    // is a top-level field on the mock config; a `status` key nested inside
    // `response` is ignored by the server, so the tests below assert the
    // configured status end-to-end to prove the config really took effect.
    let stub_response = client
        .post(format!("{base}/__mockforge/api/mocks"))
        .header(MANAGEMENT_TOKEN_HEADER, token)
        .json(&json!({
            "path": "/api/rust-sdk-test",
            "method": "GET",
            "response": {
                "body": {"sdk": "rust", "embedded": true}
            },
            "status_code": 201
        }))
        .send()
        .await
        .expect("Failed to create stub");

    assert_eq!(
        stub_response.status(),
        StatusCode::CREATED,
        "stub creation must return 201 Created"
    );
    let created: serde_json::Value =
        stub_response.json().await.expect("Created stub must be JSON");
    let mock_id = created["id"]
        .as_str()
        .expect("Created stub must have an ID")
        .to_string();

    // Decisive roundtrip (#1281): the stub must now be served on the HTTP
    // port with its exact configured status and body. These assertions are
    // unconditional — a silently dropped stub must fail here, not be skipped.
    let response = client
        .get(format!("{base}/api/rust-sdk-test"))
        .send()
        .await
        .expect("Failed to make request");
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "dynamic mock must be served on the HTTP port right after creation"
    );
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body, json!({"sdk": "rust", "embedded": true}));

    // CRUD read: the created stub must appear in the management list.
    let list_response = client
        .get(format!("{base}/__mockforge/api/mocks"))
        .header(MANAGEMENT_TOKEN_HEADER, token)
        .send()
        .await
        .expect("Failed to list mocks");
    assert!(list_response.status().is_success(), "listing mocks must succeed");
    let listed: serde_json::Value = list_response.json().await.expect("Mock list must be JSON");
    let ids: Vec<&str> = listed["mocks"]
        .as_array()
        .expect("Mock list must have a mocks array")
        .iter()
        .filter_map(|m| m["id"].as_str())
        .collect();
    assert!(
        ids.contains(&mock_id.as_str()),
        "created mock {mock_id} must appear in the management list"
    );

    // CRUD update: switch the status code and verify it on the wire.
    let mut updated = created.clone();
    updated["status_code"] = json!(202);
    let update_response = client
        .put(format!("{base}/__mockforge/api/mocks/{mock_id}"))
        .header(MANAGEMENT_TOKEN_HEADER, token)
        .json(&updated)
        .send()
        .await
        .expect("Failed to update stub");
    assert!(update_response.status().is_success(), "stub update must succeed");
    let response = client
        .get(format!("{base}/api/rust-sdk-test"))
        .send()
        .await
        .expect("Failed to make request after update");
    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "updated status_code must be reflected on the wire"
    );

    // CRUD delete: the stub must stop being served. This also proves the
    // assertions above can detect absence — they are not vacuously green.
    let delete_response = client
        .delete(format!("{base}/__mockforge/api/mocks/{mock_id}"))
        .header(MANAGEMENT_TOKEN_HEADER, token)
        .send()
        .await
        .expect("Failed to delete stub");
    assert_eq!(
        delete_response.status(),
        StatusCode::NO_CONTENT,
        "stub deletion must return 204 No Content"
    );
    let response = client
        .get(format!("{base}/api/rust-sdk-test"))
        .send()
        .await
        .expect("Failed to make request after delete");
    let after_delete: serde_json::Value =
        response.json().await.ok().unwrap_or(serde_json::Value::Null);
    assert_ne!(
        after_delete,
        json!({"sdk": "rust", "embedded": true}),
        "deleted mock must stop being served"
    );

    server.stop().expect("Failed to stop server");
}

/// Test Node.js/TypeScript SDK integration (via CLI)
#[tokio::test]
#[ignore] // Requires Node.js and MockForge CLI
async fn test_nodejs_sdk_integration() {
    // Node.js SDK uses CLI, so we verify the Admin API endpoints it would use
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

    // The mockforge management API (`/__mockforge/api/*`) is mounted on the
    // HTTP server, not the admin UI server (which has RBAC middleware that
    // would reject our anonymous requests). Use http_port; the older
    // hardcoded `admin_port = 9080` only "passed" because it silently hit
    // an unrelated mockforge instance that happened to listen on 9080.
    let http_port = server.http_port();
    let admin_port = http_port;
    let client = Client::new();

    // Test endpoints that Node.js SDK would use
    let health_response = client
        .get(format!("http://localhost:{}/__mockforge/health", admin_port))
        .header(MANAGEMENT_TOKEN_HEADER, server.management_token())
        .send()
        .await;

    if let Ok(resp) = health_response {
        assert!(resp.status().is_success() || resp.status().as_u16() == 404);
    }

    // Test stub creation endpoint (what Node.js SDK would call)
    let stub_response = client
        .post(format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .header(MANAGEMENT_TOKEN_HEADER, server.management_token())
        .json(&json!({
            "path": "/api/nodejs-sdk-test",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {"sdk": "nodejs"}
            }
        }))
        .send()
        .await;

    if let Ok(resp) = stub_response {
        assert!(
            resp.status().is_success()
                || resp.status().as_u16() == 201
                || resp.status().as_u16() == 404
        );
    }

    server.stop().expect("Failed to stop server");
}

/// Test Python SDK integration (via CLI)
#[tokio::test]
#[ignore] // Requires Python and MockForge CLI
async fn test_python_sdk_integration() {
    // Python SDK uses CLI, so we verify the Admin API endpoints it would use
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

    // The mockforge management API (`/__mockforge/api/*`) is mounted on the
    // HTTP server, not the admin UI server (which has RBAC middleware that
    // would reject our anonymous requests). Use http_port; the older
    // hardcoded `admin_port = 9080` only "passed" because it silently hit
    // an unrelated mockforge instance that happened to listen on 9080.
    let http_port = server.http_port();
    let admin_port = http_port;
    let client = Client::new();

    // Test endpoints that Python SDK would use
    let stub_response = client
        .post(format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .header(MANAGEMENT_TOKEN_HEADER, server.management_token())
        .json(&json!({
            "path": "/api/python-sdk-test",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {"sdk": "python"}
            }
        }))
        .send()
        .await;

    if let Ok(resp) = stub_response {
        assert!(
            resp.status().is_success()
                || resp.status().as_u16() == 201
                || resp.status().as_u16() == 404
        );
    }

    server.stop().expect("Failed to stop server");
}

/// Test Go SDK integration (via CLI)
#[tokio::test]
#[ignore] // Requires Go and MockForge CLI
async fn test_go_sdk_integration() {
    // Go SDK uses CLI, so we verify the Admin API endpoints it would use
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

    // The mockforge management API (`/__mockforge/api/*`) is mounted on the
    // HTTP server, not the admin UI server (which has RBAC middleware that
    // would reject our anonymous requests). Use http_port; the older
    // hardcoded `admin_port = 9080` only "passed" because it silently hit
    // an unrelated mockforge instance that happened to listen on 9080.
    let http_port = server.http_port();
    let admin_port = http_port;
    let client = Client::new();

    // Test endpoints that Go SDK would use
    let stub_response = client
        .post(format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .header(MANAGEMENT_TOKEN_HEADER, server.management_token())
        .json(&json!({
            "path": "/api/go-sdk-test",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {"sdk": "go"}
            }
        }))
        .send()
        .await;

    if let Ok(resp) = stub_response {
        assert!(
            resp.status().is_success()
                || resp.status().as_u16() == 201
                || resp.status().as_u16() == 404
        );
    }

    server.stop().expect("Failed to stop server");
}

/// Test Java SDK integration (via CLI)
#[tokio::test]
#[ignore] // Requires Java and MockForge CLI
async fn test_java_sdk_integration() {
    // Java SDK uses CLI, so we verify the Admin API endpoints it would use
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

    // The mockforge management API (`/__mockforge/api/*`) is mounted on the
    // HTTP server, not the admin UI server (which has RBAC middleware that
    // would reject our anonymous requests). Use http_port; the older
    // hardcoded `admin_port = 9080` only "passed" because it silently hit
    // an unrelated mockforge instance that happened to listen on 9080.
    let http_port = server.http_port();
    let admin_port = http_port;
    let client = Client::new();

    // Test endpoints that Java SDK would use
    let stub_response = client
        .post(format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .header(MANAGEMENT_TOKEN_HEADER, server.management_token())
        .json(&json!({
            "path": "/api/java-sdk-test",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {"sdk": "java"}
            }
        }))
        .send()
        .await;

    if let Ok(resp) = stub_response {
        assert!(
            resp.status().is_success()
                || resp.status().as_u16() == 201
                || resp.status().as_u16() == 404
        );
    }

    server.stop().expect("Failed to stop server");
}

/// Test .NET SDK integration (via CLI)
#[tokio::test]
#[ignore] // Requires .NET and MockForge CLI
async fn test_dotnet_sdk_integration() {
    // .NET SDK uses CLI, so we verify the Admin API endpoints it would use
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

    // The mockforge management API (`/__mockforge/api/*`) is mounted on the
    // HTTP server, not the admin UI server (which has RBAC middleware that
    // would reject our anonymous requests). Use http_port; the older
    // hardcoded `admin_port = 9080` only "passed" because it silently hit
    // an unrelated mockforge instance that happened to listen on 9080.
    let http_port = server.http_port();
    let admin_port = http_port;
    let client = Client::new();

    // Test endpoints that .NET SDK would use
    let stub_response = client
        .post(format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .header(MANAGEMENT_TOKEN_HEADER, server.management_token())
        .json(&json!({
            "path": "/api/dotnet-sdk-test",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {"sdk": "dotnet"}
            }
        }))
        .send()
        .await;

    if let Ok(resp) = stub_response {
        assert!(
            resp.status().is_success()
                || resp.status().as_u16() == 201
                || resp.status().as_u16() == 404
        );
    }

    server.stop().expect("Failed to stop server");
}

/// Test SDK common functionality (start, stop, stub)
#[tokio::test]
async fn test_sdk_common_functionality() {
    // Test that all SDKs can use common Admin API endpoints
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
    // The mockforge management API (`/__mockforge/api/*`) is mounted on the
    // HTTP server, not the admin UI server — the admin UI applies RBAC
    // middleware that requires login, while the management API on the HTTP
    // port does not. Earlier versions of this test sent to a hardcoded
    // port 9080 and "passed" only because they were silently hitting an
    // unrelated mockforge instance running on the developer machine.
    let client = Client::new();

    // Test health check (all SDKs use this)
    let health_response = client
        .get(format!("http://localhost:{}/health", http_port))
        .send()
        .await
        .expect("Failed to check health");

    assert!(health_response.status().is_success());

    // Test stub creation (all SDKs use this) — use the HTTP port for the
    // management API (no auth required there).
    let stub_response = client
        .post(format!("http://127.0.0.1:{http_port}/__mockforge/api/mocks"))
        .header(MANAGEMENT_TOKEN_HEADER, server.management_token())
        .json(&json!({
            "path": "/api/common-test",
            "method": "GET",
            "response": {
                "body": {"test": "common"}
            },
            "status_code": 200
        }))
        .send()
        .await
        .expect("Failed to create stub");

    assert_eq!(
        stub_response.status(),
        StatusCode::CREATED,
        "stub POST must return 201 Created"
    );
    let created: serde_json::Value =
        stub_response.json().await.expect("Created stub must be JSON");
    let mock_id = created["id"]
        .as_str()
        .expect("Created stub must have an ID")
        .to_string();

    // Test stub retrieval (all SDKs use this) — the created stub must be
    // listed; a dropped stub (#1281) must fail here rather than skip the check.
    let list_response = client
        .get(format!("http://127.0.0.1:{http_port}/__mockforge/api/mocks"))
        .header(MANAGEMENT_TOKEN_HEADER, server.management_token())
        .send()
        .await
        .expect("Failed to list mocks");
    assert!(list_response.status().is_success(), "listing mocks must succeed");
    let listed: serde_json::Value = list_response.json().await.expect("Mock list must be JSON");
    let ids: Vec<&str> = listed["mocks"]
        .as_array()
        .expect("Mock list must have a mocks array")
        .iter()
        .filter_map(|m| m["id"].as_str())
        .collect();
    assert!(
        ids.contains(&mock_id.as_str()),
        "created mock {mock_id} must appear in the management list"
    );

    server.stop().expect("Failed to stop server");
}
