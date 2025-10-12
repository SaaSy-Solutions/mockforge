//! Prometheus metrics integration for MockForge
//!
//! Provides a comprehensive metrics registry for tracking:
//! - Request counts by protocol (HTTP, gRPC, WebSocket, GraphQL)
//! - Request duration histograms
//! - Error rates and counts
//! - Plugin execution metrics
//! - System resource metrics

mod exporter;
mod metrics;

pub use exporter::{metrics_handler, prometheus_router};
pub use metrics::{get_global_registry, MetricsRegistry};

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
