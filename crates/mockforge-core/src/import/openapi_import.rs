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

/// Result of importing an OpenAPI specification
#[derive(Debug)]
pub struct OpenApiImportResult {
    /// Converted routes from OpenAPI paths/operations
    pub routes: Vec<MockForgeRoute>,
    /// Warnings encountered during import
    pub warnings: Vec<String>,
    /// Extracted specification metadata
    pub spec_info: OpenApiSpecInfo,
}

/// MockForge route structure for OpenAPI import
#[derive(Debug, Serialize)]
pub struct MockForgeRoute {
    /// HTTP method
    pub method: String,
    /// Request path (with Express-style path parameters)
    pub path: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Optional request body
    pub body: Option<String>,
    /// Mock response for this route
    pub response: MockForgeResponse,
}

/// MockForge response structure
#[derive(Debug, Serialize)]
pub struct MockForgeResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Value,
}

/// OpenAPI specification metadata
#[derive(Debug)]
pub struct OpenApiSpecInfo {
    /// API title
    pub title: String,
    /// API version
    pub version: String,
    /// Optional API description
    pub description: Option<String>,
    /// OpenAPI specification version (e.g., "3.0.3")
    pub openapi_version: String,
    /// List of server URLs from the spec
    pub servers: Vec<String>,
}

