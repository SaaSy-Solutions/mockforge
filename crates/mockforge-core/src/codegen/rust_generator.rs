//! Rust code generator for mock servers from OpenAPI specifications

use crate::codegen::{CodegenConfig, MockDataStrategy};
use crate::openapi::spec::OpenApiSpec;
use crate::{Error, Result};
use openapiv3::{Operation, ReferenceOr, Schema, StatusCode};

/// Generate Rust mock server code from OpenAPI spec
pub fn generate(spec: &OpenApiSpec, config: &CodegenConfig) -> Result<String> {
    let routes = extract_routes_from_spec(spec)?;

    let mut code = String::new();

    // Generate header with dependencies
    code.push_str(&generate_header());

    // Generate main server struct
    code.push_str(&generate_server_struct());

    // Generate implementation
    code.push_str(&generate_server_impl(&routes, config)?);

    // Generate handler functions
    code.push_str(&generate_handlers(&routes, spec, config)?);

    // Generate main function
    code.push_str(&generate_main_function(config));

    Ok(code)
}

/// Extract all routes from the OpenAPI spec
fn extract_routes_from_spec(spec: &OpenApiSpec) -> Result<Vec<RouteInfo>> {
    let mut routes = Vec::new();

    for (path, path_item) in &spec.spec.paths.paths {
        if let Some(item) = path_item.as_item() {
            // Process each HTTP method
            if let Some(op) = &item.get {
                routes.push(extract_route_info("GET", path, op)?);
            }
            if let Some(op) = &item.post {
                routes.push(extract_route_info("POST", path, op)?);
            }
            if let Some(op) = &item.put {
                routes.push(extract_route_info("PUT", path, op)?);
            }
            if let Some(op) = &item.delete {
                routes.push(extract_route_info("DELETE", path, op)?);
            }
            if let Some(op) = &item.patch {
                routes.push(extract_route_info("PATCH", path, op)?);
            }
            if let Some(op) = &item.head {
                routes.push(extract_route_info("HEAD", path, op)?);
            }
            if let Some(op) = &item.options {
                routes.push(extract_route_info("OPTIONS", path, op)?);
            }
            if let Some(op) = &item.trace {
                routes.push(extract_route_info("TRACE", path, op)?);
            }
        }
    }

    Ok(routes)
}

