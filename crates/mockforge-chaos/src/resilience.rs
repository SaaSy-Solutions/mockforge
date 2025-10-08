//! Advanced resilience patterns: Circuit Breaker and Bulkhead

use crate::config::{BulkheadConfig, CircuitBreakerConfig};
use prometheus::{Counter, Gauge, Histogram, HistogramOpts, IntGauge, Opts, Registry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn};

#[cfg(feature = "distributed")]
use redis::{aio::ConnectionManager, AsyncCommands, Client as RedisClient};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, requests are rejected
    Open,
    /// Circuit is half-open, testing if service recovered
    HalfOpen,
}

/// Serializable circuit breaker state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerSnapshot {
    pub state: CircuitState,
    pub consecutive_failures: u64,
    pub consecutive_successes: u64,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub rejected_requests: u64,
    pub last_state_change: Option<SystemTime>,
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub rejected_requests: u64,
    pub state: CircuitState,
    pub last_state_change: Option<Instant>,
    pub consecutive_failures: u64,
    pub consecutive_successes: u64,
}

/// Circuit breaker state change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitStateChange {
    pub endpoint: String,
    pub old_state: CircuitState,
    pub new_state: CircuitState,
    pub timestamp: SystemTime,
    pub reason: String,
}

/// Distributed circuit breaker state backend
#[cfg(feature = "distributed")]
pub struct DistributedCircuitState {
    redis: ConnectionManager,
    key_prefix: String,
}

#[cfg(feature = "distributed")]
impl DistributedCircuitState {
    pub async fn new(redis_url: &str, key_prefix: impl Into<String>) -> Result<Self, redis::RedisError> {
        let client = RedisClient::open(redis_url)?;
        let conn = ConnectionManager::new(client).await?;
        Ok(Self {
            redis: conn,
            key_prefix: key_prefix.into(),
        })
    }

    async fn key(&self, endpoint: &str) -> String {
        format!("{}:circuit:{}", self.key_prefix, endpoint)
    }

    pub async fn save_state(&mut self, endpoint: &str, snapshot: &CircuitBreakerSnapshot) -> Result<(), redis::RedisError> {
        let key = self.key(endpoint).await;
        let data = bincode::serialize(snapshot).unwrap_or_default();
        self.redis.set_ex(&key, data, 3600).await
    }

    pub async fn load_state(&mut self, endpoint: &str) -> Option<CircuitBreakerSnapshot> {
        let key = self.key(endpoint).await;
        let data: Vec<u8> = self.redis.get(&key).await.ok()?;
        bincode::deserialize(&data).ok()
    }
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    config: Arc<RwLock<CircuitBreakerConfig>>,
    state: Arc<RwLock<CircuitState>>,
    consecutive_failures: Arc<AtomicU64>,
    consecutive_successes: Arc<AtomicU64>,
    total_requests: Arc<AtomicU64>,
    successful_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    rejected_requests: Arc<AtomicU64>,
    last_state_change: Arc<RwLock<Option<Instant>>>,
    half_open_requests: Arc<AtomicUsize>,
    /// Persistence configuration
    persistence_path: Option<PathBuf>,
    /// State change notification channel
    state_tx: broadcast::Sender<CircuitStateChange>,
    /// Optional distributed state backend
    #[cfg(feature = "distributed")]
    distributed_state: Option<Arc<RwLock<DistributedCircuitState>>>,
    /// Endpoint identifier for distributed scenarios
    endpoint: String,
}

