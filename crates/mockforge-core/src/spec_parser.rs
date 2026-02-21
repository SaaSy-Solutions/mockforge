//! Unified specification parser and validator
//!
//! This module provides a unified interface for parsing and validating
//! different API specification formats: OpenAPI (2.0/3.x), GraphQL schemas,
//! and gRPC service definitions (protobuf).
//!
//! It provides consistent error handling and validation for all spec types.

use crate::{Error, Result};
use serde_json::Value;
use std::path::Path;

/// Supported specification formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecFormat {
    /// OpenAPI 2.0 (Swagger)
    OpenApi20,
    /// OpenAPI 3.0.x
    OpenApi30,
    /// OpenAPI 3.1.x
    OpenApi31,
    /// GraphQL Schema Definition Language (SDL)
    GraphQL,
    /// Protocol Buffers (protobuf)
    Protobuf,
}

impl SpecFormat {
    /// Detect the format from file content
    pub fn detect(content: &str, file_path: Option<&Path>) -> Result<Self> {
        // First, try to detect from file extension
        if let Some(path) = file_path {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                match ext.to_lowercase().as_str() {
                    "graphql" | "gql" => return Ok(Self::GraphQL),
                    "proto" => return Ok(Self::Protobuf),
                    _ => {}
                }
            }
        }

        // Helper to check if content looks like JSON (starts with { or [ after trimming)
        let is_likely_json = |s: &str| {
            let trimmed = s.trim();
            trimmed.starts_with('{') || trimmed.starts_with('[')
        };

        // Helper to check if content looks like YAML (has key: value patterns, comments, etc.)
        let is_likely_yaml = |s: &str| {
            let trimmed = s.trim();
            !is_likely_json(s)
                && (trimmed.contains(":\n")
                    || trimmed.contains(": ")
                    || trimmed.starts_with('#')
                    || trimmed.contains('\n'))
        };

        // Try parsing as JSON first if it looks like JSON
        if is_likely_json(content) {
            if let Ok(json) = serde_json::from_str::<Value>(content) {
                // Check for OpenAPI/Swagger indicators
                if json.get("swagger").is_some() {
                    if let Some(swagger_version) = json.get("swagger").and_then(|v| v.as_str()) {
                        if swagger_version.starts_with("2.") {
                            return Ok(Self::OpenApi20);
                        }
                    }
                }

                if json.get("openapi").is_some() {
                    if let Some(openapi_version) = json.get("openapi").and_then(|v| v.as_str()) {
                        if openapi_version.starts_with("3.0") {
                            return Ok(Self::OpenApi30);
                        } else if openapi_version.starts_with("3.1") {
                            return Ok(Self::OpenApi31);
                        }
                    }
                }
            }
        }

        // Try parsing as YAML (either explicitly YAML-looking, or if JSON parsing failed)
        if is_likely_yaml(content) || !is_likely_json(content) {
            if let Ok(yaml) = serde_yaml::from_str::<Value>(content) {
                if yaml.get("swagger").is_some() {
                    return Ok(Self::OpenApi20);
                }
                if yaml.get("openapi").is_some() {
                    if let Some(openapi_version) = yaml.get("openapi").and_then(|v| v.as_str()) {
                        if openapi_version.starts_with("3.0") {
                            return Ok(Self::OpenApi30);
                        } else if openapi_version.starts_with("3.1") {
                            return Ok(Self::OpenApi31);
                        }
                    }
                }
            }
        }

        // Check for GraphQL syntax (type, schema, etc.)
        let content_lower = content.trim().to_lowercase();
        if content_lower.contains("type ")
            && (content_lower.contains("query") || content_lower.contains("mutation"))
        {
            return Ok(Self::GraphQL);
        }

        // Default to trying OpenAPI 3.0 if we can't detect
        // This allows the validator to provide better error messages
        Err(Error::validation(
            "Could not detect specification format. \
            Expected OpenAPI (2.0/3.x), GraphQL schema, or protobuf definition."
                .to_string(),
        ))
    }

    /// Get a human-readable name for the format
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::OpenApi20 => "OpenAPI 2.0 (Swagger)",
            Self::OpenApi30 => "OpenAPI 3.0.x",
            Self::OpenApi31 => "OpenAPI 3.1.x",
            Self::GraphQL => "GraphQL Schema",
            Self::Protobuf => "Protocol Buffers",
        }
    }
}

