//! Adapter implementing the `PerformanceMiddlewareTrait` from `mockforge-core`.
//!
//! This module bridges the concrete `PerformanceSimulator` implementation
//! with the trait-based interface defined in `mockforge-core::middleware_traits`.

use crate::simulator::PerformanceSimulator;
use async_trait::async_trait;
use mockforge_core::middleware_traits;
use serde_json::Value;
use std::sync::Arc;

/// Adapter that wraps [`PerformanceSimulator`] and implements the core
/// [`PerformanceMiddlewareTrait`](middleware_traits::PerformanceMiddleware).
#[derive(Clone)]
pub struct PerformanceMiddlewareAdapter {
    inner: Arc<PerformanceSimulator>,
}

impl PerformanceMiddlewareAdapter {
    /// Create a new adapter wrapping the given performance simulator.
    pub fn new(simulator: Arc<PerformanceSimulator>) -> Self {
        Self { inner: simulator }
    }

    /// Get a reference to the inner simulator.
    pub fn inner(&self) -> &Arc<PerformanceSimulator> {
        &self.inner
    }
}

#[async_trait]
impl middleware_traits::PerformanceMiddleware for PerformanceMiddlewareAdapter {
    async fn record_request(&self, path: &str, method: &str, duration_ms: u64, status: u16) {
        let error = if status >= 400 {
            Some(format!("HTTP {}", status))
        } else {
            None
        };
        self.inner.record_completion(path, method, duration_ms, status, error).await;
    }

    async fn report(&self) -> Value {
        let snapshot = self.inner.get_snapshot().await;
        serde_json::to_value(&snapshot).unwrap_or(Value::Null)
    }

    async fn reset(&self) {
        self.inner.stop().await;
    }

    async fn is_running(&self) -> bool {
        self.inner.is_running().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::SimulatorConfig;
    use mockforge_core::middleware_traits::PerformanceMiddleware as PerformanceMiddlewareTrait;

    #[tokio::test]
    async fn test_adapter_creation() {
        let config = SimulatorConfig::new(10.0);
        let simulator = Arc::new(PerformanceSimulator::new(config));
        let adapter = PerformanceMiddlewareAdapter::new(simulator);

        assert!(!adapter.is_running().await);
    }

    #[tokio::test]
    async fn test_adapter_record_and_report() {
        let config = SimulatorConfig::new(10.0);
        let simulator = Arc::new(PerformanceSimulator::new(config));
        let adapter = PerformanceMiddlewareAdapter::new(simulator);

        adapter.record_request("/api/users", "GET", 50, 200).await;

        let report = adapter.report().await;
        assert!(report.is_object());
        assert!(report.get("metrics").is_some());
    }

    #[tokio::test]
    async fn test_adapter_as_trait_object() {
        let config = SimulatorConfig::new(10.0);
        let simulator = Arc::new(PerformanceSimulator::new(config));
        let adapter = PerformanceMiddlewareAdapter::new(simulator);

        let trait_obj: Arc<dyn PerformanceMiddlewareTrait> = Arc::new(adapter);
        assert!(!trait_obj.is_running().await);

        let report = trait_obj.report().await;
        assert!(report.is_object());
    }
}
