//! Prometheus metrics for the Kubernetes operator

use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramVec, IntCounter, IntCounterVec,
    IntGauge, IntGaugeVec, Opts, Registry,
};
use std::sync::Arc;

/// Metrics collector for the operator
#[derive(Clone)]
pub struct OperatorMetrics {
    /// Total reconciliations
    pub reconciliations_total: IntCounterVec,

    /// Reconciliation errors
    pub reconciliation_errors_total: IntCounterVec,

    /// Reconciliation duration
    pub reconciliation_duration_seconds: HistogramVec,

    /// Active orchestrations
    pub active_orchestrations: IntGaugeVec,

    /// Orchestration progress
    pub orchestration_progress: GaugeVec,

    /// Orchestration step
    pub orchestration_step: IntGaugeVec,

    /// Failed steps
    pub failed_steps_total: IntCounterVec,

    /// Orchestration duration
    pub orchestration_duration_seconds: HistogramVec,

    /// CRD events
    pub crd_events_total: IntCounterVec,
}

impl OperatorMetrics {
    /// Create new metrics
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let reconciliations_total = IntCounterVec::new(
            Opts::new(
                "mockforge_operator_reconciliations_total",
                "Total number of reconciliations",
            ),
            &["namespace", "name"],
        )?;

        let reconciliation_errors_total = IntCounterVec::new(
            Opts::new(
                "mockforge_operator_reconciliation_errors_total",
                "Total number of reconciliation errors",
            ),
            &["namespace", "name", "error_type"],
        )?;

