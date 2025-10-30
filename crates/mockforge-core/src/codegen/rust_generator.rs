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
    response_status: u16,
}

#[derive(Debug, Clone)]
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

    // Extract response schema (prefer 200, fallback to first success response)
    let (response_schema, response_status) = extract_response_schema(operation)?;

    Ok(RouteInfo {
        method: method.to_string(),
        path: path.to_string(),
        operation_id,
        path_params,
        query_params,
        request_body_schema,
        response_schema,
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
        if let Some(param) = param_ref.as_item() {
            if let openapiv3::Parameter::Query { parameter_data, .. } = param {
                params.push(QueryParam {
                    name: parameter_data.name.clone(),
                    required: parameter_data.required,
                });
            }
        }
    }

    params
}

fn extract_request_body_schema(operation: &Operation) -> Option<Schema> {
    operation.request_body.as_ref().and_then(|body_ref| {
        body_ref.as_item().and_then(|body| {
            body.content.get("application/json").and_then(|content| {
                content.schema.as_ref().and_then(|schema_ref| {
                    schema_ref.as_item().cloned()
                })
            })
        })
    })
}

fn extract_response_schema(operation: &Operation) -> Result<(Option<Schema>, u16)> {
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
                    if let Some(schema_ref) = &content.schema {
                        if let ReferenceOr::Item(schema) = schema_ref {
                            return Ok((Some(schema.clone()), status));
                        }
                    }
                }
                // Found success response, return even if no schema
                return Ok((None, status));
            }
        }
    }

    // Default to 200 if no response found
    Ok((None, 200))
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

"#.to_string()
}

fn generate_server_struct() -> String {
    r#"/// Generated mock server
pub struct GeneratedMockServer {
    port: u16,
}

"#.to_string()
}

fn generate_server_impl(routes: &[RouteInfo], config: &CodegenConfig) -> Result<String> {
    let mut code = String::new();

    code.push_str("impl GeneratedMockServer {\n");
    code.push_str("    /// Create a new mock server instance\n");
    code.push_str(&format!("    pub fn new() -> Self {{\n"));
    code.push_str(&format!("        Self {{\n"));
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

        code.push_str(&format!("            .route(\"{}\", {}(handle_{}))\n", axum_path, method, handler_name));
    }

    code.push_str("    }\n\n");

    code.push_str("    /// Start the server\n");
    code.push_str("    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {\n");
    code.push_str("        let app = self.router();\n");
    code.push_str(&format!("        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], {}));\n", config.port.unwrap_or(3000)));
    code.push_str("        println!(\"🚀 Mock server started on http://localhost:{}\", self.port);\n");
    code.push_str("        let listener = tokio::net::TcpListener::bind(addr).await?;\n");
    code.push_str("        axum::serve(listener, app).await?;\n");
    code.push_str("        Ok(())\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    Ok(code)
}

fn generate_handlers(routes: &[RouteInfo], _spec: &OpenApiSpec, config: &CodegenConfig) -> Result<String> {
    let mut code = String::new();

    for route in routes {
        code.push_str(&generate_handler(route, config)?);
        code.push_str("\n");
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
            code.push_str(&format!("    Path(params): Path<HashMap<String, String>>,\n"));
        }
    }

    // Add query parameters
    if !route.query_params.is_empty() {
        code.push_str(&format!("    Query(query): Query<HashMap<String, String>>,\n"));
    }

    // Add request body for POST/PUT/PATCH
    if matches!(route.method.as_str(), "POST" | "PUT" | "PATCH") && route.request_body_schema.is_some() {
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
        code.push_str(&format!("    tokio::time::sleep(tokio::time::Duration::from_millis({})).await;\n", delay_ms));
    }

    // Generate response
    let response_body = generate_response_body(route, config);
    code.push_str(&format!("    (StatusCode::from_u16({}).unwrap(), Json({}))\n", route.response_status, response_body));
    code.push_str("}\n");

    Ok(code)
}

fn generate_response_body(route: &RouteInfo, config: &CodegenConfig) -> String {
    match config.mock_data_strategy {
        MockDataStrategy::Examples | MockDataStrategy::ExamplesOrRandom => {
            // Try to generate from schema if available
            if let Some(ref schema) = route.response_schema {
                generate_from_schema(schema)
            } else {
                generate_basic_mock_response(route)
            }
        }
        MockDataStrategy::Random => {
            // Always generate from schema structure
            if let Some(ref schema) = route.response_schema {
                generate_from_schema(schema)
            } else {
                generate_basic_mock_response(route)
            }
        }
        MockDataStrategy::Defaults => {
            // Use schema defaults
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
fn generate_from_schema(schema: &openapiv3::Schema) -> String {
    // Basic schema-based generation
    // For now, create a simple structure based on schema type
    // TODO: Implement more sophisticated schema-aware generation
    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(_props)) => {
            // For object schemas, generate a basic object
            r#"serde_json::json!({
                "id": 1,
                "created_at": "2024-01-01T00:00:00Z"
            })"#.to_string()
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(_items)) => {
            // For array schemas, generate an empty array (or array with one item)
            r#"serde_json::json!([])"#.to_string()
        }
        _ => {
            // Default for other types
            r#"serde_json::json!({"value": "mock"})"#.to_string()
        }
    }
}

fn generate_handler_name(route: &RouteInfo) -> String {
    if let Some(ref op_id) = route.operation_id {
        // Sanitize operation ID (remove special chars, convert to snake_case)
        op_id
            .replace('-', "_")
            .replace('.', "_")
            .to_lowercase()
    } else {
        // Generate name from method + path
        let path_part = route.path
            .replace('/', "_")
            .replace('{', "")
            .replace('}', "")
            .replace('-', "_");
        format!("{}_{}", route.method.to_lowercase(), path_part)
            .trim_matches('_')
            .to_string()
    }
}

fn convert_openapi_path_to_axum(path: &str) -> String {
    // Convert OpenAPI path like /users/{id} to Axum path /users/:id
    // Axum uses :param syntax for path parameters
    // Note: Axum also supports wildcards *param, but :param is more common
    path.replace('{', ":").replace('}', "")
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
    format!(
        r#"
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {{
    let server = GeneratedMockServer::new();
    server.start().await
}}
"#
    )
}
