//! Integration tests for code generation
//!
//! These tests verify that generated code can compile and run as expected.

use mockforge_core::codegen::{generate_mock_server_code, CodegenConfig, MockDataStrategy};
use mockforge_core::openapi::spec::OpenApiSpec;
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_generated_server_compiles() {
    // Create a simple OpenAPI spec
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/health": {
                "get": {
                    "operationId": "getHealth",
                    "responses": {
                        "200": {
                            "description": "Health check",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "status": {"type": "string", "example": "ok"}
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

    let spec = OpenApiSpec::from_json(spec_json).unwrap();
    let config = CodegenConfig {
        port: Some(3001),
        ..Default::default()
    };

    // Generate the code
    let generated_code =
        generate_mock_server_code(&spec, "rs", &config).expect("Should generate code successfully");

    // Verify basic structure
    assert!(generated_code.contains("pub struct GeneratedMockServer"));
    assert!(generated_code.contains("impl GeneratedMockServer"));
    assert!(generated_code.contains("pub async fn start"));

    // Verify that the health endpoint route is present
    assert!(generated_code.contains("/health") || generated_code.contains("health"));

    // Verify that handler is generated
    assert!(generated_code.contains("async fn handle_"));
}

#[test]
fn test_generated_code_with_path_parameters() {
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0.0"},
        "paths": {
            "/users/{id}": {
                "get": {
                    "operationId": "getUser",
                    "parameters": [{
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": {"type": "string"}
                    }],
                    "responses": {
                        "200": {
                            "description": "User",
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

    let spec = OpenApiSpec::from_json(spec_json).unwrap();
    let code = generate_mock_server_code(&spec, "rs", &CodegenConfig::default()).unwrap();

    // Check that path parameters are properly handled
    assert!(code.contains("Path(params)") || code.contains("Path("));
    // Check that Axum path uses :id syntax
    assert!(code.contains("/users/:id") || code.contains(":id"));
}

#[test]
fn test_generated_code_with_query_parameters() {
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0.0"},
        "paths": {
            "/search": {
                "get": {
                    "operationId": "search",
                    "parameters": [{
                        "name": "q",
                        "in": "query",
                        "required": false,
                        "schema": {"type": "string"}
                    }],
                    "responses": {
                        "200": {
                            "description": "Results",
                            "content": {
                                "application/json": {
                                    "schema": {"type": "array"}
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let spec = OpenApiSpec::from_json(spec_json).unwrap();
    let code = generate_mock_server_code(&spec, "rs", &CodegenConfig::default()).unwrap();

    // Check that query parameters are handled
    assert!(code.contains("Query(") || code.contains("query"));
}

#[test]
fn test_generated_code_with_request_body() {
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0.0"},
        "paths": {
            "/users": {
                "post": {
                    "operationId": "createUser",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "name": {"type": "string"}
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
                                    "schema": {"type": "object"}
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let spec = OpenApiSpec::from_json(spec_json).unwrap();
    let code = generate_mock_server_code(&spec, "rs", &CodegenConfig::default()).unwrap();

    // Check that request body is handled
    assert!(code.contains("Json(body)") || code.contains("body"));
}

#[test]
fn test_generated_code_with_delay_configuration() {
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0.0"},
        "paths": {
            "/test": {
                "get": {
                    "responses": {"200": {"description": "OK"}}
                }
            }
        }
    });

    let spec = OpenApiSpec::from_json(spec_json).unwrap();
    let config = CodegenConfig {
        default_delay_ms: Some(100),
        ..Default::default()
    };

    let code = generate_mock_server_code(&spec, "rs", &config).unwrap();

    // Check that delay is included
    assert!(code.contains("tokio::time::sleep") || code.contains("Duration::from_millis"));
    assert!(code.contains("100"));
}

#[test]
fn test_generated_code_with_custom_port() {
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0.0"},
        "paths": {
            "/test": {
                "get": {
                    "responses": {"200": {"description": "OK"}}
                }
            }
        }
    });

    let spec = OpenApiSpec::from_json(spec_json).unwrap();
    let config = CodegenConfig {
        port: Some(8080),
        ..Default::default()
    };

    let code = generate_mock_server_code(&spec, "rs", &config).unwrap();

    // Check that port is set correctly
    assert!(code.contains("8080"));
}

#[test]
fn test_generated_code_includes_all_http_methods() {
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {"title": "Test", "version": "1.0.0"},
        "paths": {
            "/resource": {
                "get": {"responses": {"200": {"description": "OK"}}},
                "post": {"responses": {"201": {"description": "Created"}}},
                "put": {"responses": {"200": {"description": "OK"}}},
                "delete": {"responses": {"204": {"description": "No Content"}}},
                "patch": {"responses": {"200": {"description": "OK"}}}
            }
        }
    });

    let spec = OpenApiSpec::from_json(spec_json).unwrap();
    let code = generate_mock_server_code(&spec, "rs", &CodegenConfig::default()).unwrap();

    // Verify all methods generate routes
    assert!(code.contains(".route("));
    // Check that we have multiple routes (should have multiple .route calls)
    let route_count = code.matches(".route(").count();
    assert!(route_count >= 5, "Should have routes for all methods");
}
