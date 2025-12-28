//! Error types for the `MockForge` SDK

use thiserror::Error;

/// SDK Result type
pub type Result<T> = std::result::Result<T, Error>;

/// SDK Error types
#[derive(Error, Debug)]
pub enum Error {
    /// Server already started
    #[error("Mock server is already running on port {0}. Call stop() before starting again.")]
    ServerAlreadyStarted(u16),

    /// Server not started
    #[error("Mock server has not been started yet. Call start() first.")]
    ServerNotStarted,

    /// Port already in use
    #[error("Port {0} is already in use. Try using a different port or enable auto_port().")]
    PortInUse(u16),

    /// Port discovery failed
    #[error("Port discovery failed: {0}\nTip: Try expanding the port range using port_range(start, end).")]
    PortDiscoveryFailed(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}\nCheck your configuration file or builder settings.")]
    InvalidConfig(String),

    /// Invalid stub
    #[error("Invalid stub: {0}\nEnsure method, path, and response body are properly set.")]
    InvalidStub(String),

    /// Stub not found
    #[error("Stub not found for {method} {path}. Available stubs: {available}")]
    StubNotFound {
        /// HTTP method that was requested
        method: String,
        /// Path that was requested
        path: String,
        /// Comma-separated list of available stubs
        available: String,
    },

