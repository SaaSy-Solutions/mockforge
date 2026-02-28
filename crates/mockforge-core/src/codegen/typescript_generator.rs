//! TypeScript/JavaScript code generator for mock servers from OpenAPI specifications

use crate::codegen::{CodegenConfig, MockDataStrategy};
use crate::openapi::spec::OpenApiSpec;
use crate::{Error, Result};
use openapiv3::{Operation, ReferenceOr, Schema, StatusCode};

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

/// Generate TypeScript/JavaScript mock server code from OpenAPI spec
pub fn generate(spec: &OpenApiSpec, config: &CodegenConfig) -> Result<String> {
    let routes = extract_routes_from_spec(spec)?;

    let mut code = String::new();

    // Generate header with imports and dependencies
    code.push_str(&generate_header());

    // Generate TypeScript types from OpenAPI schemas
    code.push_str(&generate_types(spec, &routes)?);

    // Generate main server class
    code.push_str(&generate_server_class(&routes, config)?);

    // Generate route handlers
    code.push_str(&generate_handlers(&routes, config)?);

    // Generate helper functions
    code.push_str(&generate_helpers());

    // Generate main function/export
    code.push_str(&generate_main_function(config));

    Ok(code)
}

/// Extract all routes from the OpenAPI spec (reusing logic from Rust generator)
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
        }
    }

    Ok(routes)
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
    r#"// Generated mock server code from OpenAPI specification
// This file was automatically generated by MockForge.
// DO NOT EDIT THIS FILE MANUALLY.

import express, { Request, Response, Router } from 'express';

"#
    .to_string()
}

fn generate_types(spec: &OpenApiSpec, routes: &[RouteInfo]) -> Result<String> {
    let mut code = String::new();
    code.push_str("// Type definitions generated from OpenAPI schemas\n");

    // Collect all unique schemas used in routes
    let mut schema_names = std::collections::HashSet::new();
    let mut schema_refs = std::collections::HashMap::new();

    // Extract schema references from request bodies and responses
    for route in routes {
        if let Some(ref schema) = route.request_body_schema {
            extract_schema_refs(schema, &mut schema_names, &mut schema_refs);
        }
        if let Some(ref schema) = route.response_schema {
            extract_schema_refs(schema, &mut schema_names, &mut schema_refs);
        }
    }

    // Generate interfaces from all schemas in components
    if let Some(schemas) = spec.schemas() {
        for (schema_name, schema_ref) in schemas {
            if let ReferenceOr::Item(schema) = schema_ref {
                let interface_name = sanitize_type_name(schema_name);
                code.push_str(&generate_interface_from_schema(&interface_name, schema, spec)?);
                code.push('\n');
            }
        }
    }

    // Generate basic response type
    code.push_str("interface MockResponse {\n");
    code.push_str("    status: number;\n");
    code.push_str("    body: any;\n");
    code.push_str("}\n\n");

    Ok(code)
}

/// Extract schema references from a schema (handle $ref references)
fn extract_schema_refs(
    schema: &Schema,
    schema_names: &mut std::collections::HashSet<String>,
    _schema_refs: &mut std::collections::HashMap<String, String>,
) {
    // Check nested schemas in object properties
    if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj_type)) = &schema.schema_kind {
        // properties is directly an IndexMap, not Option<IndexMap>
        for (_prop_name, prop_schema_ref) in &obj_type.properties {
            if let ReferenceOr::Reference { reference } = prop_schema_ref {
                if let Some(ref_name) = reference.strip_prefix("#/components/schemas/") {
                    schema_names.insert(ref_name.to_string());
                }
            }
            // Recursively check nested schemas (properties are Box<Schema>)
            if let ReferenceOr::Item(prop_schema) = prop_schema_ref {
                extract_schema_refs(prop_schema, schema_names, _schema_refs);
            }
        }
    }
}

