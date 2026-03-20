//! Error types for the pipelines crate

use thiserror::Error;

/// Errors that can occur during pipeline execution
#[derive(Debug, Error)]
pub enum PipelineError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error
    #[error("serialization error: {0}")]
    SerializationJson(#[from] serde_json::Error),

    /// YAML serialization error
    #[error("YAML error: {0}")]
    SerializationYaml(#[from] serde_yaml::Error),

    /// Database error
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Template rendering error
    #[error("template error: {0}")]
    Template(#[from] handlebars::RenderError),

    /// Missing configuration field
    #[error("missing config: {0}")]
    MissingConfig(String),

    /// Invalid configuration value
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    /// Step execution timeout
    #[error("step timed out after {0} seconds")]
    Timeout(u64),

    /// Step execution failure
    #[error("step failed: {0}")]
    StepFailed(String),

    /// Unknown step type
    #[error("unknown step type: {0}")]
    UnknownStepType(String),

    /// Pipeline not found or other lookup failures
    #[error("{0}")]
    NotFound(String),

    /// Internal error
    #[error("{0}")]
    Internal(String),
}

/// Result type alias for pipeline operations
pub type Result<T> = std::result::Result<T, PipelineError>;
