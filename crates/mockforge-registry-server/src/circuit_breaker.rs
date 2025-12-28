//! Circuit breaker pattern for external service resilience
//!
//! Prevents cascade failures when external dependencies (Redis, S3, email) are unhealthy.
//! The circuit breaker transitions between states:
//! - CLOSED: Normal operation, requests go through
//! - OPEN: Too many failures, requests are immediately rejected
//! - HALF-OPEN: After recovery timeout, allow test requests to check if service recovered

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Circuit is open - requests are rejected immediately
    Open,
    /// Testing if service has recovered
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "CLOSED"),
            CircuitState::Open => write!(f, "OPEN"),
            CircuitState::HalfOpen => write!(f, "HALF-OPEN"),
        }
    }
}

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u32,
    /// Duration to keep circuit open before testing recovery
    pub recovery_timeout: Duration,
    /// Number of successful requests in half-open state to close circuit
    pub success_threshold: u32,
    /// Window for tracking failures (failures outside window are forgotten)
    pub failure_window: Duration,
    /// Name for logging purposes
    pub name: String,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            success_threshold: 2,
            failure_window: Duration::from_secs(60),
            name: "default".to_string(),
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new config with the given name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set failure threshold
    pub fn failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Set recovery timeout
    pub fn recovery_timeout(mut self, timeout: Duration) -> Self {
        self.recovery_timeout = timeout;
        self
    }

    /// Set success threshold for half-open state
    pub fn success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = threshold;
        self
    }

    /// Set failure tracking window
    pub fn failure_window(mut self, window: Duration) -> Self {
        self.failure_window = window;
        self
    }
}

/// Error returned when circuit is open
#[derive(Debug, Clone)]
pub struct CircuitOpenError {
    pub service_name: String,
    pub time_until_retry: Duration,
}

impl std::fmt::Display for CircuitOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Circuit breaker open for '{}': service unavailable, retry in {:?}",
            self.service_name, self.time_until_retry
        )
    }
}

impl std::error::Error for CircuitOpenError {}

/// Internal state tracking
struct CircuitBreakerState {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    last_state_change: Instant,
}

/// Circuit breaker implementation
#[derive(Clone)]
pub struct CircuitBreaker {
    config: Arc<CircuitBreakerConfig>,
    state: Arc<RwLock<CircuitBreakerState>>,
    // Atomic counters for metrics
    total_calls: Arc<AtomicU64>,
    total_failures: Arc<AtomicU64>,
    total_rejections: Arc<AtomicU64>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config: Arc::new(config),
            state: Arc::new(RwLock::new(CircuitBreakerState {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                last_state_change: Instant::now(),
            })),
            total_calls: Arc::new(AtomicU64::new(0)),
            total_failures: Arc::new(AtomicU64::new(0)),
            total_rejections: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get the current circuit state
    pub async fn state(&self) -> CircuitState {
        let state = self.state.read().await;
        self.effective_state(&state)
    }

