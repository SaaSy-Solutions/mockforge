//! Error types for behavioral cloning

/// Result type alias for behavioral cloning operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for behavioral cloning operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Generic error
    #[error("{message}")]
    Generic {
        /// Error message
        message: String,
    },

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl Error {
    /// Create a generic error
    pub fn generic<S: Into<String>>(message: S) -> Self {
        Self::Generic {
            message: message.into(),
        }
    }
}