impl Clone for CircuitBreaker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: self.state.clone(),
            consecutive_failures: self.consecutive_failures.clone(),
            consecutive_successes: self.consecutive_successes.clone(),
            total_requests: self.total_requests.clone(),
            successful_requests: self.successful_requests.clone(),
            failed_requests: self.failed_requests.clone(),
            rejected_requests: self.rejected_requests.clone(),
            last_state_change: self.last_state_change.clone(),
            half_open_requests: self.half_open_requests.clone(),
            persistence_path: self.persistence_path.clone(),
            state_tx: self.state_tx.clone(),
            #[cfg(feature = "distributed")]
            distributed_state: self.distributed_state.clone(),
            endpoint: self.endpoint.clone(),
        }
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        let (state_tx, _) = broadcast::channel(100);
        Self {
            config: Arc::new(RwLock::new(config)),
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            consecutive_failures: Arc::new(AtomicU64::new(0)),
            consecutive_successes: Arc::new(AtomicU64::new(0)),
            total_requests: Arc::new(AtomicU64::new(0)),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            rejected_requests: Arc::new(AtomicU64::new(0)),
            last_state_change: Arc::new(RwLock::new(None)),
            half_open_requests: Arc::new(AtomicUsize::new(0)),
            persistence_path: None,
            state_tx,
            #[cfg(feature = "distributed")]
            distributed_state: None,
            endpoint: "default".to_string(),
        }
    }

    /// Create a new circuit breaker with endpoint name
    pub fn with_endpoint(config: CircuitBreakerConfig, endpoint: impl Into<String>) -> Self {
        let mut breaker = Self::new(config);
        breaker.endpoint = endpoint.into();
        breaker
    }

    /// Enable persistence to file system
    pub fn with_persistence(mut self, path: PathBuf) -> Self {
        self.persistence_path = Some(path);
        self
    }

    /// Enable distributed state via Redis
    #[cfg(feature = "distributed")]
    pub async fn with_distributed_state(mut self, redis_url: &str) -> Result<Self, redis::RedisError> {
        let dist_state = DistributedCircuitState::new(redis_url, "mockforge").await?;
        self.distributed_state = Some(Arc::new(RwLock::new(dist_state)));
        Ok(self)
    }

    /// Subscribe to state changes
    pub fn subscribe_state_changes(&self) -> broadcast::Receiver<CircuitStateChange> {
        self.state_tx.subscribe()
    }

    /// Save state to disk
    pub async fn save_state(&self) -> std::io::Result<()> {
        if let Some(path) = &self.persistence_path {
            let snapshot = self.create_snapshot().await;
            let data = bincode::serialize(&snapshot)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            tokio::fs::write(path, data).await?;
            debug!("Circuit breaker state saved to {:?}", path);
        }

        // Also save to distributed state if configured
        #[cfg(feature = "distributed")]
        if let Some(dist_state) = &self.distributed_state {
            let snapshot = self.create_snapshot().await;
            if let Err(e) = dist_state.write().await.save_state(&self.endpoint, &snapshot).await {
                error!("Failed to save state to Redis: {}", e);
            }
        }

        Ok(())
    }

    /// Load state from disk
    pub async fn load_state(&self) -> std::io::Result<()> {
        // Try loading from distributed state first
        #[cfg(feature = "distributed")]
        if let Some(dist_state) = &self.distributed_state {
            if let Some(snapshot) = dist_state.write().await.load_state(&self.endpoint).await {
                self.restore_from_snapshot(snapshot).await;
                info!("Circuit breaker state loaded from Redis");
                return Ok(());
            }
        }

        // Fall back to file persistence
        if let Some(path) = &self.persistence_path {
            if path.exists() {
                let data = tokio::fs::read(path).await?;
                let snapshot: CircuitBreakerSnapshot = bincode::deserialize(&data)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                self.restore_from_snapshot(snapshot).await;
                info!("Circuit breaker state loaded from {:?}", path);
            }
        }

        Ok(())
    }

    /// Create a snapshot of current state
    async fn create_snapshot(&self) -> CircuitBreakerSnapshot {
        let state = *self.state.read().await;
        let last_change = self.last_state_change.read().await;
        let last_state_change = last_change.map(|instant| {
            SystemTime::now() - instant.elapsed()
        });

        CircuitBreakerSnapshot {
            state,
            consecutive_failures: self.consecutive_failures.load(Ordering::SeqCst),
            consecutive_successes: self.consecutive_successes.load(Ordering::SeqCst),
            total_requests: self.total_requests.load(Ordering::SeqCst),
            successful_requests: self.successful_requests.load(Ordering::SeqCst),
            failed_requests: self.failed_requests.load(Ordering::SeqCst),
            rejected_requests: self.rejected_requests.load(Ordering::SeqCst),
            last_state_change,
        }
    }

    /// Restore state from snapshot
    async fn restore_from_snapshot(&self, snapshot: CircuitBreakerSnapshot) {
        *self.state.write().await = snapshot.state;
        self.consecutive_failures.store(snapshot.consecutive_failures, Ordering::SeqCst);
        self.consecutive_successes.store(snapshot.consecutive_successes, Ordering::SeqCst);
        self.total_requests.store(snapshot.total_requests, Ordering::SeqCst);
        self.successful_requests.store(snapshot.successful_requests, Ordering::SeqCst);
        self.failed_requests.store(snapshot.failed_requests, Ordering::SeqCst);
        self.rejected_requests.store(snapshot.rejected_requests, Ordering::SeqCst);

        if let Some(system_time) = snapshot.last_state_change {
            if let Ok(elapsed) = system_time.elapsed() {
                *self.last_state_change.write().await = Some(Instant::now() - elapsed);
            }
        }
    }

    /// Check if request is allowed
    pub async fn allow_request(&self) -> bool {
        let config = self.config.read().await;

        if !config.enabled {
            return true;
        }

        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => {
                // Always allow in closed state
                true
            }
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_change = self.last_state_change.read().await;
                if let Some(last) = *last_change {
                    let elapsed = last.elapsed();
                    if elapsed >= Duration::from_millis(config.timeout_ms) {
                        drop(last_change);
                        drop(config);
                        // Transition to half-open
                        self.transition_to_half_open().await;
                        return true;
                    }
                }

                // Reject request
                self.rejected_requests.fetch_add(1, Ordering::SeqCst);
                debug!("Circuit breaker: Request rejected (circuit open)");
                false
            }
            CircuitState::HalfOpen => {
                // Allow limited requests
                let current = self.half_open_requests.load(Ordering::SeqCst);
                if current < config.half_open_max_requests as usize {
                    self.half_open_requests.fetch_add(1, Ordering::SeqCst);
                    debug!("Circuit breaker: Request allowed in half-open state ({}/{})",
                        current + 1, config.half_open_max_requests);
                    true
                } else {
                    self.rejected_requests.fetch_add(1, Ordering::SeqCst);
                    debug!("Circuit breaker: Request rejected (half-open limit reached)");
                    false
                }
            }
        }
    }

    /// Record successful request
    pub async fn record_success(&self) {
        let config = self.config.read().await;

        if !config.enabled {
            return;
        }

        self.total_requests.fetch_add(1, Ordering::SeqCst);
        self.successful_requests.fetch_add(1, Ordering::SeqCst);
        self.consecutive_failures.store(0, Ordering::SeqCst);
        let consecutive_successes = self.consecutive_successes.fetch_add(1, Ordering::SeqCst) + 1;

        let state = *self.state.read().await;

        if state == CircuitState::HalfOpen {
            self.half_open_requests.fetch_sub(1, Ordering::SeqCst);

            if consecutive_successes >= config.success_threshold {
                drop(config);
                self.transition_to_closed().await;
            }
        }

        debug!("Circuit breaker: Success recorded (consecutive: {})", consecutive_successes);
    }

    /// Record failed request
    pub async fn record_failure(&self) {
        let config = self.config.read().await;

        if !config.enabled {
            return;
        }

        self.total_requests.fetch_add(1, Ordering::SeqCst);
        self.failed_requests.fetch_add(1, Ordering::SeqCst);
        self.consecutive_successes.store(0, Ordering::SeqCst);
        let consecutive_failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;

        let state = *self.state.read().await;

        if state == CircuitState::HalfOpen {
            self.half_open_requests.fetch_sub(1, Ordering::SeqCst);
            drop(config);
            self.transition_to_open().await;
        } else if state == CircuitState::Closed {
            // Check consecutive failures
            if consecutive_failures >= config.failure_threshold {
                drop(config);
                self.transition_to_open().await;
                return;
            }

            // Check failure rate
            let total = self.total_requests.load(Ordering::SeqCst);
            if total >= config.min_requests_for_rate {
                let failed = self.failed_requests.load(Ordering::SeqCst);
                let failure_rate = (failed as f64 / total as f64) * 100.0;

                if failure_rate >= config.failure_rate_threshold {
                    drop(config);
                    self.transition_to_open().await;
                    return;
                }
            }
        }

        debug!("Circuit breaker: Failure recorded (consecutive: {})", consecutive_failures);
    }

    /// Transition to open state
    async fn transition_to_open(&self) {
        let mut state = self.state.write().await;
        if *state != CircuitState::Open {
            let old_state = *state;
            *state = CircuitState::Open;
            *self.last_state_change.write().await = Some(Instant::now());
            warn!("Circuit breaker '{}': Transitioned to OPEN state", self.endpoint);

            // Emit state change event
            let change = CircuitStateChange {
                endpoint: self.endpoint.clone(),
                old_state,
                new_state: CircuitState::Open,
                timestamp: SystemTime::now(),
                reason: "Failure threshold exceeded".to_string(),
            };
            let _ = self.state_tx.send(change);

            // Save state
            drop(state);
            if let Err(e) = self.save_state().await {
                error!("Failed to save circuit breaker state: {}", e);
            }
        }
    }

    /// Transition to half-open state
    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        if *state != CircuitState::HalfOpen {
            let old_state = *state;
            *state = CircuitState::HalfOpen;
            *self.last_state_change.write().await = Some(Instant::now());
            self.half_open_requests.store(0, Ordering::SeqCst);
            info!("Circuit breaker '{}': Transitioned to HALF-OPEN state", self.endpoint);

            // Emit state change event
            let change = CircuitStateChange {
                endpoint: self.endpoint.clone(),
                old_state,
                new_state: CircuitState::HalfOpen,
                timestamp: SystemTime::now(),
                reason: "Timeout elapsed, testing recovery".to_string(),
            };
            let _ = self.state_tx.send(change);

            // Save state
            drop(state);
            if let Err(e) = self.save_state().await {
                error!("Failed to save circuit breaker state: {}", e);
            }
        }
    }

    /// Transition to closed state
    async fn transition_to_closed(&self) {
        let mut state = self.state.write().await;
        if *state != CircuitState::Closed {
            let old_state = *state;
            *state = CircuitState::Closed;
            *self.last_state_change.write().await = Some(Instant::now());
            self.consecutive_failures.store(0, Ordering::SeqCst);
            self.consecutive_successes.store(0, Ordering::SeqCst);
            info!("Circuit breaker '{}': Transitioned to CLOSED state", self.endpoint);

            // Emit state change event
            let change = CircuitStateChange {
                endpoint: self.endpoint.clone(),
                old_state,
                new_state: CircuitState::Closed,
                timestamp: SystemTime::now(),
                reason: "Service recovered successfully".to_string(),
            };
            let _ = self.state_tx.send(change);

            // Save state
            drop(state);
            if let Err(e) = self.save_state().await {
                error!("Failed to save circuit breaker state: {}", e);
            }
        }
    }

    /// Reset circuit breaker statistics
    pub async fn reset(&self) {
        *self.state.write().await = CircuitState::Closed;
        *self.last_state_change.write().await = None;
        self.consecutive_failures.store(0, Ordering::SeqCst);
        self.consecutive_successes.store(0, Ordering::SeqCst);
        self.total_requests.store(0, Ordering::SeqCst);
        self.successful_requests.store(0, Ordering::SeqCst);
        self.failed_requests.store(0, Ordering::SeqCst);
        self.rejected_requests.store(0, Ordering::SeqCst);
        self.half_open_requests.store(0, Ordering::SeqCst);
        info!("Circuit breaker: Reset to initial state");
    }

    /// Get current statistics
    pub async fn stats(&self) -> CircuitStats {
        CircuitStats {
            total_requests: self.total_requests.load(Ordering::SeqCst),
            successful_requests: self.successful_requests.load(Ordering::SeqCst),
            failed_requests: self.failed_requests.load(Ordering::SeqCst),
            rejected_requests: self.rejected_requests.load(Ordering::SeqCst),
            state: *self.state.read().await,
            last_state_change: *self.last_state_change.read().await,
            consecutive_failures: self.consecutive_failures.load(Ordering::SeqCst),
            consecutive_successes: self.consecutive_successes.load(Ordering::SeqCst),
        }
    }

    /// Get current state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Update configuration
    pub async fn update_config(&self, config: CircuitBreakerConfig) {
        *self.config.write().await = config;
        info!("Circuit breaker: Configuration updated");
    }

    /// Get configuration
    pub async fn config(&self) -> CircuitBreakerConfig {
        self.config.read().await.clone()
    }
}

