//! HAR-to-YAML generator for custom conformance checks
//!
//! Converts HTTP Archive (HAR) files into YAML configuration files that match
//! the `--conformance-custom` format, enabling users to generate conformance
//! checks from recorded traffic.

use crate::error::{BenchError, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// ---------------------------------------------------------------------------
// Minimal HAR deserialization types (local to avoid circular deps)
// ---------------------------------------------------------------------------

/// Top-level HAR archive
#[derive(Debug, Deserialize)]
pub struct HarArchive {
    /// The HAR log
    pub log: HarLog,
}

/// HAR log containing entries
#[derive(Debug, Deserialize)]
pub struct HarLog {
    /// Recorded HTTP entries
    pub entries: Vec<HarEntry>,
}

/// A single HAR entry (request + response pair)
#[derive(Debug, Deserialize)]
pub struct HarEntry {
    /// The outgoing request
    pub request: HarRequest,
    /// The received response
    pub response: HarResponse,
}

/// HAR request
#[derive(Debug, Deserialize)]
pub struct HarRequest {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Full request URL
    pub url: String,
    /// Request headers
    #[serde(default)]
    pub headers: Vec<HarHeader>,
}

/// HAR response
#[derive(Debug, Deserialize)]
pub struct HarResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    #[serde(default)]
    pub headers: Vec<HarHeader>,
    /// Response body content
    #[serde(default)]
    pub content: Option<HarContent>,
}

/// A single HTTP header
#[derive(Debug, Deserialize)]
pub struct HarHeader {
    /// Header name
    pub name: String,
    /// Header value
    pub value: String,
}

