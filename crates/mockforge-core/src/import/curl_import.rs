//! Curl command import functionality
//!
//! This module handles parsing curl commands and converting them
//! to MockForge routes.

use regex::Regex;
use serde_json::json;
use std::collections::HashMap;

/// Parsed curl command components
#[derive(Debug)]
pub struct ParsedCurlCommand {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// MockForge route structure for curl import (similar to postman_import.rs)
#[derive(Debug, serde::Serialize)]
pub struct MockForgeRoute {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub response: MockForgeResponse,
}

/// MockForge response structure
#[derive(Debug, serde::Serialize)]
pub struct MockForgeResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: serde_json::Value,
}

/// Import result for curl commands
pub struct CurlImportResult {
    pub routes: Vec<MockForgeRoute>,
    pub warnings: Vec<String>,
}

/// Import curl command(s)
pub fn import_curl_commands(
    content: &str,
    base_url: Option<&str>,
) -> Result<CurlImportResult, String> {
    let mut routes = Vec::new();
    let mut warnings = Vec::new();

    // Split content into individual curl commands (one per line, or handle multi-line)
    let commands = split_curl_commands(content);

    for (i, command) in commands.into_iter().enumerate() {
        let trimmed = command.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue; // Skip empty lines and comments
        }

        match parse_curl_command(trimmed) {
            Ok(parsed) => match convert_curl_to_route(parsed, base_url) {
                Ok(route) => routes.push(route),
                Err(e) => warnings.push(format!("Failed to convert curl command {}: {}", i + 1, e)),
            },
            Err(e) => {
                warnings.push(format!("Failed to parse curl command {}: {}", i + 1, e));
            }
        }
    }

    Ok(CurlImportResult { routes, warnings })
}

/// Split content into individual curl commands
fn split_curl_commands(content: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut current_command = String::new();
    let mut in_quotes = false;
    let mut quote_char = '\0';
    let mut escaped = false;

    for ch in content.chars() {
        match ch {
            '"' | '\'' if !escaped => {
                if !in_quotes {
                    in_quotes = true;
                    quote_char = ch;
                } else if ch == quote_char {
                    in_quotes = false;
                    quote_char = '\0';
                }
            }
            '\\' if !escaped => {
                escaped = true;
            }
            '\n' if !in_quotes && !escaped => {
                let cmd = current_command.trim().to_string();
                if !cmd.is_empty() {
                    commands.push(cmd);
                }
                current_command.clear();
            }
            _ => {
                escaped = false;
                current_command.push(ch);
            }
        }
    }

    // Add the last command if any
    let cmd = current_command.trim().to_string();
    if !cmd.is_empty() {
        commands.push(cmd);
    }

    commands
}