/// Generate a TypeScript interface from an OpenAPI schema
fn generate_interface_from_schema(
    name: &str,
    schema: &Schema,
    spec: &OpenApiSpec,
) -> Result<String> {
    let mut code = String::new();

    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj_type)) => {
            code.push_str(&format!("export interface {} {{\n", name));

            // properties is directly an IndexMap, not Option<IndexMap>
            for (prop_name, prop_schema_ref) in &obj_type.properties {
                let prop_type = match prop_schema_ref {
                    ReferenceOr::Item(prop_schema) => {
                        // prop_schema is Box<Schema>, need to dereference
                        schema_to_typescript_type(prop_schema, spec)?
                    }
                    ReferenceOr::Reference { reference } => {
                        // Extract type name from reference
                        if let Some(type_name) = reference.strip_prefix("#/components/schemas/") {
                            sanitize_type_name(type_name)
                        } else {
                            "any".to_string()
                        }
                    }
                };

                // Check if property is required (required is Vec<String>)
                let is_optional = !obj_type.required.iter().any(|req| req == prop_name);

                let prop_name_safe = sanitize_property_name(prop_name);
                if is_optional {
                    code.push_str(&format!("    {}?: {};\n", prop_name_safe, prop_type));
                } else {
                    code.push_str(&format!("    {}: {};\n", prop_name_safe, prop_type));
                }
            }

            code.push_str("}\n");
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(array_type)) => {
            let item_type = match &array_type.items {
                Some(item_schema_ref) => match item_schema_ref {
                    ReferenceOr::Item(item_schema) => schema_to_typescript_type(item_schema, spec)?,
                    ReferenceOr::Reference { reference } => {
                        if let Some(type_name) = reference.strip_prefix("#/components/schemas/") {
                            sanitize_type_name(type_name)
                        } else {
                            "any".to_string()
                        }
                    }
                },
                None => "any".to_string(),
            };
            code.push_str(&format!("export type {} = {}[];\n", name, item_type));
        }
        _ => {
            let ts_type = schema_to_typescript_type(schema, spec)?;
            code.push_str(&format!("export type {} = {};\n", name, ts_type));
        }
    }

    Ok(code)
}

/// Convert an OpenAPI schema to a TypeScript type string
#[allow(clippy::only_used_in_recursion)]
fn schema_to_typescript_type(schema: &Schema, spec: &OpenApiSpec) -> Result<String> {
    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::String(string_type)) => {
            // Check format on StringType (format is VariantOrUnknownOrEmpty)
            if let openapiv3::VariantOrUnknownOrEmpty::Item(format) = &string_type.format {
                match format {
                    openapiv3::StringFormat::Date => Ok("string".to_string()),
                    openapiv3::StringFormat::DateTime => Ok("string".to_string()),
                    _ => Ok("string".to_string()),
                }
            } else {
                Ok("string".to_string())
            }
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) => Ok("number".to_string()),
        openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) => Ok("number".to_string()),
        openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) => Ok("boolean".to_string()),
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(array_type)) => {
            let item_type = match &array_type.items {
                Some(item_schema_ref) => match item_schema_ref {
                    ReferenceOr::Item(item_schema) => schema_to_typescript_type(item_schema, spec)?,
                    ReferenceOr::Reference { reference } => {
                        if let Some(type_name) = reference.strip_prefix("#/components/schemas/") {
                            sanitize_type_name(type_name)
                        } else {
                            "any".to_string()
                        }
                    }
                },
                None => "any".to_string(),
            };
            Ok(format!("{}[]", item_type))
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(_obj_type)) => {
            // For inline objects, generate a simple object type
            Ok("Record<string, any>".to_string())
        }
        _ => Ok("any".to_string()),
    }
}

/// Sanitize a schema name to be a valid TypeScript type name
fn sanitize_type_name(name: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for ch in name.chars() {
        match ch {
            '-' | '_' | ' ' => capitalize_next = true,
            ch if ch.is_alphanumeric() => {
                if capitalize_next {
                    result.push(ch.to_uppercase().next().unwrap_or(ch));
                    capitalize_next = false;
                } else {
                    result.push(ch);
                }
            }
            _ => {}
        }
    }

    if result.is_empty() {
        "Unknown".to_string()
    } else {
        // Ensure first character is uppercase
        let mut chars = result.chars();
        if let Some(first) = chars.next() {
            format!("{}{}", first.to_uppercase(), chars.as_str())
        } else {
            "Unknown".to_string()
        }
    }
}

