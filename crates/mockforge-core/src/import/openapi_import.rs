//! OpenAPI specification import functionality
//!
//! This module handles parsing OpenAPI/Swagger specifications and converting them
//! to MockForge routes and configurations.

use crate::import::schema_data_generator::generate_from_schema;
use crate::openapi::OpenApiSpec;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

// Pre-compiled regex for path parameter conversion
static PATH_PARAM_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{([^}]+)\}").expect("PATH_PARAM_RE regex is valid"));

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
        version: spec.api_version().to_string(),
        description: spec.description().map(|s| s.to_string()),
        openapi_version: spec.version().to_string(),
        servers: spec
            .spec
            .servers
            .iter()
            .filter_map(|server| server.url.parse::<url::Url>().ok())
            .map(|url| url.to_string())
            .collect(),
    };

    let mut routes = Vec::new();
    let mut warnings = Vec::new();

    // Process all paths and operations in deterministic order
    let path_operations = spec.all_paths_and_operations();

    // Sort paths alphabetically for deterministic ordering
    let mut sorted_paths: Vec<_> = path_operations.iter().collect();
    sorted_paths.sort_by_key(|(path, _)| path.as_str());

    for (path, operations) in sorted_paths {
        // Process operations in a specific order for deterministic results
        let method_order = [
            "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "TRACE",
        ];

        for method in method_order {
            if let Some(operation) = operations.get(method) {
                match convert_operation_to_route(&spec, method, path, operation, _base_url) {
                    Ok(route) => routes.push(route),
                    Err(e) => warnings.push(format!("Failed to convert {method} {path}: {e}")),
                }
            }
        }
    }

    Ok(OpenApiImportResult {
        routes,
        warnings,
        spec_info,
    })
}

/// Convert an OpenAPI operation to a MockForge route
fn convert_operation_to_route(
    _spec: &OpenApiSpec,
    method: &str,
    path: &str,
    operation: &openapiv3::Operation,
    _base_url: Option<&str>,
) -> Result<MockForgeRoute, String> {
    // Use the first 200-series response as the default response
    let mut response_status = 200;
    let mut response_body = Value::Object(serde_json::Map::new());
    let mut response_headers = HashMap::new();

    // Find the first success response (200-299)
    for (status_code, response_ref) in &operation.responses.responses {
        // Handle different StatusCode types
        let is_success = match status_code {
            openapiv3::StatusCode::Code(code) => (200..300).contains(code),
            openapiv3::StatusCode::Range(range) => *range == 2, // 2XX means success
        };

        if is_success {
            let status = match status_code {
                openapiv3::StatusCode::Code(code) => *code,
                openapiv3::StatusCode::Range(_) => 200, // Default to 200 for 2XX
            };

            if (200..300).contains(&status) {
                response_status = status;

                // Try to resolve the response and extract content
                if let Some(response) = response_ref.as_item() {
                    // Add default content-type header
                    response_headers
                        .insert("Content-Type".to_string(), "application/json".to_string());

                    // Try to generate a sample response from schema
                    if let Some(content) = response.content.get("application/json") {
                        // Check for examples first
                        if let Some(example) = &content.example {
                            response_body = example.clone();
                        } else if !content.examples.is_empty() {
                            // Use the first example
                            if let Some((_key, example_ref)) = content.examples.iter().next() {
                                if let Some(example_value) = example_ref.as_item() {
                                    if let Some(value) = &example_value.value {
                                        response_body = value.clone();
                                    }
                                }
                            }
                        } else if let Some(schema_ref) = &content.schema {
                            // Generate from schema
                            response_body = match schema_ref {
                                openapiv3::ReferenceOr::Item(schema_data) => {
                                    generate_response_from_openapi_schema(schema_data)
                                }
                                openapiv3::ReferenceOr::Reference { .. } => {
                                    // Can't resolve references easily, use basic mock
                                    serde_json::json!({"message": "Mock response", "path": path, "method": method})
                                }
                            };
                        } else {
                            // No schema or example, basic response
                            response_body = serde_json::json!({"message": "Success"});
                        }
                    } else {
                        // No content schema, provide a basic response
                        response_body = serde_json::json!({"message": "Success"});
                    }
                } else {
                    // Default response if reference can't be resolved
                    response_body = serde_json::json!({"message": "Mock response"});
                }
                break;
            }
        }
    }

    // Check for default response if no success response found
    if response_status == 200 && operation.responses.default.is_some() {
        response_body = serde_json::json!({"message": "Default response"});
    }

    let mock_response = MockForgeResponse {
        status: response_status,
        headers: response_headers,
        body: response_body,
    };

    // Convert OpenAPI path parameters {param} to Express-style :param
    let converted_path = convert_path_parameters(path);

    // Extract request body if present
    let request_body = if let Some(request_body_ref) = &operation.request_body {
        extract_request_body_example(request_body_ref)
    } else {
        None
    };

    Ok(MockForgeRoute {
        method: method.to_uppercase(),
        path: converted_path,
        headers: HashMap::new(), // Could extract from parameters in a full implementation
        body: request_body,
        response: mock_response,
    })
}

