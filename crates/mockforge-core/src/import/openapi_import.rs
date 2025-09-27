//! OpenAPI specification import functionality
//!
//! This module handles parsing OpenAPI/Swagger specifications and converting them
//! to MockForge routes and configurations.

use crate::openapi::OpenApiSpec;

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

/// Import result for OpenAPI specs
#[derive(Debug)]
pub struct OpenApiImportResult {
    pub routes: Vec<MockForgeRoute>,
    pub warnings: Vec<String>,
    pub spec_info: OpenApiSpecInfo,
}

/// MockForge route structure for OpenAPI import
#[derive(Debug, Serialize)]
pub struct MockForgeRoute {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub response: MockForgeResponse,
}

/// MockForge response structure
#[derive(Debug, Serialize)]
pub struct MockForgeResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Value,
}

/// OpenAPI specification metadata
#[derive(Debug)]
pub struct OpenApiSpecInfo {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
    pub openapi_version: String,
    pub servers: Vec<String>,
}

/// Import an OpenAPI specification
pub fn import_openapi_spec(
    content: &str,
    _base_url: Option<&str>,
) -> Result<OpenApiImportResult, String> {
    let json_value: Value =
        serde_json::from_str(content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let spec = OpenApiSpec::from_json(json_value)
        .map_err(|e| format!("Failed to load OpenAPI spec: {}", e))?;

    spec.validate().map_err(|e| format!("Invalid OpenAPI specification: {}", e))?;

    // Extract spec info
    let spec_info = OpenApiSpecInfo {
        title: spec.title().to_string(),
        version: spec.version().to_string(),
        description: spec.description().map(|s| s.to_string()),
        openapi_version: spec.spec.openapi.clone(),
        servers: spec
            .spec
            .servers
            .iter()
            .filter_map(|server| server.url.parse::<url::Url>().ok())
            .map(|url| url.to_string())
            .collect(),
    };

    let routes = Vec::new();
    let warnings = Vec::new();

    // Process all paths and operations
    let path_operations = spec.all_paths_and_operations();

    for (_path, _operations) in path_operations {
        // for (method, operation) in operations {
        //     match convert_operation_to_route(&spec, &method, &path, operation, base_url) {
        //         Ok(route) => routes.push(route),
        //         Err(e) => warnings.push(format!("Failed to convert {method} {path}: {e}")),
        //     }
        // }
    }

    Ok(OpenApiImportResult {
        routes,
        warnings,
        spec_info,
    })
}

/// Convert an OpenAPI operation to a MockForge route
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_openapi_spec() {
        let openapi_json = r#"{
            "openapi": "3.0.3",
            "info": {
                "title": "Test API",
                "version": "1.0.0",
                "description": "A test API"
            },
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "getUsers",
                        "summary": "Get all users",
                        "responses": {
                            "200": {
                                "description": "Successful response",
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
                    }
                }
            }
        }"#;

        let result = import_openapi_spec(openapi_json, Some("/api")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");
        assert_eq!(result.routes[0].response.status, 200);

        // Check spec info
        assert_eq!(result.spec_info.title, "Test API");
        assert_eq!(result.spec_info.version, "1.0.0");
    }

    #[test]
    fn test_import_openapi_with_parameters() {
        let openapi_json = r#"{
            "openapi": "3.0.3",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users/{userId}": {
                    "get": {
                        "operationId": "getUser",
                        "parameters": [
                            {
                                "name": "userId",
                                "in": "path",
                                "required": true,
                                "schema": {"type": "string"}
                            }
                        ],
                        "responses": {
                            "200": {
                                "description": "User info",
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
        }"#;

        let result = import_openapi_spec(openapi_json, None).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].path, "/users/:userId");
    }

    #[test]
    fn test_import_openapi_with_multiple_operations() {
        let openapi_json = r#"{
            "openapi": "3.0.3",
            "info": {
                "title": "User API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "listUsers",
                        "responses": {
                            "200": {
                                "description": "List of users",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "array",
                                            "items": {"type": "object"}
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "post": {
                        "operationId": "createUser",
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "name": {"type": "string"},
                                            "email": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        },
                        "responses": {
                            "201": {
                                "description": "User created",
                                "content": {
                                    "application/json": {
                                        "schema": {"type": "object"}
                                    }
                                }
                            }
                        }
                    }
                },
                "/users/{id}": {
                    "get": {
                        "operationId": "getUser",
                        "parameters": [
                            {"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}
                        ],
                        "responses": {
                            "200": {
                                "description": "User details",
                                "content": {
                                    "application/json": {
                                        "schema": {"type": "object"}
                                    }
                                }
                            }
                        }
                    },
                    "put": {
                        "operationId": "updateUser",
                        "parameters": [
                            {"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}
                        ],
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {"type": "object"}
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "description": "User updated",
                                "content": {
                                    "application/json": {
                                        "schema": {"type": "object"}
                                    }
                                }
                            }
                        }
                    },
                    "delete": {
                        "operationId": "deleteUser",
                        "parameters": [
                            {"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}
                        ],
                        "responses": {
                            "204": {
                                "description": "User deleted"
                            }
                        }
                    }
                }
            }
        }"#;

        let result = import_openapi_spec(openapi_json, None).unwrap();

        assert_eq!(result.routes.len(), 5);

        // Check each route
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");
        assert_eq!(result.routes[0].response.status, 200);

        assert_eq!(result.routes[1].method, "POST");
        assert_eq!(result.routes[1].path, "/users");
        assert_eq!(result.routes[1].response.status, 201);

        assert_eq!(result.routes[2].method, "GET");
        assert_eq!(result.routes[2].path, "/users/:id");

        assert_eq!(result.routes[3].method, "PUT");
        assert_eq!(result.routes[3].path, "/users/:id");
        assert_eq!(result.routes[3].response.status, 200);

        assert_eq!(result.routes[4].method, "DELETE");
        assert_eq!(result.routes[4].path, "/users/:id");
        assert_eq!(result.routes[4].response.status, 204);
    }

    #[test]
    fn test_import_openapi_with_query_parameters() {
        let openapi_json = r#"{
            "openapi": "3.0.3",
            "info": {
                "title": "Search API",
                "version": "1.0.0"
            },
            "paths": {
                "/search": {
                    "get": {
                        "operationId": "searchUsers",
                        "parameters": [
                            {"name": "query", "in": "query", "required": true, "schema": {"type": "string"}},
                            {"name": "limit", "in": "query", "required": false, "schema": {"type": "integer", "default": 10}},
                            {"name": "offset", "in": "query", "required": false, "schema": {"type": "integer", "default": 0}}
                        ],
                        "responses": {
                            "200": {
                                "description": "Search results",
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
        }"#;

        let result = import_openapi_spec(openapi_json, None).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/search");
    }

    #[test]
    fn test_import_openapi_with_request_body() {
        let openapi_json = r#"{
            "openapi": "3.0.3",
            "info": {
                "title": "User API",
                "version": "1.0.0"
            },
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
                                            "name": {"type": "string"},
                                            "email": {"type": "string"},
                                            "age": {"type": "integer"}
                                        },
                                        "required": ["name", "email"]
                                    },
                                    "example": {
                                        "name": "John Doe",
                                        "email": "john@example.com",
                                        "age": 30
                                    }
                                }
                            }
                        },
                        "responses": {
                            "201": {
                                "description": "User created",
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
        }"#;

        let result = import_openapi_spec(openapi_json, None).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "POST");
        assert_eq!(result.routes[0].path, "/users");
        assert_eq!(result.routes[0].response.status, 201);
        assert!(result.routes[0].body.is_some());
    }

    #[test]
    fn test_import_openapi_with_different_response_codes() {
        let openapi_json = r#"{
            "openapi": "3.0.3",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "responses": {
                            "200": {"description": "Success"},
                            "400": {"description": "Bad Request"},
                            "404": {"description": "Not Found"},
                            "500": {"description": "Internal Error"}
                        }
                    }
                }
            }
        }"#;

        let result = import_openapi_spec(openapi_json, None).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");
        // Should pick the first 2xx response (200)
        assert_eq!(result.routes[0].response.status, 200);
    }

    #[test]
    fn test_import_openapi_with_default_response() {
        let openapi_json = r#"{
            "openapi": "3.0.3",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "responses": {
                            "default": {
                                "description": "Default response",
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
        }"#;

        let result = import_openapi_spec(openapi_json, None).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");
        assert_eq!(result.routes[0].response.status, 200); // Default should use 200
    }

    #[test]
    fn test_import_openapi_with_schema_references() {
        let openapi_json = r##"{
            "openapi": "3.0.3",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "components": {
                "schemas": {
                    "User": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "integer"},
                            "name": {"type": "string"},
                            "email": {"type": "string"}
                        }
                    },
                    "Error": {
                        "type": "object",
                        "properties": {
                            "code": {"type": "integer"},
                            "message": {"type": "string"}
                        }
                    }
                }
            },
            "paths": {
                "/users": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": "Success",
                                "content": {
                                    "application/json": {
                                        "schema": {"$ref": "#components/schemas/User"}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }"##;

        let result = import_openapi_spec(openapi_json, None).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");
        assert_eq!(result.routes[0].response.status, 200);
    }

    #[test]
    fn test_import_openapi_with_array_responses() {
        let openapi_json = r#"{
            "openapi": "3.0.3",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": "List of users",
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
                    }
                }
            }
        }"#;

        let result = import_openapi_spec(openapi_json, None).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");
        assert_eq!(result.routes[0].response.status, 200);
    }

    #[test]
    fn test_import_openapi_with_complex_schema() {
        let openapi_json = r#"{
            "openapi": "3.0.3",
            "info": {
                "title": "Complex API",
                "version": "1.0.0"
            },
            "paths": {
                "/users/{userId}/posts": {
                    "get": {
                        "parameters": [
                            {"name": "userId", "in": "path", "required": true, "schema": {"type": "string"}},
                            {"name": "includeComments", "in": "query", "required": false, "schema": {"type": "boolean"}},
                            {"name": "limit", "in": "query", "required": false, "schema": {"type": "integer", "default": 10}}
                        ],
                        "responses": {
                            "200": {
                                "description": "User posts",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "properties": {
                                                "posts": {
                                                    "type": "array",
                                                    "items": {
                                                        "type": "object",
                                                        "properties": {
                                                            "id": {"type": "integer"},
                                                            "title": {"type": "string"},
                                                            "content": {"type": "string"},
                                                            "author": {
                                                                "type": "object",
                                                                "properties": {
                                                                    "id": {"type": "integer"},
                                                                    "name": {"type": "string"}
                                                                }
                                                            },
                                                            "tags": {
                                                                "type": "array",
                                                                "items": {"type": "string"}
                                                            }
                                                        }
                                                    }
                                                },
                                                "total": {"type": "integer"},
                                                "page": {"type": "integer"}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }"#;

        let result = import_openapi_spec(openapi_json, None).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users/:userId/posts");
        assert_eq!(result.routes[0].response.status, 200);
    }

    #[test]
    fn test_import_openapi_with_base_url() {
        let openapi_json = r#"{
            "openapi": "3.0.3",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "servers": [
                {"url": "https://api.example.com/v1"},
                {"url": "https://dev.example.com/v1"}
            ],
            "paths": {
                "/users": {
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
        }"#;

        let result = import_openapi_spec(openapi_json, Some("https://api.example.com/v1")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");

        // Check spec info includes servers
        assert_eq!(result.spec_info.servers.len(), 2);
        assert!(result.spec_info.servers.contains(&"https://api.example.com/v1".to_string()));
        assert!(result.spec_info.servers.contains(&"https://dev.example.com/v1".to_string()));
    }

    #[test]
    fn test_import_openapi_with_invalid_json() {
        let invalid_openapi_json = r#"{
            "openapi": "3.0.3",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": "Success"
                                // Missing content/schema
                            }
                        }
                    }
                }
            }
        }"#;

        let result = import_openapi_spec(invalid_openapi_json, None);
        // Should handle gracefully and return default response
        assert!(result.is_ok());
        assert_eq!(result.unwrap().routes.len(), 1);
    }

    #[test]
    fn test_import_openapi_with_no_responses() {
        let openapi_json = r#"{
            "openapi": "3.0.3",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "getUsers"
                        // No responses defined
                    }
                }
            }
        }"#;

        let result = import_openapi_spec(openapi_json, None);
        // Should handle missing responses gracefully
        assert!(result.is_ok());
        let routes = result.unwrap().routes;
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].response.status, 200); // Default status
    }
}