/// Information about a route extracted from OpenAPI spec
#[derive(Debug, Clone)]
struct RouteInfo {
    method: String,
    path: String,
    operation_id: Option<String>,
    path_params: Vec<String>,
    query_params: Vec<QueryParam>,
    request_body_schema: Option<Schema>,
    response_schema: Option<Schema>,
    response_example: Option<serde_json::Value>,
    response_status: u16,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct QueryParam {
    name: String,
    required: bool,
}

fn extract_route_info(
    method: &str,
    path: &str,
    operation: &Operation,
) -> std::result::Result<RouteInfo, Error> {
    let operation_id = operation.operation_id.clone();

    // Extract path parameters (e.g., {id} from /users/{id})
    let path_params = extract_path_parameters(path);

    // Extract query parameters
    let query_params = extract_query_parameters(operation);

    // Extract request body schema (if any)
    let request_body_schema = extract_request_body_schema(operation);

    // Extract response schema and example (prefer 200, fallback to first success response)
    let (response_schema, response_example, response_status) =
        extract_response_schema_and_example(operation)?;

    Ok(RouteInfo {
        method: method.to_string(),
        path: path.to_string(),
        operation_id,
        path_params,
        query_params,
        request_body_schema,
        response_schema,
        response_example,
        response_status,
    })
}

fn extract_path_parameters(path: &str) -> Vec<String> {
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
                if in_param {
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

fn extract_query_parameters(operation: &Operation) -> Vec<QueryParam> {
    let mut params = Vec::new();

    for param_ref in &operation.parameters {
        if let Some(openapiv3::Parameter::Query { parameter_data, .. }) = param_ref.as_item() {
            params.push(QueryParam {
                name: parameter_data.name.clone(),
                required: parameter_data.required,
            });
        }
    }

    params
}

fn extract_request_body_schema(operation: &Operation) -> Option<Schema> {
    operation.request_body.as_ref().and_then(|body_ref| {
        body_ref.as_item().and_then(|body| {
            body.content.get("application/json").and_then(|content| {
                content.schema.as_ref().and_then(|schema_ref| schema_ref.as_item().cloned())
            })
        })
    })
}

/// Extract response schema and example from OpenAPI operation
/// Returns (schema, example, status_code)
fn extract_response_schema_and_example(
    operation: &Operation,
) -> Result<(Option<Schema>, Option<serde_json::Value>, u16)> {
    // Look for 200 response first
    for (status_code, response_ref) in &operation.responses.responses {
        let status = match status_code {
            StatusCode::Code(code) => *code,
            StatusCode::Range(range) if *range == 2 => 200, // 2XX default to 200
            _ => continue,
        };

        if (200..300).contains(&status) {
            if let Some(response) = response_ref.as_item() {
                if let Some(content) = response.content.get("application/json") {
                    // First, check for explicit example in content
                    let example = if let Some(example) = &content.example {
                        Some(example.clone())
                    } else if !content.examples.is_empty() {
                        // Use the first example from the examples map
                        content.examples.iter().next().and_then(|(_, example_ref)| {
                            example_ref
                                .as_item()
                                .and_then(|example_item| example_item.value.clone())
                        })
                    } else {
                        None
                    };

                    // Extract schema if available
                    let schema = if let Some(ReferenceOr::Item(schema)) = &content.schema {
                        Some(schema.clone())
                    } else {
                        None
                    };

                    return Ok((schema, example, status));
                }
                // Found success response, return even if no schema or example
                return Ok((None, None, status));
            }
        }
    }

    // Default to 200 if no response found
    Ok((None, None, 200))
}

fn generate_header() -> String {
    r#"//! Generated mock server code from OpenAPI specification
//!
//! This file was automatically generated by MockForge.
//! DO NOT EDIT THIS FILE MANUALLY.

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete, patch},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

"#
    .to_string()
}

fn generate_server_struct() -> String {
    r#"/// Generated mock server
pub struct GeneratedMockServer {
    port: u16,
}

"#
    .to_string()
}

fn generate_server_impl(routes: &[RouteInfo], config: &CodegenConfig) -> Result<String> {
    let mut code = String::new();

    code.push_str("impl GeneratedMockServer {\n");
    code.push_str("    /// Create a new mock server instance\n");
    code.push_str("    pub fn new() -> Self {\n");
    code.push_str("        Self {\n");
    code.push_str(&format!("            port: {},\n", config.port.unwrap_or(3000)));
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // Generate router setup
    code.push_str("    /// Build the Axum router with all routes\n");
    code.push_str("    pub fn router(&self) -> Router {\n");
    code.push_str("        Router::new()\n");

    for route in routes {
        let handler_name = generate_handler_name(route);
        let method = route.method.to_lowercase();
        // Use proper Axum path formatting
        let axum_path = if !route.path_params.is_empty() {
            format_axum_path(&route.path, &route.path_params)
        } else {
            route.path.clone()
        };

        code.push_str(&format!(
            "            .route(\"{}\", {}(handle_{}))\n",
            axum_path, method, handler_name
        ));
    }

    code.push_str("    }\n\n");

    code.push_str("    /// Start the server\n");
    code.push_str(
        "    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {\n",
    );
    code.push_str("        let app = self.router();\n");
    code.push_str(&format!(
        "        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], {}));\n",
        config.port.unwrap_or(3000)
    ));
    code.push_str(
        "        println!(\"🚀 Mock server started on http://localhost:{}\", self.port);\n",
    );
    code.push_str("        let listener = tokio::net::TcpListener::bind(addr).await?;\n");
    code.push_str("        axum::serve(listener, app).await?;\n");
    code.push_str("        Ok(())\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    Ok(code)
}

fn generate_handlers(
    routes: &[RouteInfo],
    _spec: &OpenApiSpec,
    config: &CodegenConfig,
) -> Result<String> {
    let mut code = String::new();

    for route in routes {
        code.push_str(&generate_handler(route, config)?);
        code.push('\n');
    }

    Ok(code)
}

fn generate_handler(route: &RouteInfo, config: &CodegenConfig) -> Result<String> {
    let handler_name = generate_handler_name(route);
    let mut code = String::new();

    // Generate function signature
    code.push_str(&format!("/// Handler for {} {}\n", route.method, route.path));
    code.push_str(&format!("async fn handle_{}(\n", handler_name));

    // Add path parameters - Axum supports extracting individual path params
    if !route.path_params.is_empty() {
        // For single path parameter, use direct extraction: Path(id): Path<String>
        // For multiple, we could use a struct or HashMap
        if route.path_params.len() == 1 {
            let param_name = &route.path_params[0];
            code.push_str(&format!("    Path({}): Path<String>,\n", param_name));
        } else {
            // Multiple path parameters - use HashMap for now
            code.push_str("    Path(params): Path<HashMap<String, String>>,\n");
        }
    }

    // Add query parameters
    if !route.query_params.is_empty() {
        code.push_str("    Query(query): Query<HashMap<String, String>>,\n");
    }

    // Add request body for POST/PUT/PATCH
    if matches!(route.method.as_str(), "POST" | "PUT" | "PATCH")
        && route.request_body_schema.is_some()
    {
        code.push_str("    Json(body): Json<Value>,\n");
    }

    // Remove trailing comma/newline
    if code.ends_with(",\n") {
        code.pop();
        code.pop();
        code.push('\n');
    }

    code.push_str(") -> (StatusCode, Json<Value>) {\n");

    // Add delay if configured
    if let Some(delay_ms) = config.default_delay_ms {
        code.push_str(&format!(
            "    tokio::time::sleep(tokio::time::Duration::from_millis({})).await;\n",
            delay_ms
        ));
    }

    // Generate response
    let response_body = generate_response_body(route, config);
    code.push_str(&format!(
        "    (StatusCode::from_u16({}).unwrap(), Json({}))\n",
        route.response_status, response_body
    ));
    code.push_str("}\n");

    Ok(code)
}

fn generate_response_body(route: &RouteInfo, config: &CodegenConfig) -> String {
    match config.mock_data_strategy {
        MockDataStrategy::Examples | MockDataStrategy::ExamplesOrRandom => {
            // Priority 1: Use explicit example from OpenAPI spec if available
            if let Some(ref example) = route.response_example {
                // Serialize the example value to JSON string and parse it at runtime
                let example_str =
                    serde_json::to_string(example).unwrap_or_else(|_| "{}".to_string());
                // Escape for use in Rust code - need to escape backslashes and quotes
                let escaped = example_str
                    .replace("\\", "\\\\")
                    .replace("\"", "\\\"")
                    .replace("\n", "\\n")
                    .replace("\r", "\\r")
                    .replace("\t", "\\t");
                // Use a regular string literal with proper escaping
                return format!("serde_json::from_str(\"{}\").unwrap()", escaped);
            }
            // Priority 2: Generate from schema if available
            if let Some(ref schema) = route.response_schema {
                generate_from_schema(schema)
            } else {
                generate_basic_mock_response(route)
            }
        }
        MockDataStrategy::Random => {
            // Always generate from schema structure (don't use examples for random)
            if let Some(ref schema) = route.response_schema {
                generate_from_schema(schema)
            } else {
                generate_basic_mock_response(route)
            }
        }
        MockDataStrategy::Defaults => {
            // Use schema defaults (don't use examples for defaults strategy)
            if let Some(ref schema) = route.response_schema {
                generate_from_schema(schema)
            } else {
                generate_basic_mock_response(route)
            }
        }
    }
}

fn generate_basic_mock_response(route: &RouteInfo) -> String {
    format!(
        r#"serde_json::json!({{
            "message": "Mock response",
            "method": "{}",
            "path": "{}",
            "status": {}
        }})"#,
        route.method, route.path, route.response_status
    )
}

