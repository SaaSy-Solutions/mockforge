//! Swagger 2.0 to OpenAPI 3.0 conversion
//!
//! This module provides functionality to convert Swagger 2.0 specifications
//! to OpenAPI 3.0 format, enabling tools that only support OpenAPI 3.0 to
//! work with legacy Swagger 2.0 specs.

use serde_json::{json, Map, Value};

/// Convert a Swagger 2.0 specification to OpenAPI 3.0 format
///
/// This performs the following conversions:
/// - `swagger: "2.0"` → `openapi: "3.0.3"`
/// - `host` + `basePath` + `schemes` → `servers`
/// - `consumes`/`produces` → per-operation `requestBody`/`responses` content types
/// - Parameter `type`/`format` → `schema` object
/// - `definitions` → `components.schemas`
/// - `securityDefinitions` → `components.securitySchemes`
pub fn convert_swagger_to_openapi3(swagger: &Value) -> Result<Value, String> {
    // Verify this is a Swagger 2.0 spec
    if swagger.get("swagger").and_then(|v| v.as_str()) != Some("2.0") {
        return Err("Not a Swagger 2.0 specification".to_string());
    }

    let mut openapi = Map::new();

    // Set OpenAPI version
    openapi.insert("openapi".to_string(), json!("3.0.3"));

    // Copy info section (mostly compatible)
    if let Some(info) = swagger.get("info") {
        openapi.insert("info".to_string(), info.clone());
    }

    // Convert servers from host/basePath/schemes
    let servers = convert_servers(swagger);
    if !servers.is_empty() {
        openapi.insert("servers".to_string(), json!(servers));
    }

    // Copy tags
    if let Some(tags) = swagger.get("tags") {
        openapi.insert("tags".to_string(), tags.clone());
    }

    // Convert paths
    if let Some(paths) = swagger.get("paths") {
        let global_consumes =
            swagger.get("consumes").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let global_produces =
            swagger.get("produces").and_then(|v| v.as_array()).cloned().unwrap_or_default();

        let converted_paths = convert_paths(paths, &global_consumes, &global_produces);
        openapi.insert("paths".to_string(), converted_paths);
    }

    // Build components section
    let mut components = Map::new();

    // Convert definitions to components/schemas
    if let Some(definitions) = swagger.get("definitions") {
        components.insert("schemas".to_string(), definitions.clone());
    }

    // Convert securityDefinitions to components/securitySchemes
    if let Some(security_defs) = swagger.get("securityDefinitions") {
        let converted = convert_security_definitions(security_defs);
        components.insert("securitySchemes".to_string(), converted);
    }

    if !components.is_empty() {
        openapi.insert("components".to_string(), json!(components));
    }

    // Copy global security
    if let Some(security) = swagger.get("security") {
        openapi.insert("security".to_string(), security.clone());
    }

    // Copy externalDocs
    if let Some(external_docs) = swagger.get("externalDocs") {
        openapi.insert("externalDocs".to_string(), external_docs.clone());
    }

    Ok(Value::Object(openapi))
}

/// Convert Swagger 2.0 host/basePath/schemes to OpenAPI 3.0 servers
fn convert_servers(swagger: &Value) -> Vec<Value> {
    let host = swagger.get("host").and_then(|v| v.as_str());
    let base_path = swagger.get("basePath").and_then(|v| v.as_str()).unwrap_or("");
    let schemes = swagger
        .get("schemes")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(|| vec![json!("https")]);

    if let Some(host) = host {
        schemes
            .iter()
            .filter_map(|scheme| {
                scheme.as_str().map(|s| {
                    json!({
                        "url": format!("{}://{}{}", s, host, base_path)
                    })
                })
            })
            .collect()
    } else {
        // No host specified, use relative URL
        vec![json!({
            "url": base_path
        })]
    }
}

/// Convert Swagger 2.0 paths to OpenAPI 3.0 format
fn convert_paths(paths: &Value, global_consumes: &[Value], global_produces: &[Value]) -> Value {
    let Some(paths_obj) = paths.as_object() else {
        return paths.clone();
    };

    let mut converted = Map::new();

    for (path, path_item) in paths_obj {
        if let Some(path_item_obj) = path_item.as_object() {
            let converted_path_item =
                convert_path_item(path_item_obj, global_consumes, global_produces);
            converted.insert(path.clone(), Value::Object(converted_path_item));
        }
    }

    Value::Object(converted)
}

