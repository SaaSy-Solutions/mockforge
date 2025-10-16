//! OpenAPI schema validation and handling
//!
//! This module provides functionality for working with OpenAPI schemas,
//! including validation, type checking, and schema manipulation.

use crate::{Error, Result};
use jsonschema::{self, Draft};
use openapiv3::Schema;
use serde_json::Value;

/// OpenAPI schema wrapper with additional functionality
#[derive(Debug, Clone)]
pub struct OpenApiSchema {
    /// The underlying OpenAPI schema
    pub schema: Schema,
}

impl OpenApiSchema {
    /// Create a new OpenApiSchema from an OpenAPI Schema
    pub fn new(schema: Schema) -> Self {
        Self { schema }
    }

    /// Validate a value against this schema
    pub fn validate(&self, value: &Value) -> Result<()> {
        // Convert OpenAPI schema to JSON Schema for validation
        match serde_json::to_value(&self.schema) {
            Ok(schema_json) => {
                // Create JSON Schema validator
                match jsonschema::options().with_draft(Draft::Draft7).build(&schema_json) {
                    Ok(validator) => {
                        // Validate the value against the schema
                        let errors: Vec<String> =
                            validator.iter_errors(value).map(|error| error.to_string()).collect();
                        if errors.is_empty() {
                            Ok(())
                        } else {
                            Err(Error::validation(errors.join("; ")))
                        }
                    }
                    Err(e) => {
                        Err(Error::validation(format!("Failed to create schema validator: {}", e)))
                    }
                }
            }
            Err(e) => {
                Err(Error::validation(format!("Failed to convert OpenAPI schema to JSON: {}", e)))
            }
        }
    }
}

/// Schema validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Validation errors if any
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    /// Create a failed validation result with errors
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }
}
