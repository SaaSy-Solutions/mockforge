//! Prometheus metrics for chaos engineering
//!
//! Provides real-time metrics that can be integrated with Grafana
//! for monitoring chaos orchestrations, scenarios, and system impact.

use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, CounterVec, GaugeVec,
    HistogramVec, Registry,
};

/// Chaos orchestration metrics
pub struct ChaosMetrics {
    /// Number of scenarios executed
    pub scenarios_total: CounterVec,

    /// Number of faults injected
    pub faults_injected_total: CounterVec,

    /// Latency injected (histogram)
    pub latency_injected: HistogramVec,

    /// Rate limit violations
    pub rate_limit_violations_total: CounterVec,

    /// Circuit breaker state
    pub circuit_breaker_state: GaugeVec,

    /// Bulkhead concurrent requests
    pub bulkhead_concurrent: GaugeVec,

    /// Orchestration step duration
    pub orchestration_step_duration: HistogramVec,

    /// Orchestration execution status
    pub orchestration_executions_total: CounterVec,

    /// Active orchestrations
    pub active_orchestrations: GaugeVec,

    /// Assertion results
    pub assertion_results_total: CounterVec,

    /// Hook executions
    pub hook_executions_total: CounterVec,

    /// Recommendation count
    pub recommendations_total: GaugeVec,

    /// System impact score
    pub chaos_impact_score: GaugeVec,
}

