//! Error types for the FTP crate

use thiserror::Error;

/// Errors that can occur in the FTP server
#[derive(Debug, Error)]
pub enum FtpError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error (JSON)
    #[error("serialization error: {0}")]
    SerializationJson(#[from] serde_json::Error),

    /// Serialization error (YAML)
    #[error("YAML error: {0}")]
    SerializationYaml(#[from] serde_yaml::Error),

    /// Template rendering error
    #[error("template error: {0}")]
    Template(#[from] handlebars::RenderError),

    /// FTP server error
    #[error("FTP server error: {0}")]
    Server(#[from] libunftp::ServerError),

    /// Virtual filesystem error
    #[error("VFS error: {0}")]
    Vfs(String),

    /// Validation error
    #[error("validation error: {0}")]
    Validation(String),

    /// Internal error
    #[error("{0}")]
    Internal(String),
}

/// Result type alias for FTP operations
pub type Result<T> = std::result::Result<T, FtpError>;
