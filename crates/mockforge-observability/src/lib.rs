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
pub use prometheus::{get_global_registry, MetricsRegistry};
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
    use std::collections::HashSet;

    #[test]
    fn test_protocol_display() {
        assert_eq!(Protocol::Http.to_string(), "http");
        assert_eq!(Protocol::Grpc.to_string(), "grpc");
        assert_eq!(Protocol::WebSocket.to_string(), "websocket");
        assert_eq!(Protocol::GraphQL.to_string(), "graphql");
    }

    #[test]
    fn test_protocol_as_str() {
        assert_eq!(Protocol::Http.as_str(), "http");
        assert_eq!(Protocol::Grpc.as_str(), "grpc");
        assert_eq!(Protocol::WebSocket.as_str(), "websocket");
        assert_eq!(Protocol::GraphQL.as_str(), "graphql");
    }

    #[test]
    fn test_protocol_debug() {
        assert_eq!(format!("{:?}", Protocol::Http), "Http");
        assert_eq!(format!("{:?}", Protocol::Grpc), "Grpc");
        assert_eq!(format!("{:?}", Protocol::WebSocket), "WebSocket");
        assert_eq!(format!("{:?}", Protocol::GraphQL), "GraphQL");
    }

    #[test]
    fn test_protocol_clone() {
        let proto = Protocol::Http;
        let cloned = proto.clone();
        assert_eq!(proto, cloned);
    }

    #[test]
    fn test_protocol_copy() {
        let proto = Protocol::Grpc;
        let copied = proto;
        assert_eq!(Protocol::Grpc, copied);
        assert_eq!(proto, Protocol::Grpc); // proto still accessible
    }

    #[test]
    fn test_protocol_eq() {
        assert_eq!(Protocol::Http, Protocol::Http);
        assert_eq!(Protocol::Grpc, Protocol::Grpc);
        assert_ne!(Protocol::Http, Protocol::Grpc);
        assert_ne!(Protocol::WebSocket, Protocol::GraphQL);
    }

    #[test]
    fn test_protocol_hash() {
        let mut set = HashSet::new();
        set.insert(Protocol::Http);
        set.insert(Protocol::Grpc);
        set.insert(Protocol::WebSocket);
        set.insert(Protocol::GraphQL);

        assert_eq!(set.len(), 4);
        assert!(set.contains(&Protocol::Http));
        assert!(set.contains(&Protocol::Grpc));
        assert!(set.contains(&Protocol::WebSocket));
        assert!(set.contains(&Protocol::GraphQL));

        // Duplicate insertion shouldn't increase size
        set.insert(Protocol::Http);
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_protocol_all_variants() {
        let protocols = [
            Protocol::Http,
            Protocol::Grpc,
            Protocol::WebSocket,
            Protocol::GraphQL,
        ];

        for proto in protocols {
            let str_repr = proto.as_str();
            assert!(!str_repr.is_empty());
            assert_eq!(proto.to_string(), str_repr);
        }
    }
}
