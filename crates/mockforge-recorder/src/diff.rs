//! Request/response diff viewer with content-aware comparison

use serde_json::Value;
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;

/// Difference type
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum DifferenceType {
    /// Value added in new response
    Added { path: String, value: String },
    /// Value removed from original
    Removed { path: String, value: String },
    /// Value changed
    Changed {
        path: String,
        original: String,
        current: String,
    },
    /// Type changed (e.g., string -> number)
    TypeChanged {
        path: String,
        original_type: String,
        current_type: String,
    },
}

/// Difference between original and current response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Difference {
    /// JSON path or header name
    pub path: String,
    /// Type of difference
    pub difference_type: DifferenceType,
    /// Human-readable description
    pub description: String,
}

impl Difference {
    /// Create a new difference
    pub fn new(path: String, difference_type: DifferenceType) -> Self {
        let description = match &difference_type {
            DifferenceType::Added { value, .. } => format!("Added: {}", value),
            DifferenceType::Removed { value, .. } => format!("Removed: {}", value),
            DifferenceType::Changed {
                original, current, ..
            } => {
                format!("Changed from '{}' to '{}'", original, current)
            }
            DifferenceType::TypeChanged {
                original_type,
                current_type,
                ..
            } => format!("Type changed from {} to {}", original_type, current_type),
        };

        Self {
            path,
            difference_type,
            description,
        }
    }
}

/// Result of comparing two responses
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComparisonResult {
    /// Do responses match exactly?
    pub matches: bool,
    /// Status code comparison
    pub status_match: bool,
    /// Headers match?
    pub headers_match: bool,
    /// Body match?
    pub body_match: bool,
    /// List of all differences
    pub differences: Vec<Difference>,
    /// Summary statistics
    pub summary: ComparisonSummary,
}

/// Summary of comparison
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComparisonSummary {
    pub total_differences: usize,
    pub added_fields: usize,
    pub removed_fields: usize,
    pub changed_fields: usize,
    pub type_changes: usize,
}

impl ComparisonSummary {
    pub fn from_differences(differences: &[Difference]) -> Self {
        let mut summary = Self {
            total_differences: differences.len(),
            added_fields: 0,
            removed_fields: 0,
            changed_fields: 0,
            type_changes: 0,
        };

        for diff in differences {
            match &diff.difference_type {
                DifferenceType::Added { .. } => summary.added_fields += 1,
                DifferenceType::Removed { .. } => summary.removed_fields += 1,
                DifferenceType::Changed { .. } => summary.changed_fields += 1,
                DifferenceType::TypeChanged { .. } => summary.type_changes += 1,
            }
        }

        summary
    }
}

/// Response comparator with content-aware diffing
pub struct ResponseComparator;

impl ResponseComparator {
    /// Compare two responses
    pub fn compare(
        original_status: i32,
        original_headers: &HashMap<String, String>,
        original_body: &[u8],
        current_status: i32,
        current_headers: &HashMap<String, String>,
        current_body: &[u8],
    ) -> ComparisonResult {
        let mut differences = Vec::new();

        // Compare status codes
        let status_match = original_status == current_status;
        if !status_match {
            differences.push(Difference::new(
                "status_code".to_string(),
                DifferenceType::Changed {
                    path: "status_code".to_string(),
                    original: original_status.to_string(),
                    current: current_status.to_string(),
                },
            ));
        }

        // Compare headers
        let header_diffs = Self::compare_headers(original_headers, current_headers);
        let headers_match = header_diffs.is_empty();
        differences.extend(header_diffs);

        // Compare bodies based on content type
        let content_type = original_headers
            .get("content-type")
            .or_else(|| current_headers.get("content-type"))
            .map(|s| s.to_lowercase());

        let body_diffs = Self::compare_bodies(original_body, current_body, content_type.as_deref());
        let body_match = body_diffs.is_empty();
        differences.extend(body_diffs);

        let matches = differences.is_empty();
        let summary = ComparisonSummary::from_differences(&differences);

        ComparisonResult {
            matches,
            status_match,
            headers_match,
            body_match,
            differences,
            summary,
        }
    }