/// Convert a single path item
fn convert_path_item(
    path_item: &Map<String, Value>,
    global_consumes: &[Value],
    global_produces: &[Value],
) -> Map<String, Value> {
    let mut converted = Map::new();

    for (key, value) in path_item {
        match key.as_str() {
            "get" | "post" | "put" | "delete" | "patch" | "head" | "options" => {
                if let Some(op) = value.as_object() {
                    let converted_op = convert_operation(op, global_consumes, global_produces);
                    converted.insert(key.clone(), Value::Object(converted_op));
                }
            }
            "parameters" => {
                // Path-level parameters
                if let Some(params) = value.as_array() {
                    let converted_params: Vec<Value> =
                        params.iter().map(convert_parameter).collect();
                    converted.insert(key.clone(), json!(converted_params));
                }
            }
            "$ref" => {
                // Convert reference
                if let Some(ref_str) = value.as_str() {
                    converted.insert(key.clone(), json!(convert_ref(ref_str)));
                }
            }
            _ => {
                // Copy other fields as-is
                converted.insert(key.clone(), value.clone());
            }
        }
    }

    converted
}

/// Convert a single operation
fn convert_operation(
    operation: &Map<String, Value>,
    global_consumes: &[Value],
    global_produces: &[Value],
) -> Map<String, Value> {
    let mut converted = Map::new();

    // Get operation-level consumes/produces or fall back to global
    let consumes: Vec<String> = operation
        .get("consumes")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter())
        .unwrap_or_else(|| global_consumes.iter())
        .filter_map(|v| v.as_str().map(String::from))
        .collect::<Vec<_>>();
    let consumes = if consumes.is_empty() {
        vec!["application/json".to_string()]
    } else {
        consumes
    };

    let produces: Vec<String> = operation
        .get("produces")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter())
        .unwrap_or_else(|| global_produces.iter())
        .filter_map(|v| v.as_str().map(String::from))
        .collect::<Vec<_>>();
    let produces = if produces.is_empty() {
        vec!["application/json".to_string()]
    } else {
        produces
    };

    // Process parameters - separate body parameters for requestBody
    let mut non_body_params = Vec::new();
    let mut body_param: Option<&Value> = None;

    if let Some(params) = operation.get("parameters").and_then(|v| v.as_array()) {
        for param in params {
            if param.get("in").and_then(|v| v.as_str()) == Some("body") {
                body_param = Some(param);
            } else {
                non_body_params.push(convert_parameter(param));
            }
        }
    }

    // Add converted parameters
    if !non_body_params.is_empty() {
        converted.insert("parameters".to_string(), json!(non_body_params));
    }

    // Convert body parameter to requestBody
    if let Some(body) = body_param {
        let request_body = convert_body_to_request_body(body, &consumes);
        converted.insert("requestBody".to_string(), request_body);
    }

    // Convert responses
    if let Some(responses) = operation.get("responses") {
        let converted_responses = convert_responses(responses, &produces);
        converted.insert("responses".to_string(), converted_responses);
    }

    // Copy other fields
    for (key, value) in operation {
        match key.as_str() {
            "parameters" | "responses" | "consumes" | "produces" => {
                // Already handled
            }
            _ => {
                converted.insert(key.clone(), value.clone());
            }
        }
    }

    converted
}

/// Convert a Swagger 2.0 parameter to OpenAPI 3.0 format
fn convert_parameter(param: &Value) -> Value {
    let Some(param_obj) = param.as_object() else {
        return param.clone();
    };

    let mut converted = Map::new();

    // Copy basic fields
    for key in &["name", "in", "description", "required", "allowEmptyValue"] {
        if let Some(value) = param_obj.get(*key) {
            converted.insert(key.to_string(), value.clone());
        }
    }

    // Convert type/format/items to schema
    let param_in = param_obj.get("in").and_then(|v| v.as_str());

    // Skip body parameters (handled separately) and formData (converted to requestBody)
    if param_in == Some("body") || param_in == Some("formData") {
        return param.clone();
    }

    // Build schema from type/format/items/enum/default
    let mut schema = Map::new();

    if let Some(param_type) = param_obj.get("type") {
        schema.insert("type".to_string(), param_type.clone());
    }
    if let Some(format) = param_obj.get("format") {
        schema.insert("format".to_string(), format.clone());
    }
    if let Some(items) = param_obj.get("items") {
        schema.insert("items".to_string(), items.clone());
    }
    if let Some(enum_values) = param_obj.get("enum") {
        schema.insert("enum".to_string(), enum_values.clone());
    }
    if let Some(default) = param_obj.get("default") {
        schema.insert("default".to_string(), default.clone());
    }
    if let Some(minimum) = param_obj.get("minimum") {
        schema.insert("minimum".to_string(), minimum.clone());
    }
    if let Some(maximum) = param_obj.get("maximum") {
        schema.insert("maximum".to_string(), maximum.clone());
    }
    if let Some(pattern) = param_obj.get("pattern") {
        schema.insert("pattern".to_string(), pattern.clone());
    }

    if !schema.is_empty() {
        converted.insert("schema".to_string(), Value::Object(schema));
    }

    Value::Object(converted)
}

