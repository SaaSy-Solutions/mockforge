//! Error types for the analytics module

use thiserror::Error;

/// Result type for analytics operations
pub type Result<T> = std::result::Result<T, AnalyticsError>;

/// Error types for analytics operations
#[derive(Debug, Error)]
pub enum AnalyticsError {
    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP error (when querying Prometheus)
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Query error
    #[error("Query error: {0}")]
    Query(String),

    /// Export error
    #[error("Export error: {0}")]
    Export(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl From<String> for AnalyticsError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

impl From<&str> for AnalyticsError {
    fn from(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_error_from_string() {
        let error: AnalyticsError = "test error".into();
        assert!(matches!(error, AnalyticsError::Other(_)));
        assert_eq!(error.to_string(), "test error");
    }

    #[test]
    fn test_analytics_error_from_owned_string() {
        let error: AnalyticsError = String::from("owned error").into();
        assert!(matches!(error, AnalyticsError::Other(_)));
        assert_eq!(error.to_string(), "owned error");
    }

    #[test]
    fn test_analytics_error_migration() {
        let error = AnalyticsError::Migration("migration failed".to_string());
        assert!(error.to_string().contains("Migration error"));
        assert!(error.to_string().contains("migration failed"));
    }

    #[test]
    fn test_analytics_error_invalid_config() {
        let error = AnalyticsError::InvalidConfig("bad config".to_string());
        assert!(error.to_string().contains("Invalid configuration"));
        assert!(error.to_string().contains("bad config"));
    }

    #[test]
    fn test_analytics_error_query() {
        let error = AnalyticsError::Query("query failed".to_string());
        assert!(error.to_string().contains("Query error"));
        assert!(error.to_string().contains("query failed"));
    }

    #[test]
    fn test_analytics_error_export() {
        let error = AnalyticsError::Export("export failed".to_string());
        assert!(error.to_string().contains("Export error"));
        assert!(error.to_string().contains("export failed"));
    }

    #[test]
    fn test_analytics_error_invalid_input() {
        let error = AnalyticsError::InvalidInput("bad input".to_string());
        assert!(error.to_string().contains("Invalid input"));
        assert!(error.to_string().contains("bad input"));
    }

    #[test]
    fn test_analytics_error_other() {
        let error = AnalyticsError::Other("other error".to_string());
        assert_eq!(error.to_string(), "other error");
    }

    #[test]
    fn test_analytics_error_debug() {
        let error = AnalyticsError::Query("test".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("Query"));
    }

    #[test]
    fn test_analytics_error_from_serde_json() {
        let json_error = serde_json::from_str::<i32>("not a number").unwrap_err();
        let error: AnalyticsError = json_error.into();
        assert!(matches!(error, AnalyticsError::Serialization(_)));
        assert!(error.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(AnalyticsError::Query("failed".to_string()));
        assert!(result.is_err());
    }
}