/// Import an OpenAPI specification
pub fn import_openapi_spec(
    content: &str,
    _base_url: Option<&str>,
) -> Result<OpenApiImportResult, String> {
    // Detect format and validate using enhanced validator
    let format = crate::spec_parser::SpecFormat::detect(content, None)
        .map_err(|e| format!("Failed to detect spec format: {}", e))?;

    // Parse as JSON value first for validation - optimized to avoid double parsing
    // Try JSON first, then YAML (more robust detection)
    let json_value: Value = match serde_json::from_str::<Value>(content) {
        Ok(val) => val,
        Err(_) => {
            // Try YAML if JSON parsing fails
            serde_yaml::from_str(content)
                .map_err(|e| format!("Failed to parse as JSON or YAML: {}", e))?
        }
    };

    // Validate using enhanced validator for better error messages
    match format {
        crate::spec_parser::SpecFormat::OpenApi20 => {
            let validation = crate::spec_parser::OpenApiValidator::validate(&json_value, format);
            if !validation.is_valid {
                // Format errors on separate lines for better readability
                let error_msg = validation
                    .errors
                    .iter()
                    .map(|e| format!("  - {}", e))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(format!("Invalid OpenAPI 2.0 (Swagger) specification:\n{}", error_msg));
            }

            // Note: OpenAPI 2.0 support is currently limited to validation.
            // Full parsing requires conversion to OpenAPI 3.x format.
            // For now, return a helpful error suggesting conversion.
            return Err("OpenAPI 2.0 (Swagger) specifications are detected but not yet fully supported for parsing. \
                Please convert your Swagger 2.0 spec to OpenAPI 3.x format. \
                You can use tools like 'swagger2openapi' or the online converter at https://editor.swagger.io/ to convert your spec.".to_string());
        }
        crate::spec_parser::SpecFormat::OpenApi30 | crate::spec_parser::SpecFormat::OpenApi31 => {
            let validation = crate::spec_parser::OpenApiValidator::validate(&json_value, format);
            if !validation.is_valid {
                // Format errors on separate lines for better readability
                let error_msg = validation
                    .errors
                    .iter()
                    .map(|e| format!("  - {}", e))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(format!("Invalid OpenAPI specification:\n{}", error_msg));
            }
            // Continue with parsing
        }
        _ => {
            return Err(format!(
                "Unsupported specification format: {}. Only OpenAPI 3.x is currently supported for parsing.",
                format.display_name()
            ));
        }
    }

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
    spec: &OpenApiSpec,
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
                            // Generate from schema, resolving $ref if needed
                            response_body = if let Some(resolved) =
                                resolve_schema_ref(schema_ref, &spec.spec)
                            {
                                generate_response_from_openapi_schema(&resolved)
                            } else {
                                serde_json::json!({"message": "Mock response", "path": path, "method": method})
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
        extract_request_body_example(request_body_ref, &spec.spec)
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
    spec: &openapiv3::OpenAPI,
) -> Option<String> {
    let request_body = match request_body_ref {
        openapiv3::ReferenceOr::Item(rb) => rb.clone(),
        openapiv3::ReferenceOr::Reference { reference } => {
            // Resolve $ref like "#/components/requestBodies/MyBody"
            let name = reference.strip_prefix("#/components/requestBodies/")?;
            let components = spec.components.as_ref()?;
            let rb_ref = components.request_bodies.get(name)?;
            match rb_ref {
                openapiv3::ReferenceOr::Item(rb) => rb.clone(),
                openapiv3::ReferenceOr::Reference { .. } => return None,
            }
        }
    };

    // Look for application/json content type
    let media_type = request_body.content.get("application/json")?;

    // Check if there's an explicit example
    if let Some(example) = &media_type.example {
        if let Ok(example_str) = serde_json::to_string(example) {
            return Some(example_str);
        }
    }

    // Generate mock data from schema
    if let Some(schema_ref) = &media_type.schema {
        let schema = resolve_schema_ref(schema_ref, spec);
        if let Some(s) = schema {
            let json_schema = openapi_schema_to_json_schema(&s);
            let generated = generate_from_schema(&json_schema);
            if let Ok(s) = serde_json::to_string(&generated) {
                return Some(s);
            }
        }
    }

    None
}

/// Resolve a schema reference to an owned Schema
fn resolve_schema_ref(
    schema_ref: &openapiv3::ReferenceOr<openapiv3::Schema>,
    spec: &openapiv3::OpenAPI,
) -> Option<openapiv3::Schema> {
    match schema_ref {
        openapiv3::ReferenceOr::Item(schema) => Some(schema.clone()),
        openapiv3::ReferenceOr::Reference { reference } => {
            let name = reference.strip_prefix("#/components/schemas/")?;
            let components = spec.components.as_ref()?;
            let resolved = components.schemas.get(name)?;
            match resolved {
                openapiv3::ReferenceOr::Item(schema) => Some(schema.clone()),
                openapiv3::ReferenceOr::Reference { .. } => None,
            }
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
        openapiv3::SchemaKind::OneOf { one_of } => {
            // Use the first variant for mock data generation
            if let Some(first) = one_of.first() {
                if let Some(schema) = first.as_item() {
                    return openapi_schema_to_json_schema(schema);
                }
            }
            json!({"type": "object"})
        }
        openapiv3::SchemaKind::AllOf { all_of } => {
            // Merge all schemas into a single object with combined properties
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();
            for schema_ref in all_of {
                if let Some(sub_schema) = schema_ref.as_item() {
                    let converted = openapi_schema_to_json_schema(sub_schema);
                    if let Some(obj) = converted.as_object() {
                        if let Some(props) = obj.get("properties").and_then(|p| p.as_object()) {
                            for (k, v) in props {
                                properties.insert(k.clone(), v.clone());
                            }
                        }
                        if let Some(req) = obj.get("required").and_then(|r| r.as_array()) {
                            for r in req {
                                if let Some(s) = r.as_str() {
                                    required.push(json!(s));
                                }
                            }
                        }
                    }
                }
            }
            let mut result = serde_json::Map::new();
            result.insert("type".to_string(), json!("object"));
            if !properties.is_empty() {
                result.insert("properties".to_string(), Value::Object(properties));
            }
            if !required.is_empty() {
                result.insert("required".to_string(), Value::Array(required));
            }
            Value::Object(result)
        }
        openapiv3::SchemaKind::AnyOf { any_of } => {
            // Use the first variant for mock data generation
            if let Some(first) = any_of.first() {
                if let Some(schema) = first.as_item() {
                    return openapi_schema_to_json_schema(schema);
                }
            }
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