    /// Compare headers
    fn compare_headers(
        original: &HashMap<String, String>,
        current: &HashMap<String, String>,
    ) -> Vec<Difference> {
        let mut differences = Vec::new();

        // Check for removed or changed headers
        for (key, original_value) in original {
            // Skip dynamic headers
            if Self::is_dynamic_header(key) {
                continue;
            }

            match current.get(key) {
                Some(current_value) if current_value != original_value => {
                    differences.push(Difference::new(
                        format!("headers.{}", key),
                        DifferenceType::Changed {
                            path: format!("headers.{}", key),
                            original: original_value.clone(),
                            current: current_value.clone(),
                        },
                    ));
                }
                None => {
                    differences.push(Difference::new(
                        format!("headers.{}", key),
                        DifferenceType::Removed {
                            path: format!("headers.{}", key),
                            value: original_value.clone(),
                        },
                    ));
                }
                _ => {}
            }
        }

        // Check for added headers
        for (key, current_value) in current {
            if Self::is_dynamic_header(key) {
                continue;
            }

            if !original.contains_key(key) {
                differences.push(Difference::new(
                    format!("headers.{}", key),
                    DifferenceType::Added {
                        path: format!("headers.{}", key),
                        value: current_value.clone(),
                    },
                ));
            }
        }

        differences
    }

    /// Check if header is dynamic and should be ignored in comparisons
    fn is_dynamic_header(key: &str) -> bool {
        let key_lower = key.to_lowercase();
        matches!(
            key_lower.as_str(),
            "date" | "x-request-id" | "x-trace-id" | "set-cookie" | "age" | "expires"
        )
    }

    /// Compare bodies based on content type
    fn compare_bodies(
        original: &[u8],
        current: &[u8],
        content_type: Option<&str>,
    ) -> Vec<Difference> {
        // Detect content type
        let is_json = content_type
            .map(|ct| ct.contains("json"))
            .unwrap_or_else(|| Self::is_likely_json(original));

        if is_json {
            Self::compare_json_bodies(original, current)
        } else {
            Self::compare_text_bodies(original, current)
        }
    }

    /// Check if bytes are likely JSON
    fn is_likely_json(data: &[u8]) -> bool {
        if data.is_empty() {
            return false;
        }
        let first_char = data[0];
        first_char == b'{' || first_char == b'['
    }

    /// Compare JSON bodies with deep diff
    fn compare_json_bodies(original: &[u8], current: &[u8]) -> Vec<Difference> {
        let original_json: Result<Value, _> = serde_json::from_slice(original);
        let current_json: Result<Value, _> = serde_json::from_slice(current);

        match (original_json, current_json) {
            (Ok(orig), Ok(curr)) => Self::compare_json_values(&orig, &curr, "body"),
            _ => {
                // Fallback to text comparison if not valid JSON
                Self::compare_text_bodies(original, current)
            }
        }
    }

