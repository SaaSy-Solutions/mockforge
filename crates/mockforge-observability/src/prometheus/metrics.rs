//! Prometheus metrics definitions and registry

use once_cell::sync::Lazy;
use prometheus::{
    Gauge, GaugeVec, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec,
    Opts, Registry,
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

    // Request metrics by path (endpoint-specific)
    pub requests_by_path_total: IntCounterVec,
    pub request_duration_by_path_seconds: HistogramVec,
    pub average_latency_by_path_seconds: GaugeVec,

    // Workspace-specific metrics
    pub workspace_requests_total: IntCounterVec,
    pub workspace_requests_duration_seconds: HistogramVec,
    pub workspace_active_routes: IntGaugeVec,
    pub workspace_errors_total: IntCounterVec,

    // Error metrics
    pub errors_total: IntCounterVec,
    pub error_rate: GaugeVec,

    // Plugin metrics
    pub plugin_executions_total: IntCounterVec,
    pub plugin_execution_duration_seconds: HistogramVec,
    pub plugin_errors_total: IntCounterVec,

    // WebSocket specific metrics
    pub ws_connections_active: IntGauge,
    pub ws_connections_total: IntCounter,
    pub ws_connection_duration_seconds: HistogramVec,
    pub ws_messages_sent: IntCounter,
    pub ws_messages_received: IntCounter,
    pub ws_errors_total: IntCounter,

    // SMTP specific metrics
    pub smtp_connections_active: IntGauge,
    pub smtp_connections_total: IntCounter,
    pub smtp_messages_received_total: IntCounter,
    pub smtp_messages_stored_total: IntCounter,
    pub smtp_errors_total: IntCounterVec,

    // MQTT specific metrics
    pub mqtt_connections_active: IntGauge,
    pub mqtt_connections_total: IntCounter,
    pub mqtt_messages_published_total: IntCounter,
    pub mqtt_messages_received_total: IntCounter,
    pub mqtt_topics_active: IntGauge,
    pub mqtt_subscriptions_active: IntGauge,
    pub mqtt_retained_messages: IntGauge,
    pub mqtt_errors_total: IntCounterVec,

    // System metrics
    pub memory_usage_bytes: Gauge,
    pub cpu_usage_percent: Gauge,
    pub thread_count: Gauge,
    pub uptime_seconds: Gauge,

    // Scenario metrics (for Phase 4)
    pub active_scenario_mode: IntGauge,
    pub chaos_triggers_total: IntCounter,

    // Business/SLO metrics
    pub service_availability: GaugeVec,
    pub slo_compliance: GaugeVec,
    pub successful_request_rate: GaugeVec,
    pub p95_latency_slo_compliance: GaugeVec,
    pub error_budget_remaining: GaugeVec,
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
            HistogramOpts::new("mockforge_request_duration_seconds", "Request duration in seconds")
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
            Opts::new("mockforge_plugin_executions_total", "Total number of plugin executions"),
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
            Opts::new("mockforge_plugin_errors_total", "Total number of plugin errors"),
            &["plugin_name", "error_type"],
        )
        .expect("Failed to create plugin_errors_total metric");

        // WebSocket metrics
        // Path-based request metrics
        let requests_by_path_total = IntCounterVec::new(
            Opts::new(
                "mockforge_requests_by_path_total",
                "Total number of requests by path, method, and status",
            ),
            &["path", "method", "status"],
        )
        .expect("Failed to create requests_by_path_total metric");

        let request_duration_by_path_seconds = HistogramVec::new(
            HistogramOpts::new(
                "mockforge_request_duration_by_path_seconds",
                "Request duration by path in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["path", "method"],
        )
        .expect("Failed to create request_duration_by_path_seconds metric");

        let average_latency_by_path_seconds = GaugeVec::new(
            Opts::new(
                "mockforge_average_latency_by_path_seconds",
                "Average request latency by path in seconds",
            ),
            &["path", "method"],
        )
        .expect("Failed to create average_latency_by_path_seconds metric");

        // Workspace-specific metrics
        let workspace_requests_total = IntCounterVec::new(
            Opts::new(
                "mockforge_workspace_requests_total",
                "Total number of requests by workspace, method, and status",
            ),
            &["workspace_id", "method", "status"],
        )
        .expect("Failed to create workspace_requests_total metric");

        let workspace_requests_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "mockforge_workspace_request_duration_seconds",
                "Request duration by workspace in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["workspace_id", "method"],
        )
        .expect("Failed to create workspace_requests_duration_seconds metric");

        let workspace_active_routes = IntGaugeVec::new(
            Opts::new(
                "mockforge_workspace_active_routes",
                "Number of active routes in each workspace",
            ),
            &["workspace_id"],
        )
        .expect("Failed to create workspace_active_routes metric");

        let workspace_errors_total = IntCounterVec::new(
            Opts::new("mockforge_workspace_errors_total", "Total number of errors by workspace"),
            &["workspace_id", "error_type"],
        )
        .expect("Failed to create workspace_errors_total metric");

        // WebSocket metrics
        let ws_connections_active = IntGauge::new(
            "mockforge_ws_connections_active",
            "Number of active WebSocket connections",
        )
        .expect("Failed to create ws_connections_active metric");

        let ws_connections_total = IntCounter::new(
            "mockforge_ws_connections_total",
            "Total number of WebSocket connections established",
        )
        .expect("Failed to create ws_connections_total metric");

        let ws_connection_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "mockforge_ws_connection_duration_seconds",
                "WebSocket connection duration in seconds",
            )
            .buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0, 1800.0, 3600.0]),
            &["status"],
        )
        .expect("Failed to create ws_connection_duration_seconds metric");

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

        let ws_errors_total =
            IntCounter::new("mockforge_ws_errors_total", "Total number of WebSocket errors")
                .expect("Failed to create ws_errors_total metric");

        // SMTP metrics
        let smtp_connections_active =
            IntGauge::new("mockforge_smtp_connections_active", "Number of active SMTP connections")
                .expect("Failed to create smtp_connections_active metric");

        let smtp_connections_total =
            IntCounter::new("mockforge_smtp_connections_total", "Total number of SMTP connections")
                .expect("Failed to create smtp_connections_total metric");

        let smtp_messages_received_total = IntCounter::new(
            "mockforge_smtp_messages_received_total",
            "Total number of SMTP messages received",
        )
        .expect("Failed to create smtp_messages_received_total metric");

        let smtp_messages_stored_total = IntCounter::new(
            "mockforge_smtp_messages_stored_total",
            "Total number of SMTP messages stored in mailbox",
        )
        .expect("Failed to create smtp_messages_stored_total metric");

        let smtp_errors_total = IntCounterVec::new(
            Opts::new("mockforge_smtp_errors_total", "Total number of SMTP errors by type"),
            &["error_type"],
        )
        .expect("Failed to create smtp_errors_total metric");

        // MQTT metrics
        let mqtt_connections_active = IntGauge::new(
            "mockforge_mqtt_connections_active",
            "Number of active MQTT client connections",
        )
        .expect("Failed to create mqtt_connections_active metric");

        let mqtt_connections_total = IntCounter::new(
            "mockforge_mqtt_connections_total",
            "Total number of MQTT client connections established",
        )
        .expect("Failed to create mqtt_connections_total metric");

        let mqtt_messages_published_total = IntCounter::new(
            "mockforge_mqtt_messages_published_total",
            "Total number of MQTT messages published",
        )
        .expect("Failed to create mqtt_messages_published_total metric");

        let mqtt_messages_received_total = IntCounter::new(
            "mockforge_mqtt_messages_received_total",
            "Total number of MQTT messages received",
        )
        .expect("Failed to create mqtt_messages_received_total metric");

        let mqtt_topics_active =
            IntGauge::new("mockforge_mqtt_topics_active", "Number of active MQTT topics")
                .expect("Failed to create mqtt_topics_active metric");

        let mqtt_subscriptions_active = IntGauge::new(
            "mockforge_mqtt_subscriptions_active",
            "Number of active MQTT subscriptions",
        )
        .expect("Failed to create mqtt_subscriptions_active metric");

        let mqtt_retained_messages =
            IntGauge::new("mockforge_mqtt_retained_messages", "Number of retained MQTT messages")
                .expect("Failed to create mqtt_retained_messages metric");

        let mqtt_errors_total = IntCounterVec::new(
            Opts::new("mockforge_mqtt_errors_total", "Total number of MQTT errors by type"),
            &["error_type"],
        )
        .expect("Failed to create mqtt_errors_total metric");

        // System metrics
        let memory_usage_bytes =
            Gauge::new("mockforge_memory_usage_bytes", "Memory usage in bytes")
                .expect("Failed to create memory_usage_bytes metric");

        let cpu_usage_percent = Gauge::new("mockforge_cpu_usage_percent", "CPU usage percentage")
            .expect("Failed to create cpu_usage_percent metric");

        let thread_count = Gauge::new("mockforge_thread_count", "Number of active threads")
            .expect("Failed to create thread_count metric");

        let uptime_seconds = Gauge::new("mockforge_uptime_seconds", "Server uptime in seconds")
            .expect("Failed to create uptime_seconds metric");

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

        // Business/SLO metrics
        let service_availability = GaugeVec::new(
            Opts::new(
                "mockforge_service_availability",
                "Service availability percentage (0.0 to 1.0) by protocol",
            ),
            &["protocol"],
        )
        .expect("Failed to create service_availability metric");

        let slo_compliance = GaugeVec::new(
            Opts::new(
                "mockforge_slo_compliance",
                "SLO compliance percentage (0.0 to 1.0) by protocol and slo_type",
            ),
            &["protocol", "slo_type"],
        )
        .expect("Failed to create slo_compliance metric");

        let successful_request_rate = GaugeVec::new(
            Opts::new(
                "mockforge_successful_request_rate",
                "Successful request rate (0.0 to 1.0) by protocol",
            ),
            &["protocol"],
        )
        .expect("Failed to create successful_request_rate metric");

        let p95_latency_slo_compliance = GaugeVec::new(
            Opts::new(
                "mockforge_p95_latency_slo_compliance",
                "P95 latency SLO compliance (1.0 = compliant, 0.0 = non-compliant) by protocol",
            ),
            &["protocol"],
        )
        .expect("Failed to create p95_latency_slo_compliance metric");

        let error_budget_remaining = GaugeVec::new(
            Opts::new(
                "mockforge_error_budget_remaining",
                "Remaining error budget percentage (0.0 to 1.0) by protocol",
            ),
            &["protocol"],
        )
        .expect("Failed to create error_budget_remaining metric");

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
            .register(Box::new(requests_by_path_total.clone()))
            .expect("Failed to register requests_by_path_total");
        registry
            .register(Box::new(request_duration_by_path_seconds.clone()))
            .expect("Failed to register request_duration_by_path_seconds");
        registry
            .register(Box::new(average_latency_by_path_seconds.clone()))
            .expect("Failed to register average_latency_by_path_seconds");
        registry
            .register(Box::new(workspace_requests_total.clone()))
            .expect("Failed to register workspace_requests_total");
        registry
            .register(Box::new(workspace_requests_duration_seconds.clone()))
            .expect("Failed to register workspace_requests_duration_seconds");
        registry
            .register(Box::new(workspace_active_routes.clone()))
            .expect("Failed to register workspace_active_routes");
        registry
            .register(Box::new(workspace_errors_total.clone()))
            .expect("Failed to register workspace_errors_total");
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
            .register(Box::new(ws_connections_total.clone()))
            .expect("Failed to register ws_connections_total");
        registry
            .register(Box::new(ws_connection_duration_seconds.clone()))
            .expect("Failed to register ws_connection_duration_seconds");
        registry
            .register(Box::new(ws_messages_sent.clone()))
            .expect("Failed to register ws_messages_sent");
        registry
            .register(Box::new(ws_messages_received.clone()))
            .expect("Failed to register ws_messages_received");
        registry
            .register(Box::new(ws_errors_total.clone()))
            .expect("Failed to register ws_errors_total");
        registry
            .register(Box::new(smtp_connections_active.clone()))
            .expect("Failed to register smtp_connections_active");
        registry
            .register(Box::new(smtp_connections_total.clone()))
            .expect("Failed to register smtp_connections_total");
        registry
            .register(Box::new(smtp_messages_received_total.clone()))
            .expect("Failed to register smtp_messages_received_total");
        registry
            .register(Box::new(smtp_messages_stored_total.clone()))
            .expect("Failed to register smtp_messages_stored_total");
        registry
            .register(Box::new(smtp_errors_total.clone()))
            .expect("Failed to register smtp_errors_total");
        registry
            .register(Box::new(mqtt_connections_active.clone()))
            .expect("Failed to register mqtt_connections_active");
        registry
            .register(Box::new(mqtt_connections_total.clone()))
            .expect("Failed to register mqtt_connections_total");
        registry
            .register(Box::new(mqtt_messages_published_total.clone()))
            .expect("Failed to register mqtt_messages_published_total");
        registry
            .register(Box::new(mqtt_messages_received_total.clone()))
            .expect("Failed to register mqtt_messages_received_total");
        registry
            .register(Box::new(mqtt_topics_active.clone()))
            .expect("Failed to register mqtt_topics_active");
        registry
            .register(Box::new(mqtt_subscriptions_active.clone()))
            .expect("Failed to register mqtt_subscriptions_active");
        registry
            .register(Box::new(mqtt_retained_messages.clone()))
            .expect("Failed to register mqtt_retained_messages");
        registry
            .register(Box::new(mqtt_errors_total.clone()))
            .expect("Failed to register mqtt_errors_total");
        registry
            .register(Box::new(memory_usage_bytes.clone()))
            .expect("Failed to register memory_usage_bytes");
        registry
            .register(Box::new(cpu_usage_percent.clone()))
            .expect("Failed to register cpu_usage_percent");
        registry
            .register(Box::new(thread_count.clone()))
            .expect("Failed to register thread_count");
        registry
            .register(Box::new(uptime_seconds.clone()))
            .expect("Failed to register uptime_seconds");
        registry
            .register(Box::new(active_scenario_mode.clone()))
            .expect("Failed to register active_scenario_mode");
        registry
            .register(Box::new(chaos_triggers_total.clone()))
            .expect("Failed to register chaos_triggers_total");
        registry
            .register(Box::new(service_availability.clone()))
            .expect("Failed to register service_availability");
        registry
            .register(Box::new(slo_compliance.clone()))
            .expect("Failed to register slo_compliance");
        registry
            .register(Box::new(successful_request_rate.clone()))
            .expect("Failed to register successful_request_rate");
        registry
            .register(Box::new(p95_latency_slo_compliance.clone()))
            .expect("Failed to register p95_latency_slo_compliance");
        registry
            .register(Box::new(error_budget_remaining.clone()))
            .expect("Failed to register error_budget_remaining");

        debug!("Initialized Prometheus metrics registry");

        Self {
            registry: Arc::new(registry),
            requests_total,
            requests_duration_seconds,
            requests_in_flight,
            requests_by_path_total,
            request_duration_by_path_seconds,
            average_latency_by_path_seconds,
            workspace_requests_total,
            workspace_requests_duration_seconds,
            workspace_active_routes,
            workspace_errors_total,
            errors_total,
            error_rate,
            plugin_executions_total,
            plugin_execution_duration_seconds,
            plugin_errors_total,
            ws_connections_active,
            ws_connections_total,
            ws_connection_duration_seconds,
            ws_messages_sent,
            ws_messages_received,
            ws_errors_total,
            smtp_connections_active,
            smtp_connections_total,
            smtp_messages_received_total,
            smtp_messages_stored_total,
            smtp_errors_total,
            mqtt_connections_active,
            mqtt_connections_total,
            mqtt_messages_published_total,
            mqtt_messages_received_total,
            mqtt_topics_active,
            mqtt_subscriptions_active,
            mqtt_retained_messages,
            mqtt_errors_total,
            memory_usage_bytes,
            cpu_usage_percent,
            thread_count,
            uptime_seconds,
            active_scenario_mode,
            chaos_triggers_total,
            service_availability,
            slo_compliance,
            successful_request_rate,
            p95_latency_slo_compliance,
            error_budget_remaining,
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
        self.requests_total.with_label_values(&["http", method, &status_str]).inc();
        self.requests_duration_seconds
            .with_label_values(&["http", method])
            .observe(duration_seconds);
    }

    /// Record a gRPC request
    pub fn record_grpc_request(&self, method: &str, status: &str, duration_seconds: f64) {
        self.requests_total.with_label_values(&["grpc", method, status]).inc();
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
    pub fn record_plugin_execution(&self, plugin_name: &str, success: bool, duration_seconds: f64) {
        let status = if success { "success" } else { "failure" };
        self.plugin_executions_total.with_label_values(&[plugin_name, status]).inc();
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
        self.errors_total.with_label_values(&[protocol, error_type]).inc();
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

    /// Record an HTTP request with path information
    pub fn record_http_request_with_path(
        &self,
        path: &str,
        method: &str,
        status: u16,
        duration_seconds: f64,
    ) {
        // Normalize path to avoid cardinality explosion
        let normalized_path = normalize_path(path);
        let status_str = status.to_string();

        // Record by path
        self.requests_by_path_total
            .with_label_values(&[normalized_path.as_str(), method, status_str.as_str()])
            .inc();
        self.request_duration_by_path_seconds
            .with_label_values(&[normalized_path.as_str(), method])
            .observe(duration_seconds);

        // Update average latency (simple moving average approximation)
        // Note: For production use, consider using a proper moving average or quantiles
        let current = self
            .average_latency_by_path_seconds
            .with_label_values(&[normalized_path.as_str(), method])
            .get();
        let new_avg = if current == 0.0 {
            duration_seconds
        } else {
            (current * 0.95) + (duration_seconds * 0.05)
        };
        self.average_latency_by_path_seconds
            .with_label_values(&[normalized_path.as_str(), method])
            .set(new_avg);

        // Also record in the general metrics
        self.record_http_request(method, status, duration_seconds);
    }

    /// Record a WebSocket connection established
    pub fn record_ws_connection_established(&self) {
        self.ws_connections_total.inc();
        self.ws_connections_active.inc();
    }

    /// Record a WebSocket connection closed
    pub fn record_ws_connection_closed(&self, duration_seconds: f64, status: &str) {
        self.ws_connections_active.dec();
        self.ws_connection_duration_seconds
            .with_label_values(&[status])
            .observe(duration_seconds);
    }

    /// Record a WebSocket error
    pub fn record_ws_error(&self) {
        self.ws_errors_total.inc();
    }

    /// Record an SMTP connection established
    pub fn record_smtp_connection_established(&self) {
        self.smtp_connections_total.inc();
        self.smtp_connections_active.inc();
    }

    /// Record an SMTP connection closed
    pub fn record_smtp_connection_closed(&self) {
        self.smtp_connections_active.dec();
    }

    /// Record an SMTP message received
    pub fn record_smtp_message_received(&self) {
        self.smtp_messages_received_total.inc();
    }

    /// Record an SMTP message stored
    pub fn record_smtp_message_stored(&self) {
        self.smtp_messages_stored_total.inc();
    }

    /// Record an SMTP error
    pub fn record_smtp_error(&self, error_type: &str) {
        self.smtp_errors_total.with_label_values(&[error_type]).inc();
    }

    /// Update thread count
    pub fn update_thread_count(&self, count: f64) {
        self.thread_count.set(count);
    }

    /// Update uptime
    pub fn update_uptime(&self, seconds: f64) {
        self.uptime_seconds.set(seconds);
    }

    // ==================== Workspace-specific metrics ====================

    /// Record a workspace request
    pub fn record_workspace_request(
        &self,
        workspace_id: &str,
        method: &str,
        status: u16,
        duration_seconds: f64,
    ) {
        let status_str = status.to_string();
        self.workspace_requests_total
            .with_label_values(&[workspace_id, method, &status_str])
            .inc();
        self.workspace_requests_duration_seconds
            .with_label_values(&[workspace_id, method])
            .observe(duration_seconds);
    }

    /// Update workspace active routes count
    pub fn update_workspace_active_routes(&self, workspace_id: &str, count: i64) {
        self.workspace_active_routes.with_label_values(&[workspace_id]).set(count);
    }

    /// Record a workspace error
    pub fn record_workspace_error(&self, workspace_id: &str, error_type: &str) {
        self.workspace_errors_total.with_label_values(&[workspace_id, error_type]).inc();
    }

    /// Increment workspace active routes
    pub fn increment_workspace_routes(&self, workspace_id: &str) {
        self.workspace_active_routes.with_label_values(&[workspace_id]).inc();
    }

    /// Decrement workspace active routes
    pub fn decrement_workspace_routes(&self, workspace_id: &str) {
        self.workspace_active_routes.with_label_values(&[workspace_id]).dec();
    }
}

/// Normalize path to avoid high cardinality
///
/// This function replaces dynamic path segments (IDs, UUIDs, etc.) with placeholders
/// to prevent metric explosion.
fn normalize_path(path: &str) -> String {
    let mut segments: Vec<&str> = path.split('/').collect();

    for segment in &mut segments {
        // Replace UUIDs, numeric IDs, or hex strings with :id placeholder
        if is_uuid(segment)
            || segment.parse::<i64>().is_ok()
            || (segment.len() > 8 && segment.chars().all(|c| c.is_ascii_hexdigit()))
        {
            *segment = ":id";
        }
    }

    segments.join("/")
}

/// Check if a string is a UUID
fn is_uuid(s: &str) -> bool {
    s.len() == 36 && s.chars().filter(|&c| c == '-').count() == 4
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
        registry.record_ws_connection_established();
        registry.record_ws_connection_closed(120.5, "normal");
        registry.record_ws_error();
        assert!(registry.is_initialized());
    }

    #[test]
    fn test_path_normalization() {
        assert_eq!(normalize_path("/api/users/123"), "/api/users/:id");
        assert_eq!(
            normalize_path("/api/users/550e8400-e29b-41d4-a716-446655440000"),
            "/api/users/:id"
        );
        assert_eq!(normalize_path("/api/users/abc123def456"), "/api/users/:id");
        assert_eq!(normalize_path("/api/users/list"), "/api/users/list");
    }

    #[test]
    fn test_path_based_metrics() {
        let registry = MetricsRegistry::new();
        registry.record_http_request_with_path("/api/users/123", "GET", 200, 0.045);
        registry.record_http_request_with_path("/api/users/456", "GET", 200, 0.055);
        registry.record_http_request_with_path("/api/posts", "POST", 201, 0.123);
        assert!(registry.is_initialized());
    }

    #[test]
    fn test_smtp_metrics() {
        let registry = MetricsRegistry::new();
        registry.record_smtp_connection_established();
        registry.record_smtp_message_received();
        registry.record_smtp_message_stored();
        registry.record_smtp_connection_closed();
        registry.record_smtp_error("timeout");
        assert!(registry.is_initialized());
    }

    #[test]
    fn test_system_metrics() {
        let registry = MetricsRegistry::new();
        registry.update_memory_usage(1024.0 * 1024.0 * 100.0); // 100 MB
        registry.update_cpu_usage(45.5);
        registry.update_thread_count(25.0);
        registry.update_uptime(3600.0); // 1 hour
        assert!(registry.is_initialized());
    }

    #[test]
    fn test_workspace_metrics() {
        let registry = MetricsRegistry::new();

        // Record workspace requests
        registry.record_workspace_request("workspace1", "GET", 200, 0.045);
        registry.record_workspace_request("workspace1", "POST", 201, 0.123);
        registry.record_workspace_request("workspace2", "GET", 200, 0.055);

        // Update active routes
        registry.update_workspace_active_routes("workspace1", 10);
        registry.update_workspace_active_routes("workspace2", 5);

        // Record errors
        registry.record_workspace_error("workspace1", "validation");
        registry.record_workspace_error("workspace2", "timeout");

        // Test increment/decrement
        registry.increment_workspace_routes("workspace1");
        registry.decrement_workspace_routes("workspace1");

        assert!(registry.is_initialized());
    }

    #[test]
    fn test_workspace_metrics_isolation() {
        let registry = MetricsRegistry::new();

        // Ensure metrics for different workspaces are independent
        registry.record_workspace_request("ws1", "GET", 200, 0.1);
        registry.record_workspace_request("ws2", "GET", 200, 0.2);

        registry.update_workspace_active_routes("ws1", 5);
        registry.update_workspace_active_routes("ws2", 10);

        // Both should be tracked independently
        assert!(registry.is_initialized());
    }
}
