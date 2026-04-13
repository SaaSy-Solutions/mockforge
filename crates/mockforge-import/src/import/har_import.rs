//! HAR (HTTP Archive) import functionality
//!
//! This module handles parsing HAR files and converting them
//! to MockForge routes and configurations.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// HAR log structure (root object)
#[derive(Debug, Deserialize)]
pub struct HarLog {
    /// HAR format version
    pub version: String,
    /// Tool that created the HAR file
    pub creator: HarCreator,
    /// Optional browser information
    #[serde(default)]
    pub browser: Option<HarBrowser>,
    /// Optional page information
    #[serde(default)]
    pub pages: Vec<HarPage>,
    /// Array of HTTP request/response entries
    pub entries: Vec<HarEntry>,
}

/// HAR creator information
#[derive(Debug, Deserialize)]
pub struct HarCreator {
    /// Creator name (e.g., "Chrome DevTools")
    pub name: String,
    /// Creator version
    pub version: String,
}

/// HAR browser information (optional)
#[derive(Debug, Deserialize)]
pub struct HarBrowser {
    /// Browser name
    pub name: String,
    /// Browser version
    pub version: String,
}

/// HAR page information (optional)
#[derive(Debug, Deserialize)]
pub struct HarPage {
    /// Page load start timestamp
    pub started_date_time: String,
    /// Unique page identifier
    pub id: String,
    /// Page title
    pub title: String,
    /// Page load timing information
    pub page_timings: HarPageTimings,
}

/// HAR page timings
#[derive(Debug, Deserialize)]
pub struct HarPageTimings {
    /// Time to load page content (ms)
    pub on_content_load: Option<f64>,
    /// Time to complete page load (ms)
    pub on_load: Option<f64>,
}

/// HAR entry (request/response pair)
#[derive(Debug, Deserialize)]
pub struct HarEntry {
    /// Page reference ID (if applicable)
    pub pageref: Option<String>,
    /// Request start timestamp
    #[serde(rename = "startedDateTime")]
    pub started_date_time: String,
    /// Total request/response time (ms)
    pub time: f64,
    /// HTTP request details
    pub request: HarRequest,
    /// HTTP response details
    pub response: HarResponse,
    /// Cache information
    pub cache: HarCache,
    /// Detailed timing breakdown
    pub timings: HarTimings,
}

/// HAR request structure
#[derive(Debug, Deserialize)]
pub struct HarRequest {
    /// HTTP method
    pub method: String,
    /// Request URL
    pub url: String,
    /// HTTP version (e.g., "HTTP/1.1")
    #[serde(rename = "httpVersion")]
    pub http_version: String,
    /// Request cookies
    pub cookies: Vec<HarCookie>,
    /// Request headers
    pub headers: Vec<HarHeader>,
    /// Query parameters
    #[serde(default, rename = "queryString")]
    pub query_string: Vec<HarQueryParam>,
    /// POST data (if any)
    #[serde(rename = "postData")]
    pub post_data: Option<HarPostData>,
    /// Size of request headers (bytes)
    #[serde(rename = "headersSize")]
    pub headers_size: i64,
    /// Size of request body (bytes)
    #[serde(rename = "bodySize")]
    pub body_size: i64,
}

/// HAR response structure
#[derive(Debug, Deserialize)]
pub struct HarResponse {
    /// HTTP status code
    pub status: u16,
    /// HTTP status text
    #[serde(rename = "statusText")]
    pub status_text: String,
    /// HTTP version (e.g., "HTTP/1.1")
    #[serde(rename = "httpVersion")]
    pub http_version: String,
    /// Response cookies
    pub cookies: Vec<HarCookie>,
    /// Response headers
    pub headers: Vec<HarHeader>,
    /// Response content
    pub content: HarContent,
    /// Redirect URL (if applicable)
    #[serde(rename = "redirectURL")]
    pub redirect_url: String,
    /// Size of response headers (bytes)
    #[serde(rename = "headersSize")]
    pub headers_size: i64,
    /// Size of response body (bytes)
    #[serde(rename = "bodySize")]
    pub body_size: i64,
}

