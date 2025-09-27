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
pub fn import_postman_collection(
    content: &str,
    base_url: Option<&str>,
) -> Result<ImportResult, String> {
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
                    Err(e) => {
                        warnings.push(format!("Failed to convert request '{}': {}", item.name, e))
                    }
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
    _name: &str,
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
                    let path: Vec<String> = path_parts
                        .iter()
                        .map(|part| match part {
                            StringOrVariable::String(s) => resolve_variables(s, variables),
                            StringOrVariable::Variable(var) => {
                                if let Some(value) = variables.get(&var.key) {
                                    value.clone()
                                } else {
                                    var.key.clone()
                                }
                            }
                        })
                        .collect();
                    url_parts.push(path.join("/"));
                }

                // Query
                let query_parts: Vec<String> = structured
                    .query
                    .iter()
                    .filter(|q| !q.disabled && q.key.is_some())
                    .map(|q| {
                        let key = resolve_variables(q.key.as_ref().unwrap(), variables);
                        let value = q
                            .value
                            .as_ref()
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
fn generate_mock_response(
    request: &PostmanRequest,
    _variables: &HashMap<String, String>,
) -> MockForgeResponse {
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

        let result =
            import_postman_collection(collection_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");
        assert!(result.routes[0].headers.contains_key("Authorization"));
    }

    #[test]
    fn test_parse_postman_collection_with_multiple_requests() {
        let collection_json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Get Users",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": "https://api.example.com/users"
                    }
                },
                {
                    "name": "Create User",
                    "request": {
                        "method": "POST",
                        "header": [{"key": "Content-Type", "value": "application/json"}],
                        "url": "https://api.example.com/users",
                        "body": {
                            "mode": "raw",
                            "raw": "{\"name\": \"John\", \"age\": 30}"
                        }
                    }
                },
                {
                    "name": "Update User",
                    "request": {
                        "method": "PUT",
                        "header": [{"key": "Content-Type", "value": "application/json"}],
                        "url": "https://api.example.com/users/123",
                        "body": {
                            "mode": "raw",
                            "raw": "{\"name\": \"Jane\"}"
                        }
                    }
                }
            ]
        }"#;

        let result =
            import_postman_collection(collection_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 3);

        // Check first route (GET)
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");

        // Check second route (POST)
        assert_eq!(result.routes[1].method, "POST");
        assert_eq!(result.routes[1].path, "/users");
        assert_eq!(result.routes[1].body, Some("{\"name\": \"John\", \"age\": 30}".to_string()));
        assert_eq!(
            result.routes[1].headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );

        // Check third route (PUT)
        assert_eq!(result.routes[2].method, "PUT");
        assert_eq!(result.routes[2].path, "/users/123");
        assert_eq!(result.routes[2].body, Some("{\"name\": \"Jane\"}".to_string()));
    }

    #[test]
    fn test_parse_postman_collection_with_folders() {
        let collection_json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "User Operations",
                    "item": [
                        {
                            "name": "Get Users",
                            "request": {
                                "method": "GET",
                                "header": [],
                                "url": "https://api.example.com/users"
                            }
                        },
                        {
                            "name": "Create User",
                            "request": {
                                "method": "POST",
                                "header": [],
                                "url": "https://api.example.com/users"
                            }
                        }
                    ]
                },
                {
                    "name": "Admin Operations",
                    "item": [
                        {
                            "name": "Get Stats",
                            "request": {
                                "method": "GET",
                                "header": [],
                                "url": "https://api.example.com/admin/stats"
                            }
                        }
                    ]
                }
            ]
        }"#;

        let result =
            import_postman_collection(collection_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 3);

        // Check routes are created from nested items
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");

        assert_eq!(result.routes[1].method, "POST");
        assert_eq!(result.routes[1].path, "/users");

        assert_eq!(result.routes[2].method, "GET");
        assert_eq!(result.routes[2].path, "/admin/stats");
    }

    #[test]
    fn test_parse_postman_collection_with_query_parameters() {
        let collection_json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Search Users",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": {
                            "raw": "https://api.example.com/search?q=test&page=1&limit=10",
                            "host": ["api", "example", "com"],
                            "path": ["search"],
                            "query": [
                                {"key": "q", "value": "test"},
                                {"key": "page", "value": "1"},
                                {"key": "limit", "value": "10"}
                            ]
                        }
                    }
                }
            ]
        }"#;

        let result =
            import_postman_collection(collection_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/search?q=test&page=1&limit=10");
    }

    #[test]
    fn test_parse_postman_collection_with_different_methods() {
        let collection_json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {"name": "GET Request", "request": {"method": "GET", "url": "https://api.example.com/get"}},
                {"name": "POST Request", "request": {"method": "POST", "url": "https://api.example.com/post"}},
                {"name": "PUT Request", "request": {"method": "PUT", "url": "https://api.example.com/put"}},
                {"name": "DELETE Request", "request": {"method": "DELETE", "url": "https://api.example.com/delete"}},
                {"name": "PATCH Request", "request": {"method": "PATCH", "url": "https://api.example.com/patch"}},
                {"name": "HEAD Request", "request": {"method": "HEAD", "url": "https://api.example.com/head"}},
                {"name": "OPTIONS Request", "request": {"method": "OPTIONS", "url": "https://api.example.com/options"}}
            ]
        }"#;

        let result =
            import_postman_collection(collection_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 7);

        let expected_methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
        for (i, expected_method) in expected_methods.iter().enumerate() {
            assert_eq!(result.routes[i].method, *expected_method);
            assert_eq!(result.routes[i].path, format!("/{}", expected_method.to_lowercase()));
        }
    }

    #[test]
    fn test_parse_postman_collection_with_form_data() {
        let collection_json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Form Submit",
                    "request": {
                        "method": "POST",
                        "header": [{"key": "Content-Type", "value": "application/x-www-form-urlencoded"}],
                        "url": "https://api.example.com/form",
                        "body": {
                            "mode": "urlencoded",
                            "urlencoded": [
                                {"key": "username", "value": "john_doe"},
                                {"key": "password", "value": "secret123"},
                                {"key": "remember", "value": "true"}
                            ]
                        }
                    }
                }
            ]
        }"#;

        let result =
            import_postman_collection(collection_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "POST");
        assert_eq!(result.routes[0].path, "/form");
        assert_eq!(
            result.routes[0].body,
            Some("username=john_doe&password=secret123&remember=true".to_string())
        );
    }

    #[test]
    fn test_parse_postman_collection_with_raw_body() {
        let collection_json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "JSON Post",
                    "request": {
                        "method": "POST",
                        "header": [{"key": "Content-Type", "value": "application/json"}],
                        "url": "https://api.example.com/json",
                        "body": {
                            "mode": "raw",
                            "raw": "{\"message\": \"Hello World\", \"data\": {\"key\": \"value\"}}"
                        }
                    }
                },
                {
                    "name": "XML Post",
                    "request": {
                        "method": "POST",
                        "header": [{"key": "Content-Type", "value": "application/xml"}],
                        "url": "https://api.example.com/xml",
                        "body": {
                            "mode": "raw",
                            "raw": "<root><message>Hello</message><data><key>value</key></data></root>"
                        }
                    }
                }
            ]
        }"#;

        let result =
            import_postman_collection(collection_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 2);

        // Check JSON request
        assert_eq!(result.routes[0].method, "POST");
        assert_eq!(result.routes[0].path, "/json");
        assert_eq!(
            result.routes[0].body,
            Some("{\"message\": \"Hello World\", \"data\": {\"key\": \"value\"}}".to_string())
        );

        // Check XML request
        assert_eq!(result.routes[1].method, "POST");
        assert_eq!(result.routes[1].path, "/xml");
        assert_eq!(
            result.routes[1].body,
            Some("<root><message>Hello</message><data><key>value</key></data></root>".to_string())
        );
    }

    #[test]
    fn test_parse_postman_collection_with_auth() {
        let collection_json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Protected Request",
                    "request": {
                        "method": "GET",
                        "header": [
                            {"key": "Authorization", "value": "Bearer {{token}}"},
                            {"key": "X-API-Key", "value": "api-key-123"}
                        ],
                        "url": "https://api.example.com/protected",
                        "auth": {
                            "type": "bearer",
                            "bearer": [
                                {"key": "token", "value": "{{token}}", "type": "string"}
                            ]
                        }
                    }
                }
            ],
            "variable": [
                {"key": "token", "value": "test-token-abc"}
            ]
        }"#;

        let result =
            import_postman_collection(collection_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/protected");
        assert_eq!(
            result.routes[0].headers.get("Authorization"),
            Some(&"Bearer test-token-abc".to_string())
        );
        assert_eq!(result.routes[0].headers.get("X-API-Key"), Some(&"api-key-123".to_string()));
    }

    #[test]
    fn test_parse_postman_collection_with_variables() {
        let collection_json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Variable Test",
                    "request": {
                        "method": "GET",
                        "header": [
                            {"key": "X-User-ID", "value": "{{userId}}"},
                            {"key": "X-Environment", "value": "{{environment}}"}
                        ],
                        "url": "{{baseUrl}}/test/{{userId}}?env={{environment}}"
                    }
                }
            ],
            "variable": [
                {"key": "baseUrl", "value": "https://api.example.com"},
                {"key": "userId", "value": "12345"},
                {"key": "environment", "value": "production"}
            ]
        }"#;

        let result =
            import_postman_collection(collection_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/test/12345?env=production");
        assert_eq!(result.routes[0].headers.get("X-User-ID"), Some(&"12345".to_string()));
        assert_eq!(result.routes[0].headers.get("X-Environment"), Some(&"production".to_string()));
    }

    #[test]
    fn test_parse_postman_collection_with_disabled_items() {
        let collection_json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Enabled Request",
                    "request": {
                        "method": "GET",
                        "url": "https://api.example.com/enabled"
                    }
                },
                {
                    "name": "Disabled Request",
                    "request": {
                        "method": "GET",
                        "url": "https://api.example.com/disabled"
                    }
                }
            ]
        }"#;

        let result =
            import_postman_collection(collection_json, Some("https://api.example.com")).unwrap();

        // All requests should be imported regardless of disabled status in Postman
        // (disabled in Postman means not executed during collection run, not that it's invalid)
        assert_eq!(result.routes.len(), 2);
    }

    #[test]
    fn test_parse_postman_collection_with_complex_headers() {
        let collection_json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Complex Headers",
                    "request": {
                        "method": "GET",
                        "header": [
                            {"key": "Authorization", "value": "Bearer token123"},
                            {"key": "Content-Type", "value": "application/json"},
                            {"key": "Accept", "value": "application/json"},
                            {"key": "X-Custom-Header", "value": "custom-value"},
                            {"key": "X-Request-ID", "value": "req-123"},
                            {"key": "User-Agent", "value": "PostmanRuntime/7.29.0"},
                            {"key": "Cache-Control", "value": "no-cache"}
                        ],
                        "url": "https://api.example.com/complex"
                    }
                }
            ]
        }"#;

        let result =
            import_postman_collection(collection_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/complex");

        // Check that all headers are preserved
        let headers = &result.routes[0].headers;
        assert_eq!(headers.get("Authorization"), Some(&"Bearer token123".to_string()));
        assert_eq!(headers.get("Content-Type"), Some(&"application/json".to_string()));
        assert_eq!(headers.get("Accept"), Some(&"application/json".to_string()));
        assert_eq!(headers.get("X-Custom-Header"), Some(&"custom-value".to_string()));
        assert_eq!(headers.get("X-Request-ID"), Some(&"req-123".to_string()));
        assert_eq!(headers.get("User-Agent"), Some(&"PostmanRuntime/7.29.0".to_string()));
        assert_eq!(headers.get("Cache-Control"), Some(&"no-cache".to_string()));
    }
}
