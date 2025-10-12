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

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
    }
}
