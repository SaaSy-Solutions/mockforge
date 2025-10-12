//! OpenAPI specification handling and utilities
//!
//! This module has been refactored into sub-modules for better organization:
//! - spec: OpenAPI specification loading and parsing
//! - schema: Schema validation and handling
//! - route: Route generation from OpenAPI paths
//! - validation: Request/response validation against schemas
//! - response: Mock response generation based on schemas

// Re-export sub-modules for backward compatibility
pub mod response;
pub mod route;
pub mod schema;
pub mod spec;
pub mod validation;

// Re-export commonly used types (avoiding conflicts)
pub use response::*;
pub use route::*;
pub use schema::*;
pub use spec::*;
pub use validation::*;

/// Stub OpenApiOperation for compilation
#[derive(Debug, Clone)]
pub struct OpenApiOperation {
    pub method: String,
    pub path: String,
    pub operation: openapiv3::Operation,
    pub security: Option<Vec<openapiv3::SecurityRequirement>>,
}

impl OpenApiOperation {
    pub fn new(method: String, path: String, operation: openapiv3::Operation) -> Self {
        Self {
            method,
            path,
            operation: operation.clone(),
            security: operation.security.clone(),
        }
    }

    pub fn from_operation(
        method: &str,
        path: String,
        operation: &openapiv3::Operation,
        _spec: &OpenApiSpec,
    ) -> Self {
        Self::new(method.to_string(), path, operation.clone())
    }
}

/// Type alias for OpenAPI security requirements
pub type OpenApiSecurityRequirement = openapiv3::SecurityRequirement;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_operation_new() {
        let operation = openapiv3::Operation::default();
        let op = OpenApiOperation::new("GET".to_string(), "/test".to_string(), operation);

        assert_eq!(op.method, "GET");
        assert_eq!(op.path, "/test");
        assert!(op.security.is_none());
    }

    #[test]
    fn test_openapi_operation_with_security() {
        let mut operation = openapiv3::Operation::default();
        operation.security = Some(vec![]);

        let op = OpenApiOperation::new("POST".to_string(), "/secure".to_string(), operation);

        assert_eq!(op.method, "POST");
        assert_eq!(op.path, "/secure");
        assert!(op.security.is_some());
    }

    #[test]
    fn test_openapi_operation_from_operation() {
        let operation = openapiv3::Operation::default();
        let spec = OpenApiSpec::from_json(serde_json::json!({
            "openapi": "3.0.0",
            "info": {"title": "Test", "version": "1.0.0"},
            "paths": {}
        }))
        .unwrap();

        let op =
            OpenApiOperation::from_operation("PUT", "/resource".to_string(), &operation, &spec);

        assert_eq!(op.method, "PUT");
        assert_eq!(op.path, "/resource");
    }

    #[test]
    fn test_openapi_operation_clone() {
        let operation = openapiv3::Operation::default();
        let op1 = OpenApiOperation::new("GET".to_string(), "/test".to_string(), operation);
        let op2 = op1.clone();

        assert_eq!(op1.method, op2.method);
        assert_eq!(op1.path, op2.path);
    }
}