    /// HTTP error
    #[error("HTTP error: {0}\nThis may indicate a network or protocol issue.")]
    Http(#[from] axum::http::Error),

    /// IO error
    #[error("IO error: {0}\nCheck file permissions and network connectivity.")]
    Io(#[from] std::io::Error),

    /// JSON serialization error
    #[error("JSON serialization error: {0}\nEnsure your request/response body is valid JSON.")]
    Json(#[from] serde_json::Error),

    /// `MockForge` core error
    #[error("MockForge core error: {0}")]
    Core(#[from] mockforge_core::Error),

    /// Server startup timeout
    #[error("Server failed to start within {timeout_secs} seconds.\nCheck logs for details or increase timeout.")]
    StartupTimeout {
        /// Number of seconds waited before timeout
        timeout_secs: u64,
    },

    /// Server shutdown timeout
    #[error("Server failed to stop within {timeout_secs} seconds.\nSome connections may still be active.\nTip: Ensure all client connections are closed before stopping the server.")]
    ShutdownTimeout {
        /// Number of seconds waited before timeout
        timeout_secs: u64,
    },

    /// Admin API error
    #[error("Admin API error ({operation}): {message}\nEndpoint: {endpoint}")]
    AdminApiError {
        /// The operation that failed (e.g., "`create_mock`", "`list_mocks`")
        operation: String,
        /// The error message from the server or client
        message: String,
        /// The API endpoint that was called
        endpoint: String,
    },

    /// General error
    #[error("{0}")]
    General(String),
}

impl Error {
    /// Create an admin API error with context
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mockforge_sdk::Error;
    ///
    /// let err = Error::admin_api_error(
    ///     "create_mock",
    ///     "Invalid JSON",
    ///     "/api/mocks"
    /// );
    /// ```
    pub fn admin_api_error(
        operation: impl Into<String>,
        message: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self::AdminApiError {
            operation: operation.into(),
            message: message.into(),
            endpoint: endpoint.into(),
        }
    }

    /// Create a stub not found error with available stubs
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mockforge_sdk::Error;
    ///
    /// let err = Error::stub_not_found(
    ///     "GET",
    ///     "/api/missing",
    ///     vec!["GET /api/users".to_string()]
    /// );
    /// ```
    pub fn stub_not_found(
        method: impl Into<String>,
        path: impl Into<String>,
        available: Vec<String>,
    ) -> Self {
        Self::StubNotFound {
            method: method.into(),
            path: path.into(),
            available: if available.is_empty() {
                "none".to_string()
            } else {
                available.join(", ")
            },
        }
    }

    /// Format error for logging (single line, no ANSI colors)
    ///
    /// Useful for structured logging where multi-line messages aren't desired.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mockforge_sdk::Error;
    ///
    /// let err = Error::ServerNotStarted;
    /// let log_msg = err.to_log_string();
    /// // Use in logging: log::error!("{}", log_msg);
    /// ```
    #[must_use]
    pub fn to_log_string(&self) -> String {
        format!("{self}").replace('\n', " | ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_already_started_error() {
        let err = Error::ServerAlreadyStarted(3000);
        let msg = format!("{err}");
        assert!(msg.contains("3000"));
        assert!(msg.contains("already running"));
        assert!(msg.contains("stop()"));
    }

    #[test]
    fn test_server_not_started_error() {
        let err = Error::ServerNotStarted;
        let msg = format!("{err}");
        assert!(msg.contains("not been started"));
        assert!(msg.contains("start()"));
    }

    #[test]
    fn test_port_in_use_error() {
        let err = Error::PortInUse(8080);
        let msg = format!("{err}");
        assert!(msg.contains("8080"));
        assert!(msg.contains("already in use"));
        assert!(msg.contains("auto_port()"));
    }

    #[test]
    fn test_port_discovery_failed_error() {
        let err = Error::PortDiscoveryFailed("No ports available in range".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Port discovery failed"));
        assert!(msg.contains("No ports available"));
        assert!(msg.contains("port_range"));
    }

    #[test]
    fn test_invalid_config_error() {
        let err = Error::InvalidConfig("Invalid host address".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Invalid configuration"));
        assert!(msg.contains("Invalid host address"));
        assert!(msg.contains("configuration file"));
    }

    #[test]
    fn test_invalid_stub_error() {
        let err = Error::InvalidStub("Missing response body".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Invalid stub"));
        assert!(msg.contains("Missing response body"));
        assert!(msg.contains("properly set"));
    }

    #[test]
    fn test_stub_not_found_error_with_available() {
        let err = Error::stub_not_found(
            "GET",
            "/api/missing",
            vec!["GET /api/users".to_string(), "POST /api/orders".to_string()],
        );
        let msg = format!("{err}");
        assert!(msg.contains("GET"));
        assert!(msg.contains("/api/missing"));
        assert!(msg.contains("GET /api/users"));
        assert!(msg.contains("POST /api/orders"));
    }

    #[test]
    fn test_stub_not_found_error_no_available() {
        let err = Error::stub_not_found("DELETE", "/api/users/1", vec![]);
        let msg = format!("{err}");
        assert!(msg.contains("DELETE"));
        assert!(msg.contains("/api/users/1"));
        assert!(msg.contains("none"));
    }

    #[test]
    fn test_startup_timeout_error() {
        let err = Error::StartupTimeout { timeout_secs: 30 };
        let msg = format!("{err}");
        assert!(msg.contains("30 seconds"));
        assert!(msg.contains("failed to start"));
    }

    #[test]
    fn test_shutdown_timeout_error() {
        let err = Error::ShutdownTimeout { timeout_secs: 10 };
        let msg = format!("{err}");
        assert!(msg.contains("10 seconds"));
        assert!(msg.contains("failed to stop"));
        assert!(msg.contains("connections"));
    }

    #[test]
    fn test_admin_api_error() {
        let err = Error::admin_api_error("create_mock", "Invalid JSON payload", "/api/mocks");
        let msg = format!("{err}");
        assert!(msg.contains("create_mock"));
        assert!(msg.contains("Invalid JSON payload"));
        assert!(msg.contains("/api/mocks"));
    }

    #[test]
    fn test_general_error() {
        let err = Error::General("Something went wrong".to_string());
        let msg = format!("{err}");
        assert_eq!(msg, "Something went wrong");
    }

    #[test]
    fn test_to_log_string_single_line() {
        let err = Error::General("Simple error".to_string());
        let log_str = err.to_log_string();
        assert_eq!(log_str, "Simple error");
        assert!(!log_str.contains('\n'));
    }

    #[test]
    fn test_to_log_string_multiline() {
        let err = Error::InvalidConfig("Line 1\nLine 2\nLine 3".to_string());
        let log_str = err.to_log_string();
        assert!(!log_str.contains('\n'));
        assert!(log_str.contains(" | "));
        assert!(log_str.contains("Line 1"));
        assert!(log_str.contains("Line 2"));
        assert!(log_str.contains("Line 3"));
    }

    #[test]
    fn test_http_error_conversion() {
        // Create an HTTP error using an invalid header value (control characters are invalid)
        let http_err: axum::http::Error =
            axum::http::header::HeaderValue::from_bytes(&[0x00]).unwrap_err().into();
        let err = Error::from(http_err);
        let msg = format!("{err}");
        assert!(msg.contains("HTTP error"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = Error::from(io_err);
        let msg = format!("{err}");
        assert!(msg.contains("IO error"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_str = "{invalid json";
        let json_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let err = Error::from(json_err);
        let msg = format!("{err}");
        assert!(msg.contains("JSON serialization error"));
    }

    #[test]
    fn test_error_debug_format() {
        let err = Error::ServerNotStarted;
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("ServerNotStarted"));
    }

    #[test]
    fn test_stub_not_found_with_single_available() {
        let err = Error::stub_not_found("POST", "/api/create", vec!["GET /api/list".to_string()]);
        let msg = format!("{err}");
        assert!(msg.contains("GET /api/list"));
        assert!(!msg.contains(", ")); // No comma for single item
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(Error::ServerNotStarted);
        assert!(result.is_err());
    }
}
