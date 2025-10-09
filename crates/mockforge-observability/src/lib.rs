//! MockForge Observability
//!
//! Provides comprehensive observability features including:
//! - Structured logging with JSON support
//! - Prometheus metrics export
//! - OpenTelemetry distributed tracing
//! - Request/response recording (flight recorder)
//! - Scenario control and chaos engineering
//! - System metrics collection (CPU, memory, threads)
//!
//! # Example
//!
//! ```rust
//! use mockforge_observability::prometheus::MetricsRegistry;
//!
//! let registry = MetricsRegistry::new();
//! registry.record_http_request("GET", 200, 0.045);
//! ```

pub mod logging;
pub mod prometheus;
pub mod system_metrics;
pub mod tracing_integration;

// Re-export commonly used items
pub use logging::{init_logging, init_logging_with_otel, LoggingConfig};
pub use prometheus::{MetricsRegistry, get_global_registry};
pub use system_metrics::{start_system_metrics_collector, SystemMetricsConfig};
pub use tracing_integration::{init_with_otel, shutdown_otel, OtelTracingConfig};

/// Protocol types for metrics tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    Http,
    Grpc,
    WebSocket,
    GraphQL,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Http => "http",
            Protocol::Grpc => "grpc",
            Protocol::WebSocket => "websocket",
            Protocol::GraphQL => "graphql",
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_display() {
        assert_eq!(Protocol::Http.to_string(), "http");
        assert_eq!(Protocol::Grpc.to_string(), "grpc");
        assert_eq!(Protocol::WebSocket.to_string(), "websocket");
        assert_eq!(Protocol::GraphQL.to_string(), "graphql");
    }
}
