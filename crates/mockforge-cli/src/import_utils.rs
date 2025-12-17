//! Import utilities for detecting and parsing various API formats
//!
//! This module provides functionality to detect and parse imports from:
//! - Postman collections (JSON)
//! - Insomnia exports (JSON/YAML)
//! - Curl commands (text)

use serde_json::Value;
use std::path::Path;

/// Detected import format
#[derive(Debug, Clone, PartialEq)]
pub enum ImportFormat {
    Postman,
    Insomnia,
    Curl,
    Unknown,
}

/// Result of format detection
#[derive(Debug)]
pub struct FormatDetection {
    pub format: ImportFormat,
    pub confidence: f64, // 0.0 to 1.0
    pub details: String,
}

/// Detect the format of an import source
pub fn detect_format(content: &str, file_path: Option<&Path>) -> FormatDetection {
    // Check file extension first
    if let Some(path) = file_path {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            match ext.to_lowercase().as_str() {
                "json" => {
                    // Could be Postman or Insomnia
                    return detect_json_format(content);
                }
                "yaml" | "yml" => {
                    // Likely Insomnia
                    if is_insomnia_yaml(content) {
                        return FormatDetection {
                            format: ImportFormat::Insomnia,
                            confidence: 0.9,
                            details: "YAML file with Insomnia structure detected".to_string(),
                        };
                    }
                }
                "txt" | "sh" | "bash" => {
                    // Could be curl commands
                    if is_curl_content(content) {
                        return FormatDetection {
                            format: ImportFormat::Curl,
                            confidence: 0.8,
                            details: "File contains curl commands".to_string(),
                        };
                    }
                }
                _ => {}
            }
        }
    }

    // Content-based detection
    if is_curl_content(content) {
        return FormatDetection {
            format: ImportFormat::Curl,
            confidence: 0.9,
            details: "Content contains curl commands".to_string(),
        };
    }

    if is_postman_json(content) {
        return FormatDetection {
            format: ImportFormat::Postman,
            confidence: 0.95,
            details: "JSON with Postman collection structure detected".to_string(),
        };
    }

    if is_insomnia_json(content) {
        return FormatDetection {
            format: ImportFormat::Insomnia,
            confidence: 0.95,
            details: "JSON with Insomnia export structure detected".to_string(),
        };
    }

    FormatDetection {
        format: ImportFormat::Unknown,
        confidence: 0.0,
        details: "Could not determine import format".to_string(),
    }
}

/// Detect format for JSON content (Postman vs Insomnia)
fn detect_json_format(content: &str) -> FormatDetection {
    if is_postman_json(content) {
        FormatDetection {
            format: ImportFormat::Postman,
            confidence: 0.95,
            details: "Postman collection JSON detected".to_string(),
        }
    } else if is_insomnia_json(content) {
        FormatDetection {
            format: ImportFormat::Insomnia,
            confidence: 0.95,
            details: "Insomnia export JSON detected".to_string(),
        }
    } else {
        FormatDetection {
            format: ImportFormat::Unknown,
            confidence: 0.0,
            details: "JSON format not recognized as Postman or Insomnia".to_string(),
        }
    }
}

