//! Error types for MockForge Core

/// Result type alias for MockForge operations
pub type Result<T> = std::result::Result<T, Error>;

/// Core error types for MockForge operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Validation error (schema/format validation failed)
    #[error("Validation error: {message}")]
    Validation {
        /// Validation error message
        message: String,
    },

    /// Routing error (route not found or invalid)
    #[error("Routing error: {message}")]
    Routing {
        /// Routing error message
        message: String,
    },

    /// Proxy error (proxy request failed)
    #[error("Proxy error: {message}")]
    Proxy {
        /// Proxy error message
        message: String,
    },

    /// Latency simulation error (latency injection failed)
    #[error("Latency simulation error: {message}")]
    Latency {
        /// Latency error message
        message: String,
    },

    /// Configuration error (invalid config or missing required fields)
    #[error("Configuration error: {message}")]
    Config {
        /// Configuration error message
        message: String,
    },

    /// Protocol not found (requested protocol is not registered)
    #[error("Protocol not found: {message}")]
    ProtocolNotFound {
        /// Protocol not found error message
        message: String,
    },

    /// Protocol disabled (protocol exists but is disabled)
    #[error("Protocol disabled: {message}")]
    ProtocolDisabled {
        /// Protocol disabled error message
        message: String,
    },

    /// Protocol handler in use (handler already registered)
    #[error("Protocol handler in use: {message}")]
    ProtocolHandlerInUse {
        /// Protocol handler conflict error message
        message: String,
    },

    /// Protocol validation error (protocol-specific validation failed)
    #[error("Protocol validation error: {message}")]
    ProtocolValidationError {
        /// Protocol name that failed validation
        protocol: String,
        /// Validation error message
        message: String
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

    /// Encryption/decryption operation error
    #[error("Encryption error: {0}")]
    Encryption(#[from] crate::encryption::EncryptionError),

    /// JavaScript evaluation error (template engine, etc.)
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

    /// Create a protocol not found error
    pub fn protocol_not_found<S: Into<String>>(message: S) -> Self {
        Self::ProtocolNotFound {
            message: message.into(),
        }
    }

    /// Create a protocol disabled error
    pub fn protocol_disabled<S: Into<String>>(message: S) -> Self {
        Self::ProtocolDisabled {
            message: message.into(),
        }
    }

    /// Create a protocol handler in use error
    pub fn protocol_handler_in_use<S: Into<String>>(message: S) -> Self {
        Self::ProtocolHandlerInUse {
            message: message.into(),
        }
    }

    /// Create a protocol validation error
    pub fn protocol_validation_error<S: Into<String>>(protocol: S, message: S) -> Self {
        Self::ProtocolValidationError {
            protocol: protocol.into(),
            message: message.into(),
        }
    }

    /// Create a generic error
    pub fn generic<S: Into<String>>(message: S) -> Self {
        Self::Generic(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error() {
        let err = Error::validation("test validation");
        assert!(err.to_string().contains("Validation error"));
        assert!(err.to_string().contains("test validation"));
    }

    #[test]
    fn test_routing_error() {
        let err = Error::routing("test routing");
        assert!(err.to_string().contains("Routing error"));
        assert!(err.to_string().contains("test routing"));
    }

    #[test]
    fn test_proxy_error() {
        let err = Error::proxy("test proxy");
        assert!(err.to_string().contains("Proxy error"));
        assert!(err.to_string().contains("test proxy"));
    }

    #[test]
    fn test_latency_error() {
        let err = Error::latency("test latency");
        assert!(err.to_string().contains("Latency simulation error"));
        assert!(err.to_string().contains("test latency"));
    }

    #[test]
    fn test_config_error() {
        let err = Error::config("test config");
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("test config"));
    }

    #[test]
    fn test_generic_error() {
        let err = Error::generic("test generic");
        assert!(err.to_string().contains("Generic error"));
        assert!(err.to_string().contains("test generic"));
    }

    #[test]
    fn test_from_string() {
        let err: Error = "test message".to_string().into();
        assert!(matches!(err, Error::Generic(_)));
        assert!(err.to_string().contains("test message"));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json");
        assert!(json_err.is_err());
        let err: Error = json_err.unwrap_err().into();
        assert!(matches!(err, Error::Json(_)));
    }

    #[test]
    fn test_url_parse_error_conversion() {
        let url_err = url::Url::parse("not a url");
        assert!(url_err.is_err());
        let err: Error = url_err.unwrap_err().into();
        assert!(matches!(err, Error::UrlParse(_)));
    }

    #[test]
    #[allow(clippy::invalid_regex)]
    fn test_regex_error_conversion() {
        let regex_err = regex::Regex::new("[invalid(");
        assert!(regex_err.is_err());
        let err: Error = regex_err.unwrap_err().into();
        assert!(matches!(err, Error::Regex(_)));
    }

    #[test]
    fn test_error_display() {
        let errors = vec![
            (Error::validation("msg"), "Validation error: msg"),
            (Error::routing("msg"), "Routing error: msg"),
            (Error::proxy("msg"), "Proxy error: msg"),
            (Error::latency("msg"), "Latency simulation error: msg"),
            (Error::config("msg"), "Configuration error: msg"),
            (Error::generic("msg"), "Generic error: msg"),
        ];

        for (err, expected) in errors {
            assert_eq!(err.to_string(), expected);
        }
    }
}
