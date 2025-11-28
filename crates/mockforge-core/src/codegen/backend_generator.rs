//! Backend code generation utilities
//!
//! This module provides shared utilities for generating backend server code
//! from OpenAPI specifications. These utilities can be used by backend generator
//! plugins to extract routes, convert schemas, and generate common patterns.

use crate::openapi::spec::OpenApiSpec;
use crate::Result;
use openapiv3::{Operation, ParameterSchemaOrContent, PathItem, ReferenceOr, Schema};
use std::collections::HashMap;

/// Information about a route extracted from OpenAPI spec
#[derive(Debug, Clone)]
pub struct RouteInfo {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// API path (e.g., /users/{id})
    pub path: String,
    /// Operation ID from spec
    pub operation_id: Option<String>,
    /// Summary from spec
    pub summary: Option<String>,
    /// Description from spec
    pub description: Option<String>,
    /// Path parameters (e.g., {id} -> ["id"])
    pub path_params: Vec<String>,
    /// Query parameters
    pub query_params: Vec<QueryParamInfo>,
    /// Request body schema (if any)
    pub request_body_schema: Option<Schema>,
    /// Response schemas mapped by status code
    pub responses: HashMap<u16, ResponseInfo>,
    /// Tags for grouping
    pub tags: Vec<String>,
}

/// Query parameter information
#[derive(Debug, Clone)]
pub struct QueryParamInfo {
    /// Parameter name
    pub name: String,
    /// Whether parameter is required
    pub required: bool,
    /// Parameter schema
    pub schema: Option<Schema>,
    /// Parameter description
    pub description: Option<String>,
}

/// Response information
#[derive(Debug, Clone)]
pub struct ResponseInfo {
    /// HTTP status code
    pub status_code: u16,
    /// Response description
    pub description: Option<String>,
    /// Response schema (if any)
    pub schema: Option<Schema>,
    /// Example response (if any)
    pub example: Option<serde_json::Value>,
}

/// Extract all routes from an OpenAPI specification
///
/// # Arguments
/// * `spec` - The OpenAPI specification to extract routes from
///
/// # Returns
/// Vector of route information for all operations in the spec
pub fn extract_routes(spec: &OpenApiSpec) -> Result<Vec<RouteInfo>> {
    let mut routes = Vec::new();

    for (path, path_item) in &spec.spec.paths.paths {
        if let Some(item) = path_item.as_item() {
            // Extract routes for each HTTP method
            if let Some(op) = &item.get {
                routes.push(extract_route_info("GET", path, op, item)?);
            }
            if let Some(op) = &item.post {
                routes.push(extract_route_info("POST", path, op, item)?);
            }
            if let Some(op) = &item.put {
                routes.push(extract_route_info("PUT", path, op, item)?);
            }
            if let Some(op) = &item.delete {
                routes.push(extract_route_info("DELETE", path, op, item)?);
            }
            if let Some(op) = &item.patch {
                routes.push(extract_route_info("PATCH", path, op, item)?);
            }
            if let Some(op) = &item.head {
                routes.push(extract_route_info("HEAD", path, op, item)?);
            }
            if let Some(op) = &item.options {
                routes.push(extract_route_info("OPTIONS", path, op, item)?);
            }
            if let Some(op) = &item.trace {
                routes.push(extract_route_info("TRACE", path, op, item)?);
            }
        }
    }

    Ok(routes)
}

/// Extract route information from an OpenAPI operation
fn extract_route_info(
    method: &str,
    path: &str,
    operation: &Operation,
    path_item: &PathItem,
) -> Result<RouteInfo> {
    // Extract path parameters from the path string
    let path_params = extract_path_parameters(path);

    // Extract query parameters
    let mut query_params = Vec::new();
    for param_ref in &operation.parameters {
        if let Some(param) = param_ref.as_item() {
            if let openapiv3::Parameter::Query { parameter_data, .. } = param {
                let schema =
                    if let ParameterSchemaOrContent::Schema(schema_ref) = &parameter_data.format {
                        schema_ref.as_item().cloned()
                    } else {
                        None
                    };

                query_params.push(QueryParamInfo {
                    name: parameter_data.name.clone(),
                    required: parameter_data.required,
                    schema,
                    description: parameter_data.description.clone(),
                });
            }
        }
    }

    // Extract request body schema
    let request_body_schema = operation
        .request_body
        .as_ref()
        .and_then(|body_ref| body_ref.as_item())
        .and_then(|body| {
            body.content
                .get("application/json")
                .and_then(|content| content.schema.as_ref())
                .and_then(|schema_ref| schema_ref.as_item().cloned())
        });

    // Extract responses
    let mut responses = HashMap::new();
    for (status_code, response_ref) in &operation.responses.responses {
        let status = match status_code {
            openapiv3::StatusCode::Code(code) => *code,
            openapiv3::StatusCode::Range(range) if *range == 2 => 200,
            openapiv3::StatusCode::Range(range) if *range == 4 => 400,
            openapiv3::StatusCode::Range(range) if *range == 5 => 500,
            _ => continue,
        };

        if let Some(response) = response_ref.as_item() {
            let schema = response
                .content
                .get("application/json")
                .and_then(|content| content.schema.as_ref())
                .and_then(|schema_ref| schema_ref.as_item().cloned());

            let example = response.content.get("application/json").and_then(|content| {
                content.example.clone().or_else(|| {
                    content.examples.iter().next().and_then(|(_, example_ref)| {
                        example_ref.as_item().and_then(|example_item| example_item.value.clone())
                    })
                })
            });

            responses.insert(
                status,
                ResponseInfo {
                    status_code: status,
                    description: Some(response.description.clone()),
                    schema,
                    example,
                },
            );
        }
    }

    Ok(RouteInfo {
        method: method.to_string(),
        path: path.to_string(),
        operation_id: operation.operation_id.clone(),
        summary: operation.summary.clone(),
        description: operation.description.clone(),
        path_params,
        query_params,
        request_body_schema,
        responses,
        tags: operation.tags.clone(),
    })
}