/// Extract request body example from OpenAPI request body reference
fn extract_request_body_example(
    request_body_ref: &openapiv3::ReferenceOr<openapiv3::RequestBody>,
) -> Option<String> {
    match request_body_ref {
        openapiv3::ReferenceOr::Item(request_body) => {
            // Look for application/json content type
            if let Some(media_type) = request_body.content.get("application/json") {
                // Check if there's an example
                if let Some(example) = &media_type.example {
                    if let Ok(example_str) = serde_json::to_string(example) {
                        return Some(example_str);
                    }
                }

                // If no example, create a simple mock based on schema
                if let Some(_schema_ref) = &media_type.schema {
                    // For now, just return a simple mock object
                    return Some(r#"{"mock": "data"}"#.to_string());
                }
            }
            None
        }
        openapiv3::ReferenceOr::Reference { .. } => {
            // For referenced request bodies, we'd need to resolve the reference
            // For now, just return a simple mock
            Some(r#"{"mock": "data"}"#.to_string())
        }
    }
}

/// Convert OpenAPI path parameters {param} to Express-style :param
fn convert_path_parameters(path: &str) -> String {
    PATH_PARAM_RE.replace_all(path, ":$1").to_string()
}

/// Generate response from OpenAPI schema
fn generate_response_from_openapi_schema(schema: &openapiv3::Schema) -> Value {
    // Convert OpenAPI schema to JSON Schema format for our generator
    let json_schema = openapi_schema_to_json_schema(schema);
    generate_from_schema(&json_schema)
}

/// Convert OpenAPI Schema to JSON Schema Value
fn openapi_schema_to_json_schema(schema: &openapiv3::Schema) -> Value {
    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(type_schema) => match type_schema {
            openapiv3::Type::String(string_type) => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".to_string(), json!("string"));

                // Format is VariantOrUnknownOrEmpty, check if it has a value
                if !matches!(string_type.format, openapiv3::VariantOrUnknownOrEmpty::Empty) {
                    obj.insert("format".to_string(), json!(format!("{:?}", string_type.format)));
                }

                // enumeration is Vec<Option<String>>, not Option
                if !string_type.enumeration.is_empty() {
                    let enum_values: Vec<Value> = string_type
                        .enumeration
                        .iter()
                        .filter_map(|s| s.as_ref().map(|s| json!(s)))
                        .collect();
                    if !enum_values.is_empty() {
                        obj.insert("enum".to_string(), json!(enum_values));
                    }
                }

                Value::Object(obj)
            }
            openapiv3::Type::Number(_) => {
                json!({"type": "number"})
            }
            openapiv3::Type::Integer(_) => {
                json!({"type": "integer"})
            }
            openapiv3::Type::Boolean(_) => {
                json!({"type": "boolean"})
            }
            openapiv3::Type::Array(array_type) => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".to_string(), json!("array"));

                if let Some(items) = &array_type.items {
                    if let Some(item_schema) = items.as_item() {
                        obj.insert("items".to_string(), openapi_schema_to_json_schema(item_schema));
                    }
                }

                Value::Object(obj)
            }
            openapiv3::Type::Object(object_type) => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".to_string(), json!("object"));

                if !object_type.properties.is_empty() {
                    let mut props = serde_json::Map::new();
                    for (name, schema_ref) in &object_type.properties {
                        if let Some(prop_schema) = schema_ref.as_item() {
                            props.insert(name.clone(), openapi_schema_to_json_schema(prop_schema));
                        }
                    }
                    obj.insert("properties".to_string(), Value::Object(props));
                }

                if !object_type.required.is_empty() {
                    obj.insert("required".to_string(), json!(object_type.required));
                }

                Value::Object(obj)
            }
        },
        openapiv3::SchemaKind::OneOf { .. } => {
            // For oneOf, just return a basic object
            json!({"type": "object"})
        }
        openapiv3::SchemaKind::AllOf { .. } => {
            // For allOf, just return a basic object
            json!({"type": "object"})
        }
        openapiv3::SchemaKind::AnyOf { .. } => {
            // For anyOf, just return a basic object
            json!({"type": "object"})
        }
        openapiv3::SchemaKind::Not { .. } => {
            json!({"type": "object"})
        }
        openapiv3::SchemaKind::Any(_) => {
            json!({"type": "object"})
        }
    }
}

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
                        "operationId": "getUsers",
                        "responses": {}
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
