//! Target file parsing for multi-target bench testing
//!
//! Supports two formats:
//! 1. Simple text file: one target URL/IP/hostname per line
//! 2. JSON format: array of target objects with optional per-target configuration

use crate::error::{BenchError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Configuration for a single target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    /// Target URL, IP address, or hostname
    pub url: String,
    /// Optional authentication header value (e.g., "Bearer token123")
    pub auth: Option<String>,
    /// Optional custom headers for this target
    pub headers: Option<HashMap<String, String>>,
    /// Optional per-target OpenAPI spec file (JSON format only)
    pub spec: Option<PathBuf>,
}

impl TargetConfig {
    /// Create a new TargetConfig from a URL string
    pub fn from_url(url: String) -> Self {
        Self {
            url,
            auth: None,
            headers: None,
            spec: None,
        }
    }

    /// Normalize the URL to ensure it has a protocol
    pub fn normalize_url(&mut self) {
        // If URL doesn't start with http:// or https://, assume http://
        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            // Check if it looks like it has a port (contains colon but not after http/https)
            if self.url.contains(':') && !self.url.starts_with("http") {
                // It's likely an IP:port or hostname:port
                self.url = format!("http://{}", self.url);
            } else {
                // It's likely just a hostname
                self.url = format!("http://{}", self.url);
            }
        }
    }
}

/// JSON format for target file (array of targets)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonTargetFile {
    #[serde(rename = "targets")]
    targets: Option<Vec<JsonTarget>>,
}

/// Individual target in JSON format
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonTarget {
    /// Simple string format: just the URL
    Simple(String),
    /// Object format: URL with optional config
    Object {
        url: String,
        auth: Option<String>,
        headers: Option<HashMap<String, String>>,
        spec: Option<PathBuf>,
    },
}

/// Parse a targets file and return a vector of TargetConfig
///
/// Automatically detects the format based on file extension and content:
/// - `.json` files are parsed as JSON
/// - Other files are parsed as simple text (one target per line)
pub fn parse_targets_file(path: &Path) -> Result<Vec<TargetConfig>> {
    // Read file content
    let content = std::fs::read_to_string(path)
        .map_err(|e| BenchError::Other(format!("Failed to read targets file: {}", e)))?;

    // Detect format based on extension and content
    let is_json = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
        || content.trim_start().starts_with('[')
        || content.trim_start().starts_with('{');

    if is_json {
        parse_json_targets(&content)
    } else {
        parse_text_targets(&content)
    }
}

/// Parse targets from JSON format
fn parse_json_targets(content: &str) -> Result<Vec<TargetConfig>> {
    // Try parsing as array of targets directly
    let json_value: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| BenchError::Other(format!("Failed to parse JSON: {}", e)))?;

    let targets = match json_value {
        serde_json::Value::Array(arr) => {
            // Direct array format: [{"url": "...", ...}, ...]
            arr.into_iter()
                .map(|item| {
                    if let Ok(target) = serde_json::from_value::<JsonTarget>(item) {
                        Ok(match target {
                            JsonTarget::Simple(url) => TargetConfig::from_url(url),
                            JsonTarget::Object {
                                url,
                                auth,
                                headers,
                                spec,
                            } => TargetConfig {
                                url,
                                auth,
                                headers,
                                spec,
                            },
                        })
                    } else {
                        Err(BenchError::Other("Invalid target format in JSON array".to_string()))
                    }
                })
                .collect::<Result<Vec<_>>>()?
        }
        serde_json::Value::Object(obj) => {
            // Object with "targets" key: {"targets": [...]}
            if let Some(targets_val) = obj.get("targets") {
                if let Some(arr) = targets_val.as_array() {
                    arr.iter()
                        .map(|item| {
                            if let Ok(target) = serde_json::from_value::<JsonTarget>(item.clone()) {
                                Ok(match target {
                                    JsonTarget::Simple(url) => TargetConfig::from_url(url),
                                    JsonTarget::Object {
                                        url,
                                        auth,
                                        headers,
                                        spec,
                                    } => TargetConfig {
                                        url,
                                        auth,
                                        headers,
                                        spec,
                                    },
                                })
                            } else {
                                Err(BenchError::Other("Invalid target format in JSON".to_string()))
                            }
                        })
                        .collect::<Result<Vec<_>>>()?
                } else {
                    return Err(BenchError::Other("Expected 'targets' to be an array".to_string()));
                }
            } else {
                return Err(BenchError::Other(
                    "JSON object must contain 'targets' array".to_string(),
                ));
            }
        }
        _ => {
            return Err(BenchError::Other(
                "JSON must be an array or object with 'targets' key".to_string(),
            ));
        }
    };

    if targets.is_empty() {
        return Err(BenchError::Other("No targets found in JSON file".to_string()));
    }

    // Normalize URLs
    let mut normalized_targets = targets;
    for target in &mut normalized_targets {
        target.normalize_url();
    }

    Ok(normalized_targets)
}

