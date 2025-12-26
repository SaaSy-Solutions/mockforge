//! Dynamic parameter placeholder processing for k6 script generation
//!
//! This module handles placeholders like `${__VU}`, `${__ITER}`, `${__TIMESTAMP}`, and `${__UUID}`
//! that are replaced with k6 runtime expressions in the generated scripts.

use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Regex pattern for dynamic placeholders: ${__VU}, ${__ITER}, ${__TIMESTAMP}, ${__UUID}, ${__RANDOM}, ${__COUNTER}
static PLACEHOLDER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{__([A-Z_]+)\}").expect("Invalid placeholder regex"));

/// Supported dynamic parameter placeholders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicPlaceholder {
    /// ${__VU} -> Virtual User ID (1-indexed)
    VU,
    /// ${__ITER} -> Iteration count (0-indexed)
    Iteration,
    /// ${__TIMESTAMP} -> Current timestamp in milliseconds
    Timestamp,
    /// ${__UUID} -> Random UUID
    UUID,
    /// ${__RANDOM} -> Random float 0-1
    Random,
    /// ${__COUNTER} -> Global incrementing counter
    Counter,
    /// ${__DATE} -> Current date in ISO format
    Date,
    /// ${__VU_ITER} -> Combined VU and iteration for unique IDs
    VuIter,
}

impl DynamicPlaceholder {
    /// Parse a placeholder name into a variant
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "VU" => Some(Self::VU),
            "ITER" => Some(Self::Iteration),
            "TIMESTAMP" => Some(Self::Timestamp),
            "UUID" => Some(Self::UUID),
            "RANDOM" => Some(Self::Random),
            "COUNTER" => Some(Self::Counter),
            "DATE" => Some(Self::Date),
            "VU_ITER" => Some(Self::VuIter),
            _ => None,
        }
    }

    /// Get the k6 JavaScript expression for this placeholder
    pub fn to_k6_expression(&self) -> &'static str {
        match self {
            Self::VU => "__VU",
            Self::Iteration => "__ITER",
            Self::Timestamp => "Date.now()",
            Self::UUID => "crypto.randomUUID()",
            Self::Random => "Math.random()",
            Self::Counter => "globalCounter++",
            Self::Date => "new Date().toISOString()",
            Self::VuIter => "`${__VU}-${__ITER}`",
        }
    }

    /// Check if this placeholder requires additional k6 imports
    pub fn requires_import(&self) -> Option<&'static str> {
        match self {
            Self::UUID => Some("import { crypto } from 'k6/experimental/webcrypto';"),
            _ => None,
        }
    }

    /// Check if this placeholder requires global state initialization
    pub fn requires_global_init(&self) -> Option<&'static str> {
        match self {
            Self::Counter => Some("let globalCounter = 0;"),
            _ => None,
        }
    }
}

/// Result of processing a value for dynamic placeholders
#[derive(Debug, Clone)]
pub struct ProcessedValue {
    /// The processed value (may be a k6 template literal)
    pub value: String,
    /// Whether the value contains dynamic placeholders
    pub is_dynamic: bool,
    /// Set of placeholders found in the value
    pub placeholders: HashSet<DynamicPlaceholder>,
}

impl ProcessedValue {
    /// Create a static (non-dynamic) processed value
    pub fn static_value(value: String) -> Self {
        Self {
            value,
            is_dynamic: false,
            placeholders: HashSet::new(),
        }
    }
}

/// Result of processing a JSON body
#[derive(Debug, Clone)]
pub struct ProcessedBody {
    /// The processed body as a string (may contain JS expressions)
    pub value: String,
    /// Whether the body contains dynamic placeholders
    pub is_dynamic: bool,
    /// Set of all placeholders found in the body
    pub placeholders: HashSet<DynamicPlaceholder>,
}

/// Processor for dynamic parameter placeholders
pub struct DynamicParamProcessor;

impl DynamicParamProcessor {
    /// Check if a string contains any dynamic placeholders
    pub fn has_dynamic_placeholders(value: &str) -> bool {
        PLACEHOLDER_REGEX.is_match(value)
    }

    /// Extract all placeholders from a string
    pub fn extract_placeholders(value: &str) -> HashSet<DynamicPlaceholder> {
        let mut placeholders = HashSet::new();

        for cap in PLACEHOLDER_REGEX.captures_iter(value) {
            if let Some(name) = cap.get(1) {
                if let Some(placeholder) = DynamicPlaceholder::from_name(name.as_str()) {
                    placeholders.insert(placeholder);
                }
            }
        }

        placeholders
    }

