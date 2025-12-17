//! Error types for MockForge test utilities

use std::io;
use thiserror::Error;

/// Result type for MockForge test operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when using MockForge test utilities
#[derive(Error, Debug)]
pub enum Error {
    /// Server failed to start
    #[error("Failed to start MockForge server: {0}")]
    ServerStartFailed(String),

    /// Server health check failed
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    /// Server health check timed out
    #[error("Health check timed out after {0}s")]
    HealthCheckTimeout(u64),

    /// HTTP request error
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    /// Process error
    #[error("Process error: {0}")]
    ProcessError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Scenario switching error
    #[error("Failed to switch scenario: {0}")]
    ScenarioError(String),

    /// Port allocation error
    #[error("Port {0} is already in use")]
    PortInUse(u16),

    /// MockForge binary not found
    #[error("MockForge binary not found. Please ensure it's installed or in PATH")]
    BinaryNotFound,

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    /// Workspace error
    #[error("Workspace error: {0}")]
    WorkspaceError(String),

    /// Invalid response from server
    #[error("Invalid response from server: {0}")]
    InvalidResponse(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_server_start_failed() {
        let error = Error::ServerStartFailed("connection refused".to_string());
        let display = error.to_string();
        assert!(display.contains("Failed to start MockForge server"));
        assert!(display.contains("connection refused"));
    }

    #[test]
    fn test_error_health_check_failed() {
        let error = Error::HealthCheckFailed("HTTP 500".to_string());
        let display = error.to_string();
        assert!(display.contains("Health check failed"));
        assert!(display.contains("HTTP 500"));
    }

    #[test]
    fn test_error_health_check_timeout() {
        let error = Error::HealthCheckTimeout(30);
        let display = error.to_string();
        assert!(display.contains("Health check timed out"));
        assert!(display.contains("30s"));
    }

    #[test]
    fn test_error_process_error() {
        let error = Error::ProcessError("spawn failed".to_string());
        let display = error.to_string();
        assert!(display.contains("Process error"));
        assert!(display.contains("spawn failed"));
    }

    #[test]
    fn test_error_config_error() {
        let error = Error::ConfigError("invalid port".to_string());
        let display = error.to_string();
        assert!(display.contains("Configuration error"));
        assert!(display.contains("invalid port"));
    }

    #[test]
    fn test_error_scenario_error() {
        let error = Error::ScenarioError("scenario not found".to_string());
        let display = error.to_string();
        assert!(display.contains("Failed to switch scenario"));
        assert!(display.contains("scenario not found"));
    }

    #[test]
    fn test_error_port_in_use() {
        let error = Error::PortInUse(3000);
        let display = error.to_string();
        assert!(display.contains("Port 3000 is already in use"));
    }

    #[test]
    fn test_error_binary_not_found() {
        let error = Error::BinaryNotFound;
        let display = error.to_string();
        assert!(display.contains("MockForge binary not found"));
        assert!(display.contains("PATH"));
    }

    #[test]
    fn test_error_workspace_error() {
        let error = Error::WorkspaceError("file not found".to_string());
        let display = error.to_string();
        assert!(display.contains("Workspace error"));
        assert!(display.contains("file not found"));
    }

    #[test]
    fn test_error_invalid_response() {
        let error = Error::InvalidResponse("unexpected format".to_string());
        let display = error.to_string();
        assert!(display.contains("Invalid response from server"));
        assert!(display.contains("unexpected format"));
    }

    #[test]
    fn test_error_from_io() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let error: Error = io_error.into();
        assert!(matches!(error, Error::IoError(_)));
        assert!(error.to_string().contains("IO error"));
    }

    #[test]
    fn test_error_from_json() {
        let json_str = "invalid json";
        let json_error: serde_json::Error =
            serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let error: Error = json_error.into();
        assert!(matches!(error, Error::JsonError(_)));
        assert!(error.to_string().contains("JSON parsing error"));
    }

    #[test]
    fn test_error_from_yaml() {
        let yaml_str = "invalid: yaml: syntax:";
        let yaml_error: serde_yaml::Error =
            serde_yaml::from_str::<serde_yaml::Value>(yaml_str).unwrap_err();
        let error: Error = yaml_error.into();
        assert!(matches!(error, Error::YamlError(_)));
        assert!(error.to_string().contains("YAML parsing error"));
    }

    #[test]
    fn test_error_debug() {
        let error = Error::BinaryNotFound;
        let debug = format!("{:?}", error);
        assert!(debug.contains("BinaryNotFound"));
    }

    #[test]
    fn test_error_debug_with_message() {
        let error = Error::ServerStartFailed("test message".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("ServerStartFailed"));
        assert!(debug.contains("test message"));
    }
}
