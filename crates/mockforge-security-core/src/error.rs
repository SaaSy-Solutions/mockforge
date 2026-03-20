//! Error types for MockForge Security Core

/// Result type alias for security operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for security operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Generic error with message string
    #[error("Security error: {0}")]
    Generic(String),

    /// I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Encryption error
    #[error("Encryption error: {0}")]
    Encryption(#[from] crate::encryption::EncryptionError),
}

impl Error {
    /// Create a generic error
    pub fn generic<S: Into<String>>(message: S) -> Self {
        Self::Generic(message.into())
    }
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self::Generic(message)
    }
}