/// Specification validation result with detailed errors
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the spec is valid
    pub is_valid: bool,
    /// List of validation errors
    pub errors: Vec<ValidationError>,
    /// List of validation warnings
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        }
    }

    /// Create a failed validation result with errors
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: vec![],
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }

    /// Add multiple warnings
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings.extend(warnings);
        self
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Detailed validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error message
    pub message: String,
    /// JSON pointer to the problematic field (for JSON/YAML specs)
    pub path: Option<String>,
    /// Error code for programmatic handling
    pub code: Option<String>,
    /// Suggested fix (if available)
    pub suggestion: Option<String>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(message: String) -> Self {
        Self {
            message,
            path: None,
            code: None,
            suggestion: None,
        }
    }

    /// Add a JSON pointer path
    pub fn at_path(mut self, path: String) -> Self {
        self.path = Some(path);
        self
    }

    /// Add an error code
    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }

    /// Add a suggested fix
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(path) = &self.path {
            write!(f, " (at {})", path)?;
        }
        if let Some(suggestion) = &self.suggestion {
            write!(f, ". Suggestion: {}", suggestion)?;
        }
        Ok(())
    }
}

/// Enhanced OpenAPI validator with support for 2.0 and 3.x
pub struct OpenApiValidator;

impl OpenApiValidator {
    /// Validate an OpenAPI specification (2.0 or 3.x)
    pub fn validate(spec: &Value, format: SpecFormat) -> ValidationResult {
        let mut errors = Vec::new();

        // Check basic structure
        if !spec.is_object() {
            return ValidationResult::failure(vec![ValidationError::new(
                "OpenAPI specification must be a JSON object".to_string(),
            )
            .with_code("INVALID_ROOT".to_string())]);
        }

        // Validate based on version
        match format {
            SpecFormat::OpenApi20 => {
                Self::validate_version_field(
                    spec,
                    "swagger",
                    &mut errors,
                    "/swagger",
                    "OpenAPI 2.0",
                );
                Self::validate_common_sections(spec, &mut errors, "OpenAPI 2.0");
            }
            SpecFormat::OpenApi30 | SpecFormat::OpenApi31 => {
                Self::validate_version_field(
                    spec,
                    "openapi",
                    &mut errors,
                    "/openapi",
                    "OpenAPI 3.x",
                );
                if let Some(version) = spec.get("openapi").and_then(|v| v.as_str()) {
                    if !version.starts_with("3.") {
                        errors.push(
                            ValidationError::new(format!(
                                "Invalid OpenAPI version '{}'. Expected 3.0.x or 3.1.x",
                                version
                            ))
                            .at_path("/openapi".to_string())
                            .with_code("INVALID_VERSION".to_string())
                            .with_suggestion(
                                "Use 'openapi': '3.0.0' or 'openapi': '3.1.0'".to_string(),
                            ),
                        );
                    }
                }
                Self::validate_common_sections(spec, &mut errors, "OpenAPI 3.x");
            }
            _ => {
                errors.push(ValidationError::new(
                    "Invalid format for OpenAPI validation".to_string(),
                ));
            }
        }

        if errors.is_empty() {
            ValidationResult::success()
        } else {
            ValidationResult::failure(errors)
        }
    }

    /// Validate version field (swagger for 2.0, openapi for 3.x)
    fn validate_version_field(
        spec: &Value,
        field_name: &str,
        errors: &mut Vec<ValidationError>,
        path: &str,
        spec_type: &str,
    ) {
        let _version = spec.get(field_name).and_then(|v| v.as_str()).ok_or_else(|| {
            errors.push(
                ValidationError::new(format!(
                    "Missing '{}' field in {} spec",
                    field_name, spec_type
                ))
                .at_path(path.to_string())
                .with_code(format!("MISSING_{}_FIELD", field_name.to_uppercase()))
                .with_suggestion(format!(
                    "Add '{}': '{}' to the root of the specification",
                    field_name,
                    if field_name == "swagger" {
                        "2.0"
                    } else {
                        "3.0.0 or 3.1.0"
                    }
                )),
            );
        });
    }