    /// Get metrics for this circuit breaker
    pub fn metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            name: self.config.name.clone(),
            total_calls: self.total_calls.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            total_rejections: self.total_rejections.load(Ordering::Relaxed),
        }
    }

    /// Check if a request should be allowed through
    /// Returns Ok(()) if allowed, Err with time until retry if circuit is open
    pub async fn check(&self) -> Result<(), CircuitOpenError> {
        let state = self.state.read().await;
        let effective_state = self.effective_state(&state);

        match effective_state {
            CircuitState::Closed => Ok(()),
            CircuitState::HalfOpen => Ok(()), // Allow test requests
            CircuitState::Open => {
                self.total_rejections.fetch_add(1, Ordering::Relaxed);
                let elapsed = state.last_state_change.elapsed();
                let time_until_retry = self.config.recovery_timeout.saturating_sub(elapsed);
                Err(CircuitOpenError {
                    service_name: self.config.name.clone(),
                    time_until_retry,
                })
            }
        }
    }

    /// Execute a fallible operation with circuit breaker protection
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::error::Error,
    {
        self.total_calls.fetch_add(1, Ordering::Relaxed);

        // Check if circuit allows the request
        self.check().await.map_err(CircuitBreakerError::Open)?;

        // Execute the operation
        match operation.await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(e) => {
                self.record_failure().await;
                Err(CircuitBreakerError::ServiceError(e))
            }
        }
    }

    /// Record a successful operation
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;
        let effective_state = self.effective_state(&state);

        match effective_state {
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                if state.failure_count > 0 {
                    debug!(
                        "Circuit breaker '{}': success in closed state, resetting failure count",
                        self.config.name
                    );
                }
                state.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                state.success_count += 1;
                debug!(
                    "Circuit breaker '{}': success in half-open state ({}/{})",
                    self.config.name, state.success_count, self.config.success_threshold
                );

                if state.success_count >= self.config.success_threshold {
                    debug!(
                        "Circuit breaker '{}': closing circuit after {} successful requests",
                        self.config.name, state.success_count
                    );
                    state.state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                    state.last_state_change = Instant::now();
                }
            }
            CircuitState::Open => {
                // Should not happen since we check before calling
            }
        }
    }

    /// Record a failed operation
    pub async fn record_failure(&self) {
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        let mut state = self.state.write().await;
        let effective_state = self.effective_state(&state);

        match effective_state {
            CircuitState::Closed => {
                // Check if previous failures are outside the window
                if let Some(last_failure) = state.last_failure_time {
                    if last_failure.elapsed() > self.config.failure_window {
                        state.failure_count = 0;
                    }
                }

                state.failure_count += 1;
                state.last_failure_time = Some(Instant::now());

                debug!(
                    "Circuit breaker '{}': failure in closed state ({}/{})",
                    self.config.name, state.failure_count, self.config.failure_threshold
                );

                if state.failure_count >= self.config.failure_threshold {
                    warn!(
                        "Circuit breaker '{}': opening circuit after {} failures",
                        self.config.name, state.failure_count
                    );
                    state.state = CircuitState::Open;
                    state.last_state_change = Instant::now();
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open immediately opens the circuit
                warn!(
                    "Circuit breaker '{}': failure in half-open state, re-opening circuit",
                    self.config.name
                );
                state.state = CircuitState::Open;
                state.success_count = 0;
                state.last_state_change = Instant::now();
            }
            CircuitState::Open => {
                // Already open, no action needed
            }
        }
    }

    /// Calculate the effective state, considering recovery timeout
    fn effective_state(&self, state: &CircuitBreakerState) -> CircuitState {
        match state.state {
            CircuitState::Open => {
                // Check if recovery timeout has elapsed
                if state.last_state_change.elapsed() >= self.config.recovery_timeout {
                    CircuitState::HalfOpen
                } else {
                    CircuitState::Open
                }
            }
            other => other,
        }
    }

    /// Force the circuit to close (for testing or manual intervention)
    pub async fn force_close(&self) {
        let mut state = self.state.write().await;
        warn!("Circuit breaker '{}': manually closing circuit", self.config.name);
        state.state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.last_state_change = Instant::now();
    }

    /// Force the circuit to open (for testing or manual intervention)
    pub async fn force_open(&self) {
        let mut state = self.state.write().await;
        warn!("Circuit breaker '{}': manually opening circuit", self.config.name);
        state.state = CircuitState::Open;
        state.last_state_change = Instant::now();
    }
}

/// Error type that wraps either a circuit open error or the underlying service error
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open, request was not attempted
    Open(CircuitOpenError),
    /// The underlying service returned an error
    ServiceError(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::Open(e) => write!(f, "{}", e),
            CircuitBreakerError::ServiceError(e) => write!(f, "{}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CircuitBreakerError::Open(e) => Some(e),
            CircuitBreakerError::ServiceError(e) => Some(e),
        }
    }
}

/// Metrics for circuit breaker monitoring
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub name: String,
    pub total_calls: u64,
    pub total_failures: u64,
    pub total_rejections: u64,
}

/// Registry of circuit breakers for different services
#[derive(Clone, Default)]
pub struct CircuitBreakerRegistry {
    breakers: Arc<RwLock<std::collections::HashMap<String, CircuitBreaker>>>,
}

impl CircuitBreakerRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a circuit breaker
    pub async fn register(&self, name: impl Into<String>, breaker: CircuitBreaker) {
        let name = name.into();
        let mut breakers = self.breakers.write().await;
        breakers.insert(name, breaker);
    }

    /// Get a circuit breaker by name
    pub async fn get(&self, name: &str) -> Option<CircuitBreaker> {
        let breakers = self.breakers.read().await;
        breakers.get(name).cloned()
    }

    /// Get or create a circuit breaker with default config
    pub async fn get_or_create(&self, name: impl Into<String>) -> CircuitBreaker {
        let name = name.into();
        let breakers = self.breakers.read().await;
        if let Some(breaker) = breakers.get(&name) {
            return breaker.clone();
        }
        drop(breakers);

        let breaker = CircuitBreaker::new(CircuitBreakerConfig::with_name(&name));
        let mut breakers = self.breakers.write().await;
        breakers.entry(name).or_insert(breaker).clone()
    }

    /// Get all circuit breaker metrics
    pub async fn all_metrics(&self) -> Vec<CircuitBreakerMetrics> {
        let breakers = self.breakers.read().await;
        breakers.values().map(|b| b.metrics()).collect()
    }

    /// Get all circuit breaker states
    pub async fn all_states(&self) -> Vec<(String, CircuitState)> {
        let breakers = self.breakers.read().await;
        let mut states = Vec::new();
        for (name, breaker) in breakers.iter() {
            let state = breaker.state().await;
            states.push((name.clone(), state));
        }
        states
    }
}

/// Predefined circuit breaker configurations for common services
pub mod presets {
    use super::*;

    /// Circuit breaker config for Redis (fast recovery, low threshold)
    pub fn redis() -> CircuitBreakerConfig {
        CircuitBreakerConfig::with_name("redis")
            .failure_threshold(3)
            .recovery_timeout(Duration::from_secs(10))
            .success_threshold(2)
            .failure_window(Duration::from_secs(30))
    }

