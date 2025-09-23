//! Postman collection import functionality
//!
//! This module handles parsing Postman collections and converting them
//! to MockForge routes and configurations.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Postman collection structure
#[derive(Debug, Deserialize)]
pub struct PostmanCollection {
    pub info: CollectionInfo,
    pub item: Vec<CollectionItem>,
    #[serde(default)]
    pub variable: Vec<Variable>,
}

/// Collection metadata
#[derive(Debug, Deserialize)]
pub struct CollectionInfo {
    #[serde(rename = "_postman_id")]
    pub postman_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub schema: Option<String>,
}

/// Collection item (can be a request or a folder)
#[derive(Debug, Deserialize)]
pub struct CollectionItem {
    pub name: String,
    #[serde(default)]
    pub item: Vec<CollectionItem>, // For folders
    pub request: Option<PostmanRequest>,
}

/// Postman request structure
#[derive(Debug, Deserialize)]
pub struct PostmanRequest {
    pub method: String,
    pub header: Vec<Header>,
    pub url: UrlOrString,
    #[serde(default)]
    pub body: Option<RequestBody>,
    pub auth: Option<Auth>,
}

/// URL can be a string or structured object
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum UrlOrString {
    String(String),
    Structured(StructuredUrl),
}

/// Structured URL with host, path, query, etc.
#[derive(Debug, Deserialize)]
pub struct StructuredUrl {
    pub raw: Option<String>,
    pub protocol: Option<String>,
    pub host: Option<Vec<String>>,
    pub path: Option<Vec<StringOrVariable>>,
    #[serde(default)]
    pub query: Vec<QueryParam>,
    #[serde(default)]
    pub variable: Vec<Variable>,
}

/// Variable or string in URL components
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum StringOrVariable {
    String(String),
    Variable(Variable),
}

/// Query parameter
#[derive(Debug, Deserialize)]
pub struct QueryParam {
    pub key: Option<String>,
    pub value: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub disabled: bool,
}

/// Header
#[derive(Debug, Deserialize)]
pub struct Header {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub disabled: bool,
}

/// Request body
#[derive(Debug, Deserialize)]
pub struct RequestBody {
    pub mode: String,
    pub raw: Option<String>,
    pub urlencoded: Option<Vec<FormParam>>,
    pub formdata: Option<Vec<FormParam>>,
}

/// Form parameter
#[derive(Debug, Deserialize)]
pub struct FormParam {
    pub key: String,
    pub value: String,
    #[serde(rename = "type")]
    pub param_type: Option<String>,
}

/// Authentication
#[derive(Debug, Deserialize)]
pub struct Auth {
    #[serde(rename = "type")]
    pub auth_type: String,
    #[serde(flatten)]
    pub config: Value,
}

