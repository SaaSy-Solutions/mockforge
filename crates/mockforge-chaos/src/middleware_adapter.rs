//! Adapter implementing the `ChaosMiddlewareTrait` from `mockforge-core`.
//!
//! This module bridges the concrete `ChaosMiddleware` implementation in this crate
//! with the trait-based interface defined in `mockforge-core::middleware_traits`,
//! enabling protocol servers to use chaos injection without a direct dependency
//! on this crate.

use crate::config::ChaosConfig;
use crate::latency_metrics::LatencyMetricsTracker;
use crate::middleware::ChaosMiddleware;
use async_trait::async_trait;
use mockforge_core::middleware_traits::{self, MiddlewareResult};
use rand::Rng;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Adapter that wraps [`ChaosMiddleware`] and implements the core
/// [`ChaosMiddlewareTrait`](middleware_traits::ChaosMiddleware).
#[derive(Clone)]
pub struct ChaosMiddlewareAdapter {
    inner: Arc<ChaosMiddleware>,
}

impl ChaosMiddlewareAdapter {
    /// Create a new adapter wrapping the given chaos middleware.
    pub fn new(inner: Arc<ChaosMiddleware>) -> Self {
        Self { inner }
    }

    /// Create a new adapter from a shared chaos config and latency tracker.
    pub fn from_config(
        config: Arc<RwLock<ChaosConfig>>,
        latency_tracker: Arc<LatencyMetricsTracker>,
    ) -> Self {
        Self {
            inner: Arc::new(ChaosMiddleware::new(config, latency_tracker)),
        }
    }

    /// Get a reference to the inner concrete middleware.
    pub fn inner(&self) -> &Arc<ChaosMiddleware> {
        &self.inner
    }
}

#[async_trait]
impl middleware_traits::ChaosMiddleware for ChaosMiddlewareAdapter {
    fn is_enabled(&self) -> bool {
        // We need to check the config synchronously; use try_read to avoid blocking.
        // If the lock is held, assume enabled (safe default for chaos).
        let config_arc = self.inner.config();
        // try_read returns a Result<RwLockReadGuard>; extract the value before dropping.
        config_arc.try_read().map(|guard| guard.enabled).unwrap_or(true)
    }

    async fn config(&self) -> Value {
        let config_arc = self.inner.config();
        let config = config_arc.read().await;
        serde_json::to_value(&*config).unwrap_or(Value::Null)
    }

    async fn apply(
        &self,
        path: &str,
        _method: &str,
        client_ip: &str,
    ) -> middleware_traits::ChaosEffect {
        let config_arc = self.inner.config();
        let config = config_arc.read().await;

        if !config.enabled {
            return middleware_traits::ChaosEffect::default();
        }

        let mut effect = middleware_traits::ChaosEffect::default();

        // Check circuit breaker
        {
            let circuit_breaker = self.inner.circuit_breaker();
            let cb = circuit_breaker.read().await;
            if !cb.allow_request().await {
                effect.circuit_breaker_open = true;
                return effect;
            }
        }

        // Check bulkhead
        {
            let bulkhead = self.inner.bulkhead();
            let bh = bulkhead.read().await;
            if bh.try_acquire().await.is_err() {
                effect.bulkhead_rejected = true;
                return effect;
            }
        }

        // Check rate limits
        {
            let rate_limiter = self.inner.rate_limiter();
            let rl = rate_limiter.read().await;
            if rl.check(Some(client_ip), Some(path)).is_err() {
                effect.rate_limit_exceeded = true;
                return effect;
            }
        }

        // Compute latency delay from config without calling inject() (which sleeps).
        // The trait caller is responsible for actually sleeping based on the returned effect.
        {
            let latency_injector = self.inner.latency_injector();
            let li = latency_injector.read().await;
            if li.is_enabled() {
                let delay_ms = li.calculate_delay_ms();
                if delay_ms > 0 {
                    effect.latency_ms = Some(delay_ms);
                    self.inner.latency_tracker().record_latency(delay_ms);
                }
            }
        }

        // Check fault injection
        let fault_config = config.fault_injection.as_ref();
        let should_inject_fault = fault_config.map(|f| f.enabled).unwrap_or(false);
        if should_inject_fault {
            if let Some(f) = fault_config {
                let mut rng = rand::rng();
                if rng.random::<f64>() <= f.http_error_probability && !f.http_errors.is_empty() {
                    let status = f.http_errors[rng.random_range(0..f.http_errors.len())];
                    effect.error_status = Some(status);
                }
            }
        }

        effect
    }

