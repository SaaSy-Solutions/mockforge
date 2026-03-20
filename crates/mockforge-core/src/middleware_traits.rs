//! Middleware trait interfaces for protocol servers.
//!
//! These traits define the contract between protocol servers (like HTTP) and
//! optional middleware (chaos, performance, world-state) without requiring direct
//! crate dependencies. Protocol servers can accept `Arc<dyn Trait>` objects,
//! and the concrete implementations live in their respective crates.
//!
//! # Design Principles
//!
//! - Traits use `serde_json::Value` for config/data exchange to avoid type coupling
//! - All traits require `Send + Sync` for use in async contexts
//! - Methods are async where the concrete implementations need async
//! - Error types use `Box<dyn std::error::Error + Send + Sync>` for generality

use async_trait::async_trait;
use serde_json::Value;

/// Result type for middleware operations.
pub type MiddlewareResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// The effect of chaos injection on a request.
///
/// Returned by [`ChaosMiddleware::apply`] to describe what chaos effects
/// should be applied to the current request.
#[derive(Debug, Clone, Default)]
pub struct ChaosEffect {
    /// Additional latency to inject (milliseconds).
    pub latency_ms: Option<u64>,
    /// If set, return this HTTP error status code instead of the normal response.
    pub error_status: Option<u16>,
    /// If set, replace or modify the response body with this content.
    pub body_modification: Option<String>,
    /// If true, the circuit breaker is open and the request should be rejected.
    pub circuit_breaker_open: bool,
    /// If true, the rate limit was exceeded and the request should be rejected.
    pub rate_limit_exceeded: bool,
    /// If true, the bulkhead rejected the request (too many concurrent requests).
    pub bulkhead_rejected: bool,
}

/// Middleware that can inject chaos (latency, faults, traffic shaping) into requests.
///
/// Implementations of this trait wrap the chaos engineering subsystem
/// (from `mockforge-chaos`) and expose it through a protocol-agnostic interface.
#[async_trait]
pub trait ChaosMiddleware: Send + Sync {
    /// Check if chaos injection is globally enabled.
    fn is_enabled(&self) -> bool;

    /// Get the current chaos configuration as JSON.
    ///
    /// Returns a serialized representation of the full chaos config,
    /// including latency, fault injection, rate limiting, traffic shaping,
    /// circuit breaker, and bulkhead settings.
    async fn config(&self) -> Value;

    /// Apply chaos effects to a request.
    ///
    /// This evaluates all configured chaos rules (latency, faults, rate limits,
    /// circuit breaker, bulkhead) for the given request and returns a [`ChaosEffect`]
    /// describing what should happen.
    ///
    /// # Arguments
    /// * `path` - The request path (e.g., "/api/users")
    /// * `method` - The HTTP method (e.g., "GET", "POST")
    /// * `client_ip` - The client IP address for per-IP rate limiting
    async fn apply(&self, path: &str, method: &str, client_ip: &str) -> ChaosEffect;

    /// Update chaos configuration dynamically.
    ///
    /// Accepts a JSON value representing the new chaos configuration.
    /// Implementations should validate the config and update internal state.
    async fn update_config(&self, config: Value) -> MiddlewareResult<()>;

    /// Record the outcome of a request for circuit breaker tracking.
    ///
    /// # Arguments
    /// * `success` - Whether the downstream request succeeded
    async fn record_outcome(&self, success: bool);
}

/// Middleware for performance profiling and metrics collection.
///
/// Implementations of this trait wrap the performance simulation subsystem
/// (from `mockforge-performance`) and expose recording/reporting capabilities.
#[async_trait]
pub trait PerformanceMiddleware: Send + Sync {
    /// Record a request's timing and status.
    ///
    /// Should be called after each request completes.
    ///
    /// # Arguments
    /// * `path` - The request path
    /// * `method` - The HTTP method
    /// * `duration_ms` - How long the request took in milliseconds
    /// * `status` - The HTTP response status code
    async fn record_request(&self, path: &str, method: &str, duration_ms: u64, status: u16);

    /// Get a performance report as JSON.
    ///
    /// Returns a snapshot of current performance metrics including
    /// total requests, RPS, latency percentiles, error rates, and
    /// per-endpoint breakdowns.
    async fn report(&self) -> Value;

    /// Reset all collected metrics.
    async fn reset(&self);

    /// Check if the performance simulator is currently running.
    async fn is_running(&self) -> bool;
}