    /// Deep compare JSON values
    fn compare_json_values(original: &Value, current: &Value, path: &str) -> Vec<Difference> {
        let mut differences = Vec::new();

        match (original, current) {
            (Value::Object(orig_map), Value::Object(curr_map)) => {
                // Check for removed or changed keys
                for (key, orig_value) in orig_map {
                    let new_path = format!("{}.{}", path, key);
                    match curr_map.get(key) {
                        Some(curr_value) => {
                            differences.extend(Self::compare_json_values(
                                orig_value, curr_value, &new_path,
                            ));
                        }
                        None => {
                            differences.push(Difference::new(
                                new_path.clone(),
                                DifferenceType::Removed {
                                    path: new_path,
                                    value: orig_value.to_string(),
                                },
                            ));
                        }
                    }
                }

                // Check for added keys
                for (key, curr_value) in curr_map {
                    if !orig_map.contains_key(key) {
                        let new_path = format!("{}.{}", path, key);
                        differences.push(Difference::new(
                            new_path.clone(),
                            DifferenceType::Added {
                                path: new_path,
                                value: curr_value.to_string(),
                            },
                        ));
                    }
                }
            }
            (Value::Array(orig_arr), Value::Array(curr_arr)) => {
                let max_len = orig_arr.len().max(curr_arr.len());
                for i in 0..max_len {
                    let new_path = format!("{}[{}]", path, i);

                    match (orig_arr.get(i), curr_arr.get(i)) {
                        (Some(orig_val), Some(curr_val)) => {
                            differences
                                .extend(Self::compare_json_values(orig_val, curr_val, &new_path));
                        }
                        (Some(orig_val), None) => {
                            differences.push(Difference::new(
                                new_path.clone(),
                                DifferenceType::Removed {
                                    path: new_path,
                                    value: orig_val.to_string(),
                                },
                            ));
                        }
                        (None, Some(curr_val)) => {
                            differences.push(Difference::new(
                                new_path.clone(),
                                DifferenceType::Added {
                                    path: new_path,
                                    value: curr_val.to_string(),
                                },
                            ));
                        }
                        (None, None) => unreachable!(),
                    }
                }
            }
            (orig, curr) if orig != curr => {
                // Check for type changes
                if std::mem::discriminant(orig) != std::mem::discriminant(curr) {
                    differences.push(Difference::new(
                        path.to_string(),
                        DifferenceType::TypeChanged {
                            path: path.to_string(),
                            original_type: Self::json_type_name(orig),
                            current_type: Self::json_type_name(curr),
                        },
                    ));
                } else {
                    // Same type, different value
                    differences.push(Difference::new(
                        path.to_string(),
                        DifferenceType::Changed {
                            path: path.to_string(),
                            original: orig.to_string(),
                            current: curr.to_string(),
                        },
                    ));
                }
            }
            _ => {
                // Values match, no difference
            }
        }

        differences
    }

