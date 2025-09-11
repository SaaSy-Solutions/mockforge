//! Schema validation logic for MockForge

use crate::{Error, Result};
use jsonschema::{self, Draft, Validator as JSONSchema};
use serde_json::Value;

/// Schema validator for different formats
#[derive(Debug)]
pub enum Validator {
    /// JSON Schema validator
    JsonSchema(JSONSchema),
    /// OpenAPI schema validator (placeholder)
    OpenApi,
    /// Protobuf validator (placeholder)
    Protobuf,
}

impl Validator {
    /// Create a JSON Schema validator from a schema
    pub fn from_json_schema(schema: &Value) -> Result<Self> {
        let compiled = jsonschema::options()
            .with_draft(Draft::Draft7)
            .build(schema)
            .map_err(|e| Error::validation(format!("Failed to compile JSON schema: {}", e)))?;

        Ok(Self::JsonSchema(compiled))
    }

    /// Create an OpenAPI validator
    pub fn from_openapi(spec: &Value) -> Result<Self> {
        // Validate that it's a valid OpenAPI spec
        if let Some(openapi_version) = spec.get("openapi") {
            if let Some(version_str) = openapi_version.as_str() {
                if !version_str.starts_with("3.") {
                    return Err(Error::validation(format!(
                        "Unsupported OpenAPI version: {}. Only 3.x is supported",
                        version_str
                    )));
                }
            }
        }

        // TODO: Could store the spec for more advanced validation
        Ok(Self::OpenApi)
    }

    /// Create a Protobuf validator (placeholder implementation)
    pub fn from_protobuf(_descriptor: &[u8]) -> Result<Self> {
        // TODO: Implement Protobuf validation
        Ok(Self::Protobuf)
    }

    /// Validate data against the schema
    pub fn validate(&self, data: &Value) -> Result<()> {
        match self {
            Self::JsonSchema(schema) => {
                let mut errors = Vec::new();
                for error in schema.iter_errors(data) {
                    errors.push(error.to_string());
                }

                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(Error::validation(format!("Validation failed: {}", errors.join(", "))))
                }
            }
            Self::OpenApi => {
                // Basic OpenAPI validation - for now just check if it's valid JSON
                // TODO: Implement full OpenAPI schema validation
                if data.is_object() {
                    Ok(())
                } else {
                    Err(Error::validation("OpenAPI validation expects an object".to_string()))
                }
            }
            Self::Protobuf => {
                // TODO: Implement Protobuf validation
                tracing::warn!("Protobuf validation not yet implemented");
                Ok(())
            }
        }
    }

    /// Check if validation is supported for this validator type
    pub fn is_implemented(&self) -> bool {
        match self {
            Self::JsonSchema(_) => true,
            Self::OpenApi => false,
            Self::Protobuf => false,
        }
    }
}

/// Validation result with detailed error information
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Validation errors (empty if valid)
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed validation result
    pub fn failure(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add a warning to the result
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }
}

/// Validate JSON data against a JSON Schema
pub fn validate_json_schema(data: &Value, schema: &Value) -> ValidationResult {
    match Validator::from_json_schema(schema) {
        Ok(validator) => match validator.validate(data) {
            Ok(_) => ValidationResult::success(),
            Err(Error::Validation { message }) => ValidationResult::failure(vec![message]),
            Err(e) => ValidationResult::failure(vec![format!("Unexpected error: {}", e)]),
        },
        Err(e) => ValidationResult::failure(vec![format!("Schema compilation error: {}", e)]),
    }
}

/// Validate OpenAPI spec compliance
pub fn validate_openapi(data: &Value, spec: &Value) -> ValidationResult {
    // Basic validation - check if the spec has required OpenAPI fields
    if !spec.is_object() {
        return ValidationResult::failure(vec!["OpenAPI spec must be an object".to_string()]);
    }

    let spec_obj = spec.as_object().unwrap();

    // Check required fields
    let mut errors = Vec::new();

    if !spec_obj.contains_key("openapi") {
        errors.push("Missing required 'openapi' field".to_string());
    } else if let Some(version) = spec_obj.get("openapi").and_then(|v| v.as_str()) {
        if !version.starts_with("3.") {
            errors.push(format!("Unsupported OpenAPI version: {}. Only 3.x is supported", version));
        }
    }

    if !spec_obj.contains_key("info") {
        errors.push("Missing required 'info' field".to_string());
    } else if let Some(info) = spec_obj.get("info").and_then(|v| v.as_object()) {
        if !info.contains_key("title") {
            errors.push("Missing required 'info.title' field".to_string());
        }
        if !info.contains_key("version") {
            errors.push("Missing required 'info.version' field".to_string());
        }
    }

    if !spec_obj.contains_key("paths") {
        errors.push("Missing required 'paths' field".to_string());
    }

    if !errors.is_empty() {
        return ValidationResult::failure(errors);
    }

    // For now, just validate that data is a valid object
    if !data.is_object() {
        return ValidationResult::failure(vec![
            "Request/response data must be a JSON object".to_string()
        ]);
    }

    ValidationResult::success()
        .with_warning("Full OpenAPI schema validation not yet implemented".to_string())
}

/// Validate Protobuf message (placeholder)
pub fn validate_protobuf(_data: &[u8], _descriptor: &[u8]) -> ValidationResult {
    ValidationResult::success().with_warning("Protobuf validation not yet implemented".to_string())
}
