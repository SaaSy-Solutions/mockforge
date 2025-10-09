//! Prometheus metrics integration for MockForge
//!
//! Provides a comprehensive metrics registry for tracking:
//! - Request counts by protocol (HTTP, gRPC, WebSocket, GraphQL)
//! - Request duration histograms
//! - Error rates and counts
//! - Plugin execution metrics
//! - System resource metrics

mod metrics;
mod exporter;

pub use metrics::{MetricsRegistry, get_global_registry};
pub use exporter::{prometheus_router, metrics_handler};

// Re-export prometheus types for users who need to access metrics directly
pub use prometheus;
pub use prometheus::proto::MetricFamily;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registry_creation() {
        let registry = MetricsRegistry::new();
        assert!(registry.is_initialized());
    }
}
