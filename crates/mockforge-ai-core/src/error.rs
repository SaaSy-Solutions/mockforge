//! Error types for AI core operations

/// Result type alias for AI core operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for AI core operations
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

    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl Error {
    /// Create an internal error.
    ///
    /// Constructs the [`Error::Generic`] variant. Named `internal` to match the
    /// 2026-04-01 `Error::generic` -> `Error::internal` rename that swept the
    /// call sites across the workspace; this constructor was missed because the
    /// crate was not yet a workspace member (see #796).
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Generic {
            message: message.into(),
        }
    }
}
