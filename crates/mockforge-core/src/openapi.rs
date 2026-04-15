//! OpenAPI specification handling and utilities.
//!
//! The entire OpenAPI domain model (`OpenApiSpec`, `OpenApiSchema`, response
//! selection/generation, route construction, trace instrumentation, multi-spec,
//! validation, spec format detection, and the Swagger 2.0 → OpenAPI 3.0
//! converter) lives in the dedicated [`mockforge_openapi`] crate. This module
//! re-exports the public surface so existing consumers importing from
//! `mockforge_core::openapi::*` continue to resolve unchanged.

pub use mockforge_openapi::{
    multi_spec, response, response_selection, response_trace, route, schema, spec, swagger_convert,
    validation, OpenApiOperation, OpenApiSchema, OpenApiSecurityRequirement, OpenApiSpec,
    ResponseSelectionMode, ResponseSelector,
};

// Mirror the glob re-exports the legacy module exposed so downstream
// `use mockforge_core::openapi::*` keeps pulling in ResponseGenerator,
// AiGenerator, OpenApiRoute, etc.
pub use response::*;
pub use route::*;