/// Bulkhead statistics
#[derive(Debug, Clone)]
pub struct BulkheadStats {
    pub active_requests: u32,
    pub queued_requests: u32,
    pub total_requests: u64,
    pub rejected_requests: u64,
    pub timeout_requests: u64,
}

/// Bulkhead pattern implementation
pub struct Bulkhead {
    config: Arc<RwLock<BulkheadConfig>>,
    active_requests: Arc<AtomicUsize>,
    queued_requests: Arc<AtomicUsize>,
    total_requests: Arc<AtomicU64>,
    rejected_requests: Arc<AtomicU64>,
    timeout_requests: Arc<AtomicU64>,
}

impl Clone for Bulkhead {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            active_requests: self.active_requests.clone(),
            queued_requests: self.queued_requests.clone(),
            total_requests: self.total_requests.clone(),
            rejected_requests: self.rejected_requests.clone(),
            timeout_requests: self.timeout_requests.clone(),
        }
    }
}

impl Bulkhead {
    /// Create a new bulkhead
    pub fn new(config: BulkheadConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            active_requests: Arc::new(AtomicUsize::new(0)),
            queued_requests: Arc::new(AtomicUsize::new(0)),
            total_requests: Arc::new(AtomicU64::new(0)),
            rejected_requests: Arc::new(AtomicU64::new(0)),
            timeout_requests: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Try to acquire a slot
    pub async fn try_acquire(&self) -> Result<BulkheadGuard, BulkheadError> {
        let config = self.config.read().await;

        if !config.enabled {
            return Ok(BulkheadGuard::new(self.clone(), false));
        }

        self.total_requests.fetch_add(1, Ordering::SeqCst);

        let active = self.active_requests.load(Ordering::SeqCst);

        // Check if we can accept immediately
        if active < config.max_concurrent_requests as usize {
            self.active_requests.fetch_add(1, Ordering::SeqCst);
            debug!("Bulkhead: Request accepted ({}/{})",
                active + 1, config.max_concurrent_requests);
            return Ok(BulkheadGuard::new(self.clone(), true));
        }

        // Check if we can queue
        if config.max_queue_size == 0 {
            self.rejected_requests.fetch_add(1, Ordering::SeqCst);
            warn!("Bulkhead: Request rejected (no queue)");
            return Err(BulkheadError::Rejected);
        }

        let queued = self.queued_requests.load(Ordering::SeqCst);
        if queued >= config.max_queue_size as usize {
            self.rejected_requests.fetch_add(1, Ordering::SeqCst);
            warn!("Bulkhead: Request rejected (queue full: {}/{})",
                queued, config.max_queue_size);
            return Err(BulkheadError::Rejected);
        }

        // Queue the request
        self.queued_requests.fetch_add(1, Ordering::SeqCst);
        debug!("Bulkhead: Request queued ({}/{})",
            queued + 1, config.max_queue_size);

        let timeout = Duration::from_millis(config.queue_timeout_ms);
        drop(config);

        // Wait for a slot with timeout
        let start = Instant::now();
        loop {
            if start.elapsed() >= timeout {
                self.queued_requests.fetch_sub(1, Ordering::SeqCst);
                self.timeout_requests.fetch_add(1, Ordering::SeqCst);
                warn!("Bulkhead: Request timeout in queue");
                return Err(BulkheadError::Timeout);
            }

            let active = self.active_requests.load(Ordering::SeqCst);
            let config = self.config.read().await;

            if active < config.max_concurrent_requests as usize {
                self.active_requests.fetch_add(1, Ordering::SeqCst);
                self.queued_requests.fetch_sub(1, Ordering::SeqCst);
                debug!("Bulkhead: Queued request accepted");
                return Ok(BulkheadGuard::new(self.clone(), true));
            }

            drop(config);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Release a slot
    fn release(&self) {
        let prev = self.active_requests.fetch_sub(1, Ordering::SeqCst);
        debug!("Bulkhead: Request completed ({}/{})", prev - 1, prev);
    }

    /// Get current statistics
    pub async fn stats(&self) -> BulkheadStats {
        BulkheadStats {
            active_requests: self.active_requests.load(Ordering::SeqCst) as u32,
            queued_requests: self.queued_requests.load(Ordering::SeqCst) as u32,
            total_requests: self.total_requests.load(Ordering::SeqCst),
            rejected_requests: self.rejected_requests.load(Ordering::SeqCst),
            timeout_requests: self.timeout_requests.load(Ordering::SeqCst),
        }
    }

    /// Reset statistics
    pub async fn reset(&self) {
        self.total_requests.store(0, Ordering::SeqCst);
        self.rejected_requests.store(0, Ordering::SeqCst);
        self.timeout_requests.store(0, Ordering::SeqCst);
        info!("Bulkhead: Statistics reset");
    }

    /// Update configuration
    pub async fn update_config(&self, config: BulkheadConfig) {
        *self.config.write().await = config;
        info!("Bulkhead: Configuration updated");
    }

    /// Get configuration
    pub async fn config(&self) -> BulkheadConfig {
        self.config.read().await.clone()
    }
}

/// RAII guard for bulkhead
pub struct BulkheadGuard {
    bulkhead: Option<Bulkhead>,
    should_release: bool,
}

impl BulkheadGuard {
    fn new(bulkhead: Bulkhead, should_release: bool) -> Self {
        Self {
            bulkhead: Some(bulkhead),
            should_release,
        }
    }
}

impl Drop for BulkheadGuard {
    fn drop(&mut self) {
        if self.should_release {
            if let Some(bulkhead) = &self.bulkhead {
                bulkhead.release();
            }
        }
    }
}

/// Bulkhead error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkheadError {
    /// Request rejected (queue full or no queue)
    Rejected,
    /// Request timed out in queue
    Timeout,
}

impl std::fmt::Display for BulkheadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BulkheadError::Rejected => write!(f, "Request rejected by bulkhead"),
            BulkheadError::Timeout => write!(f, "Request timed out in bulkhead queue"),
        }
    }
}

