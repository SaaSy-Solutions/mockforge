//! Error types for the bench module

use thiserror::Error;

pub type Result<T> = std::result::Result<T, BenchError>;

#[derive(Error, Debug)]
pub enum BenchError {
    #[error("Failed to parse OpenAPI spec: {0}")]
    SpecParseError(String),

    #[error("Invalid target URL: {0}")]
    InvalidTargetUrl(String),

    #[error("Operation not found in spec: {0}")]
    OperationNotFound(String),

    #[error("k6 is not installed or not found in PATH")]
    K6NotFound,

    #[error("k6 execution failed: {0}")]
    K6ExecutionFailed(String),

    #[error("Failed to generate k6 script: {0}")]
    ScriptGenerationFailed(String),

    #[error("Failed to parse k6 results: {0}")]
    ResultsParseError(String),

    #[error("Invalid scenario: {0}")]
    InvalidScenario(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Other error: {0}")]
    Other(String),
}