/// Sanitize a property name to be a valid TypeScript property name
fn sanitize_property_name(name: &str) -> String {
    // TypeScript property names can be identifiers or quoted strings
    if name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') {
        name.to_string()
    } else {
        // For invalid identifiers, quote them
        format!("\"{}\"", name.replace('"', "\\\""))
    }
}

fn generate_server_class(routes: &[RouteInfo], config: &CodegenConfig) -> Result<String> {
    let mut code = String::new();

    code.push_str("/// Generated mock server class\n");
    code.push_str("export class GeneratedMockServer {\n");
    code.push_str("    private app: express.Application;\n");
    code.push_str(&format!("    private port: number = {};\n", config.port.unwrap_or(3000)));

    code.push_str("\n    constructor(port?: number) {\n");
    code.push_str("        this.app = express();\n");
    code.push_str("        this.app.use(express.json());\n");

    if config.enable_cors {
        code.push_str("        // Enable CORS\n");
        code.push_str("        this.app.use((req, res, next) => {\n");
        code.push_str("            res.header('Access-Control-Allow-Origin', '*');\n");
        code.push_str("            res.header('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, PATCH, OPTIONS');\n");
        code.push_str("            res.header('Access-Control-Allow-Headers', 'Content-Type, Authorization');\n");
        code.push_str("            if (req.method === 'OPTIONS') {\n");
        code.push_str("                res.sendStatus(200);\n");
        code.push_str("            } else {\n");
        code.push_str("                next();\n");
        code.push_str("            }\n");
        code.push_str("        });\n");
    }

    code.push_str("        if (port) {\n");
    code.push_str("            this.port = port;\n");
    code.push_str("        }\n");
    code.push_str("        this.setupRoutes();\n");
    code.push_str("    }\n\n");

    // Generate setupRoutes method
    code.push_str("    private setupRoutes(): void {\n");
    for route in routes {
        let handler_name = generate_handler_name(route);
        let method = route.method.to_lowercase();
        let express_path = convert_openapi_path_to_express(&route.path, &route.path_params);

        code.push_str(&format!(
            "        this.app.{}('{}', this.handle_{}.bind(this));\n",
            method, express_path, handler_name
        ));
    }
    code.push_str("    }\n\n");

    // Generate start method
    code.push_str("    public async start(): Promise<void> {\n");
    code.push_str("        return new Promise((resolve) => {\n");
    code.push_str("            this.app.listen(this.port, () => {\n");
    code.push_str(
        "                console.log(`🚀 Mock server started on http://localhost:${this.port}`);\n",
    );
    code.push_str("                resolve();\n");
    code.push_str("            });\n");
    code.push_str("        });\n");
    code.push_str("    }\n\n");

    // Generate stop method
    code.push_str("    public stop(): void {\n");
    code.push_str("        // Express doesn't have a built-in stop method\n");
    code.push_str("        // Server can be stopped by closing the Node.js process\n");
    code.push_str("        console.log('Server stopped');\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    Ok(code)
}

fn generate_handlers(routes: &[RouteInfo], config: &CodegenConfig) -> Result<String> {
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

    code.push_str(&format!("    /// Handler for {} {}\n", route.method, route.path));
    code.push_str(&format!(
        "    private async handle_{}(req: Request, res: Response): Promise<void> {{\n",
        handler_name
    ));

    // Add delay if configured
    if let Some(delay_ms) = config.default_delay_ms {
        code.push_str(&format!(
            "        await new Promise(resolve => setTimeout(resolve, {}));\n",
            delay_ms
        ));
    }

    // Extract path parameters
    if !route.path_params.is_empty() {
        for param in &route.path_params {
            code.push_str(&format!("        const {} = req.params['{}'];\n", param, param));
        }
    }

    // Extract query parameters
    if !route.query_params.is_empty() {
        code.push_str("        const query = req.query;\n");
    }

    // Extract request body
    if matches!(route.method.as_str(), "POST" | "PUT" | "PATCH")
        && route.request_body_schema.is_some()
    {
        code.push_str("        const body = req.body;\n");
    }

    // Generate response
    let response_body = generate_response_body(route, config);
    code.push_str(&format!(
        "        res.status({}).json({});\n",
        route.response_status, response_body
    ));
    code.push_str("    }\n");

    Ok(code)
}

fn generate_response_body(route: &RouteInfo, config: &CodegenConfig) -> String {
    match config.mock_data_strategy {
        MockDataStrategy::Examples | MockDataStrategy::ExamplesOrRandom => {
            // Priority 1: Use explicit example from OpenAPI spec if available
            if let Some(ref example) = route.response_example {
                // Serialize the example value to JSON string
                let example_str =
                    serde_json::to_string(example).unwrap_or_else(|_| "{}".to_string());
                // Escape backticks and ${} for template literals in TypeScript
                let escaped =
                    example_str.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");
                // Use template literal to embed JSON directly
                return format!("JSON.parse(`{}`)", escaped);
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
        r#"{{"message": "Mock response", "method": "{}", "path": "{}", "status": {}}}"#,
        route.method, route.path, route.response_status
    )
}

/// Generate a mock response based on the OpenAPI schema
fn generate_from_schema(schema: &Schema) -> String {
    // Basic schema-based generation
    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(_props)) => {
            // For object schemas, generate a basic object
            r#"{"id": 1, "created_at": "2024-01-01T00:00:00Z"}"#.to_string()
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(_items)) => {
            // For array schemas, generate an empty array (or array with one item)
            r#"[]"#.to_string()
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => r#""mock string""#.to_string(),
        openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) => r#"42"#.to_string(),
        openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) => r#"3.14"#.to_string(),
        openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) => r#"true"#.to_string(),
        _ => {
            // Default for other types
            r#"{"value": "mock"}"#.to_string()
        }
    }
}