    async fn update_config(&self, config: Value) -> MiddlewareResult<()> {
        let new_config: ChaosConfig = serde_json::from_value(config)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let config_arc = self.inner.config();
        {
            let mut cfg = config_arc.write().await;
            *cfg = new_config;
        }
        self.inner.update_from_config().await;
        Ok(())
    }

    async fn record_outcome(&self, success: bool) {
        let circuit_breaker = self.inner.circuit_breaker();
        let cb = circuit_breaker.read().await;
        if success {
            cb.record_success().await;
        } else {
            cb.record_failure().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChaosConfig, LatencyConfig};
    use mockforge_core::middleware_traits::ChaosMiddleware as ChaosMiddlewareTrait;

    #[tokio::test]
    async fn test_adapter_disabled() {
        let config = Arc::new(RwLock::new(ChaosConfig {
            enabled: false,
            ..Default::default()
        }));
        let tracker = Arc::new(LatencyMetricsTracker::new());
        let adapter = ChaosMiddlewareAdapter::from_config(config, tracker);

        assert!(!adapter.is_enabled());

        let effect = adapter.apply("/test", "GET", "127.0.0.1").await;
        assert!(effect.latency_ms.is_none());
        assert!(effect.error_status.is_none());
        assert!(!effect.circuit_breaker_open);
    }

    #[tokio::test]
    async fn test_adapter_enabled_with_latency() {
        let config = Arc::new(RwLock::new(ChaosConfig {
            enabled: true,
            latency: Some(LatencyConfig {
                enabled: true,
                fixed_delay_ms: Some(50),
                probability: 1.0,
                ..Default::default()
            }),
            ..Default::default()
        }));
        let tracker = Arc::new(LatencyMetricsTracker::new());
        let adapter = ChaosMiddlewareAdapter::from_config(config, tracker);
        adapter.inner().init_from_config().await;

        assert!(adapter.is_enabled());

        let effect = adapter.apply("/test", "GET", "127.0.0.1").await;
        assert_eq!(effect.latency_ms, Some(50));
    }

    #[tokio::test]
    async fn test_adapter_config_roundtrip() {
        let config = Arc::new(RwLock::new(ChaosConfig {
            enabled: true,
            ..Default::default()
        }));
        let tracker = Arc::new(LatencyMetricsTracker::new());
        let adapter = ChaosMiddlewareAdapter::from_config(config, tracker);

        let config_json = adapter.config().await;
        assert_eq!(config_json["enabled"], true);
    }

    #[tokio::test]
    async fn test_adapter_update_config() {
        let config = Arc::new(RwLock::new(ChaosConfig::default()));
        let tracker = Arc::new(LatencyMetricsTracker::new());
        let adapter = ChaosMiddlewareAdapter::from_config(config, tracker);

        assert!(!adapter.is_enabled());

        let new_config = serde_json::json!({
            "enabled": true,
            "latency": {
                "enabled": true,
                "fixed_delay_ms": 100,
                "jitter_percent": 0.0,
                "probability": 1.0
            }
        });
        adapter.update_config(new_config).await.unwrap();

        assert!(adapter.is_enabled());
    }

    #[tokio::test]
    async fn test_adapter_as_trait_object() {
        let config = Arc::new(RwLock::new(ChaosConfig::default()));
        let tracker = Arc::new(LatencyMetricsTracker::new());
        let adapter = ChaosMiddlewareAdapter::from_config(config, tracker);

        // Verify it works as a trait object
        let trait_obj: Arc<dyn ChaosMiddlewareTrait> = Arc::new(adapter);
        assert!(!trait_obj.is_enabled());

        let effect = trait_obj.apply("/test", "GET", "127.0.0.1").await;
        assert!(effect.latency_ms.is_none());
    }
}