/// HAR cookie entry
#[derive(Debug, Deserialize)]
pub struct HarCookie {
    /// Cookie name
    pub name: String,
    /// Cookie value
    pub value: String,
    /// Cookie path
    #[serde(default)]
    pub path: Option<String>,
    /// Cookie domain
    #[serde(default)]
    pub domain: Option<String>,
    /// Cookie expiration timestamp
    #[serde(default)]
    pub expires: Option<String>,
    /// Whether cookie is HTTP-only
    #[serde(default)]
    pub http_only: Option<bool>,
    /// Whether cookie requires HTTPS
    #[serde(default)]
    pub secure: Option<bool>,
}

/// HAR header entry
#[derive(Debug, Deserialize)]
pub struct HarHeader {
    /// Header name
    pub name: String,
    /// Header value
    pub value: String,
}

/// HAR query parameter entry
#[derive(Debug, Deserialize)]
pub struct HarQueryParam {
    /// Parameter name
    pub name: String,
    /// Parameter value
    pub value: String,
}

/// HAR POST request body data
#[derive(Debug, Deserialize)]
pub struct HarPostData {
    /// MIME type of the POST data
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Form parameters (for form-urlencoded or multipart/form-data)
    #[serde(default)]
    pub params: Vec<HarParam>,
    /// Raw text content (for raw or JSON bodies)
    #[serde(default)]
    pub text: Option<String>,
}

/// HAR POST parameter (form field or file)
#[derive(Debug, Deserialize)]
pub struct HarParam {
    /// Parameter name
    pub name: String,
    /// Parameter value (None for file uploads)
    pub value: Option<String>,
    /// Filename (for file uploads)
    #[serde(default, rename = "fileName")]
    pub file_name: Option<String>,
    /// Content type (for file uploads)
    #[serde(default, rename = "contentType")]
    pub content_type: Option<String>,
}

/// HAR response content
#[derive(Debug, Deserialize)]
pub struct HarContent {
    /// Content size in bytes
    pub size: i64,
    /// Compressed size (bytes, if compression was used)
    #[serde(default)]
    pub compression: Option<i64>,
    /// MIME type of the content
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Response body text (base64 encoded if encoding is specified)
    #[serde(default)]
    pub text: Option<String>,
    /// Content encoding (base64, gzip, etc.)
    #[serde(default)]
    pub encoding: Option<String>,
}

/// HAR cache information (currently empty struct per HAR spec)
#[derive(Debug, Deserialize)]
pub struct HarCache {}

/// HAR timing breakdown for performance analysis
#[derive(Debug, Deserialize)]
pub struct HarTimings {
    /// Time to send request (ms)
    #[serde(default)]
    pub send: Option<f64>,
    /// Time waiting for response (ms)
    #[serde(default)]
    pub wait: Option<f64>,
    /// Time to receive response (ms)
    #[serde(default)]
    pub receive: Option<f64>,
}

/// HAR archive root structure
#[derive(Debug, Deserialize)]
pub struct HarArchive {
    /// HAR log containing all entries
    pub log: HarLog,
}

