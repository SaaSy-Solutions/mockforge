//! OpenAPI command generator
//!
//! This module generates curl and HTTPie commands from OpenAPI specifications
//! for API testing and exploration.

use crate::openapi::OpenApiSpec;
use openapiv3::Operation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Generated HTTP command with examples for an OpenAPI operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCommand {
    /// The operation ID from the OpenAPI spec
    pub operation_id: String,
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    /// Full URL for the request (with parameters substituted)
    pub url: String,
    /// Path template with parameter placeholders (e.g., "/users/{id}")
    pub path_template: String,
    /// HTTP headers to include in the request
    pub headers: HashMap<String, String>,
    /// Request body content (if applicable)
    pub body: Option<String>,
    /// Generated curl command string
    pub curl_command: String,
    /// Generated HTTPie command string
    pub httpie_command: String,
    /// Optional description from the OpenAPI operation
    pub description: Option<String>,
    /// Map of parameter names to example values used
    pub parameter_examples: HashMap<String, String>,
}

/// Options for customizing command generation behavior
#[derive(Debug, Clone)]
pub struct CommandGenerationOptions {
    /// Base URL to use (overrides OpenAPI spec servers if provided)
    pub base_url: Option<String>,
    /// Command format(s) to generate (curl, httpie, or both)
    pub format: CommandFormat,
    /// Whether to include authentication headers if defined in spec
    pub include_auth: bool,
    /// Whether to generate examples for all operations or only those with examples
    pub all_operations: bool,
    /// Whether to include request body examples in generated commands
    pub include_examples: bool,
    /// Custom headers to add to all generated requests
    pub custom_headers: HashMap<String, String>,
    /// Maximum number of parameter combinations to generate per operation
    pub max_examples_per_operation: usize,
}

/// Command format options for code generation
#[derive(Debug, Clone, PartialEq)]
pub enum CommandFormat {
    /// Generate curl commands only
    Curl,
    /// Generate HTTPie commands only
    Httpie,
    /// Generate both curl and HTTPie commands
    Both,
}

/// Result of command generation from an OpenAPI specification
#[derive(Debug)]
pub struct CommandGenerationResult {
    /// Generated commands for each operation
    pub commands: Vec<GeneratedCommand>,
    /// Warnings encountered during generation
    pub warnings: Vec<String>,
    /// Metadata extracted from the OpenAPI specification
    pub spec_info: OpenApiSpecInfo,
}

/// OpenAPI specification metadata extracted during command generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpecInfo {
    /// API title from the OpenAPI spec
    pub title: String,
    /// API version from the OpenAPI spec
    pub version: String,
    /// Optional API description
    pub description: Option<String>,
    /// OpenAPI specification version (e.g., "3.0.0", "3.1.0")
    pub openapi_version: String,
    /// List of server URLs from the OpenAPI spec
    pub servers: Vec<String>,
}