impl std::error::Error for BulkheadError {}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Initial backoff duration
    pub initial_backoff_ms: u64,
    /// Maximum backoff duration
    pub max_backoff_ms: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Jitter factor (0.0-1.0)
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 30000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

/// Retry policy with exponential backoff
pub struct RetryPolicy {
    config: RetryConfig,
}

impl RetryPolicy {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Execute function with retry logic
    pub async fn execute<F, Fut, T, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        let mut attempt = 0;
        let mut backoff = self.config.initial_backoff_ms;

        loop {
            attempt += 1;

            match f().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!("Retry successful after {} attempts", attempt);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    if attempt >= self.config.max_attempts {
                        warn!("Max retry attempts ({}) reached", self.config.max_attempts);
                        return Err(err);
                    }

                    // Calculate backoff with jitter
                    let jitter = if self.config.jitter_factor > 0.0 {
                        let range = backoff as f64 * self.config.jitter_factor;
                        (rand::random::<f64>() * range * 2.0 - range) as u64
                    } else {
                        0
                    };

                    let sleep_duration = backoff.saturating_add(jitter);
                    debug!(
                        "Retry attempt {}/{} after {}ms (backoff: {}ms, jitter: {}ms)",
                        attempt, self.config.max_attempts, sleep_duration, backoff, jitter
                    );

                    tokio::time::sleep(Duration::from_millis(sleep_duration)).await;

                    // Increase backoff for next iteration
                    backoff = ((backoff as f64 * self.config.backoff_multiplier) as u64)
                        .min(self.config.max_backoff_ms);
                }
            }
        }
    }
}

/// Circuit breaker-aware retry policy
pub struct CircuitBreakerAwareRetry {
    retry_config: RetryConfig,
    circuit_breaker: Option<Arc<CircuitBreaker>>,
}

impl CircuitBreakerAwareRetry {
    pub fn new(retry_config: RetryConfig) -> Self {
        Self {
            retry_config,
            circuit_breaker: None,
        }
    }

    pub fn with_circuit_breaker(mut self, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        self.circuit_breaker = Some(circuit_breaker);
        self
    }

    /// Execute with circuit breaker awareness
    pub async fn execute<F, Fut, T, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        // Check circuit breaker before attempting
        if let Some(cb) = &self.circuit_breaker {
            if !cb.allow_request().await {
                debug!("Circuit breaker open, skipping retry");
                // Return immediately without retry if circuit is open
                return f().await;
            }
        }

        let mut attempt = 0;
        let mut backoff = self.retry_config.initial_backoff_ms;

        loop {
            // Check circuit state before each attempt
            if let Some(cb) = &self.circuit_breaker {
                if !cb.allow_request().await {
                    debug!("Circuit breaker opened during retry, aborting");
                    return f().await;
                }
            }

            attempt += 1;

            match f().await {
                Ok(result) => {
                    if let Some(cb) = &self.circuit_breaker {
                        cb.record_success().await;
                    }
                    if attempt > 1 {
                        info!("Retry successful after {} attempts", attempt);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    if let Some(cb) = &self.circuit_breaker {
                        cb.record_failure().await;
                    }

                    if attempt >= self.retry_config.max_attempts {
                        warn!("Max retry attempts ({}) reached", self.retry_config.max_attempts);
                        return Err(err);
                    }

                    // Calculate backoff with jitter
                    let jitter = if self.retry_config.jitter_factor > 0.0 {
                        let range = backoff as f64 * self.retry_config.jitter_factor;
                        (rand::random::<f64>() * range * 2.0 - range) as u64
                    } else {
                        0
                    };

                    let sleep_duration = backoff.saturating_add(jitter);
                    debug!(
                        "Retry attempt {}/{} after {}ms",
                        attempt, self.retry_config.max_attempts, sleep_duration
                    );

                    tokio::time::sleep(Duration::from_millis(sleep_duration)).await;

                    backoff = ((backoff as f64 * self.retry_config.backoff_multiplier) as u64)
                        .min(self.retry_config.max_backoff_ms);
                }
            }
        }
    }
}

/// Fallback handler trait
pub trait FallbackHandler: Send + Sync {
    fn handle(&self) -> Vec<u8>;
}

/// Simple JSON fallback handler
pub struct JsonFallbackHandler {
    response: Vec<u8>,
}

impl JsonFallbackHandler {
    pub fn new(json: serde_json::Value) -> Self {
        let response = serde_json::to_vec(&json).unwrap_or_default();
        Self { response }
    }
}

impl FallbackHandler for JsonFallbackHandler {
    fn handle(&self) -> Vec<u8> {
        self.response.clone()
    }
}

/// Circuit breaker metrics
pub struct CircuitBreakerMetrics {
    pub state_gauge: IntGauge,
    pub total_requests: Counter,
    pub successful_requests: Counter,
    pub failed_requests: Counter,
    pub rejected_requests: Counter,
    pub state_transitions: Counter,
    pub request_duration: Histogram,
}

