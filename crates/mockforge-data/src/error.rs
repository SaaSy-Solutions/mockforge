//! Error types for MockForge Data

/// Result type alias for MockForge Data operations
pub type Result<T> = std::result::Result<T, Error>;

/// Core error types for MockForge Data operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Validation error (schema/format validation failed)
    #[error("Validation error: {message}")]
    Validation {
        /// Validation error message
        message: String,
    },

    /// Configuration error (invalid config or missing required fields)
    #[error("Configuration error: {message}")]
    Config {
        /// Configuration error message
        message: String,
    },

    /// I/O error (file read/write operations)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML serialization/deserialization error
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// HTTP client request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// URL parsing error
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// Regular expression compilation error
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// Generic error with message string
    #[error("Generic error: {0}")]
    Generic(String),
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self::Generic(message)
    }
}

impl Error {
    /// Create a validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a generic error
    pub fn generic<S: Into<String>>(message: S) -> Self {
        Self::Generic(message.into())
    }
}

