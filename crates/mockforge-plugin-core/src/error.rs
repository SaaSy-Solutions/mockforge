//! Plugin error types and result handling

/// Plugin system result type
pub type Result<T> = std::result::Result<T, PluginError>;

/// Comprehensive error types for the plugin system
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// Plugin loading or validation failed
    #[error("Plugin loading error: {message}")]
    LoadError {
        /// Error message describing what went wrong
        message: String,
    },

    /// Plugin execution failed
    #[error("Plugin execution error: {message}")]
    ExecutionError {
        /// Error message describing the execution failure
        message: String,
    },

    /// Plugin violated security constraints
    #[error("Security violation: {violation}")]
    SecurityViolation {
        /// Description of the security violation
        violation: String,
    },

    /// Plugin exceeded resource limits
    #[error("Resource limit exceeded: {resource} limit={limit}, used={used}")]
    ResourceLimitExceeded {
        /// The resource that exceeded its limit
        resource: String,
        /// The configured limit
        limit: String,
        /// The amount used
        used: String,
    },

    /// Plugin configuration is invalid
    #[error("Invalid plugin configuration: {field} - {message}")]
    InvalidConfiguration {
        /// The configuration field that is invalid
        field: String,
        /// Error message describing the configuration issue
        message: String,
    },

    /// Plugin is incompatible with current system
    #[error("Plugin compatibility error: {reason}")]
    CompatibilityError {
        /// Reason for the compatibility error
        reason: String,
    },

    /// Plugin communication failed
    #[error("Plugin communication error: {message}")]
    CommunicationError {
        /// Error message describing the communication failure
        message: String,
    },

    /// Plugin timed out
    #[error("Plugin execution timeout: {timeout_ms}ms exceeded")]
    TimeoutError {
        /// Timeout duration in milliseconds
        timeout_ms: u64,
    },

    /// WebAssembly runtime error
    #[error("WebAssembly runtime error: {message}")]
    WasmError {
        /// Error message from the WASM runtime
        message: String,
    },

    /// Plugin manifest is invalid
    #[error("Invalid plugin manifest: {message}")]
    InvalidManifest {
        /// Error message describing the manifest issue
        message: String,
    },

    /// Plugin dependency not found or incompatible
    #[error("Plugin dependency error: {dependency} - {message}")]
    DependencyError {
        /// The dependency that caused the error
        dependency: String,
        /// Error message describing the dependency issue
        message: String,
    },

    /// Generic plugin system error
    #[error("Plugin system error: {message}")]
    SystemError {
        /// Error message describing the system error
        message: String,
    },
}

impl PluginError {
    /// Create a load error
    pub fn load<S: Into<String>>(message: S) -> Self {
        Self::LoadError {
            message: message.into(),
        }
    }

    /// Create an execution error
    pub fn execution<S: Into<String>>(message: S) -> Self {
        Self::ExecutionError {
            message: message.into(),
        }
    }

    /// Create a security violation error
    pub fn security<S: Into<String>>(violation: S) -> Self {
        Self::SecurityViolation {
            violation: violation.into(),
        }
    }

    /// Create a resource limit error
    pub fn resource_limit<S: Into<String>>(resource: S, limit: S, used: S) -> Self {
        Self::ResourceLimitExceeded {
            resource: resource.into(),
            limit: limit.into(),
            used: used.into(),
        }
    }

