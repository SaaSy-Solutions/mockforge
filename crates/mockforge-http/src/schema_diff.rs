//! JSON schema diff utilities for 422 responses.
//!
//! This module provides comprehensive schema validation diffing capabilities
//! for generating informative 422 error responses that help developers understand
//! exactly what schema validation issues exist in their API requests.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// JSON path to the field with validation issue
    pub path: String,
    /// Expected schema constraint or value type
    pub expected: String,
    /// What was actually found in the request
    pub found: String,
    /// Human-readable error message
    pub message: Option<String>,
    /// Error classification for client handling
    pub error_type: String,
    /// Additional context about the expected schema
    pub schema_info: Option<SchemaInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// Expected data type
    pub data_type: String,
    /// Required constraint
    pub required: Option<bool>,
    /// Format constraint (e.g., "email", "uuid")
    pub format: Option<String>,
    /// Minimum value constraint
    pub minimum: Option<f64>,
    /// Maximum value constraint
    pub maximum: Option<f64>,
    /// Minimum length for strings/arrays
    pub min_length: Option<usize>,
    /// Maximum length for strings/arrays
    pub max_length: Option<usize>,
    /// Regex pattern for strings
    pub pattern: Option<String>,
    /// Enum values if applicable
    pub enum_values: Option<Vec<Value>>,
    /// Whether this field accepts additional properties
    pub additional_properties: Option<bool>,
}

impl ValidationError {
    pub fn new(path: String, expected: String, found: String, error_type: &str) -> Self {
        Self {
            path,
            expected,
            found,
            message: None,
            error_type: error_type.to_string(),
            schema_info: None,
        }
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    pub fn with_schema_info(mut self, schema_info: SchemaInfo) -> Self {
        self.schema_info = Some(schema_info);
        self
    }
}

// Keep the old FieldError for backward compatibility
#[derive(Debug, Clone)]
pub struct FieldError {
    pub path: String,
    pub expected: String,
    pub found: String,
    pub message: Option<String>,
}

impl From<ValidationError> for FieldError {
    fn from(error: ValidationError) -> Self {
        Self {
            path: error.path,
            expected: error.expected,
            found: error.found,
            message: error.message,
        }
    }
}

pub fn diff(expected_schema: &Value, actual: &Value) -> Vec<FieldError> {
    let mut out = Vec::new();
    walk(expected_schema, actual, "", &mut out);
    out
}

fn walk(expected: &Value, actual: &Value, path: &str, out: &mut Vec<FieldError>) {
    match (expected, actual) {
        (Value::Object(eo), Value::Object(ao)) => {
            for (k, ev) in eo {
                let np = format!("{}/{}", path, k);
                if let Some(av) = ao.get(k) {
                    walk(ev, av, &np, out);
                } else {
                    out.push(FieldError {
                        path: np,
                        expected: type_of(ev),
                        found: "missing".into(),
                        message: Some("required".into()),
                    });
                }
            }
        }
        (Value::Array(ea), Value::Array(aa)) => {
            if let Some(esample) = ea.first() {
                for (i, av) in aa.iter().enumerate() {
                    let np = format!("{}/{}", path, i);
                    walk(esample, av, &np, out);
                }
            }
        }
        (e, a) => {
            let et = type_of(e);
            let at = type_of(a);
            if et != at {
                out.push(FieldError {
                    path: path.into(),
                    expected: et,
                    found: at,
                    message: None,
                });
            }
        }
    }
}

fn type_of(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "bool".to_string(),
        Value::Number(n) => if n.is_i64() { "integer" } else { "number" }.to_string(),
        Value::String(_) => "string".to_string(),
        Value::Array(_) => "array".to_string(),
        Value::Object(_) => "object".to_string(),
    }
}

pub fn to_422_json(errors: Vec<FieldError>) -> Value {
    json!({
        "error": "Schema validation failed",
        "details": errors.into_iter().map(|e| json!({
            "path": e.path,
            "expected": e.expected,
            "found": e.found,
            "message": e.message
        })).collect::<Vec<_>>()
    })
}

/// Enhanced validation diff with comprehensive error analysis
/// This function performs detailed validation between expected and actual JSON
/// and provides rich schema context for better error reporting
pub fn validation_diff(expected_schema: &Value, actual: &Value) -> Vec<ValidationError> {
    let mut out = Vec::new();
    validation_walk(expected_schema, actual, "", &mut out);
    out
}