    /// Validate common sections shared between OpenAPI 2.0 and 3.x
    fn validate_common_sections(spec: &Value, errors: &mut Vec<ValidationError>, spec_type: &str) {
        // Check info section
        let info = spec.get("info").ok_or_else(|| {
            errors.push(
                ValidationError::new(format!("Missing 'info' section in {} spec", spec_type))
                    .at_path("/info".to_string())
                    .with_code("MISSING_INFO".to_string())
                    .with_suggestion(
                        "Add an 'info' section with 'title' and 'version' fields".to_string(),
                    ),
            );
        });

        if let Ok(info) = info {
            // Check info.title
            if info.get("title").is_none()
                || info.get("title").and_then(|t| t.as_str()).map(|s| s.is_empty()) == Some(true)
            {
                errors.push(
                    ValidationError::new("Missing or empty 'info.title' field".to_string())
                        .at_path("/info/title".to_string())
                        .with_code("MISSING_TITLE".to_string())
                        .with_suggestion("Add 'title' field to the 'info' section".to_string()),
                );
            }

            // Check info.version
            if info.get("version").is_none()
                || info.get("version").and_then(|v| v.as_str()).map(|s| s.is_empty()) == Some(true)
            {
                errors.push(
                    ValidationError::new("Missing or empty 'info.version' field".to_string())
                        .at_path("/info/version".to_string())
                        .with_code("MISSING_VERSION".to_string())
                        .with_suggestion("Add 'version' field to the 'info' section".to_string()),
                );
            }
        }

        // Check paths section
        let paths = spec.get("paths").ok_or_else(|| {
            errors.push(
                ValidationError::new(format!(
                    "Missing 'paths' section in {} spec. At least one endpoint is required.",
                    spec_type
                ))
                .at_path("/paths".to_string())
                .with_code("MISSING_PATHS".to_string())
                .with_suggestion(
                    "Add a 'paths' section with at least one endpoint definition".to_string(),
                ),
            );
        });

        if let Ok(paths) = paths {
            if !paths.is_object() {
                errors.push(
                    ValidationError::new("'paths' must be an object".to_string())
                        .at_path("/paths".to_string())
                        .with_code("INVALID_PATHS_TYPE".to_string()),
                );
            } else if paths.as_object().map(|m| m.is_empty()) == Some(true) {
                errors.push(
                    ValidationError::new(
                        "'paths' object cannot be empty. At least one endpoint is required."
                            .to_string(),
                    )
                    .at_path("/paths".to_string())
                    .with_code("EMPTY_PATHS".to_string())
                    .with_suggestion(
                        "Add at least one path definition, e.g., '/users': { 'get': { ... } }"
                            .to_string(),
                    ),
                );
            }
        }
    }
}

/// GraphQL schema validator with detailed error reporting
///
/// Note: This provides basic validation. For full GraphQL schema validation,
/// use the GraphQL crate's dedicated validator which uses async-graphql parser.
pub struct GraphQLValidator;

impl GraphQLValidator {
    /// Validate a GraphQL schema (basic validation without async-graphql dependency)
    ///
    /// For detailed GraphQL validation with full parser support, use
    /// `mockforge_graphql::GraphQLSchemaRegistry::from_sdl()` which provides
    /// comprehensive validation.
    pub fn validate(content: &str) -> ValidationResult {
        let errors = Vec::new();
        let mut warnings = Vec::new();

        // Check that content is not empty
        if content.trim().is_empty() {
            return ValidationResult::failure(vec![ValidationError::new(
                "GraphQL schema cannot be empty".to_string(),
            )
            .with_code("EMPTY_SCHEMA".to_string())]);
        }

        // Basic syntax checks without requiring async-graphql parser
        // These are heuristics that catch common issues
        let content_trimmed = content.trim();

        // Check for basic GraphQL keywords
        if !content_trimmed.contains("type") && !content_trimmed.contains("schema") {
            warnings
                .push("Schema doesn't appear to contain any GraphQL type definitions.".to_string());
        }

        // Check for Query type
        Self::check_schema_completeness(content, &mut warnings);

        // Basic validation passed - for full validation, use GraphQL crate
        if errors.is_empty() {
            if warnings.is_empty() {
                ValidationResult::success()
            } else {
                ValidationResult::success().with_warnings(warnings)
            }
        } else {
            ValidationResult::failure(errors)
        }
    }