    /// Process a string value, converting placeholders to k6 expressions
    ///
    /// Input: "load-test-vu-${__VU}"
    /// Output: ProcessedValue { value: "`load-test-vu-${__VU}`", is_dynamic: true, ... }
    pub fn process_value(value: &str) -> ProcessedValue {
        let placeholders = Self::extract_placeholders(value);

        if placeholders.is_empty() {
            return ProcessedValue::static_value(value.to_string());
        }

        // Convert placeholders to k6 expressions
        let mut result = value.to_string();

        for placeholder in &placeholders {
            let pattern = match placeholder {
                DynamicPlaceholder::VU => "${__VU}",
                DynamicPlaceholder::Iteration => "${__ITER}",
                DynamicPlaceholder::Timestamp => "${__TIMESTAMP}",
                DynamicPlaceholder::UUID => "${__UUID}",
                DynamicPlaceholder::Random => "${__RANDOM}",
                DynamicPlaceholder::Counter => "${__COUNTER}",
                DynamicPlaceholder::Date => "${__DATE}",
                DynamicPlaceholder::VuIter => "${__VU_ITER}",
            };

            let replacement = format!("${{{}}}", placeholder.to_k6_expression());
            result = result.replace(pattern, &replacement);
        }

        // Wrap in backticks to make it a JS template literal
        ProcessedValue {
            value: format!("`{}`", result),
            is_dynamic: true,
            placeholders,
        }
    }

    /// Process a JSON value recursively, handling dynamic placeholders
    pub fn process_json_value(value: &Value) -> (Value, HashSet<DynamicPlaceholder>) {
        let mut all_placeholders = HashSet::new();

        let processed = match value {
            Value::String(s) => {
                let processed = Self::process_value(s);
                all_placeholders.extend(processed.placeholders);
                if processed.is_dynamic {
                    // Mark as dynamic by wrapping in a special format
                    // The template will handle rendering this as a JS expression
                    Value::String(format!("__DYNAMIC__{}", processed.value))
                } else {
                    Value::String(s.clone())
                }
            }
            Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (key, val) in map {
                    let (processed_val, placeholders) = Self::process_json_value(val);
                    all_placeholders.extend(placeholders);
                    new_map.insert(key.clone(), processed_val);
                }
                Value::Object(new_map)
            }
            Value::Array(arr) => {
                let processed_arr: Vec<Value> = arr
                    .iter()
                    .map(|v| {
                        let (processed, placeholders) = Self::process_json_value(v);
                        all_placeholders.extend(placeholders);
                        processed
                    })
                    .collect();
                Value::Array(processed_arr)
            }
            // Other types pass through unchanged
            _ => value.clone(),
        };