impl CircuitBreakerMetrics {
    pub fn new(registry: &Registry, endpoint: &str) -> Result<Self, prometheus::Error> {
        let state_gauge = IntGauge::with_opts(
            Opts::new("circuit_breaker_state", "Circuit breaker state (0=Closed, 1=Open, 2=HalfOpen)")
                .const_label("endpoint", endpoint)
        )?;
        registry.register(Box::new(state_gauge.clone()))?;

        let total_requests = Counter::with_opts(
            Opts::new("circuit_breaker_requests_total", "Total requests through circuit breaker")
                .const_label("endpoint", endpoint)
        )?;
        registry.register(Box::new(total_requests.clone()))?;

        let successful_requests = Counter::with_opts(
            Opts::new("circuit_breaker_requests_successful", "Successful requests")
                .const_label("endpoint", endpoint)
        )?;
        registry.register(Box::new(successful_requests.clone()))?;

        let failed_requests = Counter::with_opts(
            Opts::new("circuit_breaker_requests_failed", "Failed requests")
                .const_label("endpoint", endpoint)
        )?;
        registry.register(Box::new(failed_requests.clone()))?;

        let rejected_requests = Counter::with_opts(
            Opts::new("circuit_breaker_requests_rejected", "Rejected requests")
                .const_label("endpoint", endpoint)
        )?;
        registry.register(Box::new(rejected_requests.clone()))?;

        let state_transitions = Counter::with_opts(
            Opts::new("circuit_breaker_state_transitions", "Circuit breaker state transitions")
                .const_label("endpoint", endpoint)
        )?;
        registry.register(Box::new(state_transitions.clone()))?;

        let request_duration = Histogram::with_opts(
            HistogramOpts::new("circuit_breaker_request_duration_seconds", "Request duration")
                .const_label("endpoint", endpoint)
        )?;
        registry.register(Box::new(request_duration.clone()))?;

        Ok(Self {
            state_gauge,
            total_requests,
            successful_requests,
            failed_requests,
            rejected_requests,
            state_transitions,
            request_duration,
        })
    }

    pub fn update_state(&self, state: CircuitState) {
        let value = match state {
            CircuitState::Closed => 0,
            CircuitState::Open => 1,
            CircuitState::HalfOpen => 2,
        };
        self.state_gauge.set(value);
    }
}

/// Bulkhead metrics
pub struct BulkheadMetrics {
    pub active_requests: IntGauge,
    pub queued_requests: IntGauge,
    pub total_requests: Counter,
    pub rejected_requests: Counter,
    pub timeout_requests: Counter,
    pub queue_duration: Histogram,
}

impl BulkheadMetrics {
    pub fn new(registry: &Registry, service: &str) -> Result<Self, prometheus::Error> {
        let active_requests = IntGauge::with_opts(
            Opts::new("bulkhead_active_requests", "Active requests")
                .const_label("service", service)
        )?;
        registry.register(Box::new(active_requests.clone()))?;

        let queued_requests = IntGauge::with_opts(
            Opts::new("bulkhead_queued_requests", "Queued requests")
                .const_label("service", service)
        )?;
        registry.register(Box::new(queued_requests.clone()))?;

        let total_requests = Counter::with_opts(
            Opts::new("bulkhead_requests_total", "Total requests")
                .const_label("service", service)
        )?;
        registry.register(Box::new(total_requests.clone()))?;

        let rejected_requests = Counter::with_opts(
            Opts::new("bulkhead_requests_rejected", "Rejected requests")
                .const_label("service", service)
        )?;
        registry.register(Box::new(rejected_requests.clone()))?;

        let timeout_requests = Counter::with_opts(
            Opts::new("bulkhead_requests_timeout", "Timeout requests")
                .const_label("service", service)
        )?;
        registry.register(Box::new(timeout_requests.clone()))?;

        let queue_duration = Histogram::with_opts(
            HistogramOpts::new("bulkhead_queue_duration_seconds", "Time spent in queue")
                .const_label("service", service)
        )?;
        registry.register(Box::new(queue_duration.clone()))?;

        Ok(Self {
            active_requests,
            queued_requests,
            total_requests,
            rejected_requests,
            timeout_requests,
            queue_duration,
        })
    }
}

/// Dynamic threshold adjuster
pub struct DynamicThresholdAdjuster {
    /// Window for calculating metrics
    window_size: Duration,
    /// Metrics history
    history: Arc<RwLock<Vec<(Instant, bool)>>>,
    /// Minimum threshold
    min_threshold: u64,
    /// Maximum threshold
    max_threshold: u64,
    /// Target error rate (0.0-1.0)
    target_error_rate: f64,
}

impl DynamicThresholdAdjuster {
    pub fn new(window_size: Duration, min_threshold: u64, max_threshold: u64, target_error_rate: f64) -> Self {
        Self {
            window_size,
            history: Arc::new(RwLock::new(Vec::new())),
            min_threshold,
            max_threshold,
            target_error_rate,
        }
    }

    /// Record a request result
    pub async fn record(&self, success: bool) {
        let mut history = self.history.write().await;
        history.push((Instant::now(), success));

        // Clean old entries
        let cutoff = Instant::now() - self.window_size;
        history.retain(|(time, _)| *time > cutoff);
    }

    /// Calculate adaptive threshold
    pub async fn calculate_threshold(&self, current_threshold: u64) -> u64 {
        let history = self.history.read().await;

        if history.is_empty() {
            return current_threshold;
        }

        let total = history.len() as f64;
        let failures = history.iter().filter(|(_, success)| !success).count() as f64;
        let error_rate = failures / total;

        // Adjust threshold based on error rate
        let adjustment_factor = if error_rate > self.target_error_rate {
            // Increase sensitivity (lower threshold)
            0.9
        } else if error_rate < self.target_error_rate * 0.5 {
            // Decrease sensitivity (higher threshold)
            1.1
        } else {
            1.0
        };

        let new_threshold = (current_threshold as f64 * adjustment_factor) as u64;
        new_threshold.clamp(self.min_threshold, self.max_threshold)
    }
}

/// Per-endpoint circuit breaker manager
pub struct CircuitBreakerManager {
    breakers: Arc<RwLock<HashMap<String, Arc<CircuitBreaker>>>>,
    default_config: CircuitBreakerConfig,
    registry: Arc<Registry>,
    metrics: Arc<RwLock<HashMap<String, Arc<CircuitBreakerMetrics>>>>,
    threshold_adjusters: Arc<RwLock<HashMap<String, Arc<DynamicThresholdAdjuster>>>>,
}

