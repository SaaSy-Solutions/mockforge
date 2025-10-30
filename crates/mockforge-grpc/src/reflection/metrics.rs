//! Metrics collection for the reflection proxy

use mockforge_observability::get_global_registry;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace};

/// Metrics for a specific service/method
#[derive(Debug)]
pub struct MethodMetrics {
    /// Number of successful requests
    pub success_count: AtomicU64,
    /// Number of failed requests
    pub error_count: AtomicU64,
    /// Total request duration in milliseconds
    pub total_duration_ms: AtomicU64,
    /// Number of requests currently in flight
    pub in_flight: AtomicUsize,
}

impl Default for MethodMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl MethodMetrics {
    /// Create a new method metrics tracker
    pub fn new() -> Self {
        Self {
            success_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            total_duration_ms: AtomicU64::new(0),
            in_flight: AtomicUsize::new(0),
        }
    }

    /// Record a successful request
    pub fn record_success(&self, duration_ms: u64) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
        self.total_duration_ms.fetch_add(duration_ms, Ordering::Relaxed);
    }

    /// Record a failed request
    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record to Prometheus metrics
    pub fn record_to_prometheus(&self, method: &str, success: bool, duration_ms: u64) {
        let registry = get_global_registry();
        let status = if success { "ok" } else { "error" };
        let duration_seconds = duration_ms as f64 / 1000.0;
        registry.record_grpc_request(method, status, duration_seconds);

        if !success {
            registry.record_error("grpc", "grpc_error");
        }
    }

    /// Increment in-flight requests
    pub fn increment_in_flight(&self) {
        self.in_flight.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement in-flight requests
    pub fn decrement_in_flight(&self) {
        self.in_flight.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get a snapshot of the metrics
    pub fn snapshot(&self) -> MethodMetricsSnapshot {
        MethodMetricsSnapshot {
            success_count: self.success_count.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            total_duration_ms: self.total_duration_ms.load(Ordering::Relaxed),
            in_flight: self.in_flight.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of method metrics at a point in time
#[derive(Debug, Clone)]
pub struct MethodMetricsSnapshot {
    /// Total number of successful requests
    pub success_count: u64,
    /// Total number of failed requests
    pub error_count: u64,
    /// Sum of all request durations in milliseconds
    pub total_duration_ms: u64,
    /// Current number of in-flight requests
    pub in_flight: usize,
}

impl MethodMetricsSnapshot {
    /// Calculate the average duration in milliseconds
    pub fn average_duration_ms(&self) -> f64 {
        if self.success_count == 0 {
            0.0
        } else {
            self.total_duration_ms as f64 / self.success_count as f64
        }
    }

    /// Calculate the success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.error_count;
        if total == 0 {
            100.0
        } else {
            (self.success_count as f64 / total as f64) * 100.0
        }
    }
}

/// Global metrics registry
#[derive(Debug, Clone)]
pub struct MetricsRegistry {
    /// Metrics for each service/method combination
    method_metrics: Arc<RwLock<HashMap<String, Arc<MethodMetrics>>>>,
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new() -> Self {
        Self {
            method_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create metrics for a specific service/method
    pub async fn get_method_metrics(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> Arc<MethodMetrics> {
        let key = format!("{}::{}", service_name, method_name);
        trace!("Getting metrics for method: {}", key);

        // First, try to read the existing metrics
        {
            let metrics = self.method_metrics.read().await;
            if let Some(metrics) = metrics.get(&key) {
                return metrics.clone();
            }
        }

        // If they don't exist, acquire a write lock and create them
        let mut metrics = self.method_metrics.write().await;
        if let Some(metrics) = metrics.get(&key) {
            // Double-check pattern - another thread might have created them
            metrics.clone()
        } else {
            debug!("Creating new metrics for method: {}", key);
            let new_metrics = Arc::new(MethodMetrics::new());
            metrics.insert(key, new_metrics.clone());
            new_metrics
        }
    }

    /// Get all method metrics snapshots
    pub async fn get_all_snapshots(&self) -> HashMap<String, MethodMetricsSnapshot> {
        let metrics = self.method_metrics.read().await;
        let mut snapshots = HashMap::new();

        for (key, method_metrics) in metrics.iter() {
            snapshots.insert(key.clone(), method_metrics.snapshot());
        }

        snapshots
    }

    /// Get metrics snapshot for a specific service/method
    pub async fn get_method_snapshot(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> Option<MethodMetricsSnapshot> {
        let key = format!("{}::{}", service_name, method_name);
        let metrics = self.method_metrics.read().await;

        metrics.get(&key).map(|m| m.snapshot())
    }
}

/// Global metrics registry instance
static GLOBAL_REGISTRY: once_cell::sync::Lazy<MetricsRegistry> =
    once_cell::sync::Lazy::new(MetricsRegistry::new);

/// Get the global metrics registry
pub fn global_registry() -> &'static MetricsRegistry {
    &GLOBAL_REGISTRY
}

/// Record a successful request
pub async fn record_success(service_name: &str, method_name: &str, duration_ms: u64) {
    let metrics = global_registry().get_method_metrics(service_name, method_name).await;
    metrics.record_success(duration_ms);

    // Also record to Prometheus
    let method_full = format!("{}::{}", service_name, method_name);
    metrics.record_to_prometheus(&method_full, true, duration_ms);
}

/// Record a failed request
pub async fn record_error(service_name: &str, method_name: &str) {
    let metrics = global_registry().get_method_metrics(service_name, method_name).await;
    metrics.record_error();

    // Also record to Prometheus
    let method_full = format!("{}::{}", service_name, method_name);
    metrics.record_to_prometheus(&method_full, false, 0);
}

/// Increment in-flight requests
pub async fn increment_in_flight(service_name: &str, method_name: &str) {
    let metrics = global_registry().get_method_metrics(service_name, method_name).await;
    metrics.increment_in_flight();
}

/// Decrement in-flight requests
pub async fn decrement_in_flight(service_name: &str, method_name: &str) {
    let metrics = global_registry().get_method_metrics(service_name, method_name).await;
    metrics.decrement_in_flight();
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {}
}