/// Parse targets from simple text format (one per line)
fn parse_text_targets(content: &str) -> Result<Vec<TargetConfig>> {
    let mut targets = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments (lines starting with #)
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Validate that the line looks like a URL/IP/hostname
        if line.is_empty() {
            continue;
        }

        let mut target = TargetConfig::from_url(line.to_string());
        target.normalize_url();
        targets.push(target);
    }

    if targets.is_empty() {
        return Err(BenchError::Other("No valid targets found in text file".to_string()));
    }

    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_text_targets() {
        let content = r#"
https://api1.example.com
https://api2.example.com
192.168.1.100:8080
api3.example.com
# This is a comment
        "#;

        let targets = parse_text_targets(content).unwrap();
        assert_eq!(targets.len(), 4);
        assert_eq!(targets[0].url, "https://api1.example.com");
        assert_eq!(targets[1].url, "https://api2.example.com");
        assert_eq!(targets[2].url, "http://192.168.1.100:8080");
        assert_eq!(targets[3].url, "http://api3.example.com");
    }

    #[test]
    fn test_parse_json_targets_array() {
        let content = r#"
[
  {"url": "https://api1.example.com", "auth": "Bearer token1"},
  {"url": "https://api2.example.com"},
  "https://api3.example.com"
]
        "#;

        let targets = parse_json_targets(content).unwrap();
        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].url, "https://api1.example.com");
        assert_eq!(targets[0].auth, Some("Bearer token1".to_string()));
        assert_eq!(targets[1].url, "https://api2.example.com");
        assert_eq!(targets[2].url, "https://api3.example.com");
    }

    #[test]
    fn test_parse_json_targets_object() {
        let content = r#"
{
  "targets": [
    {"url": "https://api1.example.com"},
    {"url": "https://api2.example.com", "auth": "Bearer token2"}
  ]
}
        "#;

        let targets = parse_json_targets(content).unwrap();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].url, "https://api1.example.com");
        assert_eq!(targets[1].url, "https://api2.example.com");
        assert_eq!(targets[1].auth, Some("Bearer token2".to_string()));
    }

    #[test]
    fn test_normalize_url() {
        let mut target = TargetConfig::from_url("api.example.com".to_string());
        target.normalize_url();
        assert_eq!(target.url, "http://api.example.com");

        let mut target2 = TargetConfig::from_url("192.168.1.1:8080".to_string());
        target2.normalize_url();
        assert_eq!(target2.url, "http://192.168.1.1:8080");

        let mut target3 = TargetConfig::from_url("https://api.example.com".to_string());
        target3.normalize_url();
        assert_eq!(target3.url, "https://api.example.com");
    }

    #[test]
    fn test_parse_targets_file_text() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "https://api1.example.com").unwrap();
        writeln!(file, "https://api2.example.com").unwrap();
        writeln!(file, "# comment").unwrap();
        writeln!(file, "api3.example.com").unwrap();

        let targets = parse_targets_file(file.path()).unwrap();
        assert_eq!(targets.len(), 3);
    }

    #[test]
    fn test_parse_targets_file_json() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(
            r#"[
  {"url": "https://api1.example.com"},
  {"url": "https://api2.example.com"}
]"#
            .as_bytes(),
        )
        .unwrap();

        let targets = parse_targets_file(file.path()).unwrap();
        assert_eq!(targets.len(), 2);
    }

    #[test]
    fn test_parse_targets_file_empty() {
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "").unwrap();

        let result = parse_targets_file(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_targets_file_only_comments() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "# comment 1").unwrap();
        writeln!(file, "# comment 2").unwrap();

        let result = parse_targets_file(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_json_targets_with_headers() {
        let content = r#"
[
  {
    "url": "https://api1.example.com",
    "auth": "Bearer token1",
    "headers": {
      "X-Custom": "value1",
      "X-Another": "value2"
    }
  }
]
        "#;

        let targets = parse_json_targets(content).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].url, "https://api1.example.com");
        assert_eq!(targets[0].auth, Some("Bearer token1".to_string()));
        assert_eq!(
            targets[0].headers.as_ref().unwrap().get("X-Custom"),
            Some(&"value1".to_string())
        );
        assert_eq!(
            targets[0].headers.as_ref().unwrap().get("X-Another"),
            Some(&"value2".to_string())
        );
    }

    #[test]
    fn test_parse_json_targets_with_per_target_spec() {
        let content = r#"
[
  {"url": "https://api1.example.com", "spec": "spec_a.json"},
  {"url": "https://api2.example.com", "spec": "spec_b.json"},
  {"url": "https://api3.example.com"}
]
        "#;

        let targets = parse_json_targets(content).unwrap();
        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].spec, Some(PathBuf::from("spec_a.json")));
        assert_eq!(targets[1].spec, Some(PathBuf::from("spec_b.json")));
        assert_eq!(targets[2].spec, None);
    }

    #[test]
    fn test_parse_json_targets_with_per_target_spec_mixed() {
        // Targets with some specs and some without should parse correctly
        let content = r#"[
  {"url": "https://api1.example.com", "spec": "/absolute/path/spec.json"},
  {"url": "https://api2.example.com"},
  {"url": "https://api3.example.com", "spec": "relative/spec.yaml"}
]"#;

        let targets = parse_json_targets(content).unwrap();
        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].spec, Some(PathBuf::from("/absolute/path/spec.json")));
        assert_eq!(targets[1].spec, None);
        assert_eq!(targets[2].spec, Some(PathBuf::from("relative/spec.yaml")));
    }

    #[test]
    fn test_from_url_has_no_spec() {
        let target = TargetConfig::from_url("http://example.com".to_string());
        assert_eq!(target.spec, None);
        assert_eq!(target.auth, None);
        assert_eq!(target.headers, None);
    }

    #[test]
    fn test_parse_json_targets_invalid_format() {
        let content = r#"
{
  "invalid": "format"
}
        "#;

        let result = parse_json_targets(content);
        assert!(result.is_err());
    }
}
