//! MockForge Kubernetes Operator
//!
//! Kubernetes operator for managing chaos orchestrations as CRDs.

pub mod controller;
pub mod crd;
pub mod metrics;
pub mod reconciler;
pub mod webhook;

pub use controller::Controller;
pub use crd::{ChaosOrchestration, ChaosOrchestrationSpec, ChaosOrchestrationStatus};
pub use metrics::OperatorMetrics;
pub use reconciler::Reconciler;
pub use webhook::WebhookHandler;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OperatorError {
    #[error("Kubernetes error: {0}")]
    Kube(#[from] kube::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Orchestration error: {0}")]
    Orchestration(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Metrics error: {0}")]
    Metrics(#[from] prometheus::Error),
}

pub type Result<T> = std::result::Result<T, OperatorError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_error_orchestration() {
        let error = OperatorError::Orchestration("test failure".to_string());
        assert_eq!(error.to_string(), "Orchestration error: test failure");
    }

    #[test]
    fn test_operator_error_not_found() {
        let error = OperatorError::NotFound("resource".to_string());
        assert_eq!(error.to_string(), "Resource not found: resource");
    }

    #[test]
    fn test_operator_error_validation() {
        let error = OperatorError::Validation("invalid spec".to_string());
        assert_eq!(error.to_string(), "Validation error: invalid spec");
    }

    #[test]
    fn test_operator_error_serialization() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let error = OperatorError::Serialization(json_error);
        assert!(error.to_string().contains("Serialization error:"));
    }

    #[test]
    fn test_operator_error_debug() {
        let error = OperatorError::NotFound("test".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("NotFound"));
    }

    // Test Result type alias
    #[test]
    fn test_result_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_err() {
        let result: Result<i32> = Err(OperatorError::NotFound("test".to_string()));
        assert!(result.is_err());
    }

    // Test From trait implementations
    #[test]
    fn test_from_serde_json_error() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let op_error: OperatorError = json_error.into();
        assert!(matches!(op_error, OperatorError::Serialization(_)));
    }

    #[test]
    fn test_from_prometheus_error() {
        // Create a prometheus error by trying to create an invalid metric
        let registry = prometheus::Registry::new();
        // Register same metric twice to force an error
        let counter1 = prometheus::IntCounter::new("test_counter", "Test counter").unwrap();
        let counter2 = prometheus::IntCounter::new("test_counter", "Test counter").unwrap();
        registry.register(Box::new(counter1)).unwrap();
        let prometheus_error = registry.register(Box::new(counter2)).unwrap_err();

        let op_error: OperatorError = prometheus_error.into();
        assert!(matches!(op_error, OperatorError::Metrics(_)));
    }
}