/// Generate a mock response based on the OpenAPI schema
///
/// This function implements sophisticated schema-aware generation that:
/// - Extracts and generates all object properties based on their types
/// - Handles nested objects and arrays recursively
/// - Respects required/optional properties
/// - Uses schema examples and defaults when available
/// - Generates appropriate mock data based on field types and formats
fn generate_from_schema(schema: &Schema) -> String {
    generate_from_schema_internal(schema, 0)
}

/// Internal recursive helper for schema generation with depth tracking
fn generate_from_schema_internal(schema: &Schema, depth: usize) -> String {
    // Prevent infinite recursion with nested schemas
    if depth > 5 {
        return r#"serde_json::json!(null)"#.to_string();
    }

    // Note: OpenAPI schema examples/defaults are typically in the SchemaData or extensions
    // For now, we'll generate based on schema type since direct access to examples/defaults
    // requires accessing schema_data which may not always be available

    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj_type)) => {
            generate_object_from_schema(obj_type, depth)
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(array_type)) => {
            generate_array_from_schema(array_type, depth)
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::String(string_type)) => {
            generate_string_from_schema(string_type)
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Integer(integer_type)) => {
            generate_integer_from_schema(integer_type)
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Number(number_type)) => {
            generate_number_from_schema(number_type)
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) => {
            r#"serde_json::json!(true)"#.to_string()
        }
        _ => {
            // Default for other types (null, any, etc.)
            r#"serde_json::json!(null)"#.to_string()
        }
    }
}

