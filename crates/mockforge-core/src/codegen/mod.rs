//! Code generation from OpenAPI specifications
//!
//! This module provides functionality for generating executable mock server code
//! from OpenAPI specifications, supporting multiple output languages and frameworks.

pub mod backend_generator;
pub mod rust_generator;
pub mod typescript_generator;

#[cfg(test)]
mod tests;

use crate::openapi::spec::OpenApiSpec;
use crate::Result;

/// Configuration for code generation
#[derive(Debug, Clone, Default)]
pub struct CodegenConfig {
    /// Generate mock data strategy
    pub mock_data_strategy: MockDataStrategy,
    /// Server port (for generated code)
    pub port: Option<u16>,
    /// Enable CORS
    pub enable_cors: bool,
    /// Response delay simulation (milliseconds)
    pub default_delay_ms: Option<u64>,
}

/// Strategy for generating mock data in generated code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MockDataStrategy {
    /// Generate random data from schemas (fuzzing-style)
    Random,
    /// Use examples from OpenAPI spec (deterministic)
    Examples,
    /// Use schema defaults when available
    Defaults,
    /// Prefer examples, fallback to random when examples are missing
    #[default]
    ExamplesOrRandom,
}

/// Generate mock server code from OpenAPI spec
///
/// # Arguments
/// * `spec` - The OpenAPI specification to generate code from
/// * `language` - The target language (rs, ts, js)
/// * `config` - Code generation configuration
///
/// # Returns
/// Generated source code as a string
pub fn generate_mock_server_code(
    spec: &OpenApiSpec,
    language: &str,
    config: &CodegenConfig,
) -> Result<String> {
    match language {
        "rs" | "rust" => rust_generator::generate(spec, config),
        "ts" | "typescript" => typescript_generator::generate(spec, config),
        "js" | "javascript" => typescript_generator::generate(spec, config),
        _ => Err(crate::Error::generic(format!(
            "Unsupported language: {}. Supported: rust, typescript, javascript",
            language
        ))),
    }
}
