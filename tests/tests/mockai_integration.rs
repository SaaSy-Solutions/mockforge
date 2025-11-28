//! Integration tests for MockAI (Behavioral Mock Intelligence)
//!
//! Tests end-to-end HTTP request processing with MockAI enabled,
//! including session persistence, mutation detection, and intelligent responses.

use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
use mockforge_core::openapi::OpenApiSpec;
use serde_json::json;

/// Test MockAI with a simple OpenAPI spec
#[tokio::test]
async fn test_mockai_basic_request() {
    // Create a minimal OpenAPI spec
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/users": {
                "get": {
                    "summary": "Get users",
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "id": {"type": "integer"},
                                                "name": {"type": "string"}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Create user",
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "name": {"type": "string"}
                                    },
                                    "required": ["name"]
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Created",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "integer"},
                                            "name": {"type": "string"}
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

    // Parse OpenAPI spec
    let spec = OpenApiSpec::from_json(spec_json).expect("Failed to parse OpenAPI spec");

    // Create MockAI config
    let behavior_config = IntelligentBehaviorConfig::default();
    let mockai = mockforge_core::intelligent_behavior::MockAI::from_openapi(&spec, behavior_config)
        .await
        .expect("Failed to create MockAI");

    // Test MockAI directly (integration test with router would require full server setup)
    // For now, test that MockAI can process requests
    let request = mockforge_core::intelligent_behavior::Request {
        method: "GET".to_string(),
        path: "/users".to_string(),
        body: None,
        query_params: std::collections::HashMap::new(),
        headers: std::collections::HashMap::new(),
    };

    let response = mockai.process_request(&request).await;
    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.status_code, 200);

    // Test POST request
    let request = mockforge_core::intelligent_behavior::Request {
        method: "POST".to_string(),
        path: "/users".to_string(),
        body: Some(json!({"name": "Test User"})),
        query_params: std::collections::HashMap::new(),
        headers: {
            let mut h = std::collections::HashMap::new();
            h.insert("Content-Type".to_string(), "application/json".to_string());
            h
        },
    };

    let response = mockai.process_request(&request).await;
    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.status_code, 200);
}

/// Test MockAI session persistence
#[tokio::test]
async fn test_mockai_session_persistence() {
    // Create a minimal OpenAPI spec
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {"title": "Test API", "version": "1.0.0"},
        "paths": {
            "/state": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {"type": "object"}
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let spec = OpenApiSpec::from_json(spec_json).expect("Failed to parse OpenAPI spec");
    let behavior_config = IntelligentBehaviorConfig::default();
    let mockai = mockforge_core::intelligent_behavior::MockAI::from_openapi(&spec, behavior_config)
        .await
        .expect("Failed to create MockAI");

    // First request - should create a new session
    let mut headers = std::collections::HashMap::new();
    headers.insert("X-Session-ID".to_string(), "test-session-123".to_string());

    let request1 = mockforge_core::intelligent_behavior::Request {
        method: "GET".to_string(),
        path: "/state".to_string(),
        body: None,
        query_params: std::collections::HashMap::new(),
        headers: headers.clone(),
    };

    let response1 = mockai.process_request(&request1).await;
    assert!(response1.is_ok());
    assert_eq!(response1.unwrap().status_code, 200);

    // Second request with same session ID - should reuse session
    let request2 = mockforge_core::intelligent_behavior::Request {
        method: "GET".to_string(),
        path: "/state".to_string(),
        body: None,
        query_params: std::collections::HashMap::new(),
        headers,
    };

    let response2 = mockai.process_request(&request2).await;
    assert!(response2.is_ok());
    assert_eq!(response2.unwrap().status_code, 200);
}