/// Response body content
#[derive(Debug, Deserialize)]
pub struct HarContent {
    /// MIME type
    #[serde(rename = "mimeType", default)]
    pub mime_type: Option<String>,
    /// Body text
    #[serde(default)]
    pub text: Option<String>,
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options controlling HAR-to-YAML conversion
#[derive(Debug, Clone)]
pub struct HarToCustomOptions {
    /// Base URL to strip from entry URLs. If `None`, auto-detected from the
    /// first entry's scheme + host + port.
    pub base_url: Option<String>,
    /// Skip entries whose path ends with common static-asset extensions
    /// (.js, .css, .png, .jpg, .gif, .svg, .ico, .woff, .woff2, .ttf, .map).
    pub skip_static: bool,
    /// Only include these response headers in the generated checks.
    /// If empty, no header checks are generated.
    pub include_headers: Vec<String>,
    /// Maximum number of entries to process (0 = unlimited).
    pub max_entries: usize,
}

impl Default for HarToCustomOptions {
    fn default() -> Self {
        Self {
            base_url: None,
            skip_static: true,
            include_headers: Vec::new(),
            max_entries: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Output types (serialize to YAML matching CustomConformanceConfig)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct OutputConfig {
    custom_checks: Vec<OutputCheck>,
}

#[derive(Debug, Serialize)]
struct OutputCheck {
    name: String,
    path: String,
    method: String,
    expected_status: u16,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    expected_headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    expected_body_fields: Vec<OutputBodyField>,
}

#[derive(Debug, Serialize)]
struct OutputBodyField {
    name: String,
    #[serde(rename = "type")]
    field_type: String,
}

// ---------------------------------------------------------------------------
// Static-asset extensions
// ---------------------------------------------------------------------------

const STATIC_EXTENSIONS: &[&str] = &[
    ".js", ".css", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".woff", ".woff2", ".ttf",
    ".map", ".eot",
];

// ---------------------------------------------------------------------------
// Hop-by-hop / noise headers to always skip
// ---------------------------------------------------------------------------

const SKIP_HEADERS: &[&str] = &[
    "connection",
    "transfer-encoding",
    "date",
    "server",
    "content-length",
    "vary",
    "x-request-id",
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate a YAML string (matching `--conformance-custom` format) from a HAR file.
pub fn generate_custom_yaml_from_har(
    har_path: &Path,
    options: HarToCustomOptions,
) -> Result<String> {
    let raw = std::fs::read_to_string(har_path).map_err(|e| {
        BenchError::Other(format!("Failed to read HAR file '{}': {}", har_path.display(), e))
    })?;

    let archive: HarArchive = serde_json::from_str(&raw).map_err(|e| {
        BenchError::Other(format!("Failed to parse HAR file '{}': {}", har_path.display(), e))
    })?;

    generate_custom_yaml(&archive, &options)
}

/// Core generation logic, separated for testability.
fn generate_custom_yaml(archive: &HarArchive, options: &HarToCustomOptions) -> Result<String> {
    // Auto-detect base URL from first entry if not provided
    let base_url = match &options.base_url {
        Some(url) => url.trim_end_matches('/').to_string(),
        None => detect_base_url(&archive.log.entries)?,
    };

    let header_matchers = build_header_matchers(&options.include_headers);

    let mut checks = Vec::new();

    for entry in &archive.log.entries {
        if options.max_entries > 0 && checks.len() >= options.max_entries {
            break;
        }

        let path = extract_path(&entry.request.url, &base_url);

        // Skip static assets if requested
        if options.skip_static && is_static_asset(&path) {
            continue;
        }

        let method = entry.request.method.to_uppercase();

        // Build expected_headers (filtered)
        let mut expected_headers = HashMap::new();
        if !header_matchers.is_empty() {
            for h in &entry.response.headers {
                let lower = h.name.to_lowercase();
                if SKIP_HEADERS.contains(&lower.as_str()) {
                    continue;
                }
                if header_matches(&lower, &header_matchers) {
                    // Escape regex special chars in the value for a literal match
                    expected_headers.insert(h.name.clone(), regex_escape(&h.value));
                }
            }
        }

        // Extract body fields from JSON response
        let expected_body_fields = extract_body_fields(entry);

        // Build a human-readable check name
        let slug = path.replace('/', "-").trim_matches('-').to_string();
        let name =
            format!("custom:har:{}-{}-{}", method.to_lowercase(), slug, entry.response.status);

        checks.push(OutputCheck {
            name,
            path,
            method,
            expected_status: entry.response.status,
            expected_headers,
            expected_body_fields,
        });
    }

    let config = OutputConfig {
        custom_checks: checks,
    };

    serde_yaml::to_string(&config)
        .map_err(|e| BenchError::Other(format!("Failed to serialize YAML: {}", e)))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Detect the base URL (scheme + host + port) from the first entry.
fn detect_base_url(entries: &[HarEntry]) -> Result<String> {
    let first = entries
        .first()
        .ok_or_else(|| BenchError::Other("HAR file contains no entries".to_string()))?;

    let parsed = url::Url::parse(&first.request.url).map_err(|e| {
        BenchError::Other(format!("Failed to parse URL '{}': {}", first.request.url, e))
    })?;

    let mut base = format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or("localhost"));
    if let Some(port) = parsed.port() {
        base.push_str(&format!(":{}", port));
    }
    Ok(base)
}

/// Strip the base URL from a full URL to get the path (with query string).
fn extract_path(full_url: &str, base_url: &str) -> String {
    if let Some(rest) = full_url.strip_prefix(base_url) {
        if rest.is_empty() {
            "/".to_string()
        } else if rest.starts_with('/') {
            // Strip query string — conformance checks match on path only
            rest.split('?').next().unwrap_or(rest).to_string()
        } else {
            format!("/{}", rest.split('?').next().unwrap_or(rest))
        }
    } else {
        // Fallback: parse URL and use the path component
        match url::Url::parse(full_url) {
            Ok(parsed) => parsed.path().to_string(),
            Err(_) => full_url.to_string(),
        }
    }
}

fn is_static_asset(path: &str) -> bool {
    let lower = path.to_lowercase();
    STATIC_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Regex metacharacters that, when present in a header pattern, indicate it
/// should be compiled as a regex rather than matched as a literal string.
const REGEX_META: &[char] = &['*', '+', '?', '[', '|', '^', '$', '.'];

/// A compiled header matcher — either a regex pattern or an exact lowercase string.
enum HeaderMatcher {
    Regex(Regex),
    Exact(String),
}

/// Build header matchers from the user-supplied include-headers list.
/// Each entry is treated as a regex if it contains any regex metacharacter,
/// otherwise as an exact case-insensitive match.
fn build_header_matchers(include_headers: &[String]) -> Vec<HeaderMatcher> {
    include_headers
        .iter()
        .map(|h| {
            let lower = h.to_lowercase();
            if lower.contains(REGEX_META) {
                // Anchor the pattern so it must match the full header name
                let anchored = format!("^(?:{})$", lower);
                match Regex::new(&anchored) {
                    Ok(re) => HeaderMatcher::Regex(re),
                    // If the regex is invalid, fall back to exact match
                    Err(_) => HeaderMatcher::Exact(lower),
                }
            } else {
                HeaderMatcher::Exact(lower)
            }
        })
        .collect()
}

/// Check whether a lowercase header name matches any of the matchers.
fn header_matches(lower_name: &str, matchers: &[HeaderMatcher]) -> bool {
    matchers.iter().any(|m| match m {
        HeaderMatcher::Exact(exact) => lower_name == exact,
        HeaderMatcher::Regex(re) => re.is_match(lower_name),
    })
}

/// Escape regex metacharacters so the value is matched literally.
fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        if "\\^$.|?*+()[]{}".contains(ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Maximum recursion depth for nested body field extraction.
const MAX_BODY_FIELD_DEPTH: usize = 3;

/// Extract field names + JSON types from the response body (if JSON).
///
/// Recursively descends into nested objects and arrays (up to
/// [`MAX_BODY_FIELD_DEPTH`] levels) producing dot-notation paths:
///   - Nested objects: `parent.child`
///   - Arrays of objects: inspects the first element and uses `parent[].child`
fn extract_body_fields(entry: &HarEntry) -> Vec<OutputBodyField> {
    let content = match &entry.response.content {
        Some(c) => c,
        None => return Vec::new(),
    };

    // Only process JSON responses
    let mime = content.mime_type.as_deref().unwrap_or("");
    if !mime.contains("json") {
        return Vec::new();
    }

    let text = match &content.text {
        Some(t) if !t.is_empty() => t,
        _ => return Vec::new(),
    };

    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut fields = Vec::new();
    collect_body_fields(&value, "", &mut fields, 0);
    fields
}

/// Recursively collect body fields from a JSON value.
///
/// `prefix` is the dot-notation path accumulated so far (empty at root).
/// `depth` tracks the current recursion level (0 at root).
fn collect_body_fields(
    value: &serde_json::Value,
    prefix: &str,
    out: &mut Vec<OutputBodyField>,
    depth: usize,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let name = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                out.push(OutputBodyField {
                    name: name.clone(),
                    field_type: json_type_name(v),
                });
                // Recurse into nested objects/arrays if within depth limit
                if depth < MAX_BODY_FIELD_DEPTH {
                    match v {
                        serde_json::Value::Object(_) => {
                            collect_body_fields(v, &name, out, depth + 1);
                        }
                        serde_json::Value::Array(arr) => {
                            // Inspect first element of arrays
                            if let Some(serde_json::Value::Object(_)) = arr.first() {
                                let arr_prefix = format!("{}[]", name);
                                collect_body_fields(
                                    arr.first().unwrap(),
                                    &arr_prefix,
                                    out,
                                    depth + 1,
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        // Array: inspect the first element
        serde_json::Value::Array(arr) => {
            if let Some(serde_json::Value::Object(_)) = arr.first() {
                // For top-level arrays (empty prefix), extract fields without a
                // prefix to maintain backward compatibility. For nested arrays
                // the caller already appended `[]` to the prefix before recursing.
                collect_body_fields(arr.first().unwrap(), prefix, out, depth);
            }
        }
        _ => {}
    }
}

fn json_type_name(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer".to_string()
            } else {
                "number".to_string()
            }
        }
        serde_json::Value::Bool(_) => "boolean".to_string(),
        serde_json::Value::Array(_) => "array".to_string(),
        serde_json::Value::Object(_) => "object".to_string(),
        serde_json::Value::Null => "string".to_string(), // fallback
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_har() -> HarArchive {
        HarArchive {
            log: HarLog {
                entries: vec![
                    HarEntry {
                        request: HarRequest {
                            method: "GET".to_string(),
                            url: "http://localhost:3000/api/users".to_string(),
                            headers: vec![],
                        },
                        response: HarResponse {
                            status: 200,
                            headers: vec![
                                HarHeader {
                                    name: "content-type".to_string(),
                                    value: "application/json".to_string(),
                                },
                                HarHeader {
                                    name: "x-request-id".to_string(),
                                    value: "abc-123".to_string(),
                                },
                            ],
                            content: Some(HarContent {
                                mime_type: Some("application/json".to_string()),
                                text: Some(
                                    r#"[{"id": 1, "name": "Alice", "active": true}]"#.to_string(),
                                ),
                            }),
                        },
                    },
                    HarEntry {
                        request: HarRequest {
                            method: "POST".to_string(),
                            url: "http://localhost:3000/api/users".to_string(),
                            headers: vec![],
                        },
                        response: HarResponse {
                            status: 201,
                            headers: vec![HarHeader {
                                name: "content-type".to_string(),
                                value: "application/json".to_string(),
                            }],
                            content: Some(HarContent {
                                mime_type: Some("application/json".to_string()),
                                text: Some(r#"{"id": 2, "name": "Bob"}"#.to_string()),
                            }),
                        },
                    },
                    HarEntry {
                        request: HarRequest {
                            method: "GET".to_string(),
                            url: "http://localhost:3000/static/app.js".to_string(),
                            headers: vec![],
                        },
                        response: HarResponse {
                            status: 200,
                            headers: vec![],
                            content: None,
                        },
                    },
                ],
            },
        }
    }

    #[test]
    fn test_basic_generation() {
        let har = sample_har();
        let options = HarToCustomOptions {
            skip_static: true,
            ..Default::default()
        };
        let yaml = generate_custom_yaml(&har, &options).unwrap();

        // Should contain 2 checks (static .js skipped)
        let config: super::super::custom::CustomConformanceConfig =
            serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.custom_checks.len(), 2);
        assert_eq!(config.custom_checks[0].method, "GET");
        assert_eq!(config.custom_checks[0].path, "/api/users");
        assert_eq!(config.custom_checks[0].expected_status, 200);
        assert_eq!(config.custom_checks[1].method, "POST");
        assert_eq!(config.custom_checks[1].expected_status, 201);
    }

    #[test]
    fn test_body_field_extraction() {
        let har = sample_har();
        let options = HarToCustomOptions::default();
        let yaml = generate_custom_yaml(&har, &options).unwrap();

        let config: super::super::custom::CustomConformanceConfig =
            serde_yaml::from_str(&yaml).unwrap();

        // First entry is an array — should extract fields from the first element
        let fields = &config.custom_checks[0].expected_body_fields;
        assert_eq!(fields.len(), 3);
        assert!(fields.iter().any(|f| f.name == "id" && f.field_type == "integer"));
        assert!(fields.iter().any(|f| f.name == "name" && f.field_type == "string"));
        assert!(fields.iter().any(|f| f.name == "active" && f.field_type == "boolean"));
    }

    #[test]
    fn test_include_headers() {
        let har = sample_har();
        let options = HarToCustomOptions {
            include_headers: vec!["content-type".to_string()],
            ..Default::default()
        };
        let yaml = generate_custom_yaml(&har, &options).unwrap();

        let config: super::super::custom::CustomConformanceConfig =
            serde_yaml::from_str(&yaml).unwrap();

        // First check should have content-type header, x-request-id should be skipped
        let headers = &config.custom_checks[0].expected_headers;
        assert!(headers.contains_key("content-type"));
        assert!(!headers.contains_key("x-request-id"));
    }

    #[test]
    fn test_skip_static_false() {
        let har = sample_har();
        let options = HarToCustomOptions {
            skip_static: false,
            ..Default::default()
        };
        let yaml = generate_custom_yaml(&har, &options).unwrap();

        let config: super::super::custom::CustomConformanceConfig =
            serde_yaml::from_str(&yaml).unwrap();
        // Should include all 3 entries when skip_static is false
        assert_eq!(config.custom_checks.len(), 3);
    }

    #[test]
    fn test_max_entries() {
        let har = sample_har();
        let options = HarToCustomOptions {
            skip_static: false,
            max_entries: 1,
            ..Default::default()
        };
        let yaml = generate_custom_yaml(&har, &options).unwrap();

        let config: super::super::custom::CustomConformanceConfig =
            serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.custom_checks.len(), 1);
    }

    #[test]
    fn test_custom_base_url() {
        let har = sample_har();
        let options = HarToCustomOptions {
            base_url: Some("http://localhost:3000/api".to_string()),
            ..Default::default()
        };
        let yaml = generate_custom_yaml(&har, &options).unwrap();

        let config: super::super::custom::CustomConformanceConfig =
            serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.custom_checks[0].path, "/users");
    }

    #[test]
    fn test_detect_base_url() {
        let entries = vec![HarEntry {
            request: HarRequest {
                method: "GET".to_string(),
                url: "https://api.example.com:8443/v1/health".to_string(),
                headers: vec![],
            },
            response: HarResponse {
                status: 200,
                headers: vec![],
                content: None,
            },
        }];

        let base = detect_base_url(&entries).unwrap();
        assert_eq!(base, "https://api.example.com:8443");
    }

    #[test]
    fn test_empty_entries() {
        let archive = HarArchive {
            log: HarLog { entries: vec![] },
        };
        let result = detect_base_url(&archive.log.entries);
        assert!(result.is_err());
    }

    #[test]
    fn test_regex_escape() {
        assert_eq!(regex_escape("application/json"), "application/json");
        assert_eq!(regex_escape("text/html; charset=utf-8"), "text/html; charset=utf-8");
        assert_eq!(regex_escape("foo.bar"), "foo\\.bar");
        assert_eq!(regex_escape("a(b)"), "a\\(b\\)");
    }

    #[test]
    fn test_extract_path_with_query_string() {
        let path = extract_path(
            "http://localhost:3000/api/users?page=1&limit=10",
            "http://localhost:3000",
        );
        assert_eq!(path, "/api/users");
    }

    #[test]
    fn test_include_headers_regex_pattern() {
        let har = HarArchive {
            log: HarLog {
                entries: vec![HarEntry {
                    request: HarRequest {
                        method: "GET".to_string(),
                        url: "http://localhost:3000/api/data".to_string(),
                        headers: vec![],
                    },
                    response: HarResponse {
                        status: 200,
                        headers: vec![
                            HarHeader {
                                name: "content-type".to_string(),
                                value: "application/json".to_string(),
                            },
                            HarHeader {
                                name: "content-length".to_string(),
                                value: "42".to_string(),
                            },
                            HarHeader {
                                name: "x-api-version".to_string(),
                                value: "2".to_string(),
                            },
                            HarHeader {
                                name: "x-api-request-id".to_string(),
                                value: "abc".to_string(),
                            },
                            HarHeader {
                                name: "x-other".to_string(),
                                value: "ignored".to_string(),
                            },
                            HarHeader {
                                name: "cache-control".to_string(),
                                value: "no-cache".to_string(),
                            },
                        ],
                        content: None,
                    },
                }],
            },
        };

        let options = HarToCustomOptions {
            // "content-.*" is a regex pattern, "cache-control" is exact
            include_headers: vec![
                "content-.*".to_string(),
                "x-api-.*".to_string(),
                "cache-control".to_string(),
            ],
            ..Default::default()
        };
        let yaml = generate_custom_yaml(&har, &options).unwrap();
        let config: super::super::custom::CustomConformanceConfig =
            serde_yaml::from_str(&yaml).unwrap();

        let headers = &config.custom_checks[0].expected_headers;
        // content-type matches "content-.*" pattern
        assert!(headers.contains_key("content-type"), "content-type should match content-.*");
        // content-length is in SKIP_HEADERS, so it should NOT appear
        assert!(!headers.contains_key("content-length"), "content-length is in skip list");
        // x-api-version and x-api-request-id match "x-api-.*"
        assert!(headers.contains_key("x-api-version"), "x-api-version should match x-api-.*");
        assert!(
            headers.contains_key("x-api-request-id"),
            "x-api-request-id should match x-api-.*"
        );
        // x-other should NOT match any pattern
        assert!(!headers.contains_key("x-other"), "x-other should not match");
        // cache-control is exact match
        assert!(headers.contains_key("cache-control"), "cache-control exact match");
    }

    #[test]
    fn test_include_headers_exact_no_regex() {
        // Patterns without metacharacters should work as exact case-insensitive matches
        let matchers = build_header_matchers(&["x-custom".to_string()]);
        assert!(header_matches("x-custom", &matchers));
        assert!(!header_matches("x-custom-extra", &matchers));
        assert!(!header_matches("x-custo", &matchers));
    }

    #[test]
    fn test_nested_body_field_extraction() {
        let entry = HarEntry {
            request: HarRequest {
                method: "GET".to_string(),
                url: "http://localhost:3000/api/data".to_string(),
                headers: vec![],
            },
            response: HarResponse {
                status: 200,
                headers: vec![],
                content: Some(HarContent {
                    mime_type: Some("application/json".to_string()),
                    text: Some(
                        r#"{"total": 10, "results": {"name": "Alice", "count": 5}, "tags": ["a"]}"#
                            .to_string(),
                    ),
                }),
            },
        };

        let fields = extract_body_fields(&entry);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();

        // Top-level fields
        assert!(names.contains(&"total"));
        assert!(names.contains(&"results"));
        assert!(names.contains(&"tags"));
        // Nested object fields
        assert!(names.contains(&"results.name"));
        assert!(names.contains(&"results.count"));

        // Check types
        let results_name = fields.iter().find(|f| f.name == "results.name").unwrap();
        assert_eq!(results_name.field_type, "string");
        let results_count = fields.iter().find(|f| f.name == "results.count").unwrap();
        assert_eq!(results_count.field_type, "integer");
    }

    #[test]
    fn test_nested_array_body_field_extraction() {
        let entry = HarEntry {
            request: HarRequest {
                method: "GET".to_string(),
                url: "http://localhost:3000/api/data".to_string(),
                headers: vec![],
            },
            response: HarResponse {
                status: 200,
                headers: vec![],
                content: Some(HarContent {
                    mime_type: Some("application/json".to_string()),
                    text: Some(r#"{"items": [{"id": 1, "label": "foo"}]}"#.to_string()),
                }),
            },
        };

        let fields = extract_body_fields(&entry);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();

        assert!(names.contains(&"items"));
        assert!(names.contains(&"items[].id"));
        assert!(names.contains(&"items[].label"));
    }

    #[test]
    fn test_nested_depth_limit() {
        // Build a deeply nested JSON: {a: {b: {c: {d: {e: 1}}}}}
        let entry = HarEntry {
            request: HarRequest {
                method: "GET".to_string(),
                url: "http://localhost:3000/deep".to_string(),
                headers: vec![],
            },
            response: HarResponse {
                status: 200,
                headers: vec![],
                content: Some(HarContent {
                    mime_type: Some("application/json".to_string()),
                    text: Some(r#"{"a": {"b": {"c": {"d": {"e": 1}}}}}"#.to_string()),
                }),
            },
        };

        let fields = extract_body_fields(&entry);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();

        // depth 0: a (object)
        assert!(names.contains(&"a"));
        // depth 1: a.b (object)
        assert!(names.contains(&"a.b"));
        // depth 2: a.b.c (object)
        assert!(names.contains(&"a.b.c"));
        // depth 3: a.b.c.d (object) — at MAX_BODY_FIELD_DEPTH, so its children are NOT expanded
        assert!(names.contains(&"a.b.c.d"));
        // a.b.c.d.e should NOT be present (depth limit reached)
        assert!(!names.contains(&"a.b.c.d.e"), "should not recurse beyond depth 3");
    }

    #[test]
    fn test_check_name_format() {
        let har = sample_har();
        let options = HarToCustomOptions::default();
        let yaml = generate_custom_yaml(&har, &options).unwrap();

        let config: super::super::custom::CustomConformanceConfig =
            serde_yaml::from_str(&yaml).unwrap();
        assert!(config.custom_checks[0].name.starts_with("custom:"));
    }
}