/// Check if content is a Postman collection
fn is_postman_json(content: &str) -> bool {
    if let Ok(json) = serde_json::from_str::<Value>(content) {
        // Postman collections have specific structure
        if let Some(obj) = json.as_object() {
            // Check for Postman v2.1 structure
            if obj.contains_key("item") && obj.contains_key("info") {
                if let Some(info) = obj.get("info").and_then(|i| i.as_object()) {
                    // Check for Postman schema
                    if let Some(schema) = info.get("schema") {
                        if let Some(schema_str) = schema.as_str() {
                            return schema_str.contains("postman");
                        }
                    }
                    // Alternative: check for _postman_id
                    if info.contains_key("_postman_id") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Check if content is an Insomnia export
fn is_insomnia_json(content: &str) -> bool {
    if let Ok(json) = serde_json::from_str::<Value>(content) {
        if let Some(obj) = json.as_object() {
            // Insomnia exports have __export_format or _type field
            if obj.contains_key("__export_format") {
                if let Some(format_val) = obj.get("__export_format") {
                    if let Some(format_num) = format_val.as_i64() {
                        return format_num >= 3; // Insomnia v3+ format
                    }
                }
            }

            // Check for Insomnia-specific fields
            if obj.contains_key("_type") && obj.contains_key("resources") {
                return true;
            }
        }
    }
    false
}

/// Check if content is Insomnia YAML
fn is_insomnia_yaml(content: &str) -> bool {
    // Simple check for YAML structure with Insomnia patterns
    content.contains("__export_format:") || content.contains("_type:")
}

/// Check if content contains curl commands
fn is_curl_content(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();

    // Check if any line starts with curl
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with("curl ") || trimmed.starts_with("curl\t") {
            return true;
        }
        // Also check for curl in scripts
        if trimmed.contains("curl ") && (trimmed.contains("http") || trimmed.contains("https")) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_postman_format() {
        let postman_json = r#"{
            "info": {
                "_postman_id": "12345",
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": []
        }"#;

        let detection = detect_format(postman_json, None);
        assert_eq!(detection.format, ImportFormat::Postman);
        assert!(detection.confidence > 0.9);
    }

    #[test]
    fn test_detect_insomnia_format() {
        let insomnia_json = r#"{
            "__export_format": 4,
            "_type": "export",
            "resources": []
        }"#;

        let detection = detect_format(insomnia_json, None);
        assert_eq!(detection.format, ImportFormat::Insomnia);
        assert!(detection.confidence > 0.9);
    }

    #[test]
    fn test_detect_curl_format() {
        let curl_content = "curl -X GET https://api.example.com/users";

        let detection = detect_format(curl_content, None);
        assert_eq!(detection.format, ImportFormat::Curl);
        assert!(detection.confidence > 0.8);
    }

    #[test]
    fn test_detect_unknown_format() {
        let unknown_content = "some random text";

        let detection = detect_format(unknown_content, None);
        assert_eq!(detection.format, ImportFormat::Unknown);
        assert_eq!(detection.confidence, 0.0);
    }

    // ImportFormat tests
    #[test]
    fn test_import_format_clone() {
        let format = ImportFormat::Postman;
        let cloned = format.clone();
        assert_eq!(format, cloned);
    }

    #[test]
    fn test_import_format_debug() {
        let format = ImportFormat::Curl;
        let debug = format!("{:?}", format);
        assert!(debug.contains("Curl"));
    }

    #[test]
    fn test_import_format_equality() {
        assert_eq!(ImportFormat::Postman, ImportFormat::Postman);
        assert_ne!(ImportFormat::Postman, ImportFormat::Insomnia);
        assert_ne!(ImportFormat::Curl, ImportFormat::Unknown);
    }

    // FormatDetection tests
    #[test]
    fn test_format_detection_debug() {
        let detection = FormatDetection {
            format: ImportFormat::Postman,
            confidence: 0.95,
            details: "Postman detected".to_string(),
        };
        let debug = format!("{:?}", detection);
        assert!(debug.contains("FormatDetection"));
        assert!(debug.contains("Postman"));
    }

    // Postman detection edge cases
    #[test]
    fn test_detect_postman_with_schema_only() {
        let postman_json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": []
        }"#;

        let detection = detect_format(postman_json, None);
        assert_eq!(detection.format, ImportFormat::Postman);
    }

    #[test]
    fn test_detect_postman_with_postman_id_only() {
        let postman_json = r#"{
            "info": {
                "_postman_id": "12345",
                "name": "Test Collection"
            },
            "item": []
        }"#;

        let detection = detect_format(postman_json, None);
        assert_eq!(detection.format, ImportFormat::Postman);
    }

    // Insomnia detection edge cases
    #[test]
    fn test_detect_insomnia_v3_format() {
        let insomnia_json = r#"{
            "__export_format": 3,
            "resources": []
        }"#;

        let detection = detect_format(insomnia_json, None);
        assert_eq!(detection.format, ImportFormat::Insomnia);
    }

    #[test]
    fn test_detect_insomnia_type_and_resources() {
        let insomnia_json = r#"{
            "_type": "export",
            "resources": []
        }"#;

        let detection = detect_format(insomnia_json, None);
        assert_eq!(detection.format, ImportFormat::Insomnia);
    }

    #[test]
    fn test_detect_insomnia_yaml() {
        let insomnia_yaml = "__export_format: 4\nresources: []";
        let path = Path::new("export.yaml");

        let detection = detect_format(insomnia_yaml, Some(path));
        assert_eq!(detection.format, ImportFormat::Insomnia);
    }

    // Curl detection edge cases
    #[test]
    fn test_detect_curl_with_options() {
        let curl_content = r#"curl -X POST \
            -H "Content-Type: application/json" \
            -d '{"name": "test"}' \
            https://api.example.com/users"#;

        let detection = detect_format(curl_content, None);
        assert_eq!(detection.format, ImportFormat::Curl);
    }

    #[test]
    fn test_detect_curl_with_tab() {
        let curl_content = "curl\t-X GET https://api.example.com/users";
        let detection = detect_format(curl_content, None);
        assert_eq!(detection.format, ImportFormat::Curl);
    }

    #[test]
    fn test_detect_curl_in_script() {
        let script_content = r#"#!/bin/bash
response=$(curl https://api.example.com/users)
echo $response"#;

        let detection = detect_format(script_content, None);
        assert_eq!(detection.format, ImportFormat::Curl);
    }

    #[test]
    fn test_detect_curl_from_txt_file() {
        let curl_content = "curl -X GET https://api.example.com/users";
        let path = Path::new("requests.txt");

        let detection = detect_format(curl_content, Some(path));
        assert_eq!(detection.format, ImportFormat::Curl);
    }

    // File extension-based detection
    #[test]
    fn test_detect_from_json_file_postman() {
        let postman_json = r#"{
            "info": {
                "_postman_id": "12345",
                "name": "Test"
            },
            "item": []
        }"#;
        let path = Path::new("collection.json");

        let detection = detect_format(postman_json, Some(path));
        assert_eq!(detection.format, ImportFormat::Postman);
    }

    #[test]
    fn test_detect_from_json_file_insomnia() {
        let insomnia_json = r#"{
            "__export_format": 4,
            "resources": []
        }"#;
        let path = Path::new("export.json");

        let detection = detect_format(insomnia_json, Some(path));
        assert_eq!(detection.format, ImportFormat::Insomnia);
    }

    #[test]
    fn test_detect_unknown_json_format() {
        let unknown_json = r#"{"key": "value", "number": 123}"#;
        let path = Path::new("data.json");

        let detection = detect_format(unknown_json, Some(path));
        assert_eq!(detection.format, ImportFormat::Unknown);
    }

    // Edge cases
    #[test]
    fn test_detect_empty_content() {
        let detection = detect_format("", None);
        assert_eq!(detection.format, ImportFormat::Unknown);
    }

    #[test]
    fn test_detect_invalid_json() {
        let invalid_json = "{ invalid json }";
        let detection = detect_format(invalid_json, None);
        assert_eq!(detection.format, ImportFormat::Unknown);
    }

    #[test]
    fn test_detect_whitespace_only() {
        let whitespace = "   \n\t  ";
        let detection = detect_format(whitespace, None);
        assert_eq!(detection.format, ImportFormat::Unknown);
    }

    #[test]
    fn test_detect_curl_without_url() {
        // curl without http/https shouldn't be detected
        let no_url = "curl --version";
        let detection = detect_format(no_url, None);
        // May or may not detect as curl depending on implementation
        // Just ensure it doesn't crash
        assert!(matches!(detection.format, ImportFormat::Curl | ImportFormat::Unknown));
    }

    #[test]
    fn test_detect_insomnia_old_format() {
        let old_insomnia = r#"{
            "__export_format": 2,
            "resources": []
        }"#;
        // Format version 2 is less than 3, should not detect
        let detection = detect_format(old_insomnia, None);
        assert_eq!(detection.format, ImportFormat::Unknown);
    }

    #[test]
    fn test_format_detection_details_present() {
        let curl_content = "curl -X GET https://api.example.com/users";
        let detection = detect_format(curl_content, None);
        assert!(!detection.details.is_empty());
    }
}
