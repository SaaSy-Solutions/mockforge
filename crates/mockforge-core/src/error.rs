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

    /// Route not found error with method and path context
    #[error("Route not found: {method} {path}")]
    RouteNotFound {
        /// HTTP method
        method: String,
        /// Request path
        path: String,
    },

    /// Schema validation failed with structured context
    #[error("Schema validation failed at '{path}': expected {expected}, got {actual}")]
    SchemaValidationFailed {
        /// JSON path where validation failed
        path: String,
        /// Expected type or value
        expected: String,
        /// Actual type or value encountered
        actual: String,
    },

    /// Configuration error with source
    #[error("Configuration error: {message}")]
    ConfigWithSource {
        /// Configuration error message
        message: String,
        /// Underlying cause
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Entity not found
    #[error("{entity} not found: {id}")]
    NotFound {
        /// Entity type that was not found
        entity: String,
        /// Identifier that was looked up
        id: String,
    },

    /// Feature is disabled
    #[error("Feature disabled: {feature}")]
    FeatureDisabled {
        /// Feature name that is disabled
        feature: String,
    },

    /// Global component already initialized
    #[error("Already initialized: {component}")]
    AlreadyInitialized {
        /// Component name that was already initialized
        component: String,
    },

    /// Invalid state for the requested operation
    #[error("Invalid state: {message}")]
    InvalidState {
        /// Description of the invalid state
        message: String,
    },

    /// SIEM transport error
    #[error("SIEM transport error: {message}")]
    SiemTransport {
        /// SIEM transport error message
        message: String,
    },

    /// I/O error with additional context
    #[error("I/O error ({context}): {message}")]
    IoWithContext {
        /// Context describing what operation failed
        context: String,
        /// The underlying error message
        message: String,
    },

    /// Internal error wrapping an underlying cause
    #[error("Internal error: {message}")]
    Internal {
        /// Internal error message
        message: String,
    },

    /// Generic error with message string
    #[error("Generic error: {0}")]
    Generic(String),

    /// Encryption/decryption operation error
    #[error("Encryption error: {0}")]
    Encryption(#[from] crate::encryption::EncryptionError),

    /// JavaScript evaluation error (template engine, etc.)
    #[cfg(feature = "scripting")]
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

    /// Create a route-not-found error with method and path context
    pub fn route_not_found<S: Into<String>>(method: S, path: S) -> Self {
        Self::RouteNotFound {
            method: method.into(),
            path: path.into(),
        }
    }

    /// Create a schema validation error with path, expected, and actual context
    pub fn schema_validation_failed<S: Into<String>>(path: S, expected: S, actual: S) -> Self {
        Self::SchemaValidationFailed {
            path: path.into(),
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a configuration error with an underlying source error
    pub fn config_with_source<S: Into<String>>(
        message: S,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::ConfigWithSource {
            message: message.into(),
            source: Box::new(source),
        }
    }

    /// Create a not-found error
    pub fn not_found<S1: Into<String>, S2: Into<String>>(entity: S1, id: S2) -> Self {
        Self::NotFound {
            entity: entity.into(),
            id: id.into(),
        }
    }

    /// Create a feature-disabled error
    pub fn feature_disabled<S: Into<String>>(feature: S) -> Self {
        Self::FeatureDisabled {
            feature: feature.into(),
        }
    }

    /// Create an already-initialized error
    pub fn already_initialized<S: Into<String>>(component: S) -> Self {
        Self::AlreadyInitialized {
            component: component.into(),
        }
    }

    /// Create an invalid-state error
    pub fn invalid_state<S: Into<String>>(message: S) -> Self {
        Self::InvalidState {
            message: message.into(),
        }
    }

    /// Create a SIEM transport error
    pub fn siem_transport<S: Into<String>>(message: S) -> Self {
        Self::SiemTransport {
            message: message.into(),
        }
    }

    /// Create an I/O error with context
    pub fn io_with_context<S1: Into<String>, S2: Into<String>>(context: S1, message: S2) -> Self {
        Self::IoWithContext {
            context: context.into(),
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create a generic error
    #[deprecated(note = "Use a specific error variant instead of Generic")]
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
    fn test_internal_error() {
        let err = Error::internal("test internal");
        assert!(err.to_string().contains("Internal error"));
        assert!(err.to_string().contains("test internal"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_generic_error() {
        let err = Error::generic("test generic");
        assert!(err.to_string().contains("Generic error"));
        assert!(err.to_string().contains("test generic"));
    }

    #[test]
    fn test_not_found_error() {
        let err = Error::not_found("User", "user-123");
        assert_eq!(err.to_string(), "User not found: user-123");
    }

    #[test]
    fn test_feature_disabled_error() {
        let err = Error::feature_disabled("access review");
        assert_eq!(err.to_string(), "Feature disabled: access review");
    }

    #[test]
    fn test_already_initialized_error() {
        let err = Error::already_initialized("SIEM emitter");
        assert_eq!(err.to_string(), "Already initialized: SIEM emitter");
    }

    #[test]
    fn test_invalid_state_error() {
        let err = Error::invalid_state("Request is not pending approval");
        assert_eq!(err.to_string(), "Invalid state: Request is not pending approval");
    }

    #[test]
    fn test_siem_transport_error() {
        let err = Error::siem_transport("Connection refused");
        assert_eq!(err.to_string(), "SIEM transport error: Connection refused");
    }

    #[test]
    fn test_io_with_context_error() {
        let err = Error::io_with_context("reading risk register", "file not found");
        assert_eq!(err.to_string(), "I/O error (reading risk register): file not found");
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
    #[allow(deprecated)]
    fn test_error_display() {
        let errors = vec![
            (Error::validation("msg"), "Validation error: msg"),
            (Error::routing("msg"), "Routing error: msg"),
            (Error::proxy("msg"), "Proxy error: msg"),
            (Error::latency("msg"), "Latency simulation error: msg"),
            (Error::config("msg"), "Configuration error: msg"),
            (Error::internal("msg"), "Internal error: msg"),
        ];

        for (err, expected) in errors {
            assert_eq!(err.to_string(), expected);
        }
    }
}
