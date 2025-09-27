//! Request/response validation logic
//!
//! This module provides validation functionality for OpenAPI-based routes,
//! including request validation, response validation, and error handling.

use jsonschema::validate;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// Validation mode for requests and responses
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum ValidationMode {
    Disabled,
    Warn,
    Enforce,
}

impl Default for ValidationMode {
    fn default() -> Self {
        ValidationMode::Warn
    }
}

/// Validation options for OpenAPI route validation
#[derive(Debug, Clone)]
pub struct ValidationOptions {
    pub request_mode: ValidationMode,
    pub aggregate_errors: bool,
    pub validate_responses: bool,
    pub overrides: HashMap<String, ValidationMode>,
    /// Skip validation for request paths starting with any of these prefixes
    pub admin_skip_prefixes: Vec<String>,
    /// Expand templating tokens in responses/examples
    pub response_template_expand: bool,
    /// HTTP status code to return when validation fails
    pub validation_status: Option<u16>,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            request_mode: ValidationMode::Enforce,
            aggregate_errors: true,
            validate_responses: false,
            overrides: HashMap::new(),
            admin_skip_prefixes: Vec::new(),
            response_template_expand: false,
            validation_status: None,
        }
    }
}

/// Validation error information
#[derive(Debug, Clone, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub expected: Option<Value>,
    pub actual: Option<Value>,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationError>,
}

/// Validation context for tracking errors during validation
#[derive(Debug, Default)]
pub struct ValidationContext {
    errors: Vec<ValidationError>,
    warnings: Vec<ValidationError>,
}

impl ValidationContext {
    /// Create a new validation context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an error to the validation context
    pub fn add_error(&mut self, field: String, message: String) {
        self.errors.push(ValidationError {
            field,
            message,
            expected: None,
            actual: None,
        });
    }

    /// Add an error with expected and actual values
    pub fn add_error_with_values(
        &mut self,
        field: String,
        message: String,
        expected: Value,
        actual: Value,
    ) {
        self.errors.push(ValidationError {
            field,
            message,
            expected: Some(expected),
            actual: Some(actual),
        });
    }

    /// Add a warning to the validation context
    pub fn add_warning(&mut self, field: String, message: String) {
        self.warnings.push(ValidationError {
            field,
            message,
            expected: None,
            actual: None,
        });
    }

    /// Get the validation result
    pub fn result(&self) -> ValidationResult {
        ValidationResult {
            is_valid: self.errors.is_empty(),
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
        }
    }

    /// Check if validation has errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if validation has warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Validate a JSON value against a schema
pub fn validate_json_value(value: &Value, schema: &Value) -> ValidationResult {
    let mut ctx = ValidationContext::new();

    // Basic validation - check required fields and types
    validate_against_schema(value, schema, &mut ctx);

    ctx.result()
}

/// Validate a value against a JSON schema
fn validate_against_schema(value: &Value, schema: &Value, ctx: &mut ValidationContext) {
    // Use proper JSON Schema validation
    if let Err(error) = validate(schema, value) {
        let field = error.instance_path.to_string();
        let message = error.to_string();
        ctx.add_error(field, message);
    }
}