/// Middleware for world state management.
///
/// Implementations of this trait wrap the world state engine
/// (from `mockforge-world-state`) and expose snapshot/query capabilities.
#[async_trait]
pub trait WorldStateMiddleware: Send + Sync {
    /// Get current world state snapshot as JSON.
    ///
    /// Creates a fresh snapshot by aggregating state from all registered
    /// subsystem aggregators (personas, lifecycle, reality, etc.).
    async fn snapshot(&self) -> MiddlewareResult<Value>;

    /// Query the world state with optional filters.
    ///
    /// The `query` parameter is a JSON object that may contain:
    /// - `node_types`: array of node type strings to filter by
    /// - `layers`: array of layer strings to filter by
    /// - `node_ids`: array of specific node IDs to retrieve
    /// - `include_edges`: boolean (default true)
    /// - `max_depth`: maximum graph traversal depth
    async fn query(&self, query: Value) -> MiddlewareResult<Value>;

    /// Get the list of active state layers.
    fn layers(&self) -> Vec<String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock implementations to verify the traits compile and work correctly

    struct MockChaos {
        enabled: bool,
    }

    #[async_trait]
    impl ChaosMiddleware for MockChaos {
        fn is_enabled(&self) -> bool {
            self.enabled
        }

        async fn config(&self) -> Value {
            serde_json::json!({ "enabled": self.enabled })
        }

        async fn apply(&self, _path: &str, _method: &str, _client_ip: &str) -> ChaosEffect {
            if self.enabled {
                ChaosEffect {
                    latency_ms: Some(100),
                    ..Default::default()
                }
            } else {
                ChaosEffect::default()
            }
        }

        async fn update_config(&self, _config: Value) -> MiddlewareResult<()> {
            Ok(())
        }

        async fn record_outcome(&self, _success: bool) {}
    }

    struct MockPerformance;

    #[async_trait]
    impl PerformanceMiddleware for MockPerformance {
        async fn record_request(
            &self,
            _path: &str,
            _method: &str,
            _duration_ms: u64,
            _status: u16,
        ) {
        }

        async fn report(&self) -> Value {
            serde_json::json!({ "total_requests": 0 })
        }

        async fn reset(&self) {}

        async fn is_running(&self) -> bool {
            false
        }
    }

    struct MockWorldState;

    #[async_trait]
    impl WorldStateMiddleware for MockWorldState {
        async fn snapshot(&self) -> MiddlewareResult<Value> {
            Ok(serde_json::json!({ "nodes": [], "edges": [] }))
        }

        async fn query(&self, _query: Value) -> MiddlewareResult<Value> {
            Ok(serde_json::json!({ "nodes": [], "edges": [] }))
        }

        fn layers(&self) -> Vec<String> {
            vec!["personas".to_string(), "lifecycle".to_string()]
        }
    }

    #[tokio::test]
    async fn test_chaos_middleware_trait() {
        let chaos: Arc<dyn ChaosMiddleware> = Arc::new(MockChaos { enabled: true });
        assert!(chaos.is_enabled());

        let effect = chaos.apply("/api/users", "GET", "127.0.0.1").await;
        assert_eq!(effect.latency_ms, Some(100));
        assert!(effect.error_status.is_none());
        assert!(!effect.circuit_breaker_open);
    }

    #[tokio::test]
    async fn test_chaos_middleware_disabled() {
        let chaos: Arc<dyn ChaosMiddleware> = Arc::new(MockChaos { enabled: false });
        assert!(!chaos.is_enabled());

        let effect = chaos.apply("/api/users", "GET", "127.0.0.1").await;
        assert!(effect.latency_ms.is_none());
    }

    #[tokio::test]
    async fn test_performance_middleware_trait() {
        let perf: Arc<dyn PerformanceMiddleware> = Arc::new(MockPerformance);
        perf.record_request("/api/users", "GET", 50, 200).await;

        let report = perf.report().await;
        assert_eq!(report["total_requests"], 0);
        assert!(!perf.is_running().await);
    }

    #[tokio::test]
    async fn test_world_state_middleware_trait() {
        let ws: Arc<dyn WorldStateMiddleware> = Arc::new(MockWorldState);

        let snapshot = ws.snapshot().await.unwrap();
        assert!(snapshot["nodes"].is_array());

        let layers = ws.layers();
        assert_eq!(layers.len(), 2);
        assert!(layers.contains(&"personas".to_string()));
    }

    #[test]
    fn test_chaos_effect_default() {
        let effect = ChaosEffect::default();
        assert!(effect.latency_ms.is_none());
        assert!(effect.error_status.is_none());
        assert!(effect.body_modification.is_none());
        assert!(!effect.circuit_breaker_open);
        assert!(!effect.rate_limit_exceeded);
        assert!(!effect.bulkhead_rejected);
    }
}