/// Generate commands from an OpenAPI specification
pub fn generate_commands_from_openapi(
    spec_content: &str,
    options: CommandGenerationOptions,
) -> Result<CommandGenerationResult, String> {
    let json_value: serde_json::Value =
        serde_json::from_str(spec_content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let spec = OpenApiSpec::from_json(json_value)
        .map_err(|e| format!("Failed to load OpenAPI spec: {}", e))?;

    spec.validate().map_err(|e| format!("Invalid OpenAPI specification: {}", e))?;

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

    let base_url = options
        .base_url
        .clone()
        .or_else(|| spec_info.servers.first().cloned())
        .unwrap_or_else(|| "http://localhost:3000".to_string());

    let mut commands = Vec::new();
    let mut warnings = Vec::new();

    let path_operations = spec.all_paths_and_operations();

    for (path, operations) in path_operations {
        for (method, operation) in operations {
            match generate_commands_for_operation(
                &spec, &method, &path, &operation, &base_url, &options,
            ) {
                Ok(mut op_commands) => commands.append(&mut op_commands),
                Err(e) => warnings
                    .push(format!("Failed to generate commands for {} {}: {}", method, path, e)),
            }
        }
    }

    Ok(CommandGenerationResult {
        commands,
        warnings,
        spec_info,
    })
}

/// Generate commands for a single operation
fn generate_commands_for_operation(
    spec: &OpenApiSpec,
    method: &str,
    path: &str,
    operation: &Operation,
    base_url: &str,
    options: &CommandGenerationOptions,
) -> Result<Vec<GeneratedCommand>, String> {
    let operation_id = operation.operation_id.clone().unwrap_or_else(|| {
        format!("{}_{}", method.to_lowercase(), path.replace("/", "_").trim_matches('_'))
    });

    let description = operation
        .summary
        .as_ref()
        .or(operation.description.as_ref())
        .map(|s| s.to_string());

    // Generate parameter combinations
    let parameter_combinations =
        generate_parameter_combinations(spec, operation, options.max_examples_per_operation)?;

    if parameter_combinations.is_empty() && !options.all_operations {
        return Ok(Vec::new());
    }

    let mut commands = Vec::new();

    // Generate at least one command even if no parameters
    let combinations = if parameter_combinations.is_empty() {
        vec![HashMap::new()]
    } else {
        parameter_combinations
    };

    for params in combinations {
        let url = build_url_with_params(base_url, path, &params)?;
        let headers = build_headers(spec, operation, &params, options)?;
        let body = build_request_body(operation, &params, options)?;

        let curl_command = generate_curl_command(method, &url, &headers, &body, &params);
        let httpie_command = generate_httpie_command(method, &url, &headers, &body, &params);

        commands.push(GeneratedCommand {
            operation_id: operation_id.to_string(),
            method: method.to_uppercase(),
            url: url.clone(),
            path_template: path.to_string(),
            headers: headers.clone(),
            body: body.clone(),
            curl_command,
            httpie_command,
            description: description.clone(),
            parameter_examples: params,
        });
    }

    Ok(commands)
}

/// Generate parameter combinations for an operation
fn generate_parameter_combinations(
    spec: &OpenApiSpec,
    operation: &Operation,
    _max_combinations: usize,
) -> Result<Vec<HashMap<String, String>>, String> {
    let mut params = HashMap::new();

    // Extract parameters from the OpenAPI operation
    for param_ref in &operation.parameters {
        let param = match param_ref {
            openapiv3::ReferenceOr::Item(param) => Some(param.clone()),
            openapiv3::ReferenceOr::Reference { reference } => {
                // Resolve reference
                if let Some(param_name) = reference.strip_prefix("#/components/parameters/") {
                    if let Some(components) = &spec.spec.components {
                        if let Some(param_ref) = components.parameters.get(param_name) {
                            param_ref.as_item().cloned()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        if let Some(param) = param {
            let (name, example_value) = match param {
                openapiv3::Parameter::Path { parameter_data, .. }
                | openapiv3::Parameter::Query { parameter_data, .. }
                | openapiv3::Parameter::Header { parameter_data, .. }
                | openapiv3::Parameter::Cookie { parameter_data, .. } => {
                    let name = parameter_data.name.clone();
                    let example = generate_parameter_example(&parameter_data);
                    (name, example)
                }
            };
            params.insert(name, example_value);
        }
    }

    if params.is_empty() {
        Ok(vec![HashMap::new()])
    } else {
        Ok(vec![params])
    }
}

/// Generate an example value for a parameter based on its schema
fn generate_parameter_example(parameter_data: &openapiv3::ParameterData) -> String {
    // Check if there's an example in the parameter
    if let Some(example) = &parameter_data.example {
        return serde_json::to_string(example).unwrap_or_else(|_| "example".to_string());
    }

    // Generate based on schema
    match &parameter_data.format {
        openapiv3::ParameterSchemaOrContent::Schema(schema_ref) => {
            match schema_ref {
                openapiv3::ReferenceOr::Item(schema) => generate_example_from_schema(schema),
                openapiv3::ReferenceOr::Reference { .. } => {
                    // For references, use a generic example
                    "example".to_string()
                }
            }
        }
        openapiv3::ParameterSchemaOrContent::Content(_) => {
            // For content-based parameters, use a generic example
            "example".to_string()
        }
    }
}

/// Generate an example string from a schema
fn generate_example_from_schema(schema: &openapiv3::Schema) -> String {
    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => "example_string".to_string(),
        openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) => "42".to_string(),
        openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) => "3.14".to_string(),
        openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) => "true".to_string(),
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(_)) => "{}".to_string(),
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(_)) => "[]".to_string(),
        _ => "example".to_string(),
    }
}

/// Build URL with parameters substituted
fn build_url_with_params(
    base_url: &str,
    path_template: &str,
    params: &HashMap<String, String>,
) -> Result<String, String> {
    let mut url = base_url.trim_end_matches('/').to_string();
    let mut path = path_template.to_string();

    // Substitute path parameters
    for (key, value) in params {
        let placeholder = format!("{{{}}}", key);
        path = path.replace(&placeholder, value);
    }

    url.push_str(&path);

    // Add query parameters
    let query_params: Vec<String> = params
        .iter()
        .filter(|(key, _)| !path_template.contains(&format!("{{{}}}", key)))
        .map(|(key, value)| format!("{}={}", key, urlencoding::encode(value)))
        .collect();

    if !query_params.is_empty() {
        url.push('?');
        url.push_str(&query_params.join("&"));
    }

    Ok(url)
}