/// Convert body parameter to OpenAPI 3.0 requestBody
fn convert_body_to_request_body(body: &Value, consumes: &[String]) -> Value {
    let mut request_body = Map::new();

    if let Some(desc) = body.get("description") {
        request_body.insert("description".to_string(), desc.clone());
    }

    if let Some(required) = body.get("required") {
        request_body.insert("required".to_string(), required.clone());
    }

    // Build content section
    let mut content = Map::new();
    let schema = body.get("schema").cloned().unwrap_or(json!({}));

    for media_type in consumes {
        content.insert(
            media_type.clone(),
            json!({
                "schema": convert_schema_refs(&schema)
            }),
        );
    }

    request_body.insert("content".to_string(), Value::Object(content));

    Value::Object(request_body)
}

/// Convert Swagger 2.0 responses to OpenAPI 3.0 format
fn convert_responses(responses: &Value, produces: &[String]) -> Value {
    let Some(responses_obj) = responses.as_object() else {
        return responses.clone();
    };

    let mut converted = Map::new();

    for (status_code, response) in responses_obj {
        if let Some(response_obj) = response.as_object() {
            let converted_response = convert_response(response_obj, produces);
            converted.insert(status_code.clone(), Value::Object(converted_response));
        }
    }

    Value::Object(converted)
}

/// Convert a single response
fn convert_response(response: &Map<String, Value>, produces: &[String]) -> Map<String, Value> {
    let mut converted = Map::new();

    // Copy description (required in OpenAPI 3.0)
    if let Some(desc) = response.get("description") {
        converted.insert("description".to_string(), desc.clone());
    } else {
        converted.insert("description".to_string(), json!("Response"));
    }

    // Convert schema to content
    if let Some(schema) = response.get("schema") {
        let mut content = Map::new();
        for media_type in produces {
            content.insert(
                media_type.clone(),
                json!({
                    "schema": convert_schema_refs(schema)
                }),
            );
        }
        converted.insert("content".to_string(), Value::Object(content));
    }

    // Convert headers
    if let Some(headers) = response.get("headers") {
        if let Some(headers_obj) = headers.as_object() {
            let mut converted_headers = Map::new();
            for (name, header) in headers_obj {
                converted_headers.insert(name.clone(), convert_header(header));
            }
            converted.insert("headers".to_string(), Value::Object(converted_headers));
        }
    }

    // Copy examples (if any)
    if let Some(examples) = response.get("examples") {
        converted.insert("examples".to_string(), examples.clone());
    }

    converted
}

/// Convert a header definition
fn convert_header(header: &Value) -> Value {
    let Some(header_obj) = header.as_object() else {
        return header.clone();
    };

    let mut converted = Map::new();

    if let Some(desc) = header_obj.get("description") {
        converted.insert("description".to_string(), desc.clone());
    }

    // Build schema from type/format
    let mut schema = Map::new();
    if let Some(header_type) = header_obj.get("type") {
        schema.insert("type".to_string(), header_type.clone());
    }
    if let Some(format) = header_obj.get("format") {
        schema.insert("format".to_string(), format.clone());
    }

    if !schema.is_empty() {
        converted.insert("schema".to_string(), Value::Object(schema));
    }

    Value::Object(converted)
}

/// Convert Swagger 2.0 security definitions to OpenAPI 3.0 security schemes
fn convert_security_definitions(security_defs: &Value) -> Value {
    let Some(defs_obj) = security_defs.as_object() else {
        return security_defs.clone();
    };

    let mut converted = Map::new();

    for (name, def) in defs_obj {
        if let Some(def_obj) = def.as_object() {
            let converted_def = convert_security_definition(def_obj);
            converted.insert(name.clone(), Value::Object(converted_def));
        }
    }

    Value::Object(converted)
}

