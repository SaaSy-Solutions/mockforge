//! Failure scenario tests for MockForge HTTP
//!
//! This test suite covers negative cases and error handling to ensure
//! the system fails gracefully when given invalid or malformed input.

use mockforge_http::build_router;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::time::Duration;

/// Test that server starts successfully even with invalid JSON in OpenAPI spec file
#[tokio::test]
async fn test_server_starts_with_malformed_json_spec() {
    let temp_dir = tempfile::tempdir().unwrap();
    let spec_path = temp_dir.path().join("invalid.json");

    // Write malformed JSON
    tokio::fs::write(&spec_path, "{ invalid json }").await.unwrap();

    // Server should build successfully despite invalid spec
    let router = build_router(Some(spec_path.to_string_lossy().to_string()), None, None).await;

    // Start server to verify it doesn't crash
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let base_url = format!("http://{}", addr);

    // Health endpoint should still work
    let response = client.get(format!("{}/health", base_url)).send().await.unwrap();
    assert!(
        response.status().is_success(),
        "Health endpoint should work even with invalid spec"
    );

    // Cleanup
    drop(server);

    println!("✓ Server started successfully with malformed JSON spec (logged warning)");
}

/// Test that server starts with incomplete OpenAPI spec
#[tokio::test]
async fn test_server_starts_with_incomplete_openapi_spec() {
    let temp_dir = tempfile::tempdir().unwrap();
    let spec_path = temp_dir.path().join("incomplete.json");

    // Write incomplete OpenAPI spec (missing required 'paths' field)
    let incomplete_spec = serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Incomplete API",
            "version": "1.0.0"
        }
        // Missing 'paths' field
    });

    tokio::fs::write(&spec_path, serde_json::to_vec(&incomplete_spec).unwrap())
        .await
        .unwrap();

    // Server should build successfully despite incomplete spec
    let router = build_router(Some(spec_path.to_string_lossy().to_string()), None, None).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();

    // Health check should work
    let response = client.get(format!("http://{}/health", addr)).send().await.unwrap();
    assert!(response.status().is_success());

    drop(server);
    println!("✓ Server started with incomplete OpenAPI spec");
}

/// Test server with empty OpenAPI spec file
#[tokio::test]
async fn test_server_starts_with_empty_spec_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let spec_path = temp_dir.path().join("empty.json");

    // Write empty file
    tokio::fs::write(&spec_path, "").await.unwrap();

    let router = build_router(Some(spec_path.to_string_lossy().to_string()), None, None).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client.get(format!("http://{}/health", addr)).send().await.unwrap();
    assert!(response.status().is_success());

    drop(server);
    println!("✓ Server started with empty spec file");
}

/// Test server with whitespace-only OpenAPI spec file
#[tokio::test]
async fn test_server_starts_with_whitespace_only_spec() {
    let temp_dir = tempfile::tempdir().unwrap();
    let spec_path = temp_dir.path().join("whitespace.json");

    // Write file with only whitespace
    tokio::fs::write(&spec_path, "   \n\t  \n  ").await.unwrap();

    let router = build_router(Some(spec_path.to_string_lossy().to_string()), None, None).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client.get(format!("http://{}/health", addr)).send().await.unwrap();
    assert!(response.status().is_success());

    drop(server);
    println!("✓ Server started with whitespace-only spec file");
}

/// Test server with invalid OpenAPI version
#[tokio::test]
async fn test_server_starts_with_invalid_openapi_version() {
    let temp_dir = tempfile::tempdir().unwrap();
    let spec_path = temp_dir.path().join("invalid-version.json");

    // Write spec with invalid OpenAPI version
    let invalid_spec = serde_json::json!({
        "openapi": "99.0.0",  // Invalid version
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {}
    });

    tokio::fs::write(&spec_path, serde_json::to_vec(&invalid_spec).unwrap())
        .await
        .unwrap();

    let router = build_router(Some(spec_path.to_string_lossy().to_string()), None, None).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client.get(format!("http://{}/health", addr)).send().await.unwrap();
    assert!(response.status().is_success());

    drop(server);
    println!("✓ Server started with invalid OpenAPI version");
}