/// Build headers for the request
fn build_headers(
    spec: &OpenApiSpec,
    operation: &Operation,
    _params: &HashMap<String, String>,
    options: &CommandGenerationOptions,
) -> Result<HashMap<String, String>, String> {
    let mut headers = HashMap::new();

    // Add custom headers
    for (key, value) in &options.custom_headers {
        headers.insert(key.clone(), value.clone());
    }

    // Add Content-Type for request body
    if let Some(request_body) = &operation.request_body {
        if let Some(_content) =
            request_body.as_item().and_then(|rb| rb.content.get("application/json"))
        {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
    }

    // Add security scheme headers if include_auth is true
    if options.include_auth {
        add_security_headers(spec, operation, &mut headers)?;
    }

    Ok(headers)
}

/// Add security scheme headers to the request
fn add_security_headers(
    spec: &OpenApiSpec,
    operation: &Operation,
    headers: &mut HashMap<String, String>,
) -> Result<(), String> {
    // Get security requirements for this operation, or fall back to global security
    let security_requirements = if let Some(ref security_reqs) = operation.security {
        if !security_reqs.is_empty() {
            security_reqs.clone()
        } else {
            spec.get_global_security_requirements()
        }
    } else {
        spec.get_global_security_requirements()
    };

    // If no security requirements, nothing to do
    if security_requirements.is_empty() {
        return Ok(());
    }

    let security_schemes = match spec.security_schemes() {
        Some(schemes) => schemes,
        None => return Ok(()), // No security schemes defined
    };

    // Process each security requirement
    for requirement in &security_requirements {
        for (scheme_name, _scopes) in requirement {
            if let Some(scheme_ref) = security_schemes.get(scheme_name) {
                if let Some(scheme) = scheme_ref.as_item() {
                    match scheme {
                        openapiv3::SecurityScheme::HTTP { scheme, .. } => {
                            match scheme.as_str() {
                                "bearer" => {
                                    headers.insert(
                                        "Authorization".to_string(),
                                        "Bearer YOUR_TOKEN_HERE".to_string(),
                                    );
                                }
                                "basic" => {
                                    headers.insert(
                                        "Authorization".to_string(),
                                        "Basic YOUR_CREDENTIALS_HERE".to_string(),
                                    );
                                }
                                _ => {
                                    // For other HTTP schemes, add a generic placeholder
                                    headers.insert(
                                        "Authorization".to_string(),
                                        format!("{} YOUR_CREDENTIALS_HERE", scheme.to_uppercase()),
                                    );
                                }
                            }
                        }
                        openapiv3::SecurityScheme::APIKey { location, name, .. } => {
                            match location {
                                openapiv3::APIKeyLocation::Header => {
                                    headers.insert(name.clone(), "YOUR_API_KEY_HERE".to_string());
                                }
                                openapiv3::APIKeyLocation::Query => {
                                    // Query parameters are handled elsewhere, skip for headers
                                }
                                openapiv3::APIKeyLocation::Cookie => {
                                    // Cookies are not typically added as headers
                                }
                            }
                        }
                        openapiv3::SecurityScheme::OpenIDConnect { .. } => {
                            headers.insert(
                                "Authorization".to_string(),
                                "Bearer YOUR_OIDC_TOKEN_HERE".to_string(),
                            );
                        }
                        openapiv3::SecurityScheme::OAuth2 { .. } => {
                            headers.insert(
                                "Authorization".to_string(),
                                "Bearer YOUR_OAUTH_TOKEN_HERE".to_string(),
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Build request body
fn build_request_body(
    operation: &Operation,
    _params: &HashMap<String, String>,
    options: &CommandGenerationOptions,
) -> Result<Option<String>, String> {
    if !options.include_examples {
        return Ok(None);
    }

    if let Some(request_body) = &operation.request_body {
        if let Some(content) =
            request_body.as_item().and_then(|rb| rb.content.get("application/json"))
        {
            if let Some(example) = &content.example {
                return serde_json::to_string_pretty(example)
                    .map(Some)
                    .map_err(|e| format!("Failed to serialize example: {}", e));
            }
        }
    }

    Ok(None)
}

/// Generate curl command
fn generate_curl_command(
    method: &str,
    url: &str,
    headers: &HashMap<String, String>,
    body: &Option<String>,
    _params: &HashMap<String, String>,
) -> String {
    let mut cmd = format!("curl -X {} '{}'", method.to_uppercase(), url);

    // Add headers
    for (key, value) in headers {
        cmd.push_str(&format!(" \\\n  -H '{}: {}'", key, value));
    }

    // Add body
    if let Some(body_content) = body {
        cmd.push_str(&format!(" \\\n  -d '{}'", body_content.replace("'", "\\'")));
    }

    cmd
}

/// Generate HTTPie command
fn generate_httpie_command(
    method: &str,
    url: &str,
    headers: &HashMap<String, String>,
    body: &Option<String>,
    _params: &HashMap<String, String>,
) -> String {
    let mut cmd = format!("http {} '{}'", method.to_uppercase(), url);

    // Add headers
    for (key, value) in headers {
        cmd.push_str(&format!(" \\\n  {}:'{}'", key, value));
    }

    // Add body
    if let Some(body_content) = body {
        cmd.push_str(&format!(" \\\n  <<< '{}'", body_content.replace("'", "\\'")));
    }

    cmd
}