/// Extract path parameters from an OpenAPI path string
///
/// # Arguments
/// * `path` - The path string (e.g., "/users/{id}/posts/{postId}")
///
/// # Returns
/// Vector of parameter names found in the path
pub fn extract_path_parameters(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut in_param = false;
    let mut current_param = String::new();

    for ch in path.chars() {
        match ch {
            '{' => {
                in_param = true;
                current_param.clear();
            }
            '}' => {
                if in_param && !current_param.is_empty() {
                    params.push(current_param.clone());
                    in_param = false;
                }
            }
            ch if in_param => {
                current_param.push(ch);
            }
            _ => {}
        }
    }

    params
}

/// Get all schemas from OpenAPI components
///
/// # Arguments
/// * `spec` - The OpenAPI specification
///
/// # Returns
/// Map of schema name to schema definition
pub fn extract_schemas(spec: &OpenApiSpec) -> HashMap<String, Schema> {
    let mut schemas = HashMap::new();

    if let Some(components) = &spec.spec.components {
        if !components.schemas.is_empty() {
            for (name, schema_ref) in &components.schemas {
                if let ReferenceOr::Item(schema) = schema_ref {
                    schemas.insert(name.clone(), schema.clone());
                }
            }
        }
    }

    schemas
}

/// Convert OpenAPI schema type to a Rust type name
///
/// # Arguments
/// * `schema` - The OpenAPI schema
/// * `schema_name` - Optional name for the schema (used for object types)
///
/// # Returns
/// Rust type name as a string
pub fn schema_to_rust_type(schema: &Schema, schema_name: Option<&str>) -> String {
    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => "String".to_string(),
        openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) => "i64".to_string(),
        openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) => "f64".to_string(),
        openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) => "bool".to_string(),
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(array_type)) => {
            let item_type = array_type
                .items
                .as_ref()
                .and_then(|item_ref| item_ref.as_item())
                .map(|item_schema| schema_to_rust_type(item_schema, None))
                .unwrap_or_else(|| "serde_json::Value".to_string());

            format!("Vec<{}>", item_type)
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(_)) => schema_name
            .map(to_pascal_case)
            .unwrap_or_else(|| "serde_json::Value".to_string()),
        _ => "serde_json::Value".to_string(),
    }
}

/// Convert a string to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    s.split(['-', '_', ' '])
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect()
}

/// Convert a string to snake_case
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_lower = false;

    for ch in s.chars() {
        if ch.is_uppercase() && prev_lower {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap_or(ch));
        prev_lower = ch.is_lowercase() || ch.is_numeric();
    }

    result
}

/// Generate a handler function name from route information
///
/// # Arguments
/// * `route` - The route information
///
/// # Returns
/// Function name in snake_case
pub fn generate_handler_name(route: &RouteInfo) -> String {
    if let Some(ref op_id) = route.operation_id {
        // Use operation ID if available, convert to snake_case
        to_snake_case(op_id)
    } else {
        // Generate from method + path
        let method_lower = route.method.to_lowercase();
        let path_part = route
            .path
            .replace('/', "_")
            .replace(['{', '}'], "")
            .replace('-', "_")
            .trim_matches('_')
            .to_string();

        format!("{}_{}", method_lower, to_snake_case(&path_part))
    }
}

/// Sanitize a name for use in Rust identifiers
///
/// Removes or replaces invalid characters to create a valid Rust identifier
pub fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else if c == '-' || c == ' ' {
                '_'
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_path_parameters() {
        assert_eq!(extract_path_parameters("/users"), Vec::<String>::new());
        assert_eq!(extract_path_parameters("/users/{id}"), vec!["id"]);
        assert_eq!(extract_path_parameters("/users/{id}/posts/{postId}"), vec!["id", "postId"]);
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("user"), "User");
        assert_eq!(to_pascal_case("user_profile"), "UserProfile");
        assert_eq!(to_pascal_case("user-profile"), "UserProfile");
        assert_eq!(to_pascal_case("get_user_by_id"), "GetUserById");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("User"), "user");
        assert_eq!(to_snake_case("UserProfile"), "user_profile");
        assert_eq!(to_snake_case("getUserById"), "get_user_by_id");
        assert_eq!(to_snake_case("GetUserById"), "get_user_by_id");
    }

    #[test]
    fn test_generate_handler_name() {
        let route = RouteInfo {
            method: "GET".to_string(),
            path: "/users/{id}".to_string(),
            operation_id: Some("getUser".to_string()),
            summary: None,
            description: None,
            path_params: vec!["id".to_string()],
            query_params: Vec::new(),
            request_body_schema: None,
            responses: HashMap::new(),
            tags: Vec::new(),
        };

        assert_eq!(generate_handler_name(&route), "get_user");
    }
}
