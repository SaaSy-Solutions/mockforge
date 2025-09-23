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

                    out.push(ValidationError::new(path.to_string(), schema_info.data_type.clone(), "missing".to_string(), "missing_required")
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
                    (Value::String(es), Value::String(actual_str)) => {
                        // Check string constraints
                        if es.is_empty() && !actual_str.is_empty() {
                            // This is a simple example - could be expanded for length/pattern validation
                        }
                    }
                    (Value::Number(en), Value::Number(an)) => {
                        // Check number constraints - could validate min/max ranges
                        if let (Some(_en_val), Some(_an_val)) = (en.as_f64(), an.as_f64()) {
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