impl ChaosMetrics {
    /// Create new metrics
    pub fn new() -> Result<Self, prometheus::Error> {
        Ok(Self {
            scenarios_total: register_counter_vec!(
                "mockforge_chaos_scenarios_total",
                "Total number of chaos scenarios executed",
                &["scenario_type", "status"]
            )?,

            faults_injected_total: register_counter_vec!(
                "mockforge_chaos_faults_total",
                "Total number of faults injected",
                &["fault_type", "endpoint"]
            )?,

            latency_injected: register_histogram_vec!(
                "mockforge_chaos_latency_ms",
                "Latency injected in milliseconds",
                &["endpoint"],
                vec![10.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0]
            )?,

            rate_limit_violations_total: register_counter_vec!(
                "mockforge_chaos_rate_limit_violations_total",
                "Total rate limit violations",
                &["endpoint"]
            )?,

            circuit_breaker_state: register_gauge_vec!(
                "mockforge_chaos_circuit_breaker_state",
                "Circuit breaker state (0=closed, 1=open, 2=half-open)",
                &["circuit_name"]
            )?,

            bulkhead_concurrent: register_gauge_vec!(
                "mockforge_chaos_bulkhead_concurrent_requests",
                "Current concurrent requests in bulkhead",
                &["bulkhead_name"]
            )?,

            orchestration_step_duration: register_histogram_vec!(
                "mockforge_chaos_orchestration_step_duration_seconds",
                "Duration of orchestration steps in seconds",
                &["orchestration", "step"],
                vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0]
            )?,

            orchestration_executions_total: register_counter_vec!(
                "mockforge_chaos_orchestration_executions_total",
                "Total orchestration executions",
                &["orchestration", "status"]
            )?,

            active_orchestrations: register_gauge_vec!(
                "mockforge_chaos_active_orchestrations",
                "Number of active orchestrations",
                &["orchestration"]
            )?,

            assertion_results_total: register_counter_vec!(
                "mockforge_chaos_assertion_results_total",
                "Total assertion results",
                &["orchestration", "result"]
            )?,

            hook_executions_total: register_counter_vec!(
                "mockforge_chaos_hook_executions_total",
                "Total hook executions",
                &["hook_type", "status"]
            )?,

            recommendations_total: register_gauge_vec!(
                "mockforge_chaos_recommendations_total",
                "Number of AI recommendations",
                &["category", "severity"]
            )?,

            chaos_impact_score: register_gauge_vec!(
                "mockforge_chaos_impact_score",
                "Overall chaos impact score (0.0-1.0)",
                &["time_window"]
            )?,
        })
    }

    /// Record scenario execution
    pub fn record_scenario(&self, scenario_type: &str, success: bool) {
        self.scenarios_total
            .with_label_values(&[scenario_type, if success { "success" } else { "failure" }])
            .inc();
    }

    /// Record fault injection
    pub fn record_fault(&self, fault_type: &str, endpoint: &str) {
        self.faults_injected_total
            .with_label_values(&[fault_type, endpoint])
            .inc();
    }

    /// Record latency injection
    pub fn record_latency(&self, endpoint: &str, latency_ms: f64) {
        self.latency_injected
            .with_label_values(&[endpoint])
            .observe(latency_ms);
    }

    /// Record rate limit violation
    pub fn record_rate_limit_violation(&self, endpoint: &str) {
        self.rate_limit_violations_total
            .with_label_values(&[endpoint])
            .inc();
    }

    /// Update circuit breaker state
    pub fn update_circuit_breaker_state(&self, circuit_name: &str, state: f64) {
        self.circuit_breaker_state
            .with_label_values(&[circuit_name])
            .set(state);
    }

    /// Update bulkhead concurrent requests
    pub fn update_bulkhead_concurrent(&self, bulkhead_name: &str, count: f64) {
        self.bulkhead_concurrent
            .with_label_values(&[bulkhead_name])
            .set(count);
    }

    /// Record orchestration step duration
    pub fn record_step_duration(&self, orchestration: &str, step: &str, duration_secs: f64) {
        self.orchestration_step_duration
            .with_label_values(&[orchestration, step])
            .observe(duration_secs);
    }

    /// Record orchestration execution
    pub fn record_orchestration_execution(&self, orchestration: &str, success: bool) {
        self.orchestration_executions_total
            .with_label_values(&[orchestration, if success { "success" } else { "failure" }])
            .inc();
    }

    /// Update active orchestrations
    pub fn update_active_orchestrations(&self, orchestration: &str, active: bool) {
        if active {
            self.active_orchestrations
                .with_label_values(&[orchestration])
                .inc();
        } else {
            self.active_orchestrations
                .with_label_values(&[orchestration])
                .dec();
        }
    }

    /// Record assertion result
    pub fn record_assertion(&self, orchestration: &str, passed: bool) {
        self.assertion_results_total
            .with_label_values(&[orchestration, if passed { "passed" } else { "failed" }])
            .inc();
    }

    /// Record hook execution
    pub fn record_hook(&self, hook_type: &str, success: bool) {
        self.hook_executions_total
            .with_label_values(&[hook_type, if success { "success" } else { "failure" }])
            .inc();
    }

    /// Update recommendations count
    pub fn update_recommendations(&self, category: &str, severity: &str, count: f64) {
        self.recommendations_total
            .with_label_values(&[category, severity])
            .set(count);
    }

    /// Update chaos impact score
    pub fn update_impact_score(&self, time_window: &str, score: f64) {
        self.chaos_impact_score
            .with_label_values(&[time_window])
            .set(score);
    }
}

impl Default for ChaosMetrics {
    fn default() -> Self {
        Self::new().expect("Failed to create chaos metrics")
    }
}

/// Global metrics instance
pub static CHAOS_METRICS: Lazy<ChaosMetrics> = Lazy::new(|| {
    ChaosMetrics::new().expect("Failed to initialize chaos metrics")
});

/// Get the default Prometheus registry
pub fn registry() -> &'static Registry {
    prometheus::default_registry()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        // The global CHAOS_METRICS is already initialized, proving that metrics creation works.
        // Creating a second instance would fail with "AlreadyReg" because metrics are
        // registered with the global Prometheus registry.
        // Instead, verify the global instance is accessible.
        let _metrics = &*CHAOS_METRICS;
        // If we get here without panic, the metrics were successfully created
    }

    #[test]
    fn test_record_scenario() {
        let metrics = CHAOS_METRICS.scenarios_total.clone();
        let before = metrics.with_label_values(&["test", "success"]).get();

        CHAOS_METRICS.record_scenario("test", true);

        let after = metrics.with_label_values(&["test", "success"]).get();
        assert!(after > before);
    }

    #[test]
    fn test_record_latency() {
        CHAOS_METRICS.record_latency("/api/test", 100.0);
        // Just ensure it doesn't panic
    }
}