    /// Get JSON value type name
    fn json_type_name(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(_) => "boolean".to_string(),
            Value::Number(_) => "number".to_string(),
            Value::String(_) => "string".to_string(),
            Value::Array(_) => "array".to_string(),
            Value::Object(_) => "object".to_string(),
        }
    }

    /// Compare text bodies using line-by-line diff
    fn compare_text_bodies(original: &[u8], current: &[u8]) -> Vec<Difference> {
        let original_str = String::from_utf8_lossy(original);
        let current_str = String::from_utf8_lossy(current);

        if original_str == current_str {
            return vec![];
        }

        // Use line-based diff for text
        let diff = TextDiff::from_lines(&original_str, &current_str);
        let mut differences = Vec::new();

        for (idx, change) in diff.iter_all_changes().enumerate() {
            let path = format!("body.line_{}", idx);
            match change.tag() {
                ChangeTag::Delete => {
                    differences.push(Difference::new(
                        path.clone(),
                        DifferenceType::Removed {
                            path,
                            value: change.to_string().trim_end().to_string(),
                        },
                    ));
                }
                ChangeTag::Insert => {
                    differences.push(Difference::new(
                        path.clone(),
                        DifferenceType::Added {
                            path,
                            value: change.to_string().trim_end().to_string(),
                        },
                    ));
                }
                ChangeTag::Equal => {
                    // No difference for equal lines
                }
            }
        }

        // If too many line diffs, summarize
        if differences.len() > 100 {
            vec![Difference::new(
                "body".to_string(),
                DifferenceType::Changed {
                    path: "body".to_string(),
                    original: format!("{} bytes", original.len()),
                    current: format!("{} bytes", current.len()),
                },
            )]
        } else {
            differences
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_identical_responses() {
        let headers = HashMap::new();
        let body = b"test";

        let result = ResponseComparator::compare(200, &headers, body, 200, &headers, body);

        assert!(result.matches);
        assert!(result.status_match);
        assert!(result.headers_match);
        assert!(result.body_match);
        assert_eq!(result.differences.len(), 0);
    }

    #[test]
    fn test_status_code_difference() {
        let headers = HashMap::new();
        let body = b"test";

        let result = ResponseComparator::compare(200, &headers, body, 404, &headers, body);

        assert!(!result.matches);
        assert!(!result.status_match);
        assert_eq!(result.differences.len(), 1);

        match &result.differences[0].difference_type {
            DifferenceType::Changed {
                path,
                original,
                current,
            } => {
                assert_eq!(path, "status_code");
                assert_eq!(original, "200");
                assert_eq!(current, "404");
            }
            _ => panic!("Expected Changed difference"),
        }
    }

    #[test]
    fn test_header_differences() {
        let mut original_headers = HashMap::new();
        original_headers.insert("content-type".to_string(), "application/json".to_string());
        original_headers.insert("x-custom".to_string(), "value1".to_string());

        let mut current_headers = HashMap::new();
        current_headers.insert("content-type".to_string(), "text/plain".to_string());
        current_headers.insert("x-new".to_string(), "value2".to_string());

        let body = b"";

        let result =
            ResponseComparator::compare(200, &original_headers, body, 200, &current_headers, body);

        assert!(!result.matches);
        assert!(!result.headers_match);

        // Should have: content-type changed, x-custom removed, x-new added
        assert_eq!(result.differences.len(), 3);
    }

    #[test]
    fn test_json_body_differences() {
        let original = br#"{"name": "John", "age": 30}"#;
        let current = br#"{"name": "Jane", "age": 30, "city": "NYC"}"#;

        let headers = HashMap::new();
        let result = ResponseComparator::compare(200, &headers, original, 200, &headers, current);

        assert!(!result.matches);
        assert!(!result.body_match);

        // Should detect: name changed, city added
        assert!(result.differences.len() >= 2);

        // Check for name change
        let name_diff = result.differences.iter().find(|d| d.path == "body.name");
        assert!(name_diff.is_some());
    }

    #[test]
    fn test_json_array_differences() {
        let original = br#"{"items": [1, 2, 3]}"#;
        let current = br#"{"items": [1, 2, 3, 4]}"#;

        let headers = HashMap::new();
        let result = ResponseComparator::compare(200, &headers, original, 200, &headers, current);

        assert!(!result.matches);

        // Should detect array item added
        let array_diff = result.differences.iter().find(|d| d.path.contains("items[3]"));
        assert!(array_diff.is_some());
    }

    #[test]
    fn test_json_type_change() {
        let original = br#"{"value": "123"}"#;
        let current = br#"{"value": 123}"#;

        let headers = HashMap::new();
        let result = ResponseComparator::compare(200, &headers, original, 200, &headers, current);

        assert!(!result.matches);

        // Should detect type change from string to number
        let type_diff = result
            .differences
            .iter()
            .find(|d| matches!(&d.difference_type, DifferenceType::TypeChanged { .. }));
        assert!(type_diff.is_some());
    }

    #[test]
    fn test_dynamic_headers_ignored() {
        let mut original_headers = HashMap::new();
        original_headers.insert("date".to_string(), "Mon, 01 Jan 2024".to_string());

        let mut current_headers = HashMap::new();
        current_headers.insert("date".to_string(), "Tue, 02 Jan 2024".to_string());

        let body = b"";

        let result =
            ResponseComparator::compare(200, &original_headers, body, 200, &current_headers, body);

        // Date header should be ignored
        assert!(result.headers_match);
    }

    #[test]
    fn test_comparison_summary() {
        let original = br#"{"a": 1, "b": 2}"#;
        let current = br#"{"a": 2, "c": 3}"#;

        let headers = HashMap::new();
        let result = ResponseComparator::compare(200, &headers, original, 200, &headers, current);

        assert_eq!(result.summary.total_differences, 3);
        assert_eq!(result.summary.changed_fields, 1); // a changed
        assert_eq!(result.summary.removed_fields, 1); // b removed
        assert_eq!(result.summary.added_fields, 1); // c added
    }
}
