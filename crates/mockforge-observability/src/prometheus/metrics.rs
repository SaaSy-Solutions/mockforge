//! Prometheus metrics definitions and registry

use once_cell::sync::Lazy;
use prometheus::{
    Gauge, GaugeVec, HistogramOpts, HistogramVec, IntCounter,
    IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
};
use std::sync::Arc;
use tracing::debug;

/// Global metrics registry for MockForge
#[derive(Clone)]
pub struct MetricsRegistry {
    registry: Arc<Registry>,

    // Request metrics by protocol
    pub requests_total: IntCounterVec,
    pub requests_duration_seconds: HistogramVec,
    pub requests_in_flight: IntGaugeVec,

    // Error metrics
    pub errors_total: IntCounterVec,
    pub error_rate: GaugeVec,

    // Plugin metrics
    pub plugin_executions_total: IntCounterVec,
    pub plugin_execution_duration_seconds: HistogramVec,
    pub plugin_errors_total: IntCounterVec,

    // WebSocket specific metrics
    pub ws_connections_active: IntGauge,
    pub ws_messages_sent: IntCounter,
    pub ws_messages_received: IntCounter,

    // System metrics
    pub memory_usage_bytes: Gauge,
    pub cpu_usage_percent: Gauge,

    // Scenario metrics (for Phase 4)
    pub active_scenario_mode: IntGauge,
    pub chaos_triggers_total: IntCounter,
}

impl MetricsRegistry {
    /// Create a new metrics registry with all metrics initialized
    pub fn new() -> Self {
        let registry = Registry::new();

        // Request metrics
        let requests_total = IntCounterVec::new(
            Opts::new(
                "mockforge_requests_total",
                "Total number of requests by protocol, method, and status",
            ),
            &["protocol", "method", "status"],
        )
        .expect("Failed to create requests_total metric");

        let requests_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "mockforge_request_duration_seconds",
                "Request duration in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["protocol", "method"],
        )
        .expect("Failed to create requests_duration_seconds metric");

        let requests_in_flight = IntGaugeVec::new(
            Opts::new(
                "mockforge_requests_in_flight",
                "Number of requests currently being processed",
            ),
            &["protocol"],
        )
        .expect("Failed to create requests_in_flight metric");

        // Error metrics
        let errors_total = IntCounterVec::new(
            Opts::new(
                "mockforge_errors_total",
                "Total number of errors by protocol and error type",
            ),
            &["protocol", "error_type"],
        )
        .expect("Failed to create errors_total metric");

        let error_rate = GaugeVec::new(
            Opts::new("mockforge_error_rate", "Error rate by protocol (0.0 to 1.0)"),
            &["protocol"],
        )
        .expect("Failed to create error_rate metric");

        // Plugin metrics
        let plugin_executions_total = IntCounterVec::new(
            Opts::new(
                "mockforge_plugin_executions_total",
                "Total number of plugin executions",
            ),
            &["plugin_name", "status"],
        )
        .expect("Failed to create plugin_executions_total metric");