/// Test server with nonexistent spec file path
#[tokio::test]
async fn test_server_starts_with_nonexistent_spec_path() {
    let spec_path = "/nonexistent/path/to/spec.json";

    let router = build_router(Some(spec_path.to_string()), None, None).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client.get(format!("http://{}/health", addr)).send().await.unwrap();
    assert!(response.status().is_success());

    drop(server);
    println!("✓ Server started with nonexistent spec path (logged warning)");
}

/// Test that management endpoints work even when OpenAPI spec fails to load
#[tokio::test]
async fn test_management_endpoints_work_with_failed_spec() {
    let temp_dir = tempfile::tempdir().unwrap();
    let spec_path = temp_dir.path().join("broken.json");

    // Write completely broken spec
    tokio::fs::write(&spec_path, "not even json at all!@#$").await.unwrap();

    let router = build_router(Some(spec_path.to_string_lossy().to_string()), None, None).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let base_url = format!("http://{}", addr);

    // Test health endpoint
    let response = client.get(format!("{}/health", base_url)).send().await.unwrap();
    assert!(response.status().is_success());

    // Test routes endpoint (should return empty list)
    let response = client.get(format!("{}/__mockforge/routes", base_url)).send().await.unwrap();
    assert!(response.status().is_success());

    let routes: serde_json::Value = response.json().await.unwrap();
    assert_eq!(routes["total"], 0, "Should have no routes when spec fails to load");

    // Test management API health
    let response = client.get(format!("{}/__mockforge/api/health", base_url)).send().await.unwrap();
    assert!(response.status().is_success());

    drop(server);
    println!("✓ Management endpoints work even when OpenAPI spec fails");
}

/// Test server with malformed YAML spec
#[tokio::test]
async fn test_server_starts_with_malformed_yaml_spec() {
    let temp_dir = tempfile::tempdir().unwrap();
    let spec_path = temp_dir.path().join("invalid.yaml");

    // Write malformed YAML
    tokio::fs::write(&spec_path, "openapi: 3.0.0\n  invalid: indentation\n badly: [formed")
        .await
        .unwrap();

    let router = build_router(Some(spec_path.to_string_lossy().to_string()), None, None).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client.get(format!("http://{}/health", addr)).send().await.unwrap();
    assert!(response.status().is_success());

    drop(server);
    println!("✓ Server started with malformed YAML spec");
}

/// Test server with spec containing circular references
#[tokio::test]
async fn test_server_handles_spec_with_circular_refs() {
    let temp_dir = tempfile::tempdir().unwrap();
    let spec_path = temp_dir.path().join("circular.json");

    // Write spec with schema that could cause issues
    let spec = serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Circular API",
            "version": "1.0.0"
        },
        "paths": {
            "/test": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/Node"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "string"},
                        "next": {
                            "$ref": "#/components/schemas/Node"
                        }
                    }
                }
            }
        }
    });

    tokio::fs::write(&spec_path, serde_json::to_vec(&spec).unwrap()).await.unwrap();

    // Should handle gracefully
    let router = build_router(Some(spec_path.to_string_lossy().to_string()), None, None).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client.get(format!("http://{}/health", addr)).send().await.unwrap();
    assert!(response.status().is_success());

    drop(server);
    println!("✓ Server handles spec with circular references");
}

/// Test that validation options are ignored when spec fails to load
#[tokio::test]
async fn test_validation_ignored_when_spec_fails() {
    use mockforge_core::openapi_routes::{ValidationMode, ValidationOptions};

    let temp_dir = tempfile::tempdir().unwrap();
    let spec_path = temp_dir.path().join("invalid.json");

    tokio::fs::write(&spec_path, "{ broken }").await.unwrap();

    // Pass strict validation options
    let validation_options = Some(ValidationOptions {
        request_mode: ValidationMode::Enforce,
        aggregate_errors: true,
        validate_responses: true,
        overrides: HashMap::new(),
        admin_skip_prefixes: vec![],
        response_template_expand: false,
        validation_status: None,
    });

    // Should still build successfully
    let router =
        build_router(Some(spec_path.to_string_lossy().to_string()), validation_options, None).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client.get(format!("http://{}/health", addr)).send().await.unwrap();
    assert!(response.status().is_success());

    drop(server);
    println!("✓ Validation options ignored when spec fails to load");
}