/// MockForge route structure for HAR import
#[derive(Debug, Serialize)]
pub struct MockForgeRoute {
    /// HTTP method
    pub method: String,
    /// Request path
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

/// Result of importing a HAR archive
pub struct HarImportResult {
    /// Converted routes from HAR entries
    pub routes: Vec<MockForgeRoute>,
    /// Warnings encountered during import
    pub warnings: Vec<String>,
}

/// Import a HAR archive
pub fn import_har_archive(
    content: &str,
    base_url: Option<&str>,
) -> Result<HarImportResult, String> {
    let archive: HarArchive =
        serde_json::from_str(content).map_err(|e| format!("Failed to parse HAR archive: {}", e))?;

    let mut routes = Vec::new();
    let mut warnings = Vec::new();

    // Process each entry in the HAR log
    for entry in &archive.log.entries {
        match convert_entry_to_route(entry, base_url) {
            Ok(route) => routes.push(route),
            Err(e) => warnings.push(format!("Failed to convert HAR entry: {}", e)),
        }
    }

    Ok(HarImportResult { routes, warnings })
}

/// Convert a HAR entry to a MockForge route
fn convert_entry_to_route(
    entry: &HarEntry,
    base_url: Option<&str>,
) -> Result<MockForgeRoute, String> {
    let request = &entry.request;
    let response = &entry.response;

    // Extract path from URL
    let path = extract_path_from_url(&request.url, base_url)?;

    // Extract request headers
    let mut request_headers = HashMap::new();
    for header in &request.headers {
        if !header.name.is_empty() {
            request_headers.insert(header.name.clone(), header.value.clone());
        }
    }

    // Extract request body
    let body = extract_request_body(request);

    // Extract response headers
    let mut response_headers = HashMap::new();
    for header in &response.headers {
        if !header.name.is_empty() {
            response_headers.insert(header.name.clone(), header.value.clone());
        }
    }

    // Extract response body
    let response_body = extract_response_body(response);

    let mock_response = MockForgeResponse {
        status: response.status,
        headers: response_headers,
        body: response_body,
    };

    Ok(MockForgeRoute {
        method: request.method.clone(),
        path,
        headers: request_headers,
        body,
        response: mock_response,
    })
}

/// Extract path from URL, optionally making it relative to base_url
fn extract_path_from_url(url: &str, base_url: Option<&str>) -> Result<String, String> {
    if let Ok(parsed_url) = url::Url::parse(url) {
        let path = parsed_url.path();
        let query = parsed_url.query();

        let full_path = if let Some(q) = query {
            format!("{}?{}", path, q)
        } else {
            path.to_string()
        };

        // If base_url is provided, make path relative
        if let Some(base) = base_url {
            if let Ok(base_parsed) = url::Url::parse(base) {
                if parsed_url.host() == base_parsed.host() {
                    return Ok(full_path);
                }
            }
        }

        // Extract just the path part if it's an absolute URL
        Ok(full_path)
    } else {
        // Assume it's already a path
        Ok(url.to_string())
    }
}

/// Extract request body from HAR request
fn extract_request_body(request: &HarRequest) -> Option<String> {
    if let Some(post_data) = &request.post_data {
        if let Some(text) = &post_data.text {
            if !text.is_empty() {
                return Some(text.clone());
            }
        }

        // Handle form parameters
        if !post_data.params.is_empty() {
            let mut form_data = Vec::new();
            for param in &post_data.params {
                if let Some(value) = &param.value {
                    form_data.push(format!("{}={}", param.name, value));
                }
            }
            if !form_data.is_empty() {
                return Some(form_data.join("&"));
            }
        }
    }
    None
}

/// Extract response body from HAR response
fn extract_response_body(response: &HarResponse) -> Value {
    if let Some(text) = &response.content.text {
        if !text.is_empty() {
            // Try to parse as JSON first
            if let Ok(json_value) = serde_json::from_str::<Value>(text) {
                return json_value;
            }

            // If not JSON, return as string
            return Value::String(text.clone());
        }
    }

    // Default empty response
    json!({"message": "Mock response from HAR import"})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_har_archive() {
        let har_json = r#"{
            "log": {
                "version": "1.2",
                "creator": {
                    "name": "Test Creator",
                    "version": "1.0"
                },
                "entries": [
                    {
                        "startedDateTime": "2024-01-15T10:30:00Z",
                        "time": 123.45,
                        "request": {
                            "method": "GET",
                            "url": "https://api.example.com/users",
                            "httpVersion": "HTTP/1.1",
                            "cookies": [],
                            "headers": [
                                {"name": "Authorization", "value": "Bearer token123"},
                                {"name": "Content-Type", "value": "application/json"}
                            ],
                            "queryString": [],
                            "headersSize": 150,
                            "bodySize": 0
                        },
                        "response": {
                            "status": 200,
                            "statusText": "OK",
                            "httpVersion": "HTTP/1.1",
                            "cookies": [],
                            "headers": [
                                {"name": "Content-Type", "value": "application/json"}
                            ],
                            "content": {
                                "size": 45,
                                "mimeType": "application/json",
                                "text": "{\"users\": [{\"id\": 1, \"name\": \"John\"}]}"
                            },
                            "redirectURL": "",
                            "headersSize": 100,
                            "bodySize": 45
                        },
                        "cache": {},
                        "timings": {
                            "send": 10.0,
                            "wait": 100.0,
                            "receive": 13.45
                        }
                    }
                ]
            }
        }"#;

        let result = import_har_archive(har_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");
        assert!(result.routes[0].headers.contains_key("Authorization"));
        assert_eq!(result.routes[0].response.status, 200);
    }

    #[test]
    fn test_extract_path_from_url() {
        // Test with base URL
        assert_eq!(
            extract_path_from_url(
                "https://api.example.com/users/123",
                Some("https://api.example.com")
            )
            .unwrap(),
            "/users/123"
        );

        // Test with query parameters
        assert_eq!(
            extract_path_from_url(
                "https://api.example.com/search?q=test&page=1",
                Some("https://api.example.com")
            )
            .unwrap(),
            "/search?q=test&page=1"
        );

        // Test without base URL
        assert_eq!(extract_path_from_url("https://api.example.com/users", None).unwrap(), "/users");
    }

    #[test]
    fn test_extract_request_body() {
        let request_with_text = HarRequest {
            method: "POST".to_string(),
            url: "https://api.example.com/users".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: vec![],
            headers: vec![],
            query_string: vec![],
            post_data: Some(HarPostData {
                mime_type: "application/json".to_string(),
                params: vec![],
                text: Some(r#"{"name": "John"}"#.to_string()),
            }),
            headers_size: 100,
            body_size: 16,
        };

        assert_eq!(
            extract_request_body(&request_with_text),
            Some(r#"{"name": "John"}"#.to_string())
        );
    }

    #[test]
    fn test_extract_response_body() {
        let response_with_json = HarResponse {
            status: 200,
            status_text: "OK".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: vec![],
            headers: vec![],
            content: HarContent {
                size: 25,
                compression: None,
                mime_type: "application/json".to_string(),
                text: Some(r#"{"message": "success"}"#.to_string()),
                encoding: None,
            },
            redirect_url: "".to_string(),
            headers_size: 100,
            body_size: 25,
        };

        let body = extract_response_body(&response_with_json);
        assert_eq!(body, json!({"message": "success"}));
    }

    #[test]
    fn test_parse_har_with_multiple_entries() {
        let har_json = r#"{
            "log": {
                "version": "1.2",
                "creator": {
                    "name": "Test Creator",
                    "version": "1.0"
                },
                "entries": [
                    {
                        "startedDateTime": "2024-01-15T10:30:00Z",
                        "time": 123.45,
                        "request": {
                            "method": "GET",
                            "url": "https://api.example.com/users",
                            "httpVersion": "HTTP/1.1",
                            "cookies": [],
                            "headers": [],
                            "queryString": [],
                            "headersSize": 100,
                            "bodySize": 0
                        },
                        "response": {
                            "status": 200,
                            "statusText": "OK",
                            "httpVersion": "HTTP/1.1",
                            "cookies": [],
                            "headers": [],
                            "content": {
                                "size": 25,
                                "mimeType": "application/json",
                                "text": "{\"users\": []}"
                            },
                            "redirectURL": "",
                            "headersSize": 100,
                            "bodySize": 25
                        },
                        "cache": {},
                        "timings": {}
                    },
                    {
                        "startedDateTime": "2024-01-15T10:31:00Z",
                        "time": 234.56,
                        "request": {
                            "method": "POST",
                            "url": "https://api.example.com/users",
                            "httpVersion": "HTTP/1.1",
                            "cookies": [],
                            "headers": [],
                            "queryString": [],
                            "headersSize": 100,
                            "bodySize": 16
                        },
                        "response": {
                            "status": 201,
                            "statusText": "Created",
                            "httpVersion": "HTTP/1.1",
                            "cookies": [],
                            "headers": [],
                            "content": {
                                "size": 25,
                                "mimeType": "application/json",
                                "text": "{\"id\": 123}"
                            },
                            "redirectURL": "",
                            "headersSize": 100,
                            "bodySize": 25
                        },
                        "cache": {},
                        "timings": {}
                    }
                ]
            }
        }"#;

        let result = import_har_archive(har_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 2);

        // Check first route (GET)
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/users");
        assert_eq!(result.routes[0].response.status, 200);

        // Check second route (POST)
        assert_eq!(result.routes[1].method, "POST");
        assert_eq!(result.routes[1].path, "/users");
        assert_eq!(result.routes[1].response.status, 201);
    }

    #[test]
    fn test_parse_har_with_query_parameters() {
        let har_json = r#"{
            "log": {
                "version": "1.2",
                "creator": {"name": "Test", "version": "1.0"},
                "entries": [{
                    "startedDateTime": "2024-01-15T10:30:00Z",
                    "time": 123.45,
                    "request": {
                        "method": "GET",
                        "url": "https://api.example.com/search?q=test&page=1&limit=10",
                        "httpVersion": "HTTP/1.1",
                        "cookies": [],
                        "headers": [],
                        "queryString": [
                            {"name": "q", "value": "test"},
                            {"name": "page", "value": "1"},
                            {"name": "limit", "value": "10"}
                        ],
                        "headersSize": 100,
                        "bodySize": 0
                    },
                    "response": {
                        "status": 200,
                        "statusText": "OK",
                        "httpVersion": "HTTP/1.1",
                        "cookies": [],
                        "headers": [],
                        "content": {"size": 25, "mimeType": "application/json", "text": "{}"},
                        "redirectURL": "",
                        "headersSize": 100,
                        "bodySize": 25
                    },
                    "cache": {},
                    "timings": {}
                }]
            }
        }"#;

        let result = import_har_archive(har_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "GET");
        assert_eq!(result.routes[0].path, "/search?q=test&page=1&limit=10");
    }

    #[test]
    fn test_parse_har_with_post_data() {
        let har_json = r#"{
            "log": {
                "version": "1.2",
                "creator": {"name": "Test", "version": "1.0"},
                "entries": [{
                    "startedDateTime": "2024-01-15T10:30:00Z",
                    "time": 123.45,
                    "request": {
                        "method": "POST",
                        "url": "https://api.example.com/users",
                        "httpVersion": "HTTP/1.1",
                        "cookies": [],
                        "headers": [],
                        "queryString": [],
                        "postData": {
                            "mimeType": "application/json",
                            "params": [],
                            "text": "{\"name\": \"John\", \"age\": 30}"
                        },
                        "headersSize": 100,
                        "bodySize": 30
                    },
                    "response": {
                        "status": 201,
                        "statusText": "Created",
                        "httpVersion": "HTTP/1.1",
                        "cookies": [],
                        "headers": [],
                        "content": {"size": 25, "mimeType": "application/json", "text": "{}"},
                        "redirectURL": "",
                        "headersSize": 100,
                        "bodySize": 25
                    },
                    "cache": {},
                    "timings": {}
                }]
            }
        }"#;

        let result = import_har_archive(har_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "POST");
        assert_eq!(result.routes[0].path, "/users");
        assert_eq!(result.routes[0].body, Some("{\"name\": \"John\", \"age\": 30}".to_string()));
    }

    #[test]
    fn test_parse_har_with_form_data() {
        let har_json = r#"{
            "log": {
                "version": "1.2",
                "creator": {"name": "Test", "version": "1.0"},
                "entries": [{
                    "startedDateTime": "2024-01-15T10:30:00Z",
                    "time": 123.45,
                    "request": {
                        "method": "POST",
                        "url": "https://api.example.com/form",
                        "httpVersion": "HTTP/1.1",
                        "cookies": [],
                        "headers": [],
                        "queryString": [],
                        "postData": {
                            "mimeType": "application/x-www-form-urlencoded",
                            "params": [
                                {"name": "username", "value": "john_doe"},
                                {"name": "password", "value": "secret123"},
                                {"name": "remember", "value": "true"}
                            ],
                            "text": null
                        },
                        "headersSize": 100,
                        "bodySize": 50
                    },
                    "response": {
                        "status": 200,
                        "statusText": "OK",
                        "httpVersion": "HTTP/1.1",
                        "cookies": [],
                        "headers": [],
                        "content": {"size": 25, "mimeType": "application/json", "text": "{}"},
                        "redirectURL": "",
                        "headersSize": 100,
                        "bodySize": 25
                    },
                    "cache": {},
                    "timings": {}
                }]
            }
        }"#;

        let result = import_har_archive(har_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 1);
        assert_eq!(result.routes[0].method, "POST");
        assert_eq!(result.routes[0].path, "/form");
        assert_eq!(
            result.routes[0].body,
            Some("username=john_doe&password=secret123&remember=true".to_string())
        );
    }