fn validation_walk(expected: &Value, actual: &Value, path: &str, out: &mut Vec<ValidationError>) {
    match (expected, actual) {
        (Value::Object(eo), Value::Object(ao)) => {
            // Check for missing required fields
            for (k, ev) in eo {
                let np = format!("{}/{}", path, k);
                if let Some(av) = ao.get(k) {
                    // Field exists, validate its value
                    validation_walk(ev, av, &np, out);
                } else {
                    // Missing required field
                    let schema_info = SchemaInfo {
                        data_type: type_of(ev).clone(),
                        required: Some(true),
                        format: None,
                        minimum: None,
                        maximum: None,
                        min_length: None,
                        max_length: None,
                        pattern: None,
                        enum_values: None,
                        additional_properties: None,
                    };

                    let error_msg = format!("Missing required field '{}' of type {}", k, schema_info.data_type);

                    out.push(ValidationError::new(path.to_string(), schema_info.data_type, "missing".to_string(), "missing_required")
                        .with_message(error_msg)
                        .with_schema_info(schema_info));
                }
            }

            // Check for unexpected additional fields
            for k in ao.keys() {
                if !eo.contains_key(k) {
                    let np = format!("{}/{}", path, k);
                    let error_msg = format!("Unexpected additional field '{}' found", k);

                    out.push(ValidationError::new(np, "not_allowed".to_string(), type_of(&ao[k]).clone(), "additional_property")
                        .with_message(error_msg));
                }
            }
        }
        (Value::Array(ea), Value::Array(aa)) => {
            // Validate array items
            if let Some(esample) = ea.first() {
                for (i, av) in aa.iter().enumerate() {
                    let np = format!("{}/{}", path, i);
                    validation_walk(esample, av, &np, out);
                }

                // Check array length constraints if the expected specifies them
                if let Some(arr_size) = esample.as_array().map(|a| a.len()) {
                    if aa.len() != arr_size {
                        let schema_info = SchemaInfo {
                            data_type: "array".to_string(),
                            required: None,
                            format: None,
                            minimum: None,
                            maximum: None,
                            min_length: Some(arr_size),
                            max_length: Some(arr_size),
                            pattern: None,
                            enum_values: None,
                            additional_properties: None,
                        };

                        let error_msg = format!("Array size mismatch: expected {} items, found {}", arr_size, aa.len());

                        out.push(ValidationError::new(path.to_string(), format!("array[{}]", arr_size), format!("array[{}]", aa.len()), "length_mismatch")
                            .with_message(error_msg)
                            .with_schema_info(schema_info));
                    }
                }
            } else {
                // Expected array is empty but actual has items
                if !aa.is_empty() {
                    let error_msg = format!("Expected empty array, but found {} items", aa.len());

                    out.push(ValidationError::new(path.to_string(), "empty_array".to_string(), format!("array[{}]", aa.len()), "unexpected_items")
                        .with_message(error_msg));
                }
            }
        }
        (e, a) => {
            let et = type_of(e);
            let at = type_of(a);

            if et != at {
                // Type mismatch - provide detailed context based on the expected type
                let schema_info = SchemaInfo {
                    data_type: et.clone(),
                    required: None,
                    format: None,  // Could be expanded to extract format info
                    minimum: None,
                    maximum: None,
                    min_length: None,
                    max_length: None,
                    pattern: None,
                    enum_values: None,
                    additional_properties: None,
                };

                let error_msg = format!("Type mismatch: expected {}, found {}", et, at);

                out.push(ValidationError::new(path.to_string(), et, at, "type_mismatch")
                    .with_message(error_msg)
                    .with_schema_info(schema_info));
            } else {
                // Same type but might have other constraints - check string/number specifics
                match (e, a) {
                    (Value::String(es), Value::String(as)) => {
                        // Check string constraints
                        if es.is_empty() && !as.is_empty() {
                            // This is a simple example - could be expanded for length/pattern validation
                        }
                    }
                    (Value::Number(en), Value::Number(an)) => {
                        // Check number constraints - could validate min/max ranges
                        if let (Some(en_val), Some(an_val)) = (en.as_f64(), an.as_f64()) {
                            // Example: could flag if values are outside expected ranges
                        }
                    }
                    _ => {} // Other same-type validations could be added
                }
            }
        }
    }
}