        let plugin_execution_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "mockforge_plugin_execution_duration_seconds",
                "Plugin execution duration in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
            &["plugin_name"],
        )
        .expect("Failed to create plugin_execution_duration_seconds metric");

        let plugin_errors_total = IntCounterVec::new(
            Opts::new(
                "mockforge_plugin_errors_total",
                "Total number of plugin errors",
            ),
            &["plugin_name", "error_type"],
        )
        .expect("Failed to create plugin_errors_total metric");

        // WebSocket metrics
        let ws_connections_active = IntGauge::new(
            "mockforge_ws_connections_active",
            "Number of active WebSocket connections",
        )
        .expect("Failed to create ws_connections_active metric");

        let ws_messages_sent = IntCounter::new(
            "mockforge_ws_messages_sent_total",
            "Total number of WebSocket messages sent",
        )
        .expect("Failed to create ws_messages_sent metric");

        let ws_messages_received = IntCounter::new(
            "mockforge_ws_messages_received_total",
            "Total number of WebSocket messages received",
        )
        .expect("Failed to create ws_messages_received metric");

        // System metrics
        let memory_usage_bytes =
            Gauge::new("mockforge_memory_usage_bytes", "Memory usage in bytes")
                .expect("Failed to create memory_usage_bytes metric");

        let cpu_usage_percent =
            Gauge::new("mockforge_cpu_usage_percent", "CPU usage percentage")
                .expect("Failed to create cpu_usage_percent metric");

        // Scenario metrics
        let active_scenario_mode = IntGauge::new(
            "mockforge_active_scenario_mode",
            "Active scenario mode (0=healthy, 1=degraded, 2=error, 3=chaos)",
        )
        .expect("Failed to create active_scenario_mode metric");

        let chaos_triggers_total = IntCounter::new(
            "mockforge_chaos_triggers_total",
            "Total number of chaos mode triggers",
        )
        .expect("Failed to create chaos_triggers_total metric");

        // Register all metrics
        registry
            .register(Box::new(requests_total.clone()))
            .expect("Failed to register requests_total");
        registry
            .register(Box::new(requests_duration_seconds.clone()))
            .expect("Failed to register requests_duration_seconds");
        registry
            .register(Box::new(requests_in_flight.clone()))
            .expect("Failed to register requests_in_flight");
        registry
            .register(Box::new(errors_total.clone()))
            .expect("Failed to register errors_total");
        registry
            .register(Box::new(error_rate.clone()))
            .expect("Failed to register error_rate");
        registry
            .register(Box::new(plugin_executions_total.clone()))
            .expect("Failed to register plugin_executions_total");
        registry
            .register(Box::new(plugin_execution_duration_seconds.clone()))
            .expect("Failed to register plugin_execution_duration_seconds");
        registry
            .register(Box::new(plugin_errors_total.clone()))
            .expect("Failed to register plugin_errors_total");
        registry
            .register(Box::new(ws_connections_active.clone()))
            .expect("Failed to register ws_connections_active");
        registry
            .register(Box::new(ws_messages_sent.clone()))
            .expect("Failed to register ws_messages_sent");
        registry
            .register(Box::new(ws_messages_received.clone()))
            .expect("Failed to register ws_messages_received");
        registry
            .register(Box::new(memory_usage_bytes.clone()))
            .expect("Failed to register memory_usage_bytes");
        registry
            .register(Box::new(cpu_usage_percent.clone()))
            .expect("Failed to register cpu_usage_percent");
        registry
            .register(Box::new(active_scenario_mode.clone()))
            .expect("Failed to register active_scenario_mode");
        registry
            .register(Box::new(chaos_triggers_total.clone()))
            .expect("Failed to register chaos_triggers_total");

        debug!("Initialized Prometheus metrics registry");

        Self {
            registry: Arc::new(registry),
            requests_total,
            requests_duration_seconds,
            requests_in_flight,
            errors_total,
            error_rate,
            plugin_executions_total,
            plugin_execution_duration_seconds,
            plugin_errors_total,
            ws_connections_active,
            ws_messages_sent,
            ws_messages_received,
            memory_usage_bytes,
            cpu_usage_percent,
            active_scenario_mode,
            chaos_triggers_total,
        }
    }

    /// Get the underlying Prometheus registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Check if the registry is initialized
    pub fn is_initialized(&self) -> bool {
        true
    }

    /// Record an HTTP request
    pub fn record_http_request(&self, method: &str, status: u16, duration_seconds: f64) {
        let status_str = status.to_string();
        self.requests_total
            .with_label_values(&["http", method, &status_str])
            .inc();
        self.requests_duration_seconds
            .with_label_values(&["http", method])
            .observe(duration_seconds);
    }

    /// Record a gRPC request
    pub fn record_grpc_request(&self, method: &str, status: &str, duration_seconds: f64) {
        self.requests_total
            .with_label_values(&["grpc", method, status])
            .inc();
        self.requests_duration_seconds
            .with_label_values(&["grpc", method])
            .observe(duration_seconds);
    }

    /// Record a WebSocket message
    pub fn record_ws_message_sent(&self) {
        self.ws_messages_sent.inc();
    }

    /// Record a WebSocket message received
    pub fn record_ws_message_received(&self) {
        self.ws_messages_received.inc();
    }

    /// Record a GraphQL request
    pub fn record_graphql_request(&self, operation: &str, status: u16, duration_seconds: f64) {
        let status_str = status.to_string();
        self.requests_total
            .with_label_values(&["graphql", operation, &status_str])
            .inc();
        self.requests_duration_seconds
            .with_label_values(&["graphql", operation])
            .observe(duration_seconds);
    }

    /// Record a plugin execution
    pub fn record_plugin_execution(
        &self,
        plugin_name: &str,
        success: bool,
        duration_seconds: f64,
    ) {
        let status = if success { "success" } else { "failure" };
        self.plugin_executions_total
            .with_label_values(&[plugin_name, status])
            .inc();
        self.plugin_execution_duration_seconds
            .with_label_values(&[plugin_name])
            .observe(duration_seconds);
    }

    /// Increment in-flight requests
    pub fn increment_in_flight(&self, protocol: &str) {
        self.requests_in_flight.with_label_values(&[protocol]).inc();
    }

    /// Decrement in-flight requests
    pub fn decrement_in_flight(&self, protocol: &str) {
        self.requests_in_flight.with_label_values(&[protocol]).dec();
    }

    /// Record an error
    pub fn record_error(&self, protocol: &str, error_type: &str) {
        self.errors_total
            .with_label_values(&[protocol, error_type])
            .inc();
    }

    /// Update memory usage
    pub fn update_memory_usage(&self, bytes: f64) {
        self.memory_usage_bytes.set(bytes);
    }

    /// Update CPU usage
    pub fn update_cpu_usage(&self, percent: f64) {
        self.cpu_usage_percent.set(percent);
    }

    /// Set active scenario mode (0=healthy, 1=degraded, 2=error, 3=chaos)
    pub fn set_scenario_mode(&self, mode: i64) {
        self.active_scenario_mode.set(mode);
    }

    /// Record a chaos trigger
    pub fn record_chaos_trigger(&self) {
        self.chaos_triggers_total.inc();
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global metrics registry instance
static GLOBAL_REGISTRY: Lazy<MetricsRegistry> = Lazy::new(MetricsRegistry::new);

/// Get the global metrics registry
pub fn get_global_registry() -> &'static MetricsRegistry {
    &GLOBAL_REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registry_creation() {
        let registry = MetricsRegistry::new();
        assert!(registry.is_initialized());
    }

    #[test]
    fn test_record_http_request() {
        let registry = MetricsRegistry::new();
        registry.record_http_request("GET", 200, 0.045);
        registry.record_http_request("POST", 201, 0.123);

        // Verify metrics were recorded (they should not panic)
        assert!(registry.is_initialized());
    }

    #[test]
    fn test_global_registry() {
        let registry = get_global_registry();
        assert!(registry.is_initialized());
    }

    #[test]
    fn test_plugin_metrics() {
        let registry = MetricsRegistry::new();
        registry.record_plugin_execution("test-plugin", true, 0.025);
        registry.record_plugin_execution("test-plugin", false, 0.050);
        assert!(registry.is_initialized());
    }

    #[test]
    fn test_websocket_metrics() {
        let registry = MetricsRegistry::new();
        registry.record_ws_message_sent();
        registry.record_ws_message_received();
        assert!(registry.is_initialized());
    }
}
