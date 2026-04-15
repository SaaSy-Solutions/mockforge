//! OpenAPI specification handling and utilities
//!
//! The core OpenAPI data model (`OpenApiSpec`, `OpenApiSchema`, response
//! selection, multi-spec, validation, spec format detection, and the Swagger
//! 2.0 → OpenAPI 3.0 converter) lives in the dedicated
//! [`mockforge_openapi`] crate. This module re-exports those types for
//! backwards compatibility with code that still imports from
//! `mockforge_core::openapi::*`.
//!
//! Response generation (`response/`), route construction (`route.rs`), and
//! response tracing (`response_trace.rs`) remain here because they still
//! depend on core-internal types (MockAI, templating, AI response config).
//! They will move to `mockforge-openapi` in subsequent phases of the
//! extraction.

// Re-export the extracted modules so existing `mockforge_core::openapi::foo`
// paths keep resolving.
pub use mockforge_openapi::{
    multi_spec, response_selection, schema, spec, swagger_convert, validation, OpenApiOperation,
    OpenApiSchema, OpenApiSecurityRequirement, OpenApiSpec, ResponseSelectionMode,
    ResponseSelector,
};

// Modules that still live in core.
pub mod response;
pub mod response_trace;
pub mod route;

// Mirror the glob re-exports the legacy module exposed.
pub use response::*;
pub use route::*;