/// Convert a single security definition
fn convert_security_definition(def: &Map<String, Value>) -> Map<String, Value> {
    let mut converted = Map::new();

    let security_type = def.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match security_type {
        "basic" => {
            converted.insert("type".to_string(), json!("http"));
            converted.insert("scheme".to_string(), json!("basic"));
        }
        "apiKey" => {
            converted.insert("type".to_string(), json!("apiKey"));
            if let Some(name) = def.get("name") {
                converted.insert("name".to_string(), name.clone());
            }
            if let Some(in_val) = def.get("in") {
                converted.insert("in".to_string(), in_val.clone());
            }
        }
        "oauth2" => {
            converted.insert("type".to_string(), json!("oauth2"));

            // Convert OAuth2 flow
            let flow = def.get("flow").and_then(|v| v.as_str()).unwrap_or("implicit");
            let mut flows = Map::new();

            let mut flow_obj = Map::new();

            // Map Swagger 2.0 flow types to OpenAPI 3.0
            let flow_name = match flow {
                "implicit" => "implicit",
                "password" => "password",
                "application" => "clientCredentials",
                "accessCode" => "authorizationCode",
                _ => "implicit",
            };

            if let Some(auth_url) = def.get("authorizationUrl") {
                flow_obj.insert("authorizationUrl".to_string(), auth_url.clone());
            }
            if let Some(token_url) = def.get("tokenUrl") {
                flow_obj.insert("tokenUrl".to_string(), token_url.clone());
            }
            if let Some(scopes) = def.get("scopes") {
                flow_obj.insert("scopes".to_string(), scopes.clone());
            } else {
                flow_obj.insert("scopes".to_string(), json!({}));
            }

            flows.insert(flow_name.to_string(), Value::Object(flow_obj));
            converted.insert("flows".to_string(), Value::Object(flows));
        }
        _ => {
            // Unknown type, copy as-is
            for (key, value) in def {
                converted.insert(key.clone(), value.clone());
            }
        }
    }

    // Copy description if present
    if let Some(desc) = def.get("description") {
        converted.insert("description".to_string(), desc.clone());
    }

    converted
}

/// Convert $ref paths from Swagger 2.0 to OpenAPI 3.0 format
fn convert_ref(ref_str: &str) -> String {
    // #/definitions/Foo -> #/components/schemas/Foo
    if let Some(name) = ref_str.strip_prefix("#/definitions/") {
        format!("#/components/schemas/{}", name)
    } else {
        ref_str.to_string()
    }
}

/// Recursively convert $ref in schema objects
fn convert_schema_refs(schema: &Value) -> Value {
    match schema {
        Value::Object(obj) => {
            let mut converted = Map::new();
            for (key, value) in obj {
                if key == "$ref" {
                    if let Some(ref_str) = value.as_str() {
                        converted.insert(key.clone(), json!(convert_ref(ref_str)));
                    } else {
                        converted.insert(key.clone(), value.clone());
                    }
                } else {
                    converted.insert(key.clone(), convert_schema_refs(value));
                }
            }
            Value::Object(converted)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(convert_schema_refs).collect()),
        _ => schema.clone(),
    }
}

/// Check if a JSON value is a Swagger 2.0 specification
pub fn is_swagger_2(value: &Value) -> bool {
    value.get("swagger").and_then(|v| v.as_str()) == Some("2.0")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_swagger_2() {
        assert!(is_swagger_2(&json!({"swagger": "2.0"})));
        assert!(!is_swagger_2(&json!({"openapi": "3.0.0"})));
    }

    #[test]
    fn test_convert_servers() {
        let swagger = json!({
            "swagger": "2.0",
            "host": "api.example.com",
            "basePath": "/v1",
            "schemes": ["https", "http"]
        });

        let servers = convert_servers(&swagger);
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0]["url"], "https://api.example.com/v1");
        assert_eq!(servers[1]["url"], "http://api.example.com/v1");
    }

    #[test]
    fn test_convert_ref() {
        assert_eq!(convert_ref("#/definitions/User"), "#/components/schemas/User");
        assert_eq!(convert_ref("#/components/schemas/User"), "#/components/schemas/User");
    }

    #[test]
    fn test_convert_parameter() {
        let param = json!({
            "name": "userId",
            "in": "path",
            "required": true,
            "type": "string",
            "format": "uuid"
        });

        let converted = convert_parameter(&param);
        assert_eq!(converted["name"], "userId");
        assert_eq!(converted["in"], "path");
        assert_eq!(converted["schema"]["type"], "string");
        assert_eq!(converted["schema"]["format"], "uuid");
    }

    #[test]
    fn test_convert_security_basic() {
        let def = json!({
            "type": "basic",
            "description": "Basic auth"
        });

        if let Some(def_obj) = def.as_object() {
            let converted = convert_security_definition(def_obj);
            assert_eq!(converted["type"], json!("http"));
            assert_eq!(converted["scheme"], json!("basic"));
        }
    }

    #[test]
    fn test_basic_conversion() {
        let swagger = json!({
            "swagger": "2.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "host": "api.example.com",
            "basePath": "/v1",
            "schemes": ["https"],
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "getUsers",
                        "produces": ["application/json"],
                        "responses": {
                            "200": {
                                "description": "Success"
                            }
                        }
                    }
                }
            }
        });

        let result = convert_swagger_to_openapi3(&swagger).unwrap();
        assert_eq!(result["openapi"], "3.0.3");
        assert_eq!(result["info"]["title"], "Test API");
        assert!(result["servers"].as_array().is_some());
        assert!(result["paths"]["/users"]["get"].is_object());
    }
}
