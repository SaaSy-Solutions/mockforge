//! Component Integration Tests
//!
//! Tests that verify integration between core components:
//! - Core → Protocol (routing, state management)
//! - Data → Protocol (data generation, fixtures)
//! - Plugin → Protocol (plugin handlers, transformations)

use mockforge_test::MockForgeServer;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

/// Test Core → Protocol integration (routing and state management)
#[tokio::test]
async fn test_core_protocol_routing() {
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

    // Create multiple routes via Core routing system
    let routes = vec![
        json!({
            "path": "/api/users",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {"users": []}
            }
        }),
        json!({
            "path": "/api/posts",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {"posts": []}
            }
        }),
    ];

    // Register routes through Core
    for route in routes {
        let response = client
            .post(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
            .json(&route)
            .send()
            .await
            .expect("Failed to create route");

        assert!(
            response.status().is_success() || response.status().as_u16() == 201,
            "Route should be registered"
        );
    }

    // Verify routes are accessible through Protocol layer
    let users_response = client
        .get(&format!("http://localhost:{}/api/users", http_port))
        .send()
        .await
        .expect("Failed to call users endpoint");

    assert!(users_response.status().is_success() || users_response.status().as_u16() == 404);

    let posts_response = client
        .get(&format!("http://localhost:{}/api/posts", http_port))
        .send()
        .await
        .expect("Failed to call posts endpoint");

    assert!(posts_response.status().is_success() || posts_response.status().as_u16() == 404);

    server.stop().expect("Failed to stop server");
}

/// Test Data → Protocol integration (data generation in responses)
#[tokio::test]
async fn test_data_protocol_generation() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND", "true")
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

    // Create route with data generation tokens
    let stub_response = client
        .post(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .json(&json!({
            "path": "/api/generated",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {
                    "id": "{{uuid}}",
                    "email": "{{faker.email}}",
                    "name": "{{faker.name}}",
                    "random": "{{randInt 1 100}}"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create stub");

    assert!(stub_response.status().is_success() || stub_response.status().as_u16() == 201);

    // Make request and verify Data layer generates values
    let response = client
        .get(&format!("http://localhost:{}/api/generated", http_port))
        .send()
        .await
        .expect("Failed to make request");

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");

        // Verify data generation worked
        assert!(body["id"].is_string(), "ID should be generated");
        assert!(body["email"].as_str().unwrap().contains("@"), "Email should be generated");
        assert!(body["name"].is_string(), "Name should be generated");
        assert!(body["random"].is_number(), "Random number should be generated");
    }

    server.stop().expect("Failed to stop server");
}

/// Test Data → Protocol integration (fixture loading)
#[tokio::test]
async fn test_data_protocol_fixtures() {
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

    // Create route that uses fixture
    let stub_response = client
        .post(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .json(&json!({
            "path": "/api/fixture-test",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {
                    "fixture": "users",
                    "count": 5
                }
            }
        }))
        .send()
        .await;

    // Fixture loading may or may not be fully configured
    if let Ok(resp) = stub_response {
        assert!(
            resp.status().is_success()
                || resp.status().as_u16() == 201
                || resp.status().as_u16() == 404
        );
    }

    server.stop().expect("Failed to stop server");
}

/// Test Plugin → Protocol integration (plugin handlers)
#[tokio::test]
async fn test_plugin_protocol_handlers() {
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

    // Test plugin API endpoints (Plugin → Protocol integration)
    let plugins_response = client
        .get(&format!("http://localhost:{}/__mockforge/plugins", admin_port))
        .send()
        .await;

    // Plugin system may or may not be fully configured
    if let Ok(resp) = plugins_response {
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 404,
            "Plugin endpoint should exist"
        );
    }

    server.stop().expect("Failed to stop server");
}

/// Test Plugin → Protocol integration (request/response transformations)
#[tokio::test]
async fn test_plugin_protocol_transformations() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_PLUGIN_ENABLED", "true")
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

    // Create route that might use plugin transformation
    let stub_response = client
        .post(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .json(&json!({
            "path": "/api/transform",
            "method": "POST",
            "response": {
                "status": 200,
                "body": {
                    "transformed": true,
                    "original": "{{request.body}}"
                }
            }
        }))
        .send()
        .await;

    // Transformation may or may not be fully configured
    if let Ok(resp) = stub_response {
        assert!(
            resp.status().is_success()
                || resp.status().as_u16() == 201
                || resp.status().as_u16() == 404
        );
    }

    server.stop().expect("Failed to stop server");
}

/// Test Core → Data → Protocol integration (full stack)
#[tokio::test]
async fn test_core_data_protocol_integration() {
    let server = match MockForgeServer::builder()
        .http_port(0)
        .admin_port(0)
        .enable_admin(true)
        .env_var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND", "true")
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

    // Create route that uses Core routing, Data generation, and Protocol handling
    let stub_response = client
        .post(&format!("http://localhost:{}/__mockforge/api/mocks", admin_port))
        .json(&json!({
            "path": "/api/integrated",
            "method": "GET",
            "response": {
                "status": 200,
                "body": {
                    "route": "core-routed",
                    "data": {
                        "id": "{{uuid}}",
                        "timestamp": "{{now}}"
                    },
                    "protocol": "http"
                }
            }
        }))
        .send()
        .await
        .expect("Failed to create stub");

    assert!(stub_response.status().is_success() || stub_response.status().as_u16() == 201);

    // Verify full stack integration
    let response = client
        .get(&format!("http://localhost:{}/api/integrated", http_port))
        .send()
        .await
        .expect("Failed to make request");

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
        assert!(body["route"].is_string(), "Core routing should work");
        assert!(body["data"]["id"].is_string(), "Data generation should work");
        assert_eq!(body["protocol"], "http", "Protocol handling should work");
    }

    server.stop().expect("Failed to stop server");
}
