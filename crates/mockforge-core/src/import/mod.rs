//! Import functionality for MockForge
//!
//! This module provides functionality to import API definitions from external formats
//! and convert them to MockForge routes, as well as generate commands from OpenAPI specs.

pub mod asyncapi_import;
pub mod curl_import;
pub mod har_import;
pub mod import_utils;
pub mod insomnia_import;
pub mod openapi_command_generator;
pub mod openapi_import;
pub mod postman_environment;
pub mod postman_import;
pub mod schema_data_generator;

// Re-export the main functions and types
pub use asyncapi_import::{
    import_asyncapi_spec, AsyncApiImportResult, AsyncApiSpecInfo, ChannelProtocol,
    ChannelOperation, MockForgeChannel, OperationType,
};
pub use curl_import::{
    import_curl_commands, CurlImportResult, MockForgeResponse as CurlMockForgeResponse,
    MockForgeRoute as CurlMockForgeRoute,
};
pub use har_import::{
    import_har_archive, HarImportResult, MockForgeResponse as HarMockForgeResponse,
    MockForgeRoute as HarMockForgeRoute,
};
pub use import_utils::{detect_format, FormatDetection, ImportFormat};
pub use insomnia_import::{
    import_insomnia_export, InsomniaImportResult, MockForgeResponse as InsomniaMockForgeResponse,
    MockForgeRoute as InsomniaMockForgeRoute,
};
pub use openapi_command_generator::{
    generate_commands_from_openapi, CommandFormat, CommandGenerationOptions,
    CommandGenerationResult, GeneratedCommand,
};
pub use openapi_import::{
    import_openapi_spec, MockForgeResponse as OpenApiMockForgeResponse,
    MockForgeRoute as OpenApiMockForgeRoute, OpenApiImportResult,
};
pub use postman_environment::{
    import_postman_environment, EnvironmentImportResult, EnvironmentVariable, VariableSource,
};
pub use postman_import::{
    import_postman_collection, ImportResult, MockForgeResponse, MockForgeRoute,
};
pub use schema_data_generator::{generate_from_schema, generate_intelligent_response};