impl CircuitBreakerManager {
    pub fn new(default_config: CircuitBreakerConfig, registry: Arc<Registry>) -> Self {
        Self {
            breakers: Arc::new(RwLock::new(HashMap::new())),
            default_config,
            registry,
            metrics: Arc::new(RwLock::new(HashMap::new())),
            threshold_adjusters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create circuit breaker for endpoint
    pub async fn get_breaker(&self, endpoint: &str) -> Arc<CircuitBreaker> {
        let breakers = self.breakers.read().await;

        if let Some(breaker) = breakers.get(endpoint) {
            return breaker.clone();
        }

        drop(breakers);

        // Create new circuit breaker
        let mut breakers = self.breakers.write().await;

        // Double-check after acquiring write lock
        if let Some(breaker) = breakers.get(endpoint) {
            return breaker.clone();
        }

        let breaker = Arc::new(CircuitBreaker::new(self.default_config.clone()));
        breakers.insert(endpoint.to_string(), breaker.clone());

        // Create metrics
        if let Ok(metrics) = CircuitBreakerMetrics::new(&self.registry, endpoint) {
            let mut metrics_map = self.metrics.write().await;
            metrics_map.insert(endpoint.to_string(), Arc::new(metrics));
        }

        // Create threshold adjuster
        let adjuster = Arc::new(DynamicThresholdAdjuster::new(
            Duration::from_secs(60),
            2,
            20,
            0.1,
        ));
        let mut adjusters = self.threshold_adjusters.write().await;
        adjusters.insert(endpoint.to_string(), adjuster);

        info!("Created circuit breaker for endpoint: {}", endpoint);
        breaker
    }

    /// Get metrics for endpoint
    pub async fn get_metrics(&self, endpoint: &str) -> Option<Arc<CircuitBreakerMetrics>> {
        let metrics = self.metrics.read().await;
        metrics.get(endpoint).cloned()
    }

    /// Get all circuit breaker states
    pub async fn get_all_states(&self) -> HashMap<String, CircuitState> {
        let breakers = self.breakers.read().await;
        let mut states = HashMap::new();

        for (endpoint, breaker) in breakers.iter() {
            states.insert(endpoint.clone(), breaker.state().await);
        }

        states
    }

    /// Record request with dynamic threshold adjustment
    pub async fn record_with_adjustment(&self, endpoint: &str, success: bool) {
        // Record in threshold adjuster
        if let Some(adjuster) = self.threshold_adjusters.read().await.get(endpoint) {
            adjuster.record(success).await;

            // Get breaker and current config
            if let Some(breaker) = self.breakers.read().await.get(endpoint) {
                let current_config = breaker.config().await;
                let new_threshold = adjuster.calculate_threshold(current_config.failure_threshold).await;

                if new_threshold != current_config.failure_threshold {
                    let mut new_config = current_config;
                    new_config.failure_threshold = new_threshold;
                    breaker.update_config(new_config).await;
                    debug!("Adjusted threshold for {} to {}", endpoint, new_threshold);
                }
            }
        }
    }
}

impl Clone for CircuitBreakerManager {
    fn clone(&self) -> Self {
        Self {
            breakers: self.breakers.clone(),
            default_config: self.default_config.clone(),
            registry: self.registry.clone(),
            metrics: self.metrics.clone(),
            threshold_adjusters: self.threshold_adjusters.clone(),
        }
    }
}

/// Per-service bulkhead manager
pub struct BulkheadManager {
    bulkheads: Arc<RwLock<HashMap<String, Arc<Bulkhead>>>>,
    default_config: BulkheadConfig,
    registry: Arc<Registry>,
    metrics: Arc<RwLock<HashMap<String, Arc<BulkheadMetrics>>>>,
}

impl BulkheadManager {
    pub fn new(default_config: BulkheadConfig, registry: Arc<Registry>) -> Self {
        Self {
            bulkheads: Arc::new(RwLock::new(HashMap::new())),
            default_config,
            registry,
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create bulkhead for service
    pub async fn get_bulkhead(&self, service: &str) -> Arc<Bulkhead> {
        let bulkheads = self.bulkheads.read().await;

        if let Some(bulkhead) = bulkheads.get(service) {
            return bulkhead.clone();
        }

        drop(bulkheads);

        // Create new bulkhead
        let mut bulkheads = self.bulkheads.write().await;

        // Double-check after acquiring write lock
        if let Some(bulkhead) = bulkheads.get(service) {
            return bulkhead.clone();
        }

        let bulkhead = Arc::new(Bulkhead::new(self.default_config.clone()));
        bulkheads.insert(service.to_string(), bulkhead.clone());

        // Create metrics
        if let Ok(metrics) = BulkheadMetrics::new(&self.registry, service) {
            let mut metrics_map = self.metrics.write().await;
            metrics_map.insert(service.to_string(), Arc::new(metrics));
        }

        info!("Created bulkhead for service: {}", service);
        bulkhead
    }

    /// Get metrics for service
    pub async fn get_metrics(&self, service: &str) -> Option<Arc<BulkheadMetrics>> {
        let metrics = self.metrics.read().await;
        metrics.get(service).cloned()
    }

    /// Get all bulkhead statistics
    pub async fn get_all_stats(&self) -> HashMap<String, BulkheadStats> {
        let bulkheads = self.bulkheads.read().await;
        let mut stats = HashMap::new();

        for (service, bulkhead) in bulkheads.iter() {
            stats.insert(service.clone(), bulkhead.stats().await);
        }

        stats
    }
}

impl Clone for BulkheadManager {
    fn clone(&self) -> Self {
        Self {
            bulkheads: self.bulkheads.clone(),
            default_config: self.default_config.clone(),
            registry: self.registry.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

/// Health check protocol
#[derive(Clone)]
pub enum HealthCheckProtocol {
    Http { url: String },
    Https { url: String },
    Tcp { host: String, port: u16 },
    Grpc { endpoint: String },
    WebSocket { url: String },
    Custom { checker: Arc<dyn CustomHealthChecker> },
}

impl std::fmt::Debug for HealthCheckProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthCheckProtocol::Http { url } => write!(f, "Http {{ url: {:?} }}", url),
            HealthCheckProtocol::Https { url } => write!(f, "Https {{ url: {:?} }}", url),
            HealthCheckProtocol::Tcp { host, port } => write!(f, "Tcp {{ host: {:?}, port: {} }}", host, port),
            HealthCheckProtocol::Grpc { endpoint } => write!(f, "Grpc {{ endpoint: {:?} }}", endpoint),
            HealthCheckProtocol::WebSocket { url } => write!(f, "WebSocket {{ url: {:?} }}", url),
            HealthCheckProtocol::Custom { .. } => write!(f, "Custom {{ checker: <custom> }}"),
        }
    }
}

/// Custom health checker trait
pub trait CustomHealthChecker: Send + Sync {
    fn check(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>>;
}

/// Health check integration with multiple protocol support
pub struct HealthCheckIntegration {
    circuit_manager: Arc<CircuitBreakerManager>,
}

impl HealthCheckIntegration {
    pub fn new(circuit_manager: Arc<CircuitBreakerManager>) -> Self {
        Self { circuit_manager }
    }

    /// Update circuit breaker state based on health check
    pub async fn update_from_health(&self, endpoint: &str, healthy: bool) {
        let breaker = self.circuit_manager.get_breaker(endpoint).await;

        if healthy {
            breaker.record_success().await;
        } else {
            breaker.record_failure().await;
        }

        info!("Updated circuit breaker for {} based on health check: {}", endpoint, healthy);
    }

    /// Perform health check based on protocol
    pub async fn check_health(&self, protocol: &HealthCheckProtocol) -> bool {
        match protocol {
            HealthCheckProtocol::Http { url } | HealthCheckProtocol::Https { url } => {
                let client = reqwest::Client::new();
                match client.get(url).timeout(Duration::from_secs(5)).send().await {
                    Ok(response) => response.status().is_success(),
                    Err(_) => false,
                }
            }
            HealthCheckProtocol::Tcp { host, port } => {
                use tokio::net::TcpStream;
                TcpStream::connect(format!("{}:{}", host, port))
                    .await
                    .is_ok()
            }
            HealthCheckProtocol::Grpc { endpoint } => {
                // Basic gRPC health check - could be enhanced with grpc-health-probe
                let client = reqwest::Client::new();
                match client.post(format!("{}/grpc.health.v1.Health/Check", endpoint))
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await
                {
                    Ok(response) => response.status().is_success(),
                    Err(_) => false,
                }
            }
            HealthCheckProtocol::WebSocket { url } => {
                // WebSocket connection test
                use tokio_tungstenite::connect_async;
                connect_async(url).await.is_ok()
            }
            HealthCheckProtocol::Custom { checker } => {
                checker.check().await
            }
        }
    }

    /// Start periodic health check monitoring with custom protocol
    pub async fn start_monitoring(
        &self,
        endpoint: String,
        protocol: HealthCheckProtocol,
        interval: Duration,
    ) {
        let circuit_manager = self.circuit_manager.clone();
        let integration = self.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                let healthy = integration.check_health(&protocol).await;
                let breaker = circuit_manager.get_breaker(&endpoint).await;

                if healthy {
                    breaker.record_success().await;
                } else {
                    breaker.record_failure().await;
                }
            }
        });
    }
}

impl Clone for HealthCheckIntegration {
    fn clone(&self) -> Self {
        Self {
            circuit_manager: self.circuit_manager.clone(),
        }
    }
}

/// WebSocket notification handler for real-time updates
pub struct ResilienceWebSocketNotifier {
    connections: Arc<RwLock<Vec<broadcast::Sender<String>>>>,
}

impl ResilienceWebSocketNotifier {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a new WebSocket connection
    pub async fn register(&self) -> broadcast::Receiver<String> {
        let (tx, rx) = broadcast::channel(100);
        self.connections.write().await.push(tx);
        rx
    }

    /// Notify all connected clients
    pub async fn notify(&self, message: impl Into<String>) {
        let msg = message.into();
        let connections = self.connections.read().await;
        for tx in connections.iter() {
            let _ = tx.send(msg.clone());
        }
    }

    /// Start monitoring circuit breaker state changes
    pub async fn monitor_circuit_breaker(&self, breaker: Arc<CircuitBreaker>) {
        let notifier = self.clone();
        let mut rx = breaker.subscribe_state_changes();

        tokio::spawn(async move {
            while let Ok(change) = rx.recv().await {
                let message = serde_json::to_string(&change).unwrap_or_default();
                notifier.notify(message).await;
            }
        });
    }
}

impl Clone for ResilienceWebSocketNotifier {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
        }
    }
}

impl Default for ResilienceWebSocketNotifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Alert handler for circuit breaker state changes
pub struct CircuitBreakerAlertHandler {
    alert_manager: Arc<crate::alerts::AlertManager>,
}

impl CircuitBreakerAlertHandler {
    pub fn new(alert_manager: Arc<crate::alerts::AlertManager>) -> Self {
        Self { alert_manager }
    }

    /// Monitor circuit breaker and send alerts on state changes
    pub async fn monitor(&self, breaker: Arc<CircuitBreaker>) {
        let alert_manager = self.alert_manager.clone();
        let mut rx = breaker.subscribe_state_changes();

        tokio::spawn(async move {
            while let Ok(change) = rx.recv().await {
                // Only alert on transition to Open state
                if change.new_state == CircuitState::Open {
                    let alert = crate::alerts::Alert::new(
                        crate::alerts::AlertSeverity::Critical,
                        crate::alerts::AlertType::Custom {
                            message: format!("Circuit breaker opened for {}", change.endpoint),
                            metadata: {
                                let mut map = HashMap::new();
                                map.insert("endpoint".to_string(), change.endpoint.clone());
                                map.insert("reason".to_string(), change.reason.clone());
                                map.insert("timestamp".to_string(), format!("{:?}", change.timestamp));
                                map
                            },
                        },
                        format!("Circuit breaker for endpoint '{}' has opened: {}",
                            change.endpoint, change.reason),
                    );
                    alert_manager.fire_alert(alert);
                } else if change.new_state == CircuitState::Closed && change.old_state == CircuitState::Open {
                    // Resolve alert when circuit closes after being open
                    info!("Circuit breaker for '{}' recovered and closed", change.endpoint);
                }
            }
        });
    }
}

/// SLO (Service Level Objective) tracker
#[derive(Debug, Clone)]
pub struct SLOConfig {
    /// Target success rate (0.0-1.0)
    pub target_success_rate: f64,
    /// Window duration for SLO calculation
    pub window_duration: Duration,
    /// Error budget (percentage of allowed failures, 0-100)
    pub error_budget_percent: f64,
}

impl Default for SLOConfig {
    fn default() -> Self {
        Self {
            target_success_rate: 0.99, // 99% success rate
            window_duration: Duration::from_secs(300), // 5 minutes
            error_budget_percent: 1.0, // 1% error budget
        }
    }
}

/// SLO tracker for circuit breaker integration
pub struct SLOTracker {
    config: SLOConfig,
    history: Arc<RwLock<Vec<(Instant, bool)>>>,
}

