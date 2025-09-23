//! Import functionality for MockForge
//!
//! This module provides functionality to import API definitions from external formats
//! and convert them to MockForge routes, as well as generate commands from OpenAPI specs.

pub mod curl_import;
pub mod har_import;
pub mod insomnia_import;
pub mod import_utils;
pub mod openapi_command_generator;
pub mod openapi_import;
pub mod postman_environment;
pub mod postman_import;

// Re-export the main functions and types
pub use curl_import::{import_curl_commands, CurlImportResult, MockForgeRoute as CurlMockForgeRoute, MockForgeResponse as CurlMockForgeResponse};
pub use har_import::{import_har_archive, HarImportResult, MockForgeRoute as HarMockForgeRoute, MockForgeResponse as HarMockForgeResponse};
pub use import_utils::{detect_format, FormatDetection, ImportFormat};
pub use insomnia_import::{import_insomnia_export, InsomniaImportResult, MockForgeRoute as InsomniaMockForgeRoute, MockForgeResponse as InsomniaMockForgeResponse};
pub use openapi_command_generator::{generate_commands_from_openapi, CommandGenerationOptions, CommandGenerationResult, GeneratedCommand, CommandFormat};
pub use openapi_import::{import_openapi_spec, OpenApiImportResult, MockForgeRoute as OpenApiMockForgeRoute, MockForgeResponse as OpenApiMockForgeResponse};
pub use postman_environment::{import_postman_environment, EnvironmentImportResult, EnvironmentVariable, VariableSource};
pub use postman_import::{import_postman_collection, ImportResult, MockForgeRoute, MockForgeResponse};