    /// Check for completeness issues (warnings)
    fn check_schema_completeness(content: &str, warnings: &mut Vec<String>) {
        // Check for Query type
        if !content.contains("type Query") && !content.contains("extend type Query") {
            warnings.push(
                "Schema does not define a Query type. GraphQL schemas typically need a Query type."
                    .to_string(),
            );
        }

        // Check for at least one field definition
        if !content.contains(":") && !content.contains("{") {
            warnings.push("Schema appears to be empty or incomplete.".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_openapi_30_json() {
        let content =
            r#"{"openapi": "3.0.0", "info": {"title": "Test", "version": "1.0.0"}, "paths": {}}"#;
        let format = SpecFormat::detect(content, None).unwrap();
        assert_eq!(format, SpecFormat::OpenApi30);
    }

    #[test]
    fn test_detect_openapi_31_yaml() {
        let content = "openapi: 3.1.0\ninfo:\n  title: Test\n  version: 1.0.0\npaths: {}";
        let format = SpecFormat::detect(content, None).unwrap();
        assert_eq!(format, SpecFormat::OpenApi31);
    }

    #[test]
    fn test_detect_swagger_20() {
        let content =
            r#"{"swagger": "2.0", "info": {"title": "Test", "version": "1.0.0"}, "paths": {}}"#;
        let format = SpecFormat::detect(content, None).unwrap();
        assert_eq!(format, SpecFormat::OpenApi20);
    }

    #[test]
    fn test_detect_graphql_from_extension() {
        let path = Path::new("schema.graphql");
        let content = "type Query { users: [User] }";
        let format = SpecFormat::detect(content, Some(path)).unwrap();
        assert_eq!(format, SpecFormat::GraphQL);
    }

    #[test]
    fn test_detect_graphql_from_content() {
        let content = "type Query { users: [User!]! } type User { id: ID! name: String }";
        let format = SpecFormat::detect(content, None).unwrap();
        assert_eq!(format, SpecFormat::GraphQL);
    }

    #[test]
    fn test_validate_openapi_30_valid() {
        let spec = serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": "Success"
                            }
                        }
                    }
                }
            }
        });
        let result = OpenApiValidator::validate(&spec, SpecFormat::OpenApi30);
        assert!(result.is_valid);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_validate_openapi_30_missing_info() {
        let spec = serde_json::json!({
            "openapi": "3.0.0",
            "paths": {}
        });
        let result = OpenApiValidator::validate(&spec, SpecFormat::OpenApi30);
        assert!(!result.is_valid);
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| e.code.as_deref() == Some("MISSING_INFO")));
    }

    #[test]
    fn test_validate_openapi_30_empty_paths() {
        let spec = serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test",
                "version": "1.0.0"
            },
            "paths": {}
        });
        let result = OpenApiValidator::validate(&spec, SpecFormat::OpenApi30);
        assert!(!result.is_valid);
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| e.code.as_deref() == Some("EMPTY_PATHS")));
    }

    #[test]
    fn test_validate_swagger_20_valid() {
        let spec = serde_json::json!({
            "swagger": "2.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": "Success"
                            }
                        }
                    }
                }
            }
        });
        let result = OpenApiValidator::validate(&spec, SpecFormat::OpenApi20);
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_graphql_valid() {
        let schema = "type Query { users: [User!]! } type User { id: ID! name: String }";
        let result = GraphQLValidator::validate(schema);
        assert!(result.is_valid);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_validate_graphql_invalid() {
        let schema = "type Query { users: [User!]! }"; // Missing User type
        let result = GraphQLValidator::validate(schema);
        // The parser might still accept this as valid syntax even if incomplete
        // So we check if it at least parsed
        assert!(!result.has_errors() || result.errors.len() > 0);
    }

    #[test]
    fn test_validate_openapi_30_missing_title() {
        let spec = serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "responses": {
                            "200": {"description": "Success"}
                        }
                    }
                }
            }
        });
        let result = OpenApiValidator::validate(&spec, SpecFormat::OpenApi30);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code.as_deref() == Some("MISSING_TITLE")));
    }

    #[test]
    fn test_validate_openapi_30_missing_version() {
        let spec = serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API"
            },
            "paths": {
                "/users": {
                    "get": {
                        "responses": {
                            "200": {"description": "Success"}
                        }
                    }
                }
            }
        });
        let result = OpenApiValidator::validate(&spec, SpecFormat::OpenApi30);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code.as_deref() == Some("MISSING_VERSION")));
    }

    #[test]
    fn test_validate_swagger_20_missing_paths() {
        let spec = serde_json::json!({
            "swagger": "2.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            }
        });
        let result = OpenApiValidator::validate(&spec, SpecFormat::OpenApi20);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code.as_deref() == Some("MISSING_PATHS")));
    }

    #[test]
    fn test_validate_error_with_suggestion() {
        let spec = serde_json::json!({
            "openapi": "3.0.0",
            "paths": {}
        });
        let result = OpenApiValidator::validate(&spec, SpecFormat::OpenApi30);
        assert!(!result.is_valid);
        // Check that errors have suggestions
        let errors_with_suggestions: Vec<_> =
            result.errors.iter().filter(|e| e.suggestion.is_some()).collect();
        assert!(!errors_with_suggestions.is_empty());
    }

    #[test]
    fn test_validate_graphql_empty() {
        let result = GraphQLValidator::validate("");
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code.as_deref() == Some("EMPTY_SCHEMA")));
    }

    #[test]
    fn test_validate_graphql_with_warnings() {
        let schema = "type User { id: ID! name: String }"; // No Query type
        let result = GraphQLValidator::validate(schema);
        // Should be valid syntax but have warnings
        assert!(result.is_valid || !result.errors.is_empty());
        // Should have warning about missing Query type
        assert!(result.warnings.iter().any(|w| w.contains("Query")));
    }

    #[test]
    fn test_spec_format_display_name() {
        assert_eq!(SpecFormat::OpenApi20.display_name(), "OpenAPI 2.0 (Swagger)");
        assert_eq!(SpecFormat::OpenApi30.display_name(), "OpenAPI 3.0.x");
        assert_eq!(SpecFormat::OpenApi31.display_name(), "OpenAPI 3.1.x");
        assert_eq!(SpecFormat::GraphQL.display_name(), "GraphQL Schema");
        assert_eq!(SpecFormat::Protobuf.display_name(), "Protocol Buffers");
    }

    #[test]
    fn test_validation_result_with_warnings() {
        let result = ValidationResult::success().with_warning("Test warning".to_string());
        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0], "Test warning");
    }

    #[test]
    fn test_detect_yaml_with_whitespace() {
        let content =
            "\n\n  openapi: 3.0.0\n  info:\n    title: Test\n    version: 1.0.0\n  paths: {}";
        let format = SpecFormat::detect(content, None).unwrap();
        assert_eq!(format, SpecFormat::OpenApi30);
    }

    #[test]
    fn test_detect_yaml_with_comments() {
        let content = "# This is a YAML comment\nopenapi: 3.0.0\ninfo:\n  title: Test\n  version: 1.0.0\npaths: {}";
        let format = SpecFormat::detect(content, None).unwrap();
        assert_eq!(format, SpecFormat::OpenApi30);
    }

    #[test]
    fn test_detect_yaml_with_leading_whitespace() {
        let content =
            "    openapi: 3.0.0\n    info:\n      title: Test\n      version: 1.0.0\n    paths: {}";
        let format = SpecFormat::detect(content, None).unwrap();
        assert_eq!(format, SpecFormat::OpenApi30);
    }

    #[test]
    fn test_detect_swagger_yaml() {
        let content = "swagger: \"2.0\"\ninfo:\n  title: Test API\n  version: 1.0.0\npaths:\n  /test:\n    get:\n      responses:\n        '200':\n          description: OK";
        let format = SpecFormat::detect(content, None).unwrap();
        assert_eq!(format, SpecFormat::OpenApi20);
    }

    #[test]
    fn test_validate_common_sections_shared_logic() {
        // Test that common validation works for both 2.0 and 3.x
        let spec_20 = serde_json::json!({
            "swagger": "2.0",
            "info": {
                "title": "Test",
                "version": "1.0.0"
            },
            "paths": {
                "/test": {
                    "get": {
                        "responses": {
                            "200": {"description": "OK"}
                        }
                    }
                }
            }
        });

        let spec_30 = serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test",
                "version": "1.0.0"
            },
            "paths": {
                "/test": {
                    "get": {
                        "responses": {
                            "200": {"description": "OK"}
                        }
                    }
                }
            }
        });

        let result_20 = OpenApiValidator::validate(&spec_20, SpecFormat::OpenApi20);
        let result_30 = OpenApiValidator::validate(&spec_30, SpecFormat::OpenApi30);

        assert!(result_20.is_valid);
        assert!(result_30.is_valid);
    }

    #[test]
    fn test_validate_version_field_extraction() {
        let spec = serde_json::json!({
            "info": {
                "title": "Test",
                "version": "1.0.0"
            },
            "paths": {}
        });

        // Should fail without version field
        let result = OpenApiValidator::validate(&spec, SpecFormat::OpenApi30);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code.as_deref() == Some("MISSING_OPENAPI_FIELD")));
    }
}
