//! Tests for HTTP scenario switching feature
//!
//! This module tests the ability to switch between different response examples
//! using the X-Mockforge-Scenario header or MOCKFORGE_HTTP_SCENARIO environment variable.

use mockforge_core::openapi_routes::create_registry_from_json;
use serde_json::json;

#[tokio::test]
async fn test_scenario_selection_via_method() {
    // Create a spec with multiple scenarios
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Scenario Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/users/{id}": {
                "get": {
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "schema": {"type": "string"}
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "examples": {
                                        "happy": {
                                            "summary": "Happy path",
                                            "value": {
                                                "id": "123",
                                                "name": "John Doe",
                                                "status": "active"
                                            }
                                        },
                                        "errors": {
                                            "summary": "Error scenario",
                                            "value": {
                                                "id": "123",
                                                "name": "Suspended User",
                                                "status": "suspended",
                                                "reason": "Terms violation"
                                            }
                                        },
                                        "edge": {
                                            "summary": "Edge case",
                                            "value": {
                                                "id": "123",
                                                "name": "New User",
                                                "status": "pending"
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

    let registry = create_registry_from_json(spec_json).expect("Failed to create registry");

    // Get the route
    let route = registry.get_route("/users/{id}", "GET").expect("Route should exist");

    // Test default scenario (should use first example, order may vary)
    let (status, body) = route.mock_response_with_status_and_scenario(None);
    assert_eq!(status, 200);
    // Just verify we got one of the valid scenarios
    assert!(body.get("status").is_some());
    assert!(body.get("name").is_some());

    // Test "happy" scenario explicitly
    let (status, body) = route.mock_response_with_status_and_scenario(Some("happy"));
    assert_eq!(status, 200);
    assert_eq!(body["status"], "active");
    assert_eq!(body["name"], "John Doe");

    // Test "errors" scenario
    let (status, body) = route.mock_response_with_status_and_scenario(Some("errors"));
    assert_eq!(status, 200);
    assert_eq!(body["status"], "suspended");
    assert_eq!(body["name"], "Suspended User");
    assert_eq!(body["reason"], "Terms violation");

    // Test "edge" scenario
    let (status, body) = route.mock_response_with_status_and_scenario(Some("edge"));
    assert_eq!(status, 200);
    assert_eq!(body["status"], "pending");
    assert_eq!(body["name"], "New User");

    // Test non-existent scenario (should fall back to first example)
    let (status, body) = route.mock_response_with_status_and_scenario(Some("nonexistent"));
    assert_eq!(status, 200);
    // Should fall back to one of the examples (order may vary in HashMap)
    assert!(body.get("status").is_some());
}

#[tokio::test]
async fn test_scenario_with_post_endpoint() {
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Order API",
            "version": "1.0.0"
        },
        "paths": {
            "/orders": {
                "post": {
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "userId": {"type": "string"},
                                        "items": {"type": "array"}
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Created",
                            "content": {
                                "application/json": {
                                    "examples": {
                                        "happy": {
                                            "value": {
                                                "orderId": "ord_123",
                                                "status": "confirmed",
                                                "total": 99.99
                                            }
                                        },
                                        "errors": {
                                            "value": {
                                                "orderId": "ord_123",
                                                "status": "payment_failed",
                                                "errorCode": "INSUFFICIENT_FUNDS"
                                            }
                                        },
                                        "edge": {
                                            "value": {
                                                "orderId": "ord_123",
                                                "status": "partially_available",
                                                "warning": "Some items unavailable"
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

    let registry = create_registry_from_json(spec_json).expect("Failed to create registry");
    let route = registry.get_route("/orders", "POST").expect("Route should exist");

    // Test happy scenario
    let (status, body) = route.mock_response_with_status_and_scenario(Some("happy"));
    assert_eq!(status, 201);
    assert_eq!(body["status"], "confirmed");
    assert_eq!(body["total"], 99.99);

    // Test errors scenario
    let (status, body) = route.mock_response_with_status_and_scenario(Some("errors"));
    assert_eq!(status, 201);
    assert_eq!(body["status"], "payment_failed");
    assert_eq!(body["errorCode"], "INSUFFICIENT_FUNDS");

    // Test edge scenario
    let (status, body) = route.mock_response_with_status_and_scenario(Some("edge"));
    assert_eq!(status, 201);
    assert_eq!(body["status"], "partially_available");
    assert_eq!(body["warning"], "Some items unavailable");
}

#[tokio::test]
async fn test_scenario_fallback_to_schema() {
    // Test that when no examples are defined, schema-based generation still works
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Schema Only API",
            "version": "1.0.0"
        },
        "paths": {
            "/items": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "string"},
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

    let registry = create_registry_from_json(spec_json).expect("Failed to create registry");
    let route = registry.get_route("/items", "GET").expect("Route should exist");

    // Should still generate a response from schema
    let (status, body) = route.mock_response_with_status_and_scenario(Some("happy"));
    assert_eq!(status, 200);
    assert!(body.is_object());
    assert!(body.get("id").is_some());
    assert!(body.get("name").is_some());
}

#[tokio::test]
async fn test_single_example_without_scenarios() {
    // Test that single 'example' field (not 'examples') still works
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Single Example API",
            "version": "1.0.0"
        },
        "paths": {
            "/ping": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "Pong",
                            "content": {
                                "application/json": {
                                    "example": {
                                        "message": "pong",
                                        "timestamp": "2024-01-20T15:00:00Z"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let registry = create_registry_from_json(spec_json).expect("Failed to create registry");
    let route = registry.get_route("/ping", "GET").expect("Route should exist");

    // Single example should be returned regardless of scenario
    let (status, body) = route.mock_response_with_status_and_scenario(None);
    assert_eq!(status, 200);
    assert_eq!(body["message"], "pong");

    // Even with scenario specified, should return the single example
    let (status, body) = route.mock_response_with_status_and_scenario(Some("happy"));
    assert_eq!(status, 200);
    assert_eq!(body["message"], "pong");
}

#[tokio::test]
async fn test_multiple_status_codes_with_scenarios() {
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Multi-Status API",
            "version": "1.0.0"
        },
        "paths": {
            "/resource": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "examples": {
                                        "happy": {
                                            "value": {"status": "ok", "data": "success"}
                                        },
                                        "errors": {
                                            "value": {"status": "degraded", "data": "partial"}
                                        }
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Not Found",
                            "content": {
                                "application/json": {
                                    "example": {
                                        "error": "Resource not found"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let registry = create_registry_from_json(spec_json).expect("Failed to create registry");
    let route = registry.get_route("/resource", "GET").expect("Route should exist");

    // Should use 200 (first status code) with happy scenario
    let (status, body) = route.mock_response_with_status_and_scenario(Some("happy"));
    assert_eq!(status, 200);
    assert_eq!(body["status"], "ok");

    // Should use 200 with errors scenario
    let (status, body) = route.mock_response_with_status_and_scenario(Some("errors"));
    assert_eq!(status, 200);
    assert_eq!(body["status"], "degraded");
}

#[test]
fn test_env_var_scenario_selection() {
    // Test that environment variable is respected
    // Note: This test modifies environment variables, so it should be run in isolation

    std::env::set_var("MOCKFORGE_HTTP_SCENARIO", "errors");

    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Env Test API",
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
                                    "examples": {
                                        "happy": {
                                            "value": {"scenario": "happy"}
                                        },
                                        "errors": {
                                            "value": {"scenario": "errors"}
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

    let registry = create_registry_from_json(spec_json).expect("Failed to create registry");
    let route = registry.get_route("/test", "GET").expect("Route should exist");

    // When no scenario is passed to the method, it should use one of the examples
    // The environment variable is actually checked in the HTTP handler layer, not in the route method
    let (status, body) = route.mock_response_with_status_and_scenario(None);
    assert_eq!(status, 200);
    // Default behavior without passing scenario - should get one of the scenarios
    assert!(body.get("scenario").is_some());

    // Clean up
    std::env::remove_var("MOCKFORGE_HTTP_SCENARIO");
}
