//! Error types for the scenarios marketplace

use thiserror::Error;

/// Result type for scenario operations
pub type Result<T> = std::result::Result<T, ScenarioError>;

/// Errors that can occur in scenario operations
#[derive(Error, Debug)]
pub enum ScenarioError {
    /// Scenario not found
    #[error("Scenario not found: {0}")]
    NotFound(String),

    /// Invalid scenario manifest
    #[error("Invalid scenario manifest: {0}")]
    InvalidManifest(String),

    /// Invalid version specification
    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    /// Scenario already exists
    #[error("Scenario already exists: {0}")]
    AlreadyExists(String),

    /// Authentication required
    #[error("Authentication required")]
    AuthRequired,

    /// Permission denied
    #[error("Permission denied")]
    PermissionDenied,

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// File system error
    #[error("File system error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Invalid source specification
    #[error("Invalid source: {0}")]
    InvalidSource(String),

    /// Checksum verification failed
    #[error("Checksum verification failed: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// Expected checksum value
        expected: String,
        /// Actual checksum value received
        actual: String,
    },

    /// Dependency resolution failed
    #[error("Dependency resolution failed: {0}")]
    DependencyResolution(String),

    /// Generic error
    #[error("{0}")]
    Generic(String),
}

impl From<anyhow::Error> for ScenarioError {
    fn from(err: anyhow::Error) -> Self {
        ScenarioError::Generic(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = ScenarioError::NotFound("test-scenario".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Scenario not found"));
        assert!(msg.contains("test-scenario"));
    }

    #[test]
    fn test_invalid_manifest_error() {
        let err = ScenarioError::InvalidManifest("missing required field".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid scenario manifest"));
        assert!(msg.contains("missing required field"));
    }

    #[test]
    fn test_invalid_version_error() {
        let err = ScenarioError::InvalidVersion("1.2".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid version"));
        assert!(msg.contains("1.2"));
    }

    #[test]
    fn test_already_exists_error() {
        let err = ScenarioError::AlreadyExists("existing-scenario".to_string());
        let msg = err.to_string();
        assert!(msg.contains("already exists"));
        assert!(msg.contains("existing-scenario"));
    }

    #[test]
    fn test_auth_required_error() {
        let err = ScenarioError::AuthRequired;
        assert!(err.to_string().contains("Authentication required"));
    }

    #[test]
    fn test_permission_denied_error() {
        let err = ScenarioError::PermissionDenied;
        assert!(err.to_string().contains("Permission denied"));
    }

    #[test]
    fn test_storage_error() {
        let err = ScenarioError::Storage("disk full".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Storage error"));
        assert!(msg.contains("disk full"));
    }

    #[test]
    fn test_network_error() {
        let err = ScenarioError::Network("connection refused".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Network error"));
        assert!(msg.contains("connection refused"));
    }

    #[test]
    fn test_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: ScenarioError = io_err.into();
        assert!(matches!(err, ScenarioError::Io(_)));
        assert!(err.to_string().contains("File system error"));
    }

    #[test]
    fn test_serde_error() {
        let json_err: serde_json::Error = serde_json::from_str::<String>("invalid").unwrap_err();
        let err: ScenarioError = json_err.into();
        assert!(matches!(err, ScenarioError::Serde(_)));
        assert!(err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_yaml_error() {
        let yaml_content = "invalid: yaml: content:";
        let yaml_err: serde_yaml::Error = serde_yaml::from_str::<String>(yaml_content).unwrap_err();
        let err: ScenarioError = yaml_err.into();
        assert!(matches!(err, ScenarioError::Yaml(_)));
        assert!(err.to_string().contains("YAML parsing error"));
    }

    #[test]
    fn test_invalid_source_error() {
        let err = ScenarioError::InvalidSource("unknown://source".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid source"));
        assert!(msg.contains("unknown://source"));
    }

    #[test]
    fn test_checksum_mismatch_error() {
        let err = ScenarioError::ChecksumMismatch {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Checksum verification failed"));
        assert!(msg.contains("abc123"));
        assert!(msg.contains("def456"));
    }

    #[test]
    fn test_dependency_resolution_error() {
        let err = ScenarioError::DependencyResolution("circular dependency".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Dependency resolution failed"));
        assert!(msg.contains("circular dependency"));
    }

    #[test]
    fn test_generic_error() {
        let err = ScenarioError::Generic("custom error message".to_string());
        assert_eq!(err.to_string(), "custom error message");
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("something went wrong");
        let err: ScenarioError = anyhow_err.into();
        assert!(matches!(err, ScenarioError::Generic(_)));
        assert!(err.to_string().contains("something went wrong"));
    }

    #[test]
    fn test_error_debug() {
        let err = ScenarioError::NotFound("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
    }
}
