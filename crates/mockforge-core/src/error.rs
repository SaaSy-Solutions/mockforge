//! Error types for MockForge Core

/// Result type alias for MockForge operations
pub type Result<T> = std::result::Result<T, Error>;

/// Core error types for MockForge
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Routing error: {message}")]
    Routing { message: String },

    #[error("Proxy error: {message}")]
    Proxy { message: String },

    #[error("Latency simulation error: {message}")]
    Latency { message: String },

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Generic error: {0}")]
    Generic(String),

    #[error("Encryption error: {0}")]
    Encryption(#[from] crate::encryption::EncryptionError),

    #[error("JavaScript error: {0}")]
    JavaScript(#[from] rquickjs::Error),
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

    /// Create a routing error
    pub fn routing<S: Into<String>>(message: S) -> Self {
        Self::Routing {
            message: message.into(),
        }
    }

    /// Create a proxy error
    pub fn proxy<S: Into<String>>(message: S) -> Self {
        Self::Proxy {
            message: message.into(),
        }
    }

    /// Create a latency error
    pub fn latency<S: Into<String>>(message: S) -> Self {
        Self::Latency {
            message: message.into(),
        }
    }

    /// Create a config error
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