    #[test]
    fn test_extract_path_from_url_edge_cases() {
        // Test with various URL formats
        let test_cases = vec![
            (
                "https://api.example.com/users/123",
                Some("https://api.example.com"),
                "/users/123",
            ),
            (
                "https://api.example.com/users/123/details",
                Some("https://api.example.com"),
                "/users/123/details",
            ),
            (
                "https://api.example.com/search?q=test",
                Some("https://api.example.com"),
                "/search?q=test",
            ),
            (
                "https://api.example.com/search?q=test&page=1",
                Some("https://api.example.com"),
                "/search?q=test&page=1",
            ),
            ("https://api.example.com/", Some("https://api.example.com"), "/"),
            ("https://api.example.com", Some("https://api.example.com"), "/"),
            ("https://api.example.com/users", None, "/users"),
            ("https://api.example.com/users/", None, "/users/"),
            ("http://localhost:9080/api/test", Some("http://localhost:9080"), "/api/test"),
            (
                "https://subdomain.example.com/api/v1/test",
                Some("https://subdomain.example.com"),
                "/api/v1/test",
            ),
        ];

        for (url, base_url, expected) in test_cases {
            let result = extract_path_from_url(url, base_url);
            assert_eq!(result.unwrap(), expected, "Failed for URL: {}, base: {:?}", url, base_url);
        }
    }

