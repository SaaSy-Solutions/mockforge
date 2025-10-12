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
