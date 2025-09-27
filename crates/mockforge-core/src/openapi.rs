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
