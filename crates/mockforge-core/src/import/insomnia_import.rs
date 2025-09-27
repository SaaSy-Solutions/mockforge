//! Insomnia export import functionality
//!
//! This module handles parsing Insomnia exports and converting them
//! to MockForge routes and configurations.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Insomnia export structure
#[derive(Debug, Deserialize)]
pub struct InsomniaExport {
    #[serde(rename = "__export_format")]
    pub export_format: Option<i32>,
    #[serde(rename = "_type")]
    pub export_type: Option<String>,
    pub resources: Vec<InsomniaResource>,
}

/// Generic Insomnia resource
#[derive(Debug, Deserialize)]
pub struct InsomniaResource {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_type")]
    pub resource_type: String,
    pub parent_id: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
    pub method: Option<String>,
    pub headers: Option<Vec<InsomniaHeader>>,
    pub body: Option<InsomniaBody>,
    pub authentication: Option<InsomniaAuth>,
    pub parameters: Option<Vec<InsomniaParameter>>,
    pub data: Option<Value>,         // For environment data
    pub environment: Option<String>, // Environment name
}

/// Insomnia header
#[derive(Debug, Deserialize)]
pub struct InsomniaHeader {
    pub name: String,
    pub value: String,
    pub disabled: Option<bool>,
}

/// Insomnia request body
#[derive(Debug, Deserialize)]
pub struct InsomniaBody {
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub params: Option<Vec<InsomniaParameter>>,
}

/// Insomnia parameter (query params, form data, etc.)
#[derive(Debug, Deserialize)]
pub struct InsomniaParameter {
    pub name: String,
    pub value: String,
    pub disabled: Option<bool>,
}

/// Insomnia authentication
#[derive(Debug, Deserialize)]
pub struct InsomniaAuth {
    #[serde(rename = "type")]
    pub auth_type: String,
    pub disabled: Option<bool>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub prefix: Option<String>,
    pub key: Option<String>,
    pub value: Option<String>,
}

/// MockForge route structure for import
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

/// Import result for Insomnia exports
#[derive(Debug)]
pub struct InsomniaImportResult {
    pub routes: Vec<MockForgeRoute>,
    pub variables: HashMap<String, String>,
    pub warnings: Vec<String>,
}

/// Import an Insomnia export
pub fn import_insomnia_export(
    content: &str,
    environment: Option<&str>,
) -> Result<InsomniaImportResult, String> {
    let export: InsomniaExport = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse Insomnia export: {}", e))?;

    // Validate export format
    if let Some(format) = export.export_format {
        if format < 3 {
            return Err("Insomnia export format version 3 or higher is required".to_string());
        }
    }

    let mut routes = Vec::new();
    let mut variables = HashMap::new();
    let mut warnings = Vec::new();

    // Extract environment variables if specified
    if let Some(env_name) = environment {
        extract_environment_variables(&export.resources, env_name, &mut variables);
    } else {
        // Try to find default environment
        extract_environment_variables(&export.resources, "Base Environment", &mut variables);
    }

    // Process all resources to find requests
    for resource in &export.resources {
        if resource.resource_type == "request" {
            match convert_insomnia_request_to_route(resource, &variables) {
                Ok(route) => routes.push(route),
                Err(e) => warnings.push(format!(
                    "Failed to convert request '{}': {}",
                    resource.name.as_deref().unwrap_or("unnamed"),
                    e
                )),
            }
        }
    }

    Ok(InsomniaImportResult {
        routes,
        variables,
        warnings,
    })
}

/// Extract variables from specified environment
fn extract_environment_variables(
    resources: &[InsomniaResource],
    env_name: &str,
    variables: &mut HashMap<String, String>,
) {
    for resource in resources {
        if resource.resource_type == "environment" && resource.name.as_deref() == Some(env_name) {
            if let Some(data) = &resource.data {
                if let Some(obj) = data.as_object() {
                    for (key, value) in obj {
                        if let Some(str_value) = value.as_str() {
                            variables.insert(key.clone(), str_value.to_string());
                        } else if let Some(num_value) = value.as_f64() {
                            variables.insert(key.clone(), num_value.to_string());
                        } else if let Some(bool_value) = value.as_bool() {
                            variables.insert(key.clone(), bool_value.to_string());
                        }
                    }
                }
            }
        }
    }
}