    /// Circuit breaker config for S3 (slower operations, higher threshold)
    pub fn s3() -> CircuitBreakerConfig {
        CircuitBreakerConfig::with_name("s3")
            .failure_threshold(5)
            .recovery_timeout(Duration::from_secs(30))
            .success_threshold(2)
            .failure_window(Duration::from_secs(60))
    }

    /// Circuit breaker config for email service (external API, longer recovery)
    pub fn email() -> CircuitBreakerConfig {
        CircuitBreakerConfig::with_name("email")
            .failure_threshold(3)
            .recovery_timeout(Duration::from_secs(60))
            .success_threshold(1)
            .failure_window(Duration::from_secs(120))
    }

    /// Circuit breaker config for database (critical service, conservative)
    pub fn database() -> CircuitBreakerConfig {
        CircuitBreakerConfig::with_name("database")
            .failure_threshold(10)
            .recovery_timeout(Duration::from_secs(15))
            .success_threshold(3)
            .failure_window(Duration::from_secs(60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_starts_closed() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::with_name("test"));
        assert_eq!(breaker.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_opens_after_failures() {
        let config = CircuitBreakerConfig::with_name("test").failure_threshold(3);
        let breaker = CircuitBreaker::new(config);

        // Record failures
        breaker.record_failure().await;
        breaker.record_failure().await;
        assert_eq!(breaker.state().await, CircuitState::Closed);

        breaker.record_failure().await;
        assert_eq!(breaker.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_rejects_when_open() {
        let config = CircuitBreakerConfig::with_name("test")
            .failure_threshold(1)
            .recovery_timeout(Duration::from_secs(60));
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure().await;
        assert!(breaker.check().await.is_err());
    }

    #[tokio::test]
    async fn test_circuit_transitions_to_half_open() {
        let config = CircuitBreakerConfig::with_name("test")
            .failure_threshold(1)
            .recovery_timeout(Duration::from_millis(10));
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure().await;
        assert_eq!(breaker.state().await, CircuitState::Open);

        // Wait for recovery timeout
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(breaker.state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_closes_after_successes_in_half_open() {
        let config = CircuitBreakerConfig::with_name("test")
            .failure_threshold(1)
            .recovery_timeout(Duration::from_millis(10))
            .success_threshold(2);
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure().await;
        tokio::time::sleep(Duration::from_millis(20)).await;

        assert_eq!(breaker.state().await, CircuitState::HalfOpen);

        breaker.record_success().await;
        assert_eq!(breaker.state().await, CircuitState::HalfOpen);

        breaker.record_success().await;
        assert_eq!(breaker.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_failure_in_half_open_reopens_circuit() {
        let config = CircuitBreakerConfig::with_name("test")
            .failure_threshold(1)
            .recovery_timeout(Duration::from_millis(10));
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure().await;
        tokio::time::sleep(Duration::from_millis(20)).await;

        assert_eq!(breaker.state().await, CircuitState::HalfOpen);

        breaker.record_failure().await;
        assert_eq!(breaker.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_success_resets_failure_count() {
        let config = CircuitBreakerConfig::with_name("test").failure_threshold(3);
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure().await;
        breaker.record_failure().await;
        breaker.record_success().await;

        // Failure count should be reset, need 3 more failures to open
        breaker.record_failure().await;
        breaker.record_failure().await;
        assert_eq!(breaker.state().await, CircuitState::Closed);

        breaker.record_failure().await;
        assert_eq!(breaker.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_force_close() {
        let config = CircuitBreakerConfig::with_name("test").failure_threshold(1);
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure().await;
        assert_eq!(breaker.state().await, CircuitState::Open);

        breaker.force_close().await;
        assert_eq!(breaker.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_force_open() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::with_name("test"));
        assert_eq!(breaker.state().await, CircuitState::Closed);

        breaker.force_open().await;
        assert_eq!(breaker.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_metrics() {
        let config = CircuitBreakerConfig::with_name("test").failure_threshold(5);
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure().await;
        breaker.record_success().await;

        let metrics = breaker.metrics();
        assert_eq!(metrics.name, "test");
        assert_eq!(metrics.total_failures, 1);
    }

    #[tokio::test]
    async fn test_registry() {
        let registry = CircuitBreakerRegistry::new();

        let redis_breaker = CircuitBreaker::new(presets::redis());
        registry.register("redis", redis_breaker).await;

        let s3_breaker = CircuitBreaker::new(presets::s3());
        registry.register("s3", s3_breaker).await;

        assert!(registry.get("redis").await.is_some());
        assert!(registry.get("s3").await.is_some());
        assert!(registry.get("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_registry_get_or_create() {
        let registry = CircuitBreakerRegistry::new();

        let breaker1 = registry.get_or_create("test").await;
        let breaker2 = registry.get_or_create("test").await;

        // Should be the same circuit breaker
        breaker1.record_failure().await;
        assert_eq!(breaker1.metrics().total_failures, 1);
        assert_eq!(breaker2.metrics().total_failures, 1);
    }
}
