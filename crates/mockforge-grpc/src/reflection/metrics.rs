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
        self.record_to_prometheus_with_pillar(method, success, duration_ms, "unknown");
    }

    /// Record to Prometheus metrics with pillar information
    pub fn record_to_prometheus_with_pillar(
        &self,
        method: &str,
        success: bool,
        duration_ms: u64,
        pillar: &str,
    ) {
        let registry = get_global_registry();
        let status = if success { "ok" } else { "error" };
        let duration_seconds = duration_ms as f64 / 1000.0;
        registry.record_grpc_request_with_pillar(method, status, duration_seconds, pillar);

        if !success {
            registry.record_error_with_pillar("grpc", "grpc_error", pillar);
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

/// Determine pillar from gRPC service/method name
fn determine_pillar_from_grpc(service_name: &str, method_name: &str) -> &'static str {
    let service_lower = service_name.to_lowercase();
    let method_lower = method_name.to_lowercase();

    // Reality pillar patterns
    if service_lower.contains("reality")
        || service_lower.contains("persona")
        || service_lower.contains("chaos")
        || method_lower.contains("reality")
        || method_lower.contains("persona")
        || method_lower.contains("chaos")
    {
        return "reality";
    }

    // Contracts pillar patterns
    if service_lower.contains("contract")
        || service_lower.contains("validation")
        || service_lower.contains("drift")
        || method_lower.contains("contract")
        || method_lower.contains("validation")
        || method_lower.contains("drift")
    {
        return "contracts";
    }

    // DevX pillar patterns
    if service_lower.contains("sdk")
        || service_lower.contains("plugin")
        || method_lower.contains("sdk")
        || method_lower.contains("plugin")
    {
        return "devx";
    }

    // Cloud pillar patterns
    if service_lower.contains("registry")
        || service_lower.contains("workspace")
        || service_lower.contains("org")
        || method_lower.contains("registry")
        || method_lower.contains("workspace")
    {
        return "cloud";
    }

    // AI pillar patterns
    if service_lower.contains("ai")
        || service_lower.contains("mockai")
        || method_lower.contains("ai")
        || method_lower.contains("llm")
    {
        return "ai";
    }

    // Default to unknown if no pattern matches
    "unknown"
}

/// Record a successful request
pub async fn record_success(service_name: &str, method_name: &str, duration_ms: u64) {
    let metrics = global_registry().get_method_metrics(service_name, method_name).await;
    metrics.record_success(duration_ms);

    // Also record to Prometheus with pillar
    let method_full = format!("{}::{}", service_name, method_name);
    let pillar = determine_pillar_from_grpc(service_name, method_name);
    metrics.record_to_prometheus_with_pillar(&method_full, true, duration_ms, pillar);
}

/// Record a failed request
pub async fn record_error(service_name: &str, method_name: &str) {
    let metrics = global_registry().get_method_metrics(service_name, method_name).await;
    metrics.record_error();

    // Also record to Prometheus with pillar
    let method_full = format!("{}::{}", service_name, method_name);
    let pillar = determine_pillar_from_grpc(service_name, method_name);
    metrics.record_to_prometheus_with_pillar(&method_full, false, 0, pillar);
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
    use super::*;

    // ==================== MethodMetrics Tests ====================

    #[test]
    fn test_method_metrics_new() {
        let metrics = MethodMetrics::new();
        assert_eq!(metrics.success_count.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.error_count.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.total_duration_ms.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.in_flight.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_method_metrics_default() {
        let metrics = MethodMetrics::default();
        assert_eq!(metrics.success_count.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.error_count.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_method_metrics_record_success() {
        let metrics = MethodMetrics::new();
        metrics.record_success(100);

        assert_eq!(metrics.success_count.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.total_duration_ms.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn test_method_metrics_record_multiple_successes() {
        let metrics = MethodMetrics::new();
        metrics.record_success(100);
        metrics.record_success(200);
        metrics.record_success(50);

        assert_eq!(metrics.success_count.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.total_duration_ms.load(Ordering::Relaxed), 350);
    }

    #[test]
    fn test_method_metrics_record_error() {
        let metrics = MethodMetrics::new();
        metrics.record_error();

        assert_eq!(metrics.error_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_method_metrics_record_multiple_errors() {
        let metrics = MethodMetrics::new();
        metrics.record_error();
        metrics.record_error();
        metrics.record_error();

        assert_eq!(metrics.error_count.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_method_metrics_mixed_success_and_error() {
        let metrics = MethodMetrics::new();
        metrics.record_success(100);
        metrics.record_error();
        metrics.record_success(200);
        metrics.record_error();

        assert_eq!(metrics.success_count.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.error_count.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.total_duration_ms.load(Ordering::Relaxed), 300);
    }

    #[test]
    fn test_method_metrics_increment_in_flight() {
        let metrics = MethodMetrics::new();
        metrics.increment_in_flight();

        assert_eq!(metrics.in_flight.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_method_metrics_decrement_in_flight() {
        let metrics = MethodMetrics::new();
        metrics.increment_in_flight();
        metrics.increment_in_flight();
        metrics.decrement_in_flight();

        assert_eq!(metrics.in_flight.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_method_metrics_in_flight_multiple() {
        let metrics = MethodMetrics::new();
        for _ in 0..5 {
            metrics.increment_in_flight();
        }

        assert_eq!(metrics.in_flight.load(Ordering::Relaxed), 5);

        for _ in 0..3 {
            metrics.decrement_in_flight();
        }

        assert_eq!(metrics.in_flight.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_method_metrics_snapshot() {
        let metrics = MethodMetrics::new();
        metrics.record_success(100);
        metrics.record_success(200);
        metrics.record_error();
        metrics.increment_in_flight();

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.success_count, 2);
        assert_eq!(snapshot.error_count, 1);
        assert_eq!(snapshot.total_duration_ms, 300);
        assert_eq!(snapshot.in_flight, 1);
    }

    // ==================== MethodMetricsSnapshot Tests ====================

    #[test]
    fn test_snapshot_average_duration_with_requests() {
        let snapshot = MethodMetricsSnapshot {
            success_count: 4,
            error_count: 0,
            total_duration_ms: 400,
            in_flight: 0,
        };

        assert!((snapshot.average_duration_ms() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_snapshot_average_duration_zero_requests() {
        let snapshot = MethodMetricsSnapshot {
            success_count: 0,
            error_count: 0,
            total_duration_ms: 0,
            in_flight: 0,
        };

        assert!((snapshot.average_duration_ms() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_snapshot_success_rate_all_success() {
        let snapshot = MethodMetricsSnapshot {
            success_count: 10,
            error_count: 0,
            total_duration_ms: 1000,
            in_flight: 0,
        };

        assert!((snapshot.success_rate() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_snapshot_success_rate_all_errors() {
        let snapshot = MethodMetricsSnapshot {
            success_count: 0,
            error_count: 10,
            total_duration_ms: 0,
            in_flight: 0,
        };

        assert!((snapshot.success_rate() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_snapshot_success_rate_mixed() {
        let snapshot = MethodMetricsSnapshot {
            success_count: 7,
            error_count: 3,
            total_duration_ms: 700,
            in_flight: 0,
        };

        assert!((snapshot.success_rate() - 70.0).abs() < 0.001);
    }

    #[test]
    fn test_snapshot_success_rate_no_requests() {
        let snapshot = MethodMetricsSnapshot {
            success_count: 0,
            error_count: 0,
            total_duration_ms: 0,
            in_flight: 0,
        };

        // No requests should report 100% success rate (no failures)
        assert!((snapshot.success_rate() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_snapshot_clone() {
        let snapshot = MethodMetricsSnapshot {
            success_count: 5,
            error_count: 2,
            total_duration_ms: 500,
            in_flight: 1,
        };

        let cloned = snapshot.clone();

        assert_eq!(cloned.success_count, snapshot.success_count);
        assert_eq!(cloned.error_count, snapshot.error_count);
        assert_eq!(cloned.total_duration_ms, snapshot.total_duration_ms);
        assert_eq!(cloned.in_flight, snapshot.in_flight);
    }

    // ==================== MetricsRegistry Tests ====================

    #[test]
    fn test_metrics_registry_new() {
        let registry = MetricsRegistry::new();
        // Registry should be created successfully
        let _ = registry;
    }

    #[test]
    fn test_metrics_registry_default() {
        let registry = MetricsRegistry::default();
        // Default registry should be empty
        let _ = registry;
    }

    #[test]
    fn test_metrics_registry_clone() {
        let registry = MetricsRegistry::new();
        let cloned = registry.clone();
        // Both should exist
        let _ = (registry, cloned);
    }

    #[tokio::test]
    async fn test_metrics_registry_get_method_metrics() {
        let registry = MetricsRegistry::new();
        let metrics = registry.get_method_metrics("TestService", "TestMethod").await;

        // Should return new metrics with zero counts
        assert_eq!(metrics.success_count.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_metrics_registry_get_same_method_twice() {
        let registry = MetricsRegistry::new();

        let metrics1 = registry.get_method_metrics("TestService", "TestMethod").await;
        metrics1.record_success(100);

        let metrics2 = registry.get_method_metrics("TestService", "TestMethod").await;

        // Should return the same metrics instance
        assert_eq!(metrics2.success_count.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_metrics_registry_different_methods() {
        let registry = MetricsRegistry::new();

        let metrics1 = registry.get_method_metrics("Service", "Method1").await;
        let metrics2 = registry.get_method_metrics("Service", "Method2").await;

        metrics1.record_success(100);

        // Different methods should have independent metrics
        assert_eq!(metrics1.success_count.load(Ordering::Relaxed), 1);
        assert_eq!(metrics2.success_count.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_metrics_registry_different_services() {
        let registry = MetricsRegistry::new();

        let metrics1 = registry.get_method_metrics("Service1", "Method").await;
        let metrics2 = registry.get_method_metrics("Service2", "Method").await;

        metrics1.record_success(100);
        metrics2.record_error();

        // Different services should have independent metrics
        assert_eq!(metrics1.success_count.load(Ordering::Relaxed), 1);
        assert_eq!(metrics1.error_count.load(Ordering::Relaxed), 0);
        assert_eq!(metrics2.success_count.load(Ordering::Relaxed), 0);
        assert_eq!(metrics2.error_count.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_metrics_registry_get_all_snapshots_empty() {
        let registry = MetricsRegistry::new();
        let snapshots = registry.get_all_snapshots().await;

        assert!(snapshots.is_empty());
    }

    #[tokio::test]
    async fn test_metrics_registry_get_all_snapshots() {
        let registry = MetricsRegistry::new();

        let metrics1 = registry.get_method_metrics("Service1", "Method1").await;
        let metrics2 = registry.get_method_metrics("Service2", "Method2").await;

        metrics1.record_success(100);
        metrics2.record_success(200);

        let snapshots = registry.get_all_snapshots().await;

        assert_eq!(snapshots.len(), 2);
        assert!(snapshots.contains_key("Service1::Method1"));
        assert!(snapshots.contains_key("Service2::Method2"));
    }

    #[tokio::test]
    async fn test_metrics_registry_get_method_snapshot() {
        let registry = MetricsRegistry::new();

        let metrics = registry.get_method_metrics("TestService", "TestMethod").await;
        metrics.record_success(150);
        metrics.record_success(250);

        let snapshot = registry.get_method_snapshot("TestService", "TestMethod").await;

        assert!(snapshot.is_some());
        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.success_count, 2);
        assert_eq!(snapshot.total_duration_ms, 400);
    }

    #[tokio::test]
    async fn test_metrics_registry_get_method_snapshot_not_found() {
        let registry = MetricsRegistry::new();

        let snapshot = registry.get_method_snapshot("NonExistent", "Method").await;

        assert!(snapshot.is_none());
    }

    // ==================== determine_pillar_from_grpc Tests ====================

    #[test]
    fn test_determine_pillar_reality() {
        assert_eq!(determine_pillar_from_grpc("RealityService", "DoSomething"), "reality");
        assert_eq!(determine_pillar_from_grpc("PersonaService", "GetPersona"), "reality");
        assert_eq!(determine_pillar_from_grpc("ChaosService", "InjectChaos"), "reality");
        assert_eq!(determine_pillar_from_grpc("SomeService", "GetReality"), "reality");
    }

    #[test]
    fn test_determine_pillar_contracts() {
        assert_eq!(determine_pillar_from_grpc("ContractService", "Validate"), "contracts");
        assert_eq!(determine_pillar_from_grpc("ValidationService", "Check"), "contracts");
        assert_eq!(determine_pillar_from_grpc("DriftService", "CheckDrift"), "contracts");
        assert_eq!(determine_pillar_from_grpc("SomeService", "ValidateContract"), "contracts");
    }

    #[test]
    fn test_determine_pillar_devx() {
        assert_eq!(determine_pillar_from_grpc("SDKService", "Generate"), "devx");
        assert_eq!(determine_pillar_from_grpc("PluginService", "Load"), "devx");
        assert_eq!(determine_pillar_from_grpc("SomeService", "GetSDK"), "devx");
    }

    #[test]
    fn test_determine_pillar_cloud() {
        assert_eq!(determine_pillar_from_grpc("RegistryService", "Push"), "cloud");
        assert_eq!(determine_pillar_from_grpc("WorkspaceService", "Create"), "cloud");
        assert_eq!(determine_pillar_from_grpc("OrgService", "GetOrg"), "cloud");
    }

    #[test]
    fn test_determine_pillar_ai() {
        assert_eq!(determine_pillar_from_grpc("AIService", "Generate"), "ai");
        assert_eq!(determine_pillar_from_grpc("MockAIService", "Predict"), "ai");
        assert_eq!(determine_pillar_from_grpc("SomeService", "RunLLM"), "ai");
    }

    #[test]
    fn test_determine_pillar_unknown() {
        assert_eq!(determine_pillar_from_grpc("UserService", "GetUser"), "unknown");
        assert_eq!(determine_pillar_from_grpc("OrderService", "CreateOrder"), "unknown");
        assert_eq!(determine_pillar_from_grpc("PaymentService", "Process"), "unknown");
    }

    #[test]
    fn test_determine_pillar_case_insensitive() {
        assert_eq!(determine_pillar_from_grpc("REALITYSERVICE", "METHOD"), "reality");
        assert_eq!(determine_pillar_from_grpc("contractservice", "method"), "contracts");
        assert_eq!(determine_pillar_from_grpc("SdKsErViCe", "MeThOd"), "devx");
    }

    // ==================== Global Registry Tests ====================

    #[test]
    fn test_global_registry_exists() {
        let registry = global_registry();
        // Global registry should exist and be accessible
        let _ = registry;
    }

    #[tokio::test]
    async fn test_record_success_function() {
        // This test uses the global registry
        record_success("TestService", "TestMethod", 100).await;
        // Should complete without panic
    }

    #[tokio::test]
    async fn test_record_error_function() {
        record_error("TestService", "TestMethod").await;
        // Should complete without panic
    }

    #[tokio::test]
    async fn test_increment_in_flight_function() {
        increment_in_flight("TestService", "TestMethod").await;
        // Should complete without panic
    }

    #[tokio::test]
    async fn test_decrement_in_flight_function() {
        decrement_in_flight("TestService", "TestMethod").await;
        // Should complete without panic
    }

    // ==================== Prometheus Recording Tests ====================

    #[test]
    fn test_record_to_prometheus_success() {
        let metrics = MethodMetrics::new();
        metrics.record_to_prometheus("test::method", true, 100);
        // Should complete without panic
    }

    #[test]
    fn test_record_to_prometheus_error() {
        let metrics = MethodMetrics::new();
        metrics.record_to_prometheus("test::method", false, 0);
        // Should complete without panic
    }

    #[test]
    fn test_record_to_prometheus_with_pillar() {
        let metrics = MethodMetrics::new();
        metrics.record_to_prometheus_with_pillar("test::method", true, 100, "reality");
        metrics.record_to_prometheus_with_pillar("test::method", false, 0, "contracts");
        // Should complete without panic
    }
}
