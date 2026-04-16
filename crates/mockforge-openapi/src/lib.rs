//! # MockForge OpenAPI
//!
//! OpenAPI 3.x / Swagger 2.0 specification loading, parsing, schema validation,
//! and response selection primitives for MockForge.
//!
//! This crate owns the [`OpenApiSpec`] domain model used throughout MockForge to
//! drive mock responses, routing, validation, and contract analysis. It was
//! extracted from `mockforge-core` so that downstream crates (contract drift,
//! intelligence, recorders, etc.) can depend on OpenAPI types without pulling in
//! the entirety of core.
//!
//! ## Modules
//!
//! - [`spec`] — [`OpenApiSpec`] loader (file / string / JSON / YAML), schema
//!   resolution, operation iteration. Handles transparent Swagger 2.0 → OpenAPI
//!   3.0 conversion via [`swagger_convert`].
//! - [`schema`] — [`OpenApiSchema`] wrapper with JSON-Schema-backed validation.
//! - [`multi_spec`] — [`MultiSpec`], load and merge multiple OpenAPI docs with
//!   conflict strategies.
//! - [`response_selection`] — [`ResponseSelectionMode`] + [`ResponseSelector`] for
//!   choosing between multiple example responses (first / scenario / sequential /
//!   random / weighted).
//! - [`spec_parser`] — unified [`OpenApiValidator`] / [`GraphQLValidator`] +
//!   [`SpecFormat`] detector covering OpenAPI 2.0/3.0/3.1 and GraphQL.
//! - [`validation`] — request/response validation helpers against an
//!   [`OpenApiSpec`].
//! - [`swagger_convert`] — Swagger 2.0 → OpenAPI 3.0 conversion helper used by
//!   [`spec`] (re-exported for advanced callers).

pub mod custom_fixture;
pub mod multi_spec;
pub mod request_fingerprint;
pub mod response;
pub mod response_rewriter;
pub mod response_trace;
pub mod route;
pub mod schema;
pub mod spec;
pub mod spec_parser;
pub mod swagger_convert;
pub mod validation;

pub use custom_fixture::CustomFixtureLoader;
pub use request_fingerprint::RequestFingerprint;
pub use response_rewriter::ResponseRewriter;

/// `ResponseSelectionMode` / `ResponseSelector` live in
/// [`mockforge_foundation::response_selection`] — it's a generic selection
/// primitive used by non-OpenAPI response trace code. Re-export here so
/// `mockforge_openapi::response_selection::...` paths keep resolving.
pub use mockforge_foundation::response_selection;

// Mirror the blanket re-exports that the legacy `mockforge_core::openapi`
// module exposed, so consumers can continue to glob-import from the facade.
pub use response::*;
pub use response_selection::*;
pub use route::*;
pub use schema::*;
pub use spec::*;
pub use validation::*;

// Named re-exports for commonly used items.
pub use spec_parser::{GraphQLValidator, OpenApiValidator, SpecFormat};

/// Wrapper for OpenAPI operation with enhanced metadata.
#[derive(Debug, Clone)]
pub struct OpenApiOperation {
    /// HTTP method (GET, POST, PUT, etc.)
    pub method: String,
    /// API path (e.g., "/api/users/{id}")
    pub path: String,
    /// OpenAPI operation specification
    pub operation: openapiv3::Operation,
    /// Security requirements for this operation
    pub security: Option<Vec<openapiv3::SecurityRequirement>>,
}

impl OpenApiOperation {
    /// Create a new OpenAPI operation wrapper
    pub fn new(method: String, path: String, operation: openapiv3::Operation) -> Self {
        Self {
            method,
            path,
            operation: operation.clone(),
            security: operation.security.clone(),
        }
    }

    /// Create an operation from an OpenAPI operation reference
    pub fn from_operation(
        method: &str,
        path: String,
        operation: &openapiv3::Operation,
        _spec: &OpenApiSpec,
    ) -> Self {
        Self::new(method.to_string(), path, operation.clone())
    }
}

/// Type alias for OpenAPI security requirements.
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
        let operation = openapiv3::Operation {
            security: Some(vec![]),
            ..Default::default()
        };

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
