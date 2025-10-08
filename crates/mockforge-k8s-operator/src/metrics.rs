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
            Opts::new(
                "mockforge_orchestration_step",
                "Current step of orchestration",
            ),
            &["namespace", "name"],
        )?;

        let failed_steps_total = IntCounterVec::new(
            Opts::new(
                "mockforge_orchestration_failed_steps_total",
                "Total number of failed steps",
            ),
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
            Opts::new(
                "mockforge_operator_crd_events_total",
                "Total number of CRD events",
            ),
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
        self.reconciliations_total
            .with_label_values(&[namespace, name])
            .inc();
    }

    /// Record reconciliation error
    pub fn record_reconciliation_error(&self, namespace: &str, name: &str, error_type: &str) {
        self.reconciliation_errors_total
            .with_label_values(&[namespace, name, error_type])
            .inc();
    }

    /// Record CRD event
    pub fn record_crd_event(&self, event_type: &str, resource_type: &str) {
        self.crd_events_total
            .with_label_values(&[event_type, resource_type])
            .inc();
    }

    /// Update orchestration progress
    pub fn update_orchestration_progress(&self, namespace: &str, name: &str, progress: f64) {
        self.orchestration_progress
            .with_label_values(&[namespace, name])
            .set(progress);
    }

    /// Update orchestration step
    pub fn update_orchestration_step(&self, namespace: &str, name: &str, step: u32) {
        self.orchestration_step
            .with_label_values(&[namespace, name])
            .set(step as i64);
    }

    /// Record failed step
    pub fn record_failed_step(&self, namespace: &str, name: &str, step: &str) {
        self.failed_steps_total
            .with_label_values(&[namespace, name, step])
            .inc();
    }
}