/// Variable
#[derive(Debug, Deserialize)]
pub struct Variable {
    pub key: String,
    pub value: Option<String>,
    #[serde(rename = "type")]
    pub var_type: Option<String>,
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

/// Import result
pub struct ImportResult {
    pub routes: Vec<MockForgeRoute>,
    pub variables: HashMap<String, String>,
    pub warnings: Vec<String>,
}

/// Import a Postman collection
pub fn import_postman_collection(content: &str, base_url: Option<&str>) -> Result<ImportResult, String> {
    let collection: PostmanCollection = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse Postman collection: {}", e))?;

    let mut routes = Vec::new();
    let mut variables = HashMap::new();
    let mut warnings = Vec::new();

    // Extract global variables
    for var in &collection.variable {
        if let Some(value) = &var.value {
            variables.insert(var.key.clone(), value.clone());
        }
    }

    // Process all items (recursive for folders)
    process_items(&collection.item, &mut routes, &variables, base_url, &mut warnings);

    Ok(ImportResult {
        routes,
        variables,
        warnings,
    })
}

/// Recursively process collection items
fn process_items(
    items: &[CollectionItem],
    routes: &mut Vec<MockForgeRoute>,
    variables: &HashMap<String, String>,
    base_url: Option<&str>,
    warnings: &mut Vec<String>,
) {
    for item in items {
        // Check if this item has a request (flattened fields)
        if item.request.is_some() {
            if let Some(request) = &item.request {
                // This is a request
                match convert_request_to_route(request, &item.name, variables, base_url) {
                    Ok(route) => routes.push(route),
                    Err(e) => warnings.push(format!("Failed to convert request '{}': {}", item.name, e)),
                }
            }
        } else if !item.item.is_empty() {
            // This is a folder, process recursively
            process_items(&item.item, routes, variables, base_url, warnings);
        }
    }
}

/// Convert a Postman request to a MockForge route
fn convert_request_to_route(
    request: &PostmanRequest,
    name: &str,
    variables: &HashMap<String, String>,
    base_url: Option<&str>,
) -> Result<MockForgeRoute, String> {
    // Build URL
    let url = build_url(&request.url, variables, base_url)?;

    // Extract headers
    let mut headers = HashMap::new();
    for header in &request.header {
        if !header.disabled && !header.key.is_empty() {
            headers.insert(header.key.clone(), resolve_variables(&header.value, variables));
        }
    }

    // Extract body
    let body = match &request.body {
        Some(body) if body.mode == "raw" => {
            body.raw.as_ref().map(|raw| resolve_variables(raw, variables))
        }
        _ => None,
    };

    // Generate mock response
    let response = generate_mock_response(request, variables);

    Ok(MockForgeRoute {
        method: request.method.clone(),
        path: url,
        headers,
        body,
        response,
    })
}

/// Build URL from Postman URL structure
fn build_url(
    url: &UrlOrString,
    variables: &HashMap<String, String>,
    base_url: Option<&str>,
) -> Result<String, String> {
    let raw_url = match url {
        UrlOrString::String(s) => resolve_variables(s, variables),
        UrlOrString::Structured(structured) => {
            if let Some(raw) = &structured.raw {
                resolve_variables(raw, variables)
            } else {
                // Build URL from components
                let mut url_parts = Vec::new();

                // Protocol
                if let Some(protocol) = &structured.protocol {
                    url_parts.push(format!("{}://", protocol));
                }

                // Host
                if let Some(host_parts) = &structured.host {
                    let host = host_parts.join(".");
                    url_parts.push(resolve_variables(&host, variables));
                }

                // Path
                if let Some(path_parts) = &structured.path {
                    let path: Vec<String> = path_parts.iter().map(|part| match part {
                        StringOrVariable::String(s) => resolve_variables(s, variables),
                        StringOrVariable::Variable(var) => {
                            if let Some(value) = variables.get(&var.key) {
                                value.clone()
                            } else {
                                var.key.clone()
                            }
                        }
                    }).collect();
                    url_parts.push(path.join("/"));
                }

                // Query
                let query_parts: Vec<String> = structured.query.iter()
                    .filter(|q| !q.disabled && q.key.is_some())
                    .map(|q| {
                        let key = resolve_variables(q.key.as_ref().unwrap(), variables);
                        let value = q.value.as_ref()
                            .map(|v| resolve_variables(v, variables))
                            .unwrap_or_default();
                        format!("{}={}", key, value)
                    })
                    .collect();

                if !query_parts.is_empty() {
                    url_parts.push(format!("?{}", query_parts.join("&")));
                }

                url_parts.join("")
            }
        }
    };

    // If base_url is provided, make path relative
    if let Some(base) = base_url {
        if raw_url.starts_with(base) {
            return Ok(raw_url.trim_start_matches(base).trim_start_matches('/').to_string());
        }
    }

    // Extract path from full URL
    if let Ok(url) = url::Url::parse(&raw_url) {
        Ok(url.path().to_string())
    } else {
        // Assume it's already a path
        Ok(raw_url)
    }
}

/// Resolve variables in a string
fn resolve_variables(input: &str, variables: &HashMap<String, String>) -> String {
    let mut result = input.to_string();
    for (key, value) in variables {
        let pattern = format!("{{{{{}}}}}", key);
        result = result.replace(&pattern, value);
    }
    result
}

/// Generate a mock response for the request
fn generate_mock_response(request: &PostmanRequest, variables: &HashMap<String, String>) -> MockForgeResponse {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let body = match request.method.as_str() {
        "GET" => json!({"message": "Mock GET response", "method": "GET"}),
        "POST" => json!({"message": "Mock POST response", "method": "POST", "created": true}),
        "PUT" => json!({"message": "Mock PUT response", "method": "PUT", "updated": true}),
        "DELETE" => json!({"message": "Mock DELETE response", "method": "DELETE", "deleted": true}),
        "PATCH" => json!({"message": "Mock PATCH response", "method": "PATCH", "patched": true}),
        _ => json!({"message": "Mock response", "method": &request.method}),
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
    fn test_parse_postman_collection() {
        let collection_json = r#"{
            "info": {
                "_postman_id": "test-id",
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Get Users",
                    "request": {
                        "method": "GET",
                        "header": [{"key": "Authorization", "value": "Bearer {{token}}"}],
                        "url": {"raw": "{{baseUrl}}/users"}
                    }
                }
            ],
            "variable": [
                {"key": "baseUrl", "value": "https://api.example.com"},
                {"key": "token", "value": "test-token"}
            ]
        }"#;

        let result = import_postman_collection(collection_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");
        assert!(result.routes[0].headers.contains_key("Authorization"));
    }
}