/// Generate enhanced 422 error response with detailed schema information
pub fn to_enhanced_422_json(errors: Vec<ValidationError>) -> Value {
    json!({
        "error": "Schema validation failed",
        "message": "Request data doesn't match expected schema. See details below for specific issues.",
        "validation_errors": errors.iter().map(|e| {
            json!({
                "path": e.path,
                "expected": e.expected,
                "found": e.found,
                "error_type": e.error_type,
                "message": e.message,
                "schema_info": e.schema_info
            })
        }).collect::<Vec<_>>(),
        "help": {
            "tips": [
                "Check that all required fields are present",
                "Ensure field types match the expected schema",
                "Verify string formats and patterns",
                "Confirm number values are within required ranges",
                "Remove any unexpected fields"
            ],
            "documentation": "Refer to API specification for complete field definitions"
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_new() {
        let error = ValidationError::new(
            "/user/name".to_string(),
            "string".to_string(),
            "number".to_string(),
            "type_mismatch",
        );

        assert_eq!(error.path, "/user/name");
        assert_eq!(error.expected, "string");
        assert_eq!(error.found, "number");
        assert_eq!(error.error_type, "type_mismatch");
        assert!(error.message.is_none());
        assert!(error.schema_info.is_none());
    }

    #[test]
    fn test_validation_error_with_message() {
        let error = ValidationError::new(
            "/user/age".to_string(),
            "integer".to_string(),
            "string".to_string(),
            "type_mismatch",
        )
        .with_message("Expected integer, got string".to_string());

        assert_eq!(error.message, Some("Expected integer, got string".to_string()));
    }

    #[test]
    fn test_validation_error_with_schema_info() {
        let schema_info = SchemaInfo {
            data_type: "string".to_string(),
            required: Some(true),
            format: Some("email".to_string()),
            minimum: None,
            maximum: None,
            min_length: Some(5),
            max_length: Some(100),
            pattern: None,
            enum_values: None,
            additional_properties: None,
        };

        let error = ValidationError::new(
            "/user/email".to_string(),
            "string".to_string(),
            "missing".to_string(),
            "missing_required",
        )
        .with_schema_info(schema_info.clone());

        assert!(error.schema_info.is_some());
        let info = error.schema_info.unwrap();
        assert_eq!(info.data_type, "string");
        assert_eq!(info.required, Some(true));
        assert_eq!(info.format, Some("email".to_string()));
    }

    #[test]
    fn test_field_error_from_validation_error() {
        let validation_error = ValidationError::new(
            "/user/id".to_string(),
            "integer".to_string(),
            "string".to_string(),
            "type_mismatch",
        )
        .with_message("Type mismatch".to_string());

        let field_error: FieldError = validation_error.into();

        assert_eq!(field_error.path, "/user/id");
        assert_eq!(field_error.expected, "integer");
        assert_eq!(field_error.found, "string");
        assert_eq!(field_error.message, Some("Type mismatch".to_string()));
    }

    #[test]
    fn test_type_of_null() {
        let value = json!(null);
        assert_eq!(type_of(&value), "null");
    }

    #[test]
    fn test_type_of_bool() {
        let value = json!(true);
        assert_eq!(type_of(&value), "bool");
    }

    #[test]
    fn test_type_of_integer() {
        let value = json!(42);
        assert_eq!(type_of(&value), "integer");
    }

    #[test]
    fn test_type_of_number() {
        let value = json!(42.5);
        assert_eq!(type_of(&value), "number");
    }

    #[test]
    fn test_type_of_string() {
        let value = json!("hello");
        assert_eq!(type_of(&value), "string");
    }

    #[test]
    fn test_type_of_array() {
        let value = json!([1, 2, 3]);
        assert_eq!(type_of(&value), "array");
    }

    #[test]
    fn test_type_of_object() {
        let value = json!({"key": "value"});
        assert_eq!(type_of(&value), "object");
    }

    #[test]
    fn test_diff_matching_objects() {
        let expected = json!({"name": "John", "age": 30});
        let actual = json!({"name": "John", "age": 30});

        let errors = diff(&expected, &actual);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_diff_missing_field() {
        let expected = json!({"name": "John", "age": 30});
        let actual = json!({"name": "John"});

        let errors = diff(&expected, &actual);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path, "/age");
        assert_eq!(errors[0].expected, "integer");
        assert_eq!(errors[0].found, "missing");
    }

    #[test]
    fn test_diff_type_mismatch() {
        let expected = json!({"name": "John", "age": 30});
        let actual = json!({"name": "John", "age": "thirty"});

        let errors = diff(&expected, &actual);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path, "/age");
        assert_eq!(errors[0].expected, "integer");
        assert_eq!(errors[0].found, "string");
    }

    #[test]
    fn test_diff_nested_objects() {
        let expected = json!({
            "user": {
                "name": "John",
                "address": {
                    "city": "NYC"
                }
            }
        });
        let actual = json!({
            "user": {
                "name": "John",
                "address": {
                    "city": 123
                }
            }
        });

        let errors = diff(&expected, &actual);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path, "/user/address/city");
        assert_eq!(errors[0].expected, "string");
        assert_eq!(errors[0].found, "integer");
    }

    #[test]
    fn test_diff_arrays() {
        let expected = json!([{"id": 1}]);
        let actual = json!([{"id": 1}, {"id": 2}]);

        let errors = diff(&expected, &actual);
        assert_eq!(errors.len(), 0); // Both items match the expected structure
    }

    #[test]
    fn test_diff_array_type_mismatch() {
        let expected = json!([{"id": 1}]);
        let actual = json!([{"id": "one"}]);

        let errors = diff(&expected, &actual);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path, "/0/id");
        assert_eq!(errors[0].expected, "integer");
        assert_eq!(errors[0].found, "string");
    }

    #[test]
    fn test_to_422_json() {
        let errors = vec![
            FieldError {
                path: "/name".to_string(),
                expected: "string".to_string(),
                found: "number".to_string(),
                message: None,
            },
            FieldError {
                path: "/email".to_string(),
                expected: "string".to_string(),
                found: "missing".to_string(),
                message: Some("required".to_string()),
            },
        ];

        let result = to_422_json(errors);
        assert_eq!(result["error"], "Schema validation failed");
        assert_eq!(result["details"].as_array().unwrap().len(), 2);
        assert_eq!(result["details"][0]["path"], "/name");
        assert_eq!(result["details"][1]["path"], "/email");
    }

    #[test]
    fn test_validation_diff_matching_objects() {
        let expected = json!({"name": "John", "age": 30});
        let actual = json!({"name": "John", "age": 30});

        let errors = validation_diff(&expected, &actual);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validation_diff_missing_required_field() {
        let expected = json!({"name": "John", "age": 30});
        let actual = json!({"name": "John"});

        let errors = validation_diff(&expected, &actual);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type, "missing_required");
        assert!(errors[0].message.as_ref().unwrap().contains("Missing required field"));
        assert!(errors[0].schema_info.is_some());
    }

    #[test]
    fn test_validation_diff_additional_property() {
        let expected = json!({"name": "John"});
        let actual = json!({"name": "John", "age": 30});

        let errors = validation_diff(&expected, &actual);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type, "additional_property");
        assert!(errors[0].message.as_ref().unwrap().contains("Unexpected additional field"));
    }

    #[test]
    fn test_validation_diff_type_mismatch() {
        let expected = json!({"age": 30});
        let actual = json!({"age": "thirty"});

        let errors = validation_diff(&expected, &actual);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type, "type_mismatch");
        assert_eq!(errors[0].expected, "integer");
        assert_eq!(errors[0].found, "string");
        assert!(errors[0].schema_info.is_some());
    }

    #[test]
    fn test_validation_diff_array_items() {
        let expected = json!([{"id": 1}]);
        let actual = json!([{"id": "one"}]);

        let errors = validation_diff(&expected, &actual);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path, "/0/id");
        assert_eq!(errors[0].error_type, "type_mismatch");
    }

    #[test]
    fn test_validation_diff_empty_array_with_items() {
        let expected = json!([]);
        let actual = json!([1, 2, 3]);

        let errors = validation_diff(&expected, &actual);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type, "unexpected_items");
        assert!(errors[0].message.as_ref().unwrap().contains("Expected empty array"));
    }

    #[test]
    fn test_to_enhanced_422_json() {
        let errors = vec![
            ValidationError::new(
                "/name".to_string(),
                "string".to_string(),
                "number".to_string(),
                "type_mismatch",
            )
            .with_message("Type mismatch: expected string, found number".to_string()),
        ];

        let result = to_enhanced_422_json(errors);
        assert_eq!(result["error"], "Schema validation failed");
        assert!(result["message"].as_str().unwrap().contains("doesn't match expected schema"));
        assert_eq!(result["validation_errors"].as_array().unwrap().len(), 1);
        assert!(result["help"]["tips"].is_array());
        assert!(result["timestamp"].is_string());
    }

    #[test]
    fn test_validation_diff_nested_objects() {
        let expected = json!({
            "user": {
                "profile": {
                    "name": "John",
                    "age": 30
                }
            }
        });
        let actual = json!({
            "user": {
                "profile": {
                    "name": "John"
                }
            }
        });

        let errors = validation_diff(&expected, &actual);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].path.contains("/user/profile"));
        assert_eq!(errors[0].error_type, "missing_required");
    }

    #[test]
    fn test_validation_diff_multiple_errors() {
        let expected = json!({
            "name": "John",
            "age": 30,
            "email": "john@example.com"
        });
        let actual = json!({
            "name": 123,
            "extra": "field"
        });

        let errors = validation_diff(&expected, &actual);
        // Should have: type mismatch for name, missing age, missing email, additional property 'extra'
        assert!(errors.len() >= 3);

        let error_types: Vec<_> = errors.iter().map(|e| e.error_type.as_str()).collect();
        assert!(error_types.contains(&"type_mismatch"));
        assert!(error_types.contains(&"missing_required"));
        assert!(error_types.contains(&"additional_property"));
    }
}
