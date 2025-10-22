//! Error types for the MockForge SDK

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

    /// MockForge core error
    #[error("MockForge core error: {0}")]
    Core(#[from] mockforge_core::Error),

    /// Server startup timeout
    #[error("Server failed to start within {timeout_secs} seconds.\nCheck logs for details or increase timeout.")]
    StartupTimeout {
        /// Number of seconds waited before timeout
        timeout_secs: u64
    },

    /// Server shutdown timeout
    #[error("Server failed to stop within {timeout_secs} seconds.\nSome connections may still be active.")]
    ShutdownTimeout {
        /// Number of seconds waited before timeout
        timeout_secs: u64
    },

    /// Admin API error
    #[error("Admin API error ({operation}): {message}\nEndpoint: {endpoint}")]
    AdminApiError {
        /// The operation that failed (e.g., "create_mock", "list_mocks")
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
        Error::AdminApiError {
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
    pub fn stub_not_found(method: impl Into<String>, path: impl Into<String>, available: Vec<String>) -> Self {
        Error::StubNotFound {
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
    pub fn to_log_string(&self) -> String {
        format!("{}", self).replace('\n', " | ")
    }
}
