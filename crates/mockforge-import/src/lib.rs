//! Import and code generation utilities for MockForge
//!
//! This crate provides functionality to import API definitions from external formats
//! (OpenAPI, Postman, cURL, HAR, Insomnia, AsyncAPI) and generate mock server code.

#[cfg(feature = "import")]
pub mod import;

#[cfg(feature = "codegen")]
pub mod codegen;