fn generate_handler_name(route: &RouteInfo) -> String {
    if let Some(ref op_id) = route.operation_id {
        // Sanitize operation ID (remove special chars, convert to camelCase)
        let mut parts = op_id.split(&['-', '_', '.'][..]);
        let mut result = String::new();
        if let Some(first) = parts.next() {
            result.push_str(&first.to_lowercase());
        }
        for part in parts {
            if !part.is_empty() {
                let mut chars = part.chars();
                if let Some(first) = chars.next() {
                    result.push_str(&first.to_uppercase().collect::<String>());
                    result.push_str(&chars.as_str().to_lowercase());
                }
            }
        }
        result
    } else {
        // Generate name from method + path
        let path_part = route.path.replace('/', "_").replace(['{', '}'], "").replace('-', "_");
        format!("{}_{}", route.method.to_lowercase(), path_part)
            .trim_matches('_')
            .to_string()
    }
}

fn convert_openapi_path_to_express(path: &str, path_params: &[String]) -> String {
    // Express.js uses :param syntax for path parameters
    // Convert /users/{id} to /users/:id
    let mut express_path = path.to_string();
    for param in path_params {
        express_path = express_path.replace(&format!("{{{}}}", param), &format!(":{}", param));
    }
    express_path
}

fn generate_helpers() -> String {
    r#"// Helper functions for mock data generation
function delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
}

"#
    .to_string()
}

fn generate_main_function(config: &CodegenConfig) -> String {
    format!(
        r#"
// Main execution
if (require.main === module) {{
    const server = new GeneratedMockServer({});
    server.start().then(() => {{
        console.log('Mock server is running');
    }}).catch((err) => {{
        console.error('Failed to start server:', err);
        process.exit(1);
    }});
}}
"#,
        config.port.unwrap_or(3000)
    )
}