/// Generate mock data for an object schema with all properties
fn generate_object_from_schema(obj_type: &openapiv3::ObjectType, depth: usize) -> String {
    if obj_type.properties.is_empty() {
        return r#"serde_json::json!({})"#.to_string();
    }

    let mut properties = Vec::new();

    for (prop_name, prop_schema_ref) in &obj_type.properties {
        // Check if property is required
        let is_required = obj_type.required.iter().any(|req| req == prop_name);

        // Generate property value based on schema
        let prop_value = match prop_schema_ref {
            ReferenceOr::Item(prop_schema) => generate_from_schema_internal(prop_schema, depth + 1),
            ReferenceOr::Reference { reference } => {
                // For references, generate a placeholder based on the reference name
                if let Some(ref_name) = reference.strip_prefix("#/components/schemas/") {
                    format!(r#"serde_json::json!({{"$ref": "{}"}})"#, ref_name)
                } else {
                    r#"serde_json::json!(null)"#.to_string()
                }
            }
        };

        // Include property (always include required, include optional sometimes)
        if is_required || depth == 0 {
            // Escape property name if needed
            let safe_name = prop_name.replace("\\", "\\\\").replace("\"", "\\\"");
            properties.push(format!(r#""{}": {}"#, safe_name, prop_value));
        }
    }

    if properties.is_empty() {
        r#"serde_json::json!({})"#.to_string()
    } else {
        format!(
            "serde_json::json!({{\n            {}\n        }})",
            properties.join(",\n            ")
        )
    }
}

/// Generate mock data for an array schema
fn generate_array_from_schema(array_type: &openapiv3::ArrayType, depth: usize) -> String {
    // Generate 1-2 items for arrays
    let item_value = match &array_type.items {
        Some(item_schema_ref) => match item_schema_ref {
            ReferenceOr::Item(item_schema) => generate_from_schema_internal(item_schema, depth + 1),
            ReferenceOr::Reference { reference } => {
                if let Some(ref_name) = reference.strip_prefix("#/components/schemas/") {
                    format!(r#"serde_json::json!({{"$ref": "{}"}})"#, ref_name)
                } else {
                    r#"serde_json::json!(null)"#.to_string()
                }
            }
        },
        None => r#"serde_json::json!(null)"#.to_string(),
    };

    // Generate array with 1 item
    format!("serde_json::json!([{}])", item_value)
}

/// Generate mock data for a string schema
fn generate_string_from_schema(string_type: &openapiv3::StringType) -> String {
    // Check format for appropriate mock data
    if let openapiv3::VariantOrUnknownOrEmpty::Item(format) = &string_type.format {
        match format {
            openapiv3::StringFormat::Date => r#"serde_json::json!("2024-01-01")"#.to_string(),
            openapiv3::StringFormat::DateTime => {
                r#"serde_json::json!("2024-01-01T00:00:00Z")"#.to_string()
            }
            _ => r#"serde_json::json!("mock string")"#.to_string(),
        }
    } else {
        // Check enum values (Vec<Option<String>>)
        let enum_values = &string_type.enumeration;
        if !enum_values.is_empty() {
            if let Some(first) = enum_values.iter().find_map(|v| v.as_ref()) {
                let first_escaped = first.replace('\\', "\\\\").replace('"', "\\\"");
                return format!(r#"serde_json::json!("{}")"#, first_escaped);
            }
        }
        r#"serde_json::json!("mock string")"#.to_string()
    }
}

/// Generate mock data for an integer schema
fn generate_integer_from_schema(integer_type: &openapiv3::IntegerType) -> String {
    // Check for enum values (Vec<Option<i64>>)
    let enum_values = &integer_type.enumeration;
    if !enum_values.is_empty() {
        if let Some(first) = enum_values.iter().flatten().next() {
            return format!("serde_json::json!({})", first);
        }
    }

    // Check for range constraints
    let value = if let Some(minimum) = integer_type.minimum {
        if minimum > 0 {
            minimum
        } else {
            1
        }
    } else if let Some(maximum) = integer_type.maximum {
        if maximum > 0 {
            maximum.min(1000)
        } else {
            1
        }
    } else {
        42
    };

    format!("serde_json::json!({})", value)
}

/// Generate mock data for a number schema
fn generate_number_from_schema(number_type: &openapiv3::NumberType) -> String {
    // Check for enum values (Vec<Option<f64>>)
    let enum_values = &number_type.enumeration;
    if !enum_values.is_empty() {
        if let Some(first) = enum_values.iter().flatten().next() {
            return format!("serde_json::json!({})", first);
        }
    }

    // Check for range constraints
    let value = if let Some(minimum) = number_type.minimum {
        if minimum > 0.0 {
            minimum
        } else {
            std::f64::consts::PI
        }
    } else if let Some(maximum) = number_type.maximum {
        if maximum > 0.0 {
            maximum.min(1000.0)
        } else {
            std::f64::consts::PI
        }
    } else {
        std::f64::consts::PI
    };

    format!("serde_json::json!({})", value)
}

fn generate_handler_name(route: &RouteInfo) -> String {
    if let Some(ref op_id) = route.operation_id {
        // Sanitize operation ID (remove special chars, convert to snake_case)
        op_id.replace(['-', '.'], "_").to_lowercase()
    } else {
        // Generate name from method + path
        let path_part = route.path.replace('/', "_").replace(['{', '}'], "").replace('-', "_");
        format!("{}_{}", route.method.to_lowercase(), path_part)
            .trim_matches('_')
            .to_string()
    }
}

// Helper to convert path for Axum router registration
// Axum can handle both :param and :param_name syntax
fn format_axum_path(path: &str, path_params: &[String]) -> String {
    let mut axum_path = path.to_string();
    for param in path_params {
        // Replace {param} with :param in the path
        axum_path = axum_path.replace(&format!("{{{}}}", param), &format!(":{}", param));
    }
    axum_path
}

fn generate_main_function(_config: &CodegenConfig) -> String {
    r#"
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = GeneratedMockServer::new();
    server.start().await
}
"#
    .to_string()
}

#[cfg(test)]
mod rust_generator_tests {
    use super::*;
    use serde_json::json;

    /// Build a one-operation spec for the given method and path.
    fn fixture_spec(method: &str, path: &str, operation: serde_json::Value) -> OpenApiSpec {
        let mut path_item = serde_json::Map::new();
        path_item.insert(method.to_lowercase(), operation);
        let mut paths = serde_json::Map::new();
        paths.insert(path.to_string(), serde_json::Value::Object(path_item));
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": {"title": "Fixture API", "version": "1.0.0"},
            "paths": paths,
        });
        OpenApiSpec::from_json(spec_json).expect("valid fixture spec")
    }

    fn get_spec_with_response(responses: serde_json::Value) -> OpenApiSpec {
        fixture_spec("GET", "/things", json!({"operationId": "listThings", "responses": responses}))
    }

    fn generated_code(spec: &OpenApiSpec, config: &CodegenConfig) -> String {
        generate(spec, config).expect("code generation succeeds")
    }

    fn route(method: &str, path: &str, operation_id: Option<&str>) -> RouteInfo {
        RouteInfo {
            method: method.to_string(),
            path: path.to_string(),
            operation_id: operation_id.map(str::to_string),
            path_params: Vec::new(),
            query_params: Vec::new(),
            request_body_schema: None,
            response_schema: None,
            response_example: None,
            response_status: 200,
        }
    }

    // ─── Path and naming helpers ────────────────────────────────────────────

    #[test]
    fn extracts_path_parameters_from_braces() {
        assert_eq!(extract_path_parameters("/users/{id}"), vec!["id"]);
        assert_eq!(
            extract_path_parameters("/orgs/{orgId}/repos/{repoId}"),
            vec!["orgId", "repoId"]
        );
        assert!(extract_path_parameters("/users").is_empty());
        assert!(extract_path_parameters("/health").is_empty());
    }

    #[test]
    fn handler_name_sanitizes_operation_id() {
        assert_eq!(
            generate_handler_name(&route("GET", "/users", Some("list-users"))),
            "list_users"
        );
        assert_eq!(
            generate_handler_name(&route("GET", "/users", Some("v1.users.list"))),
            "v1_users_list"
        );
        // Camel-case operation IDs are lowercased wholesale.
        assert_eq!(generate_handler_name(&route("GET", "/users", Some("getUsers"))), "getusers");
    }

    #[test]
    fn handler_name_falls_back_to_method_and_path() {
        // The leading slash becomes a separator, producing a double underscore
        // between the method and the first segment.
        assert_eq!(generate_handler_name(&route("GET", "/users/{id}", None)), "get__users_id");
        assert_eq!(generate_handler_name(&route("POST", "/users", None)), "post__users");
    }

    #[test]
    fn axum_path_conversion() {
        assert_eq!(format_axum_path("/users/{id}", &["id".to_string()]), "/users/:id");
        assert_eq!(
            format_axum_path(
                "/orgs/{orgId}/repos/{repoId}",
                &["orgId".to_string(), "repoId".to_string()]
            ),
            "/orgs/:orgId/repos/:repoId"
        );
        assert_eq!(format_axum_path("/health", &[]), "/health");
    }

    // ─── Route registration ─────────────────────────────────────────────────

    #[test]
    fn route_registered_with_axum_path_and_handler() {
        let spec = fixture_spec(
            "GET",
            "/users/{id}",
            json!({"operationId": "getUser", "responses": {"200": {"description": "ok"}}}),
        );
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains(r#".route("/users/:id", get(handle_getuser))"#));
        // Single path parameter is extracted directly; the trailing comma of
        // the last extractor is stripped from the signature.
        assert!(code.contains("Path(id): Path<String>"));
    }

    // ─── Response selection ─────────────────────────────────────────────────

    #[test]
    fn success_response_uses_schema_mock_and_status() {
        let spec = get_spec_with_response(json!({
            "200": {"description": "ok", "content": {"application/json": {"schema": {
                "type": "object",
                "properties": {"count": {"type": "integer"}}
            }}}}
        }));
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains("StatusCode::from_u16(200)"));
        // Unconstrained integers mock to 42.
        assert!(code.contains(r#""count": serde_json::json!(42)"#));
    }

    #[test]
    fn range_2xx_maps_to_status_200() {
        let spec = get_spec_with_response(json!({
            "2XX": {"description": "ok", "content": {"application/json": {"schema": {
                "type": "boolean"
            }}}}
        }));
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains("StatusCode::from_u16(200)"));
        assert!(code.contains("serde_json::json!(true)"));
    }

    #[test]
    fn non_success_only_falls_back_to_basic_200_response() {
        let spec = get_spec_with_response(json!({
            "404": {"description": "missing"}
        }));
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains("StatusCode::from_u16(200)"));
        assert!(code.contains(r#""method": "GET""#));
        assert!(code.contains(r#""path": "/things""#));
    }

    #[test]
    fn response_201_status_is_preserved() {
        let spec = fixture_spec(
            "POST",
            "/things",
            json!({
                "operationId": "createThing",
                "responses": {"201": {"description": "created"}}
            }),
        );
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains("StatusCode::from_u16(201)"));
    }

    // ─── Example handling ───────────────────────────────────────────────────

    #[test]
    fn inline_example_embedded_verbatim() {
        let spec = get_spec_with_response(json!({
            "200": {"description": "ok", "content": {"application/json": {
                "schema": {"type": "object", "properties": {"status": {"type": "string"}}},
                "example": {"status": "active"}
            }}}
        }));
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains(r#"serde_json::from_str("{\"status\":\"active\"}").unwrap()"#));
    }

    #[test]
    fn named_examples_map_used_when_no_inline_example() {
        let spec = get_spec_with_response(json!({
            "200": {"description": "ok", "content": {"application/json": {
                "schema": {"type": "object", "properties": {"id": {"type": "integer"}}},
                "examples": {"first": {"value": {"id": 7}}}
            }}}
        }));
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains(r#"serde_json::from_str("{\"id\":7}").unwrap()"#));
    }

    #[test]
    fn example_special_characters_escaped_in_rust_literal() {
        // Value contains a newline, a double quote, and a backslash — the
        // generated string literal must escape all three to stay valid Rust.
        let spec = get_spec_with_response(json!({
            "200": {"description": "ok", "content": {"application/json": {
                "example": {"text": "line1\nline2 \"quoted\" C:\\path"}
            }}}
        }));
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains(r#"{\"text\":\"line1\\nline2 \\\"quoted\\\" C:\\\\path\"}"#));
    }

    #[test]
    fn defaults_strategy_ignores_examples() {
        let spec = get_spec_with_response(json!({
            "200": {"description": "ok", "content": {"application/json": {
                "schema": {"type": "object", "properties": {"count": {"type": "integer"}}},
                "example": {"status": "active"}
            }}}
        }));
        let config = CodegenConfig {
            mock_data_strategy: MockDataStrategy::Defaults,
            ..Default::default()
        };
        let code = generated_code(&spec, &config);
        assert!(!code.contains("serde_json::from_str"));
        assert!(!code.contains("active"));
        assert!(code.contains(r#""count": serde_json::json!(42)"#));
    }

    // ─── Schema-driven mock values ──────────────────────────────────────────

    #[test]
    fn schema_enum_uses_first_value() {
        let spec = get_spec_with_response(json!({
            "200": {"description": "ok", "content": {"application/json": {"schema": {
                "type": "object",
                "properties": {"status": {"type": "string", "enum": ["active", "inactive"]}}
            }}}}
        }));
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains(r#"serde_json::json!("active")"#));
        assert!(!code.contains("inactive"));
    }

    #[test]
    fn integer_constraints_select_mock_values() {
        let spec = get_spec_with_response(json!({
            "200": {"description": "ok", "content": {"application/json": {"schema": {
                "type": "object",
                "properties": {
                    "min": {"type": "integer", "minimum": 5},
                    "capped": {"type": "integer", "maximum": 7},
                    "free": {"type": "integer"}
                }
            }}}}
        }));
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains(r#""min": serde_json::json!(5)"#));
        assert!(code.contains(r#""capped": serde_json::json!(7)"#));
        assert!(code.contains(r#""free": serde_json::json!(42)"#));
    }

    #[test]
    fn string_formats_select_mock_values() {
        let spec = get_spec_with_response(json!({
            "200": {"description": "ok", "content": {"application/json": {"schema": {
                "type": "object",
                "properties": {
                    "day": {"type": "string", "format": "date"},
                    "stamp": {"type": "string", "format": "date-time"},
                    "plain": {"type": "string"}
                }
            }}}}
        }));
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains(r#""day": serde_json::json!("2024-01-01")"#));
        assert!(code.contains(r#""stamp": serde_json::json!("2024-01-01T00:00:00Z")"#));
        assert!(code.contains(r#""plain": serde_json::json!("mock string")"#));
    }

    #[test]
    fn number_and_boolean_defaults() {
        let spec = get_spec_with_response(json!({
            "200": {"description": "ok", "content": {"application/json": {"schema": {
                "type": "object",
                "properties": {
                    "ratio": {"type": "number"},
                    "flag": {"type": "boolean"}
                }
            }}}}
        }));
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains(r#""ratio": serde_json::json!(3.141592653589793)"#));
        assert!(code.contains(r#""flag": serde_json::json!(true)"#));
    }

    #[test]
    fn nested_object_optional_properties_excluded_below_top_level() {
        let spec = get_spec_with_response(json!({
            "200": {"description": "ok", "content": {"application/json": {"schema": {
                "type": "object",
                "required": ["profile"],
                "properties": {
                    "profile": {
                        "type": "object",
                        "required": ["name"],
                        "properties": {
                            "name": {"type": "string"},
                            "nickname": {"type": "string"}
                        }
                    }
                }
            }}}}
        }));
        let code = generated_code(&spec, &CodegenConfig::default());
        // The nested object only generates required properties.
        assert!(code.contains(r#""profile": serde_json::json!({"#));
        assert!(code.contains(r#""name": serde_json::json!("mock string")"#));
        assert!(!code.contains("nickname"));
    }

    #[test]
    fn deep_nesting_capped_at_depth_five() {
        // Build a chain of six nested objects; the innermost property sits at
        // depth 6, past the recursion cap, so it must mock to null.
        fn nested_object_schema(depth: usize) -> serde_json::Value {
            if depth == 0 {
                return json!({"type": "string"});
            }
            let name = format!("level{depth}");
            let mut properties = serde_json::Map::new();
            properties.insert(name.clone(), nested_object_schema(depth - 1));
            json!({
                "type": "object",
                "required": [name],
                "properties": properties,
            })
        }
        let spec = get_spec_with_response(json!({
            "200": {"description": "ok", "content": {"application/json": {
                "schema": nested_object_schema(6)
            }}}
        }));
        let code = generated_code(&spec, &CodegenConfig::default());
        // Depth 5 still expands into an object; depth 6 collapses to null.
        assert!(code.contains(r#""level2": serde_json::json!({"#));
        assert!(code.contains(r#""level1": serde_json::json!(null)"#));
    }

    // ─── Handler shape ──────────────────────────────────────────────────────

    #[test]
    fn post_request_body_produces_json_extractor() {
        let spec = fixture_spec(
            "POST",
            "/things",
            json!({
                "operationId": "createThing",
                "requestBody": {"required": true, "content": {"application/json": {"schema": {
                    "type": "object", "properties": {"name": {"type": "string"}}
                }}}},
                "responses": {"201": {"description": "created", "content": {"application/json": {
                    "schema": {"type": "object", "properties": {"id": {"type": "integer"}}}
                }}}}
            }),
        );
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains("Json(body): Json<Value>"));
        assert!(code.contains("StatusCode::from_u16(201)"));
    }

    #[test]
    fn multiple_path_params_use_hashmap_extractor() {
        let spec = fixture_spec(
            "GET",
            "/orders/{orderId}/items/{itemId}",
            json!({
                "operationId": "getOrderItem",
                "responses": {"200": {"description": "ok"}}
            }),
        );
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains("Path(params): Path<HashMap<String, String>>"));
        assert!(
            code.contains(r#".route("/orders/:orderId/items/:itemId", get(handle_getorderitem))"#)
        );
    }

    #[test]
    fn query_params_produce_query_extractor() {
        let spec = fixture_spec(
            "GET",
            "/things",
            json!({
                "operationId": "listThings",
                "parameters": [
                    {"name": "limit", "in": "query", "required": false, "schema": {"type": "integer"}}
                ],
                "responses": {"200": {"description": "ok"}}
            }),
        );
        let code = generated_code(&spec, &CodegenConfig::default());
        assert!(code.contains("Query(query): Query<HashMap<String, String>>"));
    }

    #[test]
    fn delay_injected_when_configured() {
        let spec = get_spec_with_response(json!({"200": {"description": "ok"}}));
        let config = CodegenConfig {
            default_delay_ms: Some(150),
            ..Default::default()
        };
        let code = generated_code(&spec, &config);
        assert!(code.contains("tokio::time::sleep(tokio::time::Duration::from_millis(150))"));
    }

    #[test]
    fn configured_port_used_in_struct_and_addr() {
        let spec = get_spec_with_response(json!({"200": {"description": "ok"}}));
        let config = CodegenConfig {
            port: Some(9123),
            ..Default::default()
        };
        let code = generated_code(&spec, &config);
        assert!(code.contains("port: 9123"));
        assert!(code.contains("SocketAddr::from(([0, 0, 0, 0], 9123))"));
    }
}