    /// Create a configuration error
    pub fn config<S: Into<String>>(field: S, message: S) -> Self {
        Self::InvalidConfiguration {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a compatibility error
    pub fn compatibility<S: Into<String>>(reason: S) -> Self {
        Self::CompatibilityError {
            reason: reason.into(),
        }
    }

    /// Create a communication error
    pub fn communication<S: Into<String>>(message: S) -> Self {
        Self::CommunicationError {
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(timeout_ms: u64) -> Self {
        Self::TimeoutError { timeout_ms }
    }

    /// Create a WASM error
    pub fn wasm<S: Into<String>>(message: S) -> Self {
        Self::WasmError {
            message: message.into(),
        }
    }

    /// Create a manifest error
    pub fn manifest<S: Into<String>>(message: S) -> Self {
        Self::InvalidManifest {
            message: message.into(),
        }
    }

    /// Create a dependency error
    pub fn dependency<S: Into<String>>(dependency: S, message: S) -> Self {
        Self::DependencyError {
            dependency: dependency.into(),
            message: message.into(),
        }
    }

    /// Create a system error
    pub fn system<S: Into<String>>(message: S) -> Self {
        Self::SystemError {
            message: message.into(),
        }
    }

    /// Check if this is a security-related error
    pub fn is_security_error(&self) -> bool {
        matches!(self, PluginError::SecurityViolation { .. })
    }

    /// Check if this is a resource-related error
    pub fn is_resource_error(&self) -> bool {
        matches!(self, PluginError::ResourceLimitExceeded { .. })
    }

    /// Check if this is a timeout error
    pub fn is_timeout_error(&self) -> bool {
        matches!(self, PluginError::TimeoutError { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Factory method tests
    #[test]
    fn test_load_error() {
        let err = PluginError::load("failed to load plugin");
        assert!(matches!(err, PluginError::LoadError { .. }));
        assert!(err.to_string().contains("failed to load plugin"));
    }

    #[test]
    fn test_execution_error() {
        let err = PluginError::execution("execution failed");
        assert!(matches!(err, PluginError::ExecutionError { .. }));
        assert!(err.to_string().contains("execution failed"));
    }

    #[test]
    fn test_security_error() {
        let err = PluginError::security("unauthorized access");
        assert!(matches!(err, PluginError::SecurityViolation { .. }));
        assert!(err.to_string().contains("unauthorized access"));
    }

    #[test]
    fn test_resource_limit_error() {
        let err = PluginError::resource_limit("memory", "100MB", "150MB");
        assert!(matches!(err, PluginError::ResourceLimitExceeded { .. }));
        let msg = err.to_string();
        assert!(msg.contains("memory"));
        assert!(msg.contains("100MB"));
        assert!(msg.contains("150MB"));
    }

    #[test]
    fn test_config_error() {
        let err = PluginError::config("timeout", "must be positive");
        assert!(matches!(err, PluginError::InvalidConfiguration { .. }));
        let msg = err.to_string();
        assert!(msg.contains("timeout"));
        assert!(msg.contains("must be positive"));
    }

    #[test]
    fn test_compatibility_error() {
        let err = PluginError::compatibility("API version mismatch");
        assert!(matches!(err, PluginError::CompatibilityError { .. }));
        assert!(err.to_string().contains("API version mismatch"));
    }

    #[test]
    fn test_communication_error() {
        let err = PluginError::communication("connection refused");
        assert!(matches!(err, PluginError::CommunicationError { .. }));
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn test_timeout_error() {
        let err = PluginError::timeout(5000);
        assert!(matches!(err, PluginError::TimeoutError { timeout_ms: 5000 }));
        assert!(err.to_string().contains("5000"));
    }

    #[test]
    fn test_wasm_error() {
        let err = PluginError::wasm("invalid wasm module");
        assert!(matches!(err, PluginError::WasmError { .. }));
        assert!(err.to_string().contains("invalid wasm module"));
    }

    #[test]
    fn test_manifest_error() {
        let err = PluginError::manifest("missing required field");
        assert!(matches!(err, PluginError::InvalidManifest { .. }));
        assert!(err.to_string().contains("missing required field"));
    }

    #[test]
    fn test_dependency_error() {
        let err = PluginError::dependency("plugin-foo", "version not found");
        assert!(matches!(err, PluginError::DependencyError { .. }));
        let msg = err.to_string();
        assert!(msg.contains("plugin-foo"));
        assert!(msg.contains("version not found"));
    }

    #[test]
    fn test_system_error() {
        let err = PluginError::system("internal failure");
        assert!(matches!(err, PluginError::SystemError { .. }));
        assert!(err.to_string().contains("internal failure"));
    }

    // Check methods tests
    #[test]
    fn test_is_security_error() {
        let err = PluginError::security("test");
        assert!(err.is_security_error());

        let err = PluginError::load("test");
        assert!(!err.is_security_error());
    }

    #[test]
    fn test_is_resource_error() {
        let err = PluginError::resource_limit("memory", "100MB", "150MB");
        assert!(err.is_resource_error());

        let err = PluginError::execution("test");
        assert!(!err.is_resource_error());
    }

    #[test]
    fn test_is_timeout_error() {
        let err = PluginError::timeout(1000);
        assert!(err.is_timeout_error());

        let err = PluginError::wasm("test");
        assert!(!err.is_timeout_error());
    }

    // Error Display tests
    #[test]
    fn test_load_error_display() {
        let err = PluginError::LoadError {
            message: "test message".to_string(),
        };
        assert_eq!(err.to_string(), "Plugin loading error: test message");
    }

    #[test]
    fn test_execution_error_display() {
        let err = PluginError::ExecutionError {
            message: "runtime failure".to_string(),
        };
        assert_eq!(err.to_string(), "Plugin execution error: runtime failure");
    }

    #[test]
    fn test_security_violation_display() {
        let err = PluginError::SecurityViolation {
            violation: "access denied".to_string(),
        };
        assert_eq!(err.to_string(), "Security violation: access denied");
    }

    #[test]
    fn test_resource_limit_display() {
        let err = PluginError::ResourceLimitExceeded {
            resource: "CPU".to_string(),
            limit: "50%".to_string(),
            used: "75%".to_string(),
        };
        assert_eq!(err.to_string(), "Resource limit exceeded: CPU limit=50%, used=75%");
    }

    #[test]
    fn test_invalid_config_display() {
        let err = PluginError::InvalidConfiguration {
            field: "port".to_string(),
            message: "invalid value".to_string(),
        };
        assert_eq!(err.to_string(), "Invalid plugin configuration: port - invalid value");
    }

    #[test]
    fn test_timeout_display() {
        let err = PluginError::TimeoutError { timeout_ms: 3000 };
        assert_eq!(err.to_string(), "Plugin execution timeout: 3000ms exceeded");
    }

    #[test]
    fn test_dependency_error_display() {
        let err = PluginError::DependencyError {
            dependency: "core-lib".to_string(),
            message: "not found".to_string(),
        };
        assert_eq!(err.to_string(), "Plugin dependency error: core-lib - not found");
    }

    // Debug trait tests
    #[test]
    fn test_error_debug() {
        let err = PluginError::load("test");
        let debug = format!("{:?}", err);
        assert!(debug.contains("LoadError"));
    }

    // String conversion tests
    #[test]
    fn test_load_error_from_string() {
        let err = PluginError::load(String::from("dynamic message"));
        assert!(err.to_string().contains("dynamic message"));
    }

    #[test]
    fn test_load_error_from_str() {
        let err = PluginError::load("static message");
        assert!(err.to_string().contains("static message"));
    }
}
