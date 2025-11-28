//! Runtime validation for SDKs
//!
//! This module provides runtime validation utilities that can be used in generated SDKs
//! to validate requests and responses against OpenAPI schemas at runtime.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Runtime validation error with contract diff reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeValidationError {
    /// JSON path to the invalid field (e.g., "body.user.email")
    pub schema_path: String,
    /// Expected schema type or format
    pub expected_type: String,
    /// Actual value received (serialized to string)
    pub actual_value: Option<String>,
    /// Link to contract diff entry (if available)
    pub contract_diff_id: Option<String>,
    /// Whether this is a breaking change
    pub is_breaking_change: bool,
    /// Human-readable error message
    pub message: String,
    /// Additional validation details
    pub details: Option<HashMap<String, serde_json::Value>>,
}

impl RuntimeValidationError {
    /// Create a new runtime validation error
    pub fn new(
        schema_path: impl Into<String>,
        expected_type: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            schema_path: schema_path.into(),
            expected_type: expected_type.into(),
            actual_value: None,
            contract_diff_id: None,
            is_breaking_change: false,
            message: message.into(),
            details: None,
        }
    }

    /// Set the actual value
    pub fn with_actual_value(mut self, value: impl Into<String>) -> Self {
        self.actual_value = Some(value.into());
        self
    }

    /// Set the contract diff ID
    pub fn with_contract_diff_id(mut self, diff_id: impl Into<String>) -> Self {
        self.contract_diff_id = Some(diff_id.into());
        self
    }

    /// Mark as breaking change
    pub fn as_breaking_change(mut self) -> Self {
        self.is_breaking_change = true;
        self
    }

    /// Add additional details
    pub fn with_details(mut self, details: HashMap<String, serde_json::Value>) -> Self {
        self.details = Some(details);
        self
    }
}

/// Runtime validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Validation errors (if any)
    pub errors: Vec<RuntimeValidationError>,
    /// Validation warnings (non-blocking)
    pub warnings: Vec<RuntimeValidationError>,
}

impl RuntimeValidationResult {
    /// Create a successful validation result
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed validation result
    pub fn failure(errors: Vec<RuntimeValidationError>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: RuntimeValidationError) {
        self.valid = false;
        self.errors.push(error);
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: RuntimeValidationError) {
        self.warnings.push(warning);
    }
}

/// Schema metadata for runtime validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    /// Schema identifier (e.g., "User", "CreateUserRequest")
    pub id: String,
    /// JSON Schema representation
    pub schema: serde_json::Value,
    /// Whether this schema is required
    pub required: bool,
    /// Contract diff ID (if this schema has been changed)
    pub contract_diff_id: Option<String>,
}

/// Runtime validator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeValidatorConfig {
    /// Whether to validate requests
    pub validate_requests: bool,
    /// Whether to validate responses
    pub validate_responses: bool,
    /// Whether to throw errors on validation failure (vs. just logging)
    pub throw_on_error: bool,
    /// Whether to include contract diff references in errors
    pub include_contract_diffs: bool,
    /// Schema registry (schema_id -> SchemaMetadata)
    pub schemas: HashMap<String, SchemaMetadata>,
}

impl Default for RuntimeValidatorConfig {
    fn default() -> Self {
        Self {
            validate_requests: false,
            validate_responses: false,
            throw_on_error: false,
            include_contract_diffs: true,
            schemas: HashMap::new(),
        }
    }
}

impl RuntimeValidatorConfig {
    /// Create a new runtime validator config
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable request validation
    pub fn with_request_validation(mut self, enabled: bool) -> Self {
        self.validate_requests = enabled;
        self
    }

    /// Enable response validation
    pub fn with_response_validation(mut self, enabled: bool) -> Self {
        self.validate_responses = enabled;
        self
    }

    /// Configure error throwing behavior
    pub fn with_throw_on_error(mut self, throw: bool) -> Self {
        self.throw_on_error = throw;
        self
    }

    /// Add a schema to the registry
    pub fn add_schema(&mut self, metadata: SchemaMetadata) {
        self.schemas.insert(metadata.id.clone(), metadata);
    }
}