/// Convert an Insomnia request to a MockForge route
fn convert_insomnia_request_to_route(
    resource: &InsomniaResource,
    variables: &HashMap<String, String>,
) -> Result<MockForgeRoute, String> {
    let method = resource.method.as_deref().ok_or("Request missing method")?.to_uppercase();

    let raw_url = resource.url.as_deref().ok_or("Request missing URL")?;

    let url = resolve_variables(raw_url, variables);

    // Extract path from URL
    let path = extract_path_from_url(&url)?;

    // Extract headers
    let mut headers = HashMap::new();
    if let Some(resource_headers) = &resource.headers {
        for header in resource_headers {
            if !header.disabled.unwrap_or(false) && !header.name.is_empty() {
                headers.insert(header.name.clone(), resolve_variables(&header.value, variables));
            }
        }
    }

    // Add authentication headers
    if let Some(auth) = &resource.authentication {
        if !auth.disabled.unwrap_or(false) {
            add_auth_headers(auth, &mut headers, variables);
        }
    }

    // Extract body
    let body = extract_request_body(resource, variables);

    // Generate mock response
    let response = generate_mock_response(&method);

    Ok(MockForgeRoute {
        method,
        path,
        headers,
        body,
        response,
    })
}

/// Extract path from URL, handling full URLs and relative paths
fn extract_path_from_url(url: &str) -> Result<String, String> {
    if let Ok(parsed_url) = url::Url::parse(url) {
        Ok(parsed_url.path().to_string())
    } else if url.starts_with('/') {
        Ok(url.to_string())
    } else {
        // Assume it's a relative path
        Ok(format!("/{}", url))
    }
}

/// Add authentication headers based on Insomnia auth configuration
fn add_auth_headers(
    auth: &InsomniaAuth,
    headers: &mut HashMap<String, String>,
    variables: &HashMap<String, String>,
) {
    match auth.auth_type.as_str() {
        "bearer" => {
            if let Some(token) = &auth.token {
                let resolved_token = resolve_variables(token, variables);
                headers.insert("Authorization".to_string(), format!("Bearer {}", resolved_token));
            }
        }
        "basic" => {
            if let (Some(username), Some(password)) = (&auth.username, &auth.password) {
                let user = resolve_variables(username, variables);
                let pass = resolve_variables(password, variables);
                use base64::{engine::general_purpose, Engine as _};
                let credentials = general_purpose::STANDARD.encode(format!("{}:{}", user, pass));
                headers.insert("Authorization".to_string(), format!("Basic {}", credentials));
            }
        }
        "apikey" => {
            if let (Some(key), Some(value)) = (&auth.key, &auth.value) {
                let resolved_key = resolve_variables(key, variables);
                let resolved_value = resolve_variables(value, variables);
                headers.insert(resolved_key, resolved_value);
            }
        }
        _ => {
            // Other auth types (OAuth, etc.) not yet supported
        }
    }
}

/// Extract request body from Insomnia resource
fn extract_request_body(
    resource: &InsomniaResource,
    variables: &HashMap<String, String>,
) -> Option<String> {
    if let Some(body) = &resource.body {
        if let Some(text) = &body.text {
            return Some(resolve_variables(text, variables));
        }
    }
    None
}

/// Resolve variables in a string (similar to Postman)
fn resolve_variables(input: &str, variables: &HashMap<String, String>) -> String {
    let mut result = input.to_string();
    for (key, value) in variables {
        let pattern = format!("{{{{{}}}}}", key);
        result = result.replace(&pattern, value);
    }
    result
}

/// Generate a mock response for the request
fn generate_mock_response(method: &str) -> MockForgeResponse {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let body = match method {
        "GET" => json!({"message": "Mock GET response", "method": "GET"}),
        "POST" => json!({"message": "Mock POST response", "method": "POST", "created": true}),
        "PUT" => json!({"message": "Mock PUT response", "method": "PUT", "updated": true}),
        "DELETE" => json!({"message": "Mock DELETE response", "method": "DELETE", "deleted": true}),
        "PATCH" => json!({"message": "Mock PATCH response", "method": "PATCH", "patched": true}),
        _ => json!({"message": "Mock response", "method": method}),
    };

    MockForgeResponse {
        status: 200,
        headers,
        body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_insomnia_export() {
        let export_json = r#"{
            "__export_format": 4,
            "_type": "export",
            "resources": [
                {
                    "_id": "req_1",
                    "_type": "request",
                    "name": "Get Users",
                    "method": "GET",
                    "url": "{{baseUrl}}/users",
                    "headers": [
                        {"name": "Authorization", "value": "Bearer {{token}}"}
                    ],
                    "authentication": {
                        "type": "bearer",
                        "token": "{{token}}"
                    }
                },
                {
                    "_id": "env_1",
                    "_type": "environment",
                    "name": "Base Environment",
                    "data": {
                        "baseUrl": "https://api.example.com",
                        "token": "test-token"
                    }
                }
            ]
        }"#;

        let result = import_insomnia_export(export_json, Some("Base Environment")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");
        assert!(result.routes[0].headers.contains_key("Authorization"));
        assert!(result.variables.contains_key("baseUrl"));
    }

    #[test]
    fn test_insomnia_format_validation() {
        let old_format = r#"{
            "__export_format": 2,
            "resources": []
        }"#;

        let result = import_insomnia_export(old_format, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("version 3 or higher"));
    }
}