    #[test]
    fn test_extract_request_body_comprehensive() {
        // Test with JSON data
        let request_with_json = HarRequest {
            method: "POST".to_string(),
            url: "https://api.example.com/users".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: vec![],
            headers: vec![],
            query_string: vec![],
            post_data: Some(HarPostData {
                mime_type: "application/json".to_string(),
                params: vec![],
                text: Some(r#"{"name": "John", "age": 30}"#.to_string()),
            }),
            headers_size: 100,
            body_size: 30,
        };
        assert_eq!(
            extract_request_body(&request_with_json),
            Some(r#"{"name": "John", "age": 30}"#.to_string())
        );

        // Test with form parameters
        let request_with_form = HarRequest {
            method: "POST".to_string(),
            url: "https://api.example.com/form".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: vec![],
            headers: vec![],
            query_string: vec![],
            post_data: Some(HarPostData {
                mime_type: "application/x-www-form-urlencoded".to_string(),
                params: vec![
                    HarParam {
                        name: "username".to_string(),
                        value: Some("john_doe".to_string()),
                        file_name: None,
                        content_type: None,
                    },
                    HarParam {
                        name: "password".to_string(),
                        value: Some("secret123".to_string()),
                        file_name: None,
                        content_type: None,
                    },
                    HarParam {
                        name: "remember".to_string(),
                        value: Some("true".to_string()),
                        file_name: None,
                        content_type: None,
                    },
                ],
                text: None,
            }),
            headers_size: 100,
            body_size: 50,
        };
        assert_eq!(
            extract_request_body(&request_with_form),
            Some("username=john_doe&password=secret123&remember=true".to_string())
        );

        // Test with empty params
        let request_with_empty_params = HarRequest {
            method: "POST".to_string(),
            url: "https://api.example.com/form".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: vec![],
            headers: vec![],
            query_string: vec![],
            post_data: Some(HarPostData {
                mime_type: "application/x-www-form-urlencoded".to_string(),
                params: vec![],
                text: Some("".to_string()),
            }),
            headers_size: 100,
            body_size: 0,
        };
        assert_eq!(extract_request_body(&request_with_empty_params), None);

        // Test with no post data
        let request_no_body = HarRequest {
            method: "GET".to_string(),
            url: "https://api.example.com/users".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: vec![],
            headers: vec![],
            query_string: vec![],
            post_data: None,
            headers_size: 100,
            body_size: 0,
        };
        assert_eq!(extract_request_body(&request_no_body), None);
    }

    #[test]
    fn test_extract_response_body_comprehensive() {
        // Test with valid JSON
        let response_with_json = HarResponse {
            status: 200,
            status_text: "OK".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: vec![],
            headers: vec![],
            content: HarContent {
                size: 25,
                compression: None,
                mime_type: "application/json".to_string(),
                text: Some(r#"{"message": "success"}"#.to_string()),
                encoding: None,
            },
            redirect_url: "".to_string(),
            headers_size: 100,
            body_size: 25,
        };
        assert_eq!(extract_response_body(&response_with_json), json!({"message": "success"}));

        // Test with invalid JSON (should return as string)
        let response_with_invalid_json = HarResponse {
            status: 200,
            status_text: "OK".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: vec![],
            headers: vec![],
            content: HarContent {
                size: 15,
                compression: None,
                mime_type: "text/plain".to_string(),
                text: Some("not json".to_string()),
                encoding: None,
            },
            redirect_url: "".to_string(),
            headers_size: 100,
            body_size: 15,
        };
        assert_eq!(extract_response_body(&response_with_invalid_json), json!("not json"));

        // Test with empty text
        let response_with_empty_text = HarResponse {
            status: 204,
            status_text: "No Content".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: vec![],
            headers: vec![],
            content: HarContent {
                size: 0,
                compression: None,
                mime_type: "application/json".to_string(),
                text: Some("".to_string()),
                encoding: None,
            },
            redirect_url: "".to_string(),
            headers_size: 100,
            body_size: 0,
        };
        assert_eq!(
            extract_response_body(&response_with_empty_text),
            json!({"message": "Mock response from HAR import"})
        );

        // Test with no text
        let response_with_no_text = HarResponse {
            status: 500,
            status_text: "Internal Server Error".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: vec![],
            headers: vec![],
            content: HarContent {
                size: 0,
                compression: None,
                mime_type: "application/json".to_string(),
                text: None,
                encoding: None,
            },
            redirect_url: "".to_string(),
            headers_size: 100,
            body_size: 0,
        };
        assert_eq!(
            extract_response_body(&response_with_no_text),
            json!({"message": "Mock response from HAR import"})
        );

        // Test with complex JSON
        let response_with_complex_json = HarResponse {
            status: 200,
            status_text: "OK".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: vec![],
            headers: vec![],
            content: HarContent {
                size: 100,
                compression: None,
                mime_type: "application/json".to_string(),
                text: Some(r#"{"users": [{"id": 1, "name": "John"}, {"id": 2, "name": "Jane"}], "total": 2}"#.to_string()),
                encoding: None,
            },
            redirect_url: "".to_string(),
            headers_size: 100,
            body_size: 100,
        };
        let expected =
            json!({"users": [{"id": 1, "name": "John"}, {"id": 2, "name": "Jane"}], "total": 2});
        assert_eq!(extract_response_body(&response_with_complex_json), expected);
    }

    #[test]
    fn test_parse_har_with_different_status_codes() {
        let har_json = r#"{
            "log": {
                "version": "1.2",
                "creator": {"name": "Test", "version": "1.0"},
                "entries": [
                    {
                        "startedDateTime": "2024-01-15T10:30:00Z",
                        "time": 123.45,
                        "request": {"method": "GET", "url": "https://api.example.com/test", "httpVersion": "HTTP/1.1", "cookies": [], "headers": [], "queryString": [], "headersSize": 100, "bodySize": 0},
                        "response": {"status": 404, "statusText": "Not Found", "httpVersion": "HTTP/1.1", "cookies": [], "headers": [], "content": {"size": 0, "mimeType": "application/json", "text": null}, "redirectURL": "", "headersSize": 100, "bodySize": 0},
                        "cache": {}, "timings": {}
                    },
                    {
                        "startedDateTime": "2024-01-15T10:31:00Z",
                        "time": 234.56,
                        "request": {"method": "POST", "url": "https://api.example.com/test", "httpVersion": "HTTP/1.1", "cookies": [], "headers": [], "queryString": [], "headersSize": 100, "bodySize": 0},
                        "response": {"status": 500, "statusText": "Internal Server Error", "httpVersion": "HTTP/1.1", "cookies": [], "headers": [], "content": {"size": 0, "mimeType": "application/json", "text": null}, "redirectURL": "", "headersSize": 100, "bodySize": 0},
                        "cache": {}, "timings": {}
                    }
                ]
            }
        }"#;

        let result = import_har_archive(har_json, Some("https://api.example.com")).unwrap();

        assert_eq!(result.routes.len(), 2);
        assert_eq!(result.routes[0].response.status, 404);
        assert_eq!(result.routes[1].response.status, 500);
    }

    #[test]
    fn test_parse_har_with_invalid_json() {
        let invalid_har_json = r#"{
            "log": {
                "version": "1.2",
                "creator": {"name": "Test", "version": "1.0"},
                "entries": [
                    {
                        "startedDateTime": "2024-01-15T10:30:00Z",
                        "time": "invalid_number",
                        "request": {"method": "GET", "url": "https://api.example.com/test", "httpVersion": "HTTP/1.1"},
                        "response": {"status": 200, "statusText": "OK", "httpVersion": "HTTP/1.1"},
                        "cache": {},
                        "timings": {}
                    }
                ]
            }
        }"#;

        let result = import_har_archive(invalid_har_json, Some("https://api.example.com"));
        assert!(result.is_err());
    }
}
