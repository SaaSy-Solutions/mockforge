//! End-to-end tests for fault injection functionality in the HTTP mock server.
//!
//! These tests verify that fault injection can properly simulate various failure
//! scenarios including 500 errors, timeouts, and other error conditions.

use axum::Router;
use mockforge_chaos::core_failure_injection::FailureConfig;
use mockforge_core::openapi_routes::ValidationOptions;
use mockforge_http::build_router;
use std::net::SocketAddr;

/// Test that fault injection can trigger 500 errors
#[tokio::test]
async fn test_fault_injection_triggers_500() {
    let spec = serde_json::json!({
        "openapi":"3.0.0",
        "info": {"title":"Fault Injection Test","version":"1"},
        "paths": {
            "/fault-test": {
                "get": {
                    "responses": {
                        "200": {
                            "description":"Success",
                            "content":{"application/json":{"schema":{"type":"object"}}}
                        }
                    },
                    "tags": ["faulty"]
                }
            }
        }
    });

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spec.json");
    tokio::fs::write(&path, serde_json::to_vec(&spec).unwrap()).await.unwrap();

    // Create fault injection config with 100% error rate for "faulty" tag
    let failure_config = Some(FailureConfig {
        global_error_rate: 0.0,
        default_status_codes: vec![500, 502, 503],
        tag_configs: std::collections::HashMap::from([(
            "faulty".to_string(),
            mockforge_core::failure_injection::TagFailureConfig {
                error_rate: 1.0, // 100% error rate
                status_codes: Some(vec![500]),
                error_message: Some("Injected fault for testing".to_string()),
            },
        )]),
        include_tags: Vec::new(),
        exclude_tags: Vec::new(),
    });

    // Build router with fault injection enabled
    let app: Router = build_router(
        Some(path.to_string_lossy().to_string()),
        Some(ValidationOptions::default()),
        failure_config,
    )
    .await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/fault-test", addr);

    // Make multiple requests to increase chance of hitting the fault
    let mut fault_injected = false;
    for _ in 0..10 {
        let res = client.get(&url).send().await.unwrap();
        if res.status() == reqwest::StatusCode::INTERNAL_SERVER_ERROR {
            fault_injected = true;
            let body = res.text().await.unwrap();
            assert!(body.contains("Injected fault for testing"));
            break;
        }
    }

    // Assert that we eventually got a 500 error
    assert!(fault_injected, "Fault injection should have triggered at least one 500 error");

    drop(server);
}

/// Test that fault injection can be disabled and requests succeed normally
#[tokio::test]
async fn test_fault_injection_can_be_disabled() {
    let spec = serde_json::json!({
        "openapi":"3.0.0",
        "info": {"title":"Fault Injection Disabled Test","version":"1"},
        "paths": {
            "/normal-test": {
                "get": {
                    "responses": {
                        "200": {
                            "description":"Success",
                            "content":{"application/json":{"schema":{"type":"object"}}}
                        }
                    },
                    "tags": ["faulty"]
                }
            }
        }
    });

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spec.json");
    tokio::fs::write(&path, serde_json::to_vec(&spec).unwrap()).await.unwrap();

    let app: Router = build_router(
        Some(path.to_string_lossy().to_string()),
        Some(ValidationOptions::default()),
        None,
    )
    .await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/normal-test", addr);

    // Make several requests - all should succeed when fault injection is disabled
    for _ in 0..5 {
        let res = client.get(&url).send().await.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }

    drop(server);
}