impl SLOTracker {
    pub fn new(config: SLOConfig) -> Self {
        Self {
            config,
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a request result
    pub async fn record(&self, success: bool) {
        let mut history = self.history.write().await;
        history.push((Instant::now(), success));

        // Clean old entries outside the window
        let cutoff = Instant::now() - self.config.window_duration;
        history.retain(|(time, _)| *time > cutoff);
    }

    /// Calculate current success rate
    pub async fn success_rate(&self) -> f64 {
        let history = self.history.read().await;
        if history.is_empty() {
            return 1.0;
        }

        let total = history.len() as f64;
        let successes = history.iter().filter(|(_, success)| *success).count() as f64;
        successes / total
    }

    /// Check if SLO is violated
    pub async fn is_violated(&self) -> bool {
        let rate = self.success_rate().await;
        rate < self.config.target_success_rate
    }

    /// Get remaining error budget (percentage)
    pub async fn error_budget_remaining(&self) -> f64 {
        let rate = self.success_rate().await;
        let error_rate = 1.0 - rate;
        let budget_used = (error_rate / (self.config.error_budget_percent / 100.0)) * 100.0;
        (100.0 - budget_used).max(0.0)
    }
}

/// SLO-based circuit breaker integration
pub struct SLOCircuitBreakerIntegration {
    circuit_manager: Arc<CircuitBreakerManager>,
    slo_trackers: Arc<RwLock<HashMap<String, Arc<SLOTracker>>>>,
}

impl SLOCircuitBreakerIntegration {
    pub fn new(circuit_manager: Arc<CircuitBreakerManager>) -> Self {
        Self {
            circuit_manager,
            slo_trackers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create SLO tracker for endpoint
    pub async fn get_tracker(&self, endpoint: &str, config: SLOConfig) -> Arc<SLOTracker> {
        let mut trackers = self.slo_trackers.write().await;
        trackers.entry(endpoint.to_string())
            .or_insert_with(|| Arc::new(SLOTracker::new(config)))
            .clone()
    }

    /// Record request and update both SLO and circuit breaker
    pub async fn record_request(&self, endpoint: &str, success: bool, slo_config: SLOConfig) {
        let tracker = self.get_tracker(endpoint, slo_config).await;
        tracker.record(success).await;

        // If SLO is violated, trigger circuit breaker
        if tracker.is_violated().await {
            let breaker = self.circuit_manager.get_breaker(endpoint).await;
            breaker.record_failure().await;
            warn!("SLO violated for endpoint '{}', recording failure in circuit breaker", endpoint);
        }
    }

    /// Get SLO status for endpoint
    pub async fn get_slo_status(&self, endpoint: &str) -> Option<(f64, f64, bool)> {
        let trackers = self.slo_trackers.read().await;
        if let Some(tracker) = trackers.get(endpoint) {
            let success_rate = tracker.success_rate().await;
            let budget_remaining = tracker.error_budget_remaining().await;
            let violated = tracker.is_violated().await;
            Some((success_rate, budget_remaining, violated))
        } else {
            None
        }
    }
}

/// Per-user bulkhead for resource isolation
pub struct PerUserBulkhead {
    bulkheads: Arc<RwLock<HashMap<String, Arc<Bulkhead>>>>,
    default_config: BulkheadConfig,
    registry: Arc<Registry>,
}

impl PerUserBulkhead {
    pub fn new(default_config: BulkheadConfig, registry: Arc<Registry>) -> Self {
        Self {
            bulkheads: Arc::new(RwLock::new(HashMap::new())),
            default_config,
            registry,
        }
    }

    /// Get or create bulkhead for user
    pub async fn get_bulkhead(&self, user_id: &str) -> Arc<Bulkhead> {
        let bulkheads = self.bulkheads.read().await;

        if let Some(bulkhead) = bulkheads.get(user_id) {
            return bulkhead.clone();
        }

        drop(bulkheads);

        // Create new bulkhead for user
        let mut bulkheads = self.bulkheads.write().await;

        // Double-check after acquiring write lock
        if let Some(bulkhead) = bulkheads.get(user_id) {
            return bulkhead.clone();
        }

        let bulkhead = Arc::new(Bulkhead::new(self.default_config.clone()));
        bulkheads.insert(user_id.to_string(), bulkhead.clone());

        info!("Created per-user bulkhead for user: {}", user_id);
        bulkhead
    }

    /// Try to acquire slot for user
    pub async fn try_acquire(&self, user_id: &str) -> Result<BulkheadGuard, BulkheadError> {
        let bulkhead = self.get_bulkhead(user_id).await;
        bulkhead.try_acquire().await
    }

    /// Get statistics for user
    pub async fn get_user_stats(&self, user_id: &str) -> Option<BulkheadStats> {
        let bulkheads = self.bulkheads.read().await;
        if let Some(bulkhead) = bulkheads.get(user_id) {
            Some(bulkhead.stats().await)
        } else {
            None
        }
    }

    /// Get all user statistics
    pub async fn get_all_stats(&self) -> HashMap<String, BulkheadStats> {
        let bulkheads = self.bulkheads.read().await;
        let mut stats = HashMap::new();

        for (user_id, bulkhead) in bulkheads.iter() {
            stats.insert(user_id.clone(), bulkhead.stats().await);
        }

        stats
    }

    /// Remove bulkhead for user (cleanup)
    pub async fn remove_user(&self, user_id: &str) -> bool {
        let mut bulkheads = self.bulkheads.write().await;
        bulkheads.remove(user_id).is_some()
    }
}

impl Clone for PerUserBulkhead {
    fn clone(&self) -> Self {
        Self {
            bulkheads: self.bulkheads.clone(),
            default_config: self.default_config.clone(),
            registry: self.registry.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed_to_open() {
        let config = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 3,
            ..Default::default()
        };

        let cb = CircuitBreaker::new(config);

        // Initially closed
        assert_eq!(cb.state().await, CircuitState::Closed);

        // Record failures
        for _ in 0..2 {
            assert!(cb.allow_request().await);
            cb.record_failure().await;
            assert_eq!(cb.state().await, CircuitState::Closed);
        }

        // Third failure should open circuit
        assert!(cb.allow_request().await);
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);

        // Requests should be rejected
        assert!(!cb.allow_request().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_closed() {
        let config = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 2,
            success_threshold: 2,
            timeout_ms: 100,
            ..Default::default()
        };

        let cb = CircuitBreaker::new(config);

        // Open the circuit
        for _ in 0..2 {
            cb.allow_request().await;
            cb.record_failure().await;
        }
        assert_eq!(cb.state().await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should transition to half-open
        assert!(cb.allow_request().await);
        assert_eq!(cb.state().await, CircuitState::HalfOpen);

        // Record successes
        cb.record_success().await;
        assert_eq!(cb.state().await, CircuitState::HalfOpen);

        cb.allow_request().await;
        cb.record_success().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_bulkhead_basic() {
        let config = BulkheadConfig {
            enabled: true,
            max_concurrent_requests: 2,
            max_queue_size: 0,
            ..Default::default()
        };

        let bulkhead = Bulkhead::new(config);

        // Should accept first two requests
        let _guard1 = bulkhead.try_acquire().await.unwrap();
        let _guard2 = bulkhead.try_acquire().await.unwrap();

        // Third should be rejected
        assert!(matches!(
            bulkhead.try_acquire().await,
            Err(BulkheadError::Rejected)
        ));

        // Drop one guard
        drop(_guard1);

        // Should accept now
        let _guard3 = bulkhead.try_acquire().await.unwrap();
    }

    #[tokio::test]
    async fn test_bulkhead_with_queue() {
        let config = BulkheadConfig {
            enabled: true,
            max_concurrent_requests: 1,
            max_queue_size: 2,
            queue_timeout_ms: 1000,
        };

        let bulkhead = Bulkhead::new(config);

        let guard1 = bulkhead.try_acquire().await.unwrap();

        // Spawn tasks that will queue
        let bulkhead_clone = bulkhead.clone();
        let handle = tokio::spawn(async move {
            bulkhead_clone.try_acquire().await
        });

        // Small delay to ensure queuing
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Check stats
        let stats = bulkhead.stats().await;
        assert_eq!(stats.active_requests, 1);
        assert_eq!(stats.queued_requests, 1);

        // Release first guard
        drop(guard1);

        // Queued request should be accepted
        let _guard2 = handle.await.unwrap().unwrap();
    }
}