        (processed, all_placeholders)
    }

    /// Process an entire JSON body for dynamic placeholders
    ///
    /// Returns a JavaScript-ready body string that may contain template literals
    pub fn process_json_body(body: &Value) -> ProcessedBody {
        let (processed, placeholders) = Self::process_json_value(body);
        let is_dynamic = !placeholders.is_empty();

        // Convert to a JavaScript-compatible string
        let value = if is_dynamic {
            // Generate JavaScript code that builds the object with dynamic values
            Self::generate_dynamic_body_js(&processed)
        } else {
            // Static body - just use JSON serialization
            serde_json::to_string_pretty(&processed).unwrap_or_else(|_| "{}".to_string())
        };

        ProcessedBody {
            value,
            is_dynamic,
            placeholders,
        }
    }

    /// Generate JavaScript code for a body with dynamic values
    fn generate_dynamic_body_js(value: &Value) -> String {
        match value {
            Value::String(s) if s.starts_with("__DYNAMIC__") => {
                // Remove the __DYNAMIC__ prefix and return the template literal
                s.strip_prefix("__DYNAMIC__").unwrap_or(s).to_string()
            }
            Value::String(s) => {
                // Regular string - quote it
                format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
            }
            Value::Object(map) => {
                let pairs: Vec<String> = map
                    .iter()
                    .map(|(k, v)| {
                        let key = format!("\"{}\"", k);
                        let val = Self::generate_dynamic_body_js(v);
                        format!("{}: {}", key, val)
                    })
                    .collect();
                format!("{{\n  {}\n}}", pairs.join(",\n  "))
            }
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(Self::generate_dynamic_body_js).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
        }
    }

    /// Process a URL path for dynamic placeholders
    pub fn process_path(path: &str) -> ProcessedValue {
        Self::process_value(path)
    }

    /// Get all required imports based on placeholders used
    pub fn get_required_imports(placeholders: &HashSet<DynamicPlaceholder>) -> Vec<&'static str> {
        placeholders.iter().filter_map(|p| p.requires_import()).collect()
    }

    /// Get all required global initializations based on placeholders used
    pub fn get_required_globals(placeholders: &HashSet<DynamicPlaceholder>) -> Vec<&'static str> {
        placeholders.iter().filter_map(|p| p.requires_global_init()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_dynamic_placeholders() {
        assert!(DynamicParamProcessor::has_dynamic_placeholders("test-${__VU}"));
        assert!(DynamicParamProcessor::has_dynamic_placeholders("${__ITER}-${__VU}"));
        assert!(!DynamicParamProcessor::has_dynamic_placeholders("static-value"));
        assert!(!DynamicParamProcessor::has_dynamic_placeholders("${normal_var}"));
    }

    #[test]
    fn test_extract_placeholders() {
        let placeholders = DynamicParamProcessor::extract_placeholders("vu-${__VU}-iter-${__ITER}");
        assert!(placeholders.contains(&DynamicPlaceholder::VU));
        assert!(placeholders.contains(&DynamicPlaceholder::Iteration));
        assert_eq!(placeholders.len(), 2);
    }

    #[test]
    fn test_process_value_static() {
        let result = DynamicParamProcessor::process_value("static-value");
        assert!(!result.is_dynamic);
        assert_eq!(result.value, "static-value");
        assert!(result.placeholders.is_empty());
    }

    #[test]
    fn test_process_value_dynamic() {
        let result = DynamicParamProcessor::process_value("test-${__VU}");
        assert!(result.is_dynamic);
        assert_eq!(result.value, "`test-${__VU}`");
        assert!(result.placeholders.contains(&DynamicPlaceholder::VU));
    }

    #[test]
    fn test_process_value_multiple_placeholders() {
        let result = DynamicParamProcessor::process_value("vu-${__VU}-iter-${__ITER}");
        assert!(result.is_dynamic);
        assert_eq!(result.value, "`vu-${__VU}-iter-${__ITER}`");
        assert_eq!(result.placeholders.len(), 2);
    }

    #[test]
    fn test_process_value_timestamp() {
        let result = DynamicParamProcessor::process_value("created-${__TIMESTAMP}");
        assert!(result.is_dynamic);
        assert!(result.value.contains("Date.now()"));
    }

    #[test]
    fn test_process_value_uuid() {
        let result = DynamicParamProcessor::process_value("id-${__UUID}");
        assert!(result.is_dynamic);
        assert!(result.value.contains("crypto.randomUUID()"));
    }

    #[test]
    fn test_placeholder_requires_import() {
        assert!(DynamicPlaceholder::UUID.requires_import().is_some());
        assert!(DynamicPlaceholder::VU.requires_import().is_none());
        assert!(DynamicPlaceholder::Iteration.requires_import().is_none());
    }

    #[test]
    fn test_placeholder_requires_global() {
        assert!(DynamicPlaceholder::Counter.requires_global_init().is_some());
        assert!(DynamicPlaceholder::VU.requires_global_init().is_none());
    }

    #[test]
    fn test_process_json_body_static() {
        let body = serde_json::json!({
            "name": "test",
            "count": 42
        });
        let result = DynamicParamProcessor::process_json_body(&body);
        assert!(!result.is_dynamic);
        assert!(result.placeholders.is_empty());
    }

    #[test]
    fn test_process_json_body_dynamic() {
        let body = serde_json::json!({
            "name": "test-${__VU}",
            "id": "${__UUID}"
        });
        let result = DynamicParamProcessor::process_json_body(&body);
        assert!(result.is_dynamic);
        assert!(result.placeholders.contains(&DynamicPlaceholder::VU));
        assert!(result.placeholders.contains(&DynamicPlaceholder::UUID));
    }

    #[test]
    fn test_get_required_imports() {
        let mut placeholders = HashSet::new();
        placeholders.insert(DynamicPlaceholder::UUID);
        placeholders.insert(DynamicPlaceholder::VU);

        let imports = DynamicParamProcessor::get_required_imports(&placeholders);
        assert_eq!(imports.len(), 1);
        assert!(imports[0].contains("webcrypto"));
    }
}
