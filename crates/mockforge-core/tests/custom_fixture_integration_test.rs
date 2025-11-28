//! Integration tests for custom fixture functionality
//!
//! Tests the end-to-end flow of loading and using custom fixtures
//! in the OpenAPI route registry.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use mockforge_core::custom_fixture::CustomFixtureLoader;
use mockforge_core::openapi::OpenApiSpec;
use mockforge_core::openapi_routes::OpenApiRouteRegistry;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;
use tower::ServiceExt;

/// Create a minimal OpenAPI spec for testing
fn create_test_spec() -> OpenApiSpec {
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/api/v1/test": {
                "get": {
                    "operationId": "getTest",
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "message": {
                                                "type": "string"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/hives/{hiveId}": {
                "get": {
                    "operationId": "getHive",
                    "parameters": [
                        {
                            "name": "hiveId",
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "string"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {
                                                "type": "string"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    OpenApiSpec::from_json(spec_json).unwrap()
}

#[tokio::test]
async fn test_custom_fixture_integration_exact_match() {
    // Create temporary directory for fixtures
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    // Create a custom fixture
    let fixture_content = json!({
        "method": "GET",
        "path": "/api/v1/test",
        "status": 200,
        "response": {
            "message": "Custom fixture response"
        },
        "headers": {
            "x-custom-header": "test-value"
        }
    });

    let fixture_file = fixtures_dir.join("test.json");
    fs::write(&fixture_file, serde_json::to_string_pretty(&fixture_content).unwrap())
        .await
        .unwrap();

    // Load custom fixtures
    let mut loader = CustomFixtureLoader::new(fixtures_dir, true);
    loader.load_fixtures().await.unwrap();

    // Create OpenAPI registry with custom fixture loader
    let spec = create_test_spec();
    let registry = OpenApiRouteRegistry::new(spec).with_custom_fixture_loader(Arc::new(loader));

    // Build router
    let router = registry.build_router();

    // Make a request
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/test")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Verify response
    assert_eq!(response.status(), StatusCode::OK);

    // Check custom header
    let headers = response.headers();
    assert_eq!(headers.get("x-custom-header").and_then(|h| h.to_str().ok()), Some("test-value"));

    // Check response body
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["message"], "Custom fixture response");
}

#[tokio::test]
async fn test_custom_fixture_integration_path_parameter() {
    // Create temporary directory for fixtures
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    // Create a custom fixture with path parameter
    let fixture_content = json!({
        "method": "GET",
        "path": "/api/v1/hives/{hiveId}",
        "status": 200,
        "response": {
            "id": "hive_001",
            "name": "Test Hive"
        }
    });

    let fixture_file = fixtures_dir.join("hive.json");
    fs::write(&fixture_file, serde_json::to_string_pretty(&fixture_content).unwrap())
        .await
        .unwrap();

    // Load custom fixtures
    let mut loader = CustomFixtureLoader::new(fixtures_dir, true);
    loader.load_fixtures().await.unwrap();

    // Create OpenAPI registry with custom fixture loader
    let spec = create_test_spec();
    let registry = OpenApiRouteRegistry::new(spec).with_custom_fixture_loader(Arc::new(loader));

    // Build router
    let router = registry.build_router();

    // Make a request with path parameter
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/hives/hive_001")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Verify response
    assert_eq!(response.status(), StatusCode::OK);

    // Check response body
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["id"], "hive_001");
    assert_eq!(body_json["name"], "Test Hive");
}

#[tokio::test]
async fn test_custom_fixture_priority_over_mock() {
    // Create temporary directory for fixtures
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    // Create a custom fixture that should override the generated mock
    let fixture_content = json!({
        "method": "GET",
        "path": "/api/v1/test",
        "status": 201,  // Different status to verify it's using the fixture
        "response": {
            "source": "custom_fixture",
            "message": "This is from a custom fixture"
        }
    });

    let fixture_file = fixtures_dir.join("test.json");
    fs::write(&fixture_file, serde_json::to_string_pretty(&fixture_content).unwrap())
        .await
        .unwrap();

    // Load custom fixtures
    let mut loader = CustomFixtureLoader::new(fixtures_dir, true);
    loader.load_fixtures().await.unwrap();

    // Create OpenAPI registry with custom fixture loader
    let spec = create_test_spec();
    let registry = OpenApiRouteRegistry::new(spec).with_custom_fixture_loader(Arc::new(loader));

    // Build router
    let router = registry.build_router();

    // Make a request
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/test")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Verify that custom fixture is used (status 201, not 200)
    assert_eq!(response.status(), StatusCode::CREATED);

    // Verify response body contains fixture data
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body_json["source"], "custom_fixture");
}

#[tokio::test]
async fn test_custom_fixture_disabled_falls_back_to_mock() {
    // Create temporary directory for fixtures
    let temp_dir = TempDir::new().unwrap();
    let fixtures_dir = temp_dir.path().to_path_buf();

    // Create a custom fixture
    let fixture_content = json!({
        "method": "GET",
        "path": "/api/v1/test",
        "status": 999,  // Unusual status to verify it's NOT used
        "response": {
            "source": "custom_fixture"
        }
    });

    let fixture_file = fixtures_dir.join("test.json");
    fs::write(&fixture_file, serde_json::to_string_pretty(&fixture_content).unwrap())
        .await
        .unwrap();

    // Load custom fixtures but DISABLED
    let mut loader = CustomFixtureLoader::new(fixtures_dir, false);
    loader.load_fixtures().await.unwrap();

    // Create OpenAPI registry with disabled custom fixture loader
    let spec = create_test_spec();
    let registry = OpenApiRouteRegistry::new(spec).with_custom_fixture_loader(Arc::new(loader));

    // Build router
    let router = registry.build_router();

    // Make a request
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/test")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Verify that mock is used instead (status should be 200, not 999)
    assert_eq!(response.status(), StatusCode::OK);

    // Verify response body does NOT contain fixture data
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_ne!(body_json.get("source"), Some(&json!("custom_fixture")));
}
