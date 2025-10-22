//! Schema-based mock data generation
//!
//! This module generates realistic mock data from JSON Schema definitions
//! found in OpenAPI and AsyncAPI specifications.

use serde_json::{json, Value};
use std::collections::HashMap;

/// Generate mock data from a JSON Schema
pub fn generate_from_schema(schema: &Value) -> Value {
    match schema.get("type").and_then(|t| t.as_str()) {
        Some("object") => generate_object(schema),
        Some("array") => generate_array(schema),
        Some("string") => generate_string(schema),
        Some("number") | Some("integer") => generate_number(schema),
        Some("boolean") => Value::Bool(true),
        Some("null") => Value::Null,
        _ => {
            // Check if it has properties (implicit object)
            if schema.get("properties").is_some() {
                generate_object(schema)
            } else if let Some(example) = schema.get("example") {
                example.clone()
            } else if let Some(default) = schema.get("default") {
                default.clone()
            } else {
                json!("sample-value")
            }
        }
    }
}

/// Generate mock object from schema
fn generate_object(schema: &Value) -> Value {
    let mut obj = serde_json::Map::new();

    // Get required fields (for future use in validation)
    let _required: Vec<String> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    // Process properties
    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        for (key, prop_schema) in properties {
            // Generate value for this property
            let value = if let Some(example) = prop_schema.get("example") {
                example.clone()
            } else if let Some(default) = prop_schema.get("default") {
                default.clone()
            } else {
                generate_from_schema(prop_schema)
            };

            obj.insert(key.clone(), value);
        }
    }

    // If no properties but has additionalProperties
    if obj.is_empty() {
        if let Some(additional) = schema.get("additionalProperties") {
            if additional.is_object() {
                obj.insert("sample_key".to_string(), generate_from_schema(additional));
            }
        }
    }

    Value::Object(obj)
}

/// Generate mock array from schema
fn generate_array(schema: &Value) -> Value {
    let min_items = schema.get("minItems").and_then(|v| v.as_u64()).unwrap_or(2) as usize;

    let max_items = schema
        .get("maxItems")
        .and_then(|v| v.as_u64())
        .unwrap_or(min_items.max(2) as u64) as usize;

    let count = min_items.min(max_items);

    if let Some(items_schema) = schema.get("items") {
        let items: Vec<Value> = (0..count).map(|_| generate_from_schema(items_schema)).collect();
        Value::Array(items)
    } else {
        Value::Array(vec![json!("sample-item-1"), json!("sample-item-2")])
    }
}

/// Generate mock string from schema
fn generate_string(schema: &Value) -> Value {
    // Check for format
    if let Some(format) = schema.get("format").and_then(|f| f.as_str()) {
        return Value::String(match format {
            "date" => "2025-01-15".to_string(),
            "date-time" => "2025-01-15T10:30:00Z".to_string(),
            "email" => "user@example.com".to_string(),
            "uuid" => "550e8400-e29b-41d4-a716-446655440000".to_string(),
            "uri" | "url" => "https://example.com".to_string(),
            "hostname" => "example.com".to_string(),
            "ipv4" => "192.168.1.1".to_string(),
            "ipv6" => "2001:0db8:85a3:0000:0000:8a2e:0370:7334".to_string(),
            _ => format!("sample-{}", format),
        });
    }

    // Check for enum
    if let Some(enums) = schema.get("enum").and_then(|e| e.as_array()) {
        if let Some(first) = enums.first() {
            return first.clone();
        }
    }

    // Check for pattern (simplified)
    if let Some(pattern) = schema.get("pattern").and_then(|p| p.as_str()) {
        // For common patterns, provide reasonable defaults
        if pattern.contains("^[a-zA-Z]") {
            return Value::String("example".to_string());
        } else if pattern.contains("[0-9]") {
            return Value::String("12345".to_string());
        }
    }

    // Default string based on min/max length
    let min_length = schema.get("minLength").and_then(|v| v.as_u64()).unwrap_or(5);
    let length = min_length.max(5) as usize;

    Value::String("sample-string".chars().cycle().take(length).collect())
}

/// Generate mock number from schema
fn generate_number(schema: &Value) -> Value {
    let is_integer = schema
        .get("type")
        .and_then(|t| t.as_str())
        .map(|t| t == "integer")
        .unwrap_or(false);

    // Check for enum
    if let Some(enums) = schema.get("enum").and_then(|e| e.as_array()) {
        if let Some(first) = enums.first() {
            return first.clone();
        }
    }

    let minimum = schema.get("minimum").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let maximum = schema.get("maximum").and_then(|v| v.as_f64()).unwrap_or(100.0);

    let value = (minimum + maximum) / 2.0;

    if is_integer {
        json!(value.round() as i64)
    } else {
        json!(value)
    }
}

/// Generate mock data with improved intelligence for OpenAPI responses
pub fn generate_intelligent_response(
    schema: Option<&Value>,
    examples: Option<&HashMap<String, Value>>,
) -> Value {
    // 1. Try to use example if available
    if let Some(examples_map) = examples {
        if let Some((_key, example_value)) = examples_map.iter().next() {
            return example_value.clone();
        }
    }

    // 2. Try to use schema example
    if let Some(schema_val) = schema {
        if let Some(example) = schema_val.get("example") {
            return example.clone();
        }

        // 3. Generate from schema
        return generate_from_schema(schema_val);
    }

    // 4. Default fallback
    json!({
        "message": "Success",
        "timestamp": "2025-01-15T10:30:00Z",
        "data": {}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_simple_object() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            }
        });

        let result = generate_from_schema(&schema);
        assert!(result.is_object());
        assert!(result.get("name").is_some());
        assert!(result.get("age").is_some());
    }

    #[test]
    fn test_generate_with_format() {
        let schema = json!({
            "type": "string",
            "format": "email"
        });

        let result = generate_from_schema(&schema);
        assert_eq!(result, "user@example.com");
    }

    #[test]
    fn test_generate_array() {
        let schema = json!({
            "type": "array",
            "items": {
                "type": "string"
            },
            "minItems": 3
        });

        let result = generate_from_schema(&schema);
        assert!(result.is_array());
        assert_eq!(result.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_generate_with_enum() {
        let schema = json!({
            "type": "string",
            "enum": ["active", "inactive", "pending"]
        });

        let result = generate_from_schema(&schema);
        assert_eq!(result, "active");
    }

    #[test]
    fn test_generate_nested_object() {
        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "email": { "type": "string", "format": "email" }
                    }
                }
            }
        });

        let result = generate_from_schema(&schema);
        assert!(result.is_object());
        let user = result.get("user").unwrap();
        assert!(user.is_object());
        assert!(user.get("name").is_some());
        assert_eq!(user.get("email").unwrap(), "user@example.com");
    }
}