/// Parse a single curl command
fn parse_curl_command(command: &str) -> Result<ParsedCurlCommand, String> {
    let mut method = "GET".to_string();
    let mut url = String::new();
    let mut headers = HashMap::new();
    let mut body = None;

    // Simple curl command parser using regex
    // This handles basic curl syntax: curl [options] URL

    // Extract URL first (usually the last argument)
    let url_regex = Regex::new(r#"(?:^|\s)((?:https?://|http://|www\.)[^\s"']+)"#)
        .map_err(|e| format!("Regex error: {}", e))?;

    if let Some(captures) = url_regex.captures(command) {
        if let Some(url_match) = captures.get(1) {
            url = url_match.as_str().to_string();
        }
    }

    if url.is_empty() {
        return Err("No URL found in curl command".to_string());
    }

    // Extract method from -X flag
    let method_regex = Regex::new(r#"-X\s+(\w+)"#).map_err(|e| format!("Regex error: {}", e))?;

    if let Some(captures) = method_regex.captures(command) {
        if let Some(method_match) = captures.get(1) {
            method = method_match.as_str().to_uppercase();
        }
    }

    // Extract headers from -H flags
    let header_regex =
        Regex::new(r#"-H\s+["']([^"']+)["']"#).map_err(|e| format!("Regex error: {}", e))?;

    for captures in header_regex.captures_iter(command) {
        if let Some(header_match) = captures.get(1) {
            let header_str = header_match.as_str();
            if let Some(colon_pos) = header_str.find(':') {
                let key = header_str[..colon_pos].trim();
                let value = header_str[colon_pos + 1..].trim();
                headers.insert(key.to_string(), value.to_string());
            }
        }
    }

    // Extract body from -d or --data flags
    let body_regex = Regex::new(r#"(?:-d|--data)\s+["']([^"']*)["']"#)
        .map_err(|e| format!("Regex error: {}", e))?;

    if let Some(captures) = body_regex.captures(command) {
        if let Some(body_match) = captures.get(1) {
            body = Some(body_match.as_str().to_string());
        }
    }

    Ok(ParsedCurlCommand {
        method,
        url,
        headers,
        body,
    })
}

/// Convert parsed curl command to MockForge route
fn convert_curl_to_route(
    parsed: ParsedCurlCommand,
    base_url: Option<&str>,
) -> Result<MockForgeRoute, String> {
    // Extract path from URL
    let path = extract_path_from_url(&parsed.url, base_url)?;

    // Generate mock response based on method
    let response = generate_mock_response(&parsed.method);

    Ok(MockForgeRoute {
        method: parsed.method,
        path,
        headers: parsed.headers,
        body: parsed.body,
        response,
    })
}

/// Extract path from URL
fn extract_path_from_url(url: &str, base_url: Option<&str>) -> Result<String, String> {
    // If base_url is provided, make path relative
    if let Some(base) = base_url {
        if url.starts_with(base) {
            let path = url.trim_start_matches(base).trim_start_matches('/');
            return Ok(if path.is_empty() {
                "/".to_string()
            } else {
                format!("/{}", path)
            });
        }
    }

    // Parse URL to extract path
    if let Ok(parsed_url) = url::Url::parse(url) {
        let path = parsed_url.path();
        if path.is_empty() || path == "/" {
            Ok("/".to_string())
        } else {
            Ok(path.to_string())
        }
    } else {
        Err(format!("Invalid URL: {}", url))
    }
}

/// Generate mock response based on HTTP method
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
    fn test_parse_simple_curl() {
        let command = "curl https://api.example.com/users";
        let parsed = parse_curl_command(command).unwrap();

        assert_eq!(parsed.method, "GET");
        assert_eq!(parsed.url, "https://api.example.com/users");
        assert!(parsed.headers.is_empty());
        assert!(parsed.body.is_none());
    }

    #[test]
    fn test_parse_curl_with_method() {
        let command = "curl -X POST https://api.example.com/users";
        let parsed = parse_curl_command(command).unwrap();

        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.url, "https://api.example.com/users");
    }

    #[test]
    fn test_parse_curl_with_headers() {
        let command = "curl -H 'Authorization: Bearer token' -H 'Content-Type: application/json' https://api.example.com/users";
        let parsed = parse_curl_command(command).unwrap();

        assert_eq!(parsed.headers.get("Authorization"), Some(&"Bearer token".to_string()));
        assert_eq!(parsed.headers.get("Content-Type"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_parse_curl_with_body() {
        let command = "curl -X POST -d '{\"name\":\"John\"}' https://api.example.com/users";
        let parsed = parse_curl_command(command).unwrap();

        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.body, Some("{\"name\":\"John\"}".to_string()));
    }

    #[test]
    fn test_split_curl_commands() {
        let content = r#"curl https://api.example.com/users
curl -X POST https://api.example.com/users -d '{"name":"John"}'
# This is a comment
curl -H 'Auth: token' https://api.example.com/data"#;

        let commands = split_curl_commands(content);
        assert_eq!(commands.len(), 3);
        assert!(commands[0].contains("users"));
        assert!(commands[1].contains("POST"));
        assert!(commands[2].contains("data"));
    }

    #[test]
    fn test_import_curl_commands() {
        let content = "curl -X POST https://api.example.com/users -H 'Content-Type: application/json' -d '{\"name\":\"John\"}'";

        let result = import_curl_commands(content, Some("https://api.example.com")).unwrap();
        assert_eq!(result.routes.len(), 1);

        let route = &result.routes[0];
        assert_eq!(route.method, "POST");
        assert_eq!(route.path, "/users");
        assert_eq!(route.headers.get("Content-Type"), Some(&"application/json".to_string()));
        assert_eq!(route.body, Some("{\"name\":\"John\"}".to_string()));
    }
}