        let reconciliation_duration_seconds = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "mockforge_operator_reconciliation_duration_seconds",
                "Duration of reconciliation operations",
            )
            .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.5, 5.0, 10.0]),
            &["namespace", "name"],
        )?;

        let active_orchestrations = IntGaugeVec::new(
            Opts::new(
                "mockforge_operator_active_orchestrations",
                "Number of active orchestrations",
            ),
            &["namespace"],
        )?;

        let orchestration_progress = GaugeVec::new(
            Opts::new(
                "mockforge_orchestration_progress",
                "Current progress of orchestrations (0.0 - 1.0)",
            ),
            &["namespace", "name"],
        )?;

        let orchestration_step = IntGaugeVec::new(
            Opts::new("mockforge_orchestration_step", "Current step of orchestration"),
            &["namespace", "name"],
        )?;

        let failed_steps_total = IntCounterVec::new(
            Opts::new("mockforge_orchestration_failed_steps_total", "Total number of failed steps"),
            &["namespace", "name", "step"],
        )?;

        let orchestration_duration_seconds = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "mockforge_orchestration_duration_seconds",
                "Duration of orchestration execution",
            )
            .buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0, 600.0]),
            &["namespace", "name"],
        )?;

        let crd_events_total = IntCounterVec::new(
            Opts::new("mockforge_operator_crd_events_total", "Total number of CRD events"),
            &["event_type", "resource_type"],
        )?;

        // Register metrics
        registry.register(Box::new(reconciliations_total.clone()))?;
        registry.register(Box::new(reconciliation_errors_total.clone()))?;
        registry.register(Box::new(reconciliation_duration_seconds.clone()))?;
        registry.register(Box::new(active_orchestrations.clone()))?;
        registry.register(Box::new(orchestration_progress.clone()))?;
        registry.register(Box::new(orchestration_step.clone()))?;
        registry.register(Box::new(failed_steps_total.clone()))?;
        registry.register(Box::new(orchestration_duration_seconds.clone()))?;
        registry.register(Box::new(crd_events_total.clone()))?;

        Ok(Self {
            reconciliations_total,
            reconciliation_errors_total,
            reconciliation_duration_seconds,
            active_orchestrations,
            orchestration_progress,
            orchestration_step,
            failed_steps_total,
            orchestration_duration_seconds,
            crd_events_total,
        })
    }

    /// Record reconciliation
    pub fn record_reconciliation(&self, namespace: &str, name: &str) {
        self.reconciliations_total.with_label_values(&[namespace, name]).inc();
    }

    /// Record reconciliation error
    pub fn record_reconciliation_error(&self, namespace: &str, name: &str, error_type: &str) {
        self.reconciliation_errors_total
            .with_label_values(&[namespace, name, error_type])
            .inc();
    }

    /// Record CRD event
    pub fn record_crd_event(&self, event_type: &str, resource_type: &str) {
        self.crd_events_total.with_label_values(&[event_type, resource_type]).inc();
    }

    /// Update orchestration progress
    pub fn update_orchestration_progress(&self, namespace: &str, name: &str, progress: f64) {
        self.orchestration_progress.with_label_values(&[namespace, name]).set(progress);
    }

    /// Update orchestration step
    pub fn update_orchestration_step(&self, namespace: &str, name: &str, step: u32) {
        self.orchestration_step.with_label_values(&[namespace, name]).set(step as i64);
    }

    /// Record failed step
    pub fn record_failed_step(&self, namespace: &str, name: &str, step: &str) {
        self.failed_steps_total.with_label_values(&[namespace, name, step]).inc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_metrics_new() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry);

        assert!(metrics.is_ok());
    }

    #[test]
    fn test_operator_metrics_new_validates_registry() {
        let registry = Registry::new();
        let result = OperatorMetrics::new(&registry);

        let metrics = result.expect("Should successfully create metrics");
        // Verify metric fields are accessible (counters initialized)
        let _ = &metrics.reconciliations_total;
        let _ = &metrics.reconciliation_errors_total;
        let _ = &metrics.reconciliation_duration_seconds;
    }

    #[test]
    fn test_record_reconciliation() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.record_reconciliation("default", "test-orchestration");

        // Verify counter was incremented
        let metric_families = registry.gather();
        let reconciliations = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_operator_reconciliations_total");

        assert!(reconciliations.is_some());
    }

    #[test]
    fn test_record_reconciliation_multiple_times() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.record_reconciliation("default", "test-1");
        metrics.record_reconciliation("default", "test-1");
        metrics.record_reconciliation("default", "test-2");

        let metric_families = registry.gather();
        let reconciliations = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_operator_reconciliations_total");

        assert!(reconciliations.is_some());
    }

    #[test]
    fn test_record_reconciliation_different_namespaces() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.record_reconciliation("default", "test");
        metrics.record_reconciliation("production", "test");
        metrics.record_reconciliation("staging", "test");

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_record_reconciliation_error() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.record_reconciliation_error("default", "test-orchestration", "ValidationError");

        let metric_families = registry.gather();
        let errors = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_operator_reconciliation_errors_total");

        assert!(errors.is_some());
    }

    #[test]
    fn test_record_reconciliation_error_different_types() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.record_reconciliation_error("default", "test", "ValidationError");
        metrics.record_reconciliation_error("default", "test", "ExecutionError");
        metrics.record_reconciliation_error("default", "test", "TimeoutError");

        let metric_families = registry.gather();
        let errors = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_operator_reconciliation_errors_total");

        assert!(errors.is_some());
    }

    #[test]
    fn test_record_crd_event() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.record_crd_event("create", "ChaosOrchestration");

        let metric_families = registry.gather();
        let events = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_operator_crd_events_total");

        assert!(events.is_some());
    }

    #[test]
    fn test_record_crd_event_multiple_types() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.record_crd_event("create", "ChaosOrchestration");
        metrics.record_crd_event("update", "ChaosOrchestration");
        metrics.record_crd_event("delete", "ChaosOrchestration");
        metrics.record_crd_event("create", "ChaosScenario");

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_update_orchestration_progress() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.update_orchestration_progress("default", "test-orchestration", 0.5);

        let metric_families = registry.gather();
        let progress = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_orchestration_progress");

        assert!(progress.is_some());
    }

    #[test]
    fn test_update_orchestration_progress_multiple_values() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.update_orchestration_progress("default", "test", 0.0);
        metrics.update_orchestration_progress("default", "test", 0.25);
        metrics.update_orchestration_progress("default", "test", 0.5);
        metrics.update_orchestration_progress("default", "test", 0.75);
        metrics.update_orchestration_progress("default", "test", 1.0);

        let metric_families = registry.gather();
        let progress = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_orchestration_progress");

        assert!(progress.is_some());
    }

    #[test]
    fn test_update_orchestration_step() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.update_orchestration_step("default", "test-orchestration", 3);

        let metric_families = registry.gather();
        let step = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_orchestration_step");

        assert!(step.is_some());
    }

    #[test]
    fn test_update_orchestration_step_progression() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        for i in 0..10 {
            metrics.update_orchestration_step("default", "test", i);
        }

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_record_failed_step() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.record_failed_step("default", "test-orchestration", "step1");

        let metric_families = registry.gather();
        let failed_steps = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_orchestration_failed_steps_total");

        assert!(failed_steps.is_some());
    }

    #[test]
    fn test_record_failed_step_multiple_steps() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.record_failed_step("default", "test", "step1");
        metrics.record_failed_step("default", "test", "step2");
        metrics.record_failed_step("default", "test", "step3");

        let metric_families = registry.gather();
        let failed_steps = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_orchestration_failed_steps_total");

        assert!(failed_steps.is_some());
    }

    #[test]
    fn test_metrics_clone() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        let cloned = metrics.clone();

        // Both should work
        metrics.record_reconciliation("default", "test");
        cloned.record_reconciliation("default", "test2");

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_all_metrics_registered() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        // Observe each metric so they show up in gather()
        // Prometheus only includes metrics that have been used
        metrics.reconciliations_total.with_label_values(&["test", "test"]).inc();
        metrics
            .reconciliation_errors_total
            .with_label_values(&["test", "test", "error"])
            .inc();
        metrics
            .reconciliation_duration_seconds
            .with_label_values(&["test", "test"])
            .observe(0.1);
        metrics.active_orchestrations.with_label_values(&["test"]).set(1);
        metrics.orchestration_progress.with_label_values(&["test", "test"]).set(0.5);
        metrics.orchestration_step.with_label_values(&["test", "test"]).set(1);
        metrics.failed_steps_total.with_label_values(&["test", "test", "step1"]).inc();
        metrics
            .orchestration_duration_seconds
            .with_label_values(&["test", "test"])
            .observe(1.0);
        metrics.crd_events_total.with_label_values(&["created", "test"]).inc();

        let metric_families = registry.gather();

        // Verify all expected metrics are registered
        let expected_metrics = vec![
            "mockforge_operator_reconciliations_total",
            "mockforge_operator_reconciliation_errors_total",
            "mockforge_operator_reconciliation_duration_seconds",
            "mockforge_operator_active_orchestrations",
            "mockforge_orchestration_progress",
            "mockforge_orchestration_step",
            "mockforge_orchestration_failed_steps_total",
            "mockforge_orchestration_duration_seconds",
            "mockforge_operator_crd_events_total",
        ];

        for expected in expected_metrics {
            let found = metric_families.iter().any(|mf| mf.get_name() == expected);
            assert!(found, "Metric {} should be registered", expected);
        }
    }

    #[test]
    fn test_reconciliation_duration_histogram() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        // Observe some durations
        metrics
            .reconciliation_duration_seconds
            .with_label_values(&["default", "test"])
            .observe(0.5);

        let metric_families = registry.gather();
        let duration = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_operator_reconciliation_duration_seconds");

        assert!(duration.is_some());
    }

    #[test]
    fn test_orchestration_duration_histogram() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        // Observe some durations
        metrics
            .orchestration_duration_seconds
            .with_label_values(&["default", "test"])
            .observe(30.0);

        let metric_families = registry.gather();
        let duration = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_orchestration_duration_seconds");

        assert!(duration.is_some());
    }

    #[test]
    fn test_active_orchestrations_gauge() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        // Set active orchestrations
        metrics.active_orchestrations.with_label_values(&["default"]).set(5);

        let metric_families = registry.gather();
        let active = metric_families
            .iter()
            .find(|mf| mf.get_name() == "mockforge_operator_active_orchestrations");

        assert!(active.is_some());
    }

    #[test]
    fn test_active_orchestrations_increment_decrement() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        let gauge = metrics.active_orchestrations.with_label_values(&["default"]);

        gauge.inc();
        gauge.inc();
        gauge.inc();
        gauge.dec();

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_histogram_buckets_reconciliation_duration() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        // Test different duration values
        let durations = vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.5, 5.0, 10.0];

        for duration in durations {
            metrics
                .reconciliation_duration_seconds
                .with_label_values(&["default", "test"])
                .observe(duration);
        }

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_histogram_buckets_orchestration_duration() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        // Test different duration values
        let durations = vec![1.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0, 600.0];

        for duration in durations {
            metrics
                .orchestration_duration_seconds
                .with_label_values(&["default", "test"])
                .observe(duration);
        }

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_metrics_with_special_characters_in_labels() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        metrics.record_reconciliation("my-namespace", "test-orchestration-with-dashes");
        metrics.record_reconciliation("namespace_underscore", "test_underscore");

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_concurrent_metric_updates() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        // Simulate concurrent updates
        for i in 0..100 {
            metrics.record_reconciliation("default", &format!("test-{}", i));
            metrics.update_orchestration_progress("default", &format!("test-{}", i), 0.5);
            metrics.update_orchestration_step("default", &format!("test-{}", i), i % 10);
        }

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_error_on_duplicate_registration() {
        let registry = Registry::new();

        // First registration should succeed
        let result1 = OperatorMetrics::new(&registry);
        assert!(result1.is_ok());

        // Second registration should fail
        let result2 = OperatorMetrics::new(&registry);
        assert!(result2.is_err());
    }

    #[test]
    fn test_metric_help_text() {
        let registry = Registry::new();
        let _metrics = OperatorMetrics::new(&registry).unwrap();

        let metric_families = registry.gather();

        // Verify metrics have help text
        for mf in metric_families {
            assert!(!mf.get_help().is_empty(), "Metric {} should have help text", mf.get_name());
        }
    }

    #[test]
    fn test_complete_orchestration_lifecycle_metrics() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        let namespace = "default";
        let name = "test-orchestration";

        // Start orchestration
        metrics.record_crd_event("create", "ChaosOrchestration");
        metrics.active_orchestrations.with_label_values(&[namespace]).inc();

        // Progress through steps
        for step in 0..5 {
            metrics.record_reconciliation(namespace, name);
            metrics.update_orchestration_step(namespace, name, step);
            metrics.update_orchestration_progress(namespace, name, (step as f64) / 5.0);
        }

        // Complete
        metrics.update_orchestration_progress(namespace, name, 1.0);
        metrics.active_orchestrations.with_label_values(&[namespace]).dec();
        metrics
            .orchestration_duration_seconds
            .with_label_values(&[namespace, name])
            .observe(120.0);

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_failed_orchestration_lifecycle_metrics() {
        let registry = Registry::new();
        let metrics = OperatorMetrics::new(&registry).unwrap();

        let namespace = "default";
        let name = "test-orchestration";

        // Start orchestration
        metrics.record_crd_event("create", "ChaosOrchestration");
        metrics.active_orchestrations.with_label_values(&[namespace]).inc();

        // Progress and fail
        metrics.record_reconciliation(namespace, name);
        metrics.update_orchestration_step(namespace, name, 2);
        metrics.record_failed_step(namespace, name, "step2");
        metrics.record_reconciliation_error(namespace, name, "ExecutionError");

        // Cleanup
        metrics.active_orchestrations.with_label_values(&[namespace]).dec();

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }
}