/// Test that different fault injection configurations work
#[tokio::test]
async fn test_fault_injection_different_status_codes() {
    let spec = serde_json::json!({
        "openapi":"3.0.0",
        "info": {"title":"Fault Injection Status Codes Test","version":"1"},
        "paths": {
            "/error-502": {
                "get": {
                    "responses": {
                        "200": {
                            "description":"Success",
                            "content":{"application/json":{"schema":{"type":"object"}}}
                        }
                    },
                    "tags": ["error-502"]
                }
            },
            "/error-503": {
                "get": {
                    "responses": {
                        "200": {
                            "description":"Success",
                            "content":{"application/json":{"schema":{"type":"object"}}}
                        }
                    },
                    "tags": ["error-503"]
                }
            }
        }
    });

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spec.json");
    tokio::fs::write(&path, serde_json::to_vec(&spec).unwrap()).await.unwrap();

    let failure_config = Some(FailureConfig {
        global_error_rate: 0.0,
        default_status_codes: vec![500],
        tag_configs: std::collections::HashMap::from([
            (
                "error-502".to_string(),
                mockforge_core::failure_injection::TagFailureConfig {
                    error_rate: 1.0,
                    status_codes: Some(vec![502]),
                    error_message: Some("Bad Gateway".to_string()),
                },
            ),
            (
                "error-503".to_string(),
                mockforge_core::failure_injection::TagFailureConfig {
                    error_rate: 1.0,
                    status_codes: Some(vec![503]),
                    error_message: Some("Service Unavailable".to_string()),
                },
            ),
        ]),
        include_tags: Vec::new(),
        exclude_tags: Vec::new(),
    });

    let app: Router = build_router(
        Some(path.to_string_lossy().to_string()),
        Some(ValidationOptions::default()),
        failure_config,
    )
    .await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    let client = reqwest::Client::new();

    // Test endpoint tagged "error-502" should return 502
    let url_502 = format!("http://{}/error-502", addr);
    let res_502 = client.get(&url_502).send().await.unwrap();
    assert_eq!(
        res_502.status(),
        reqwest::StatusCode::BAD_GATEWAY,
        "error-502 tagged endpoint should return 502"
    );

    // Test endpoint tagged "error-503" should return 503
    let url_503 = format!("http://{}/error-503", addr);
    let res_503 = client.get(&url_503).send().await.unwrap();
    assert_eq!(
        res_503.status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE,
        "error-503 tagged endpoint should return 503"
    );

    drop(server);
}

/// Test fault injection with include/exclude tag filters
#[tokio::test]
async fn test_fault_injection_tag_filters() {
    let spec = serde_json::json!({
        "openapi":"3.0.0",
        "info": {"title":"Fault Injection Tag Filters Test","version":"1"},
        "paths": {
            "/included-endpoint": {
                "get": {
                    "responses": {
                        "200": {
                            "description":"Success",
                            "content":{"application/json":{"schema":{"type":"object"}}}
                        }
                    },
                    "tags": ["included"]
                }
            },
            "/excluded-endpoint": {
                "get": {
                    "responses": {
                        "200": {
                            "description":"Success",
                            "content":{"application/json":{"schema":{"type":"object"}}}
                        }
                    },
                    "tags": ["excluded"]
                }
            },
            "/normal-endpoint": {
                "get": {
                    "responses": {
                        "200": {
                            "description":"Success",
                            "content":{"application/json":{"schema":{"type":"object"}}}
                        }
                    },
                    "tags": ["normal"]
                }
            }
        }
    });

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spec.json");
    tokio::fs::write(&path, serde_json::to_vec(&spec).unwrap()).await.unwrap();

    // Configure fault injection with include_tags to only affect "included" tag
    let failure_config = Some(FailureConfig {
        global_error_rate: 0.0,
        default_status_codes: vec![500],
        tag_configs: std::collections::HashMap::from([(
            "included".to_string(),
            mockforge_core::failure_injection::TagFailureConfig {
                error_rate: 1.0,
                status_codes: Some(vec![500]),
                error_message: Some("Included tag fault".to_string()),
            },
        )]),
        include_tags: vec!["included".to_string()],
        exclude_tags: vec!["excluded".to_string()],
    });

    let app: Router = build_router(
        Some(path.to_string_lossy().to_string()),
        Some(ValidationOptions::default()),
        failure_config,
    )
    .await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    let client = reqwest::Client::new();

    // "included" tag endpoint should fail (100% error rate configured)
    let url_included = format!("http://{}/included-endpoint", addr);
    let res_included = client.get(&url_included).send().await.unwrap();
    assert!(
        res_included.status().is_server_error(),
        "Included tag endpoint should fail, got {}",
        res_included.status()
    );

    // "excluded" tag endpoint should succeed (explicitly excluded from fault injection)
    let url_excluded = format!("http://{}/excluded-endpoint", addr);
    let res_excluded = client.get(&url_excluded).send().await.unwrap();
    assert!(
        res_excluded.status().is_success(),
        "Excluded tag endpoint should succeed, got {}",
        res_excluded.status()
    );

    // "normal" tag endpoint should succeed (no fault injection configured for it)
    let url_normal = format!("http://{}/normal-endpoint", addr);
    let res_normal = client.get(&url_normal).send().await.unwrap();
    assert!(
        res_normal.status().is_success(),
        "Normal tag endpoint should succeed, got {}",
        res_normal.status()
    );

    drop(server);
}
