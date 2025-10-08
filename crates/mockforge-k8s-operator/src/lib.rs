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
