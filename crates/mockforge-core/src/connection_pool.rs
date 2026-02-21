use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, warn};

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections
    pub max_connections: usize,
    /// Minimum number of idle connections to maintain
    pub min_idle: usize,
    /// Maximum idle time before connection is closed
    pub max_idle_time: Duration,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Enable connection health checks
    pub health_check_enabled: bool,
    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            min_idle: 10,
            max_idle_time: Duration::from_secs(600), // 10 minutes
            connection_timeout: Duration::from_secs(30),
            health_check_enabled: true,
            health_check_interval: Duration::from_secs(60),
        }
    }
}

/// Connection wrapper with metadata
pub struct PooledConnection<T> {
    inner: T,
    created_at: Instant,
    last_used: Instant,
}

impl<T> PooledConnection<T> {
    /// Creates a new pooled connection wrapper
    pub fn new(connection: T) -> Self {
        let now = Instant::now();
        Self {
            inner: connection,
            created_at: now,
            last_used: now,
        }
    }

    /// Gets a reference to the underlying connection
    pub fn get(&self) -> &T {
        &self.inner
    }

    /// Gets a mutable reference to the underlying connection and updates last used time
    pub fn get_mut(&mut self) -> &mut T {
        self.last_used = Instant::now();
        &mut self.inner
    }

    /// Checks if the connection is stale based on idle time
    pub fn is_stale(&self, max_idle_time: Duration) -> bool {
        self.last_used.elapsed() > max_idle_time
    }

    /// Returns the age of the connection since creation
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// Generic connection pool
pub struct ConnectionPool<T> {
    config: PoolConfig,
    available: Arc<RwLock<Vec<PooledConnection<T>>>>,
    semaphore: Arc<Semaphore>,
    metrics: Arc<RwLock<PoolMetrics>>,
}

/// Metrics for connection pool usage and health
#[derive(Debug, Default, Clone)]
pub struct PoolMetrics {
    /// Number of currently active connections
    pub active_connections: usize,
    /// Number of idle connections available
    pub idle_connections: usize,
    /// Total number of connection acquisitions
    pub total_acquired: u64,
    /// Total number of connection releases
    pub total_released: u64,
    /// Total number of connections created
    pub total_created: u64,
    /// Total number of connections closed
    pub total_closed: u64,
    /// Number of acquire timeouts
    pub acquire_timeouts: u64,
    /// Number of health check failures
    pub health_check_failures: u64,
}

impl<T> ConnectionPool<T>
where
    T: Send + 'static,
{
    /// Creates a new connection pool with the given configuration
    pub fn new(config: PoolConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.max_connections)),
            available: Arc::new(RwLock::new(Vec::with_capacity(config.max_connections))),
            metrics: Arc::new(RwLock::new(PoolMetrics::default())),
            config,
        }
    }

    /// Acquire a connection from the pool
    pub async fn acquire<F, Fut>(&self, create_fn: F) -> Result<PooledConnection<T>, PoolError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, PoolError>>,
    {
        // Wait for available slot
        let permit = tokio::time::timeout(
            self.config.connection_timeout,
            self.semaphore.clone().acquire_owned(),
        )
        .await
        .map_err(|_| {
            debug!("Connection pool acquire timeout");
            PoolError::Timeout
        })?
        .map_err(|_| PoolError::Closed)?;

        // Try to get an existing connection
        let mut available = self.available.write().await;

        // Remove stale connections
        available.retain(|conn| !conn.is_stale(self.config.max_idle_time));

        let connection = if let Some(mut conn) = available.pop() {
            // Reuse existing connection
            conn.last_used = Instant::now();
            drop(available);

            let mut metrics = self.metrics.write().await;
            metrics.total_acquired += 1;
            metrics.active_connections += 1;
            metrics.idle_connections = metrics.idle_connections.saturating_sub(1);
            drop(metrics);

            debug!("Reusing pooled connection");
            conn
        } else {
            drop(available);

            // Create new connection
            let inner = create_fn().await?;
            let conn = PooledConnection::new(inner);

            let mut metrics = self.metrics.write().await;
            metrics.total_created += 1;
            metrics.total_acquired += 1;
            metrics.active_connections += 1;
            drop(metrics);

            debug!("Created new pooled connection");
            conn
        };

        // Permit will be returned when connection is released
        std::mem::forget(permit);

        Ok(connection)
    }

    /// Release a connection back to the pool
    pub async fn release(&self, connection: PooledConnection<T>) {
        let mut available = self.available.write().await;

        // Don't return to pool if we're above max idle or connection is stale
        if available.len() >= self.config.min_idle && connection.is_stale(self.config.max_idle_time)
        {
            drop(available);

            let mut metrics = self.metrics.write().await;
            metrics.total_closed += 1;
            metrics.active_connections = metrics.active_connections.saturating_sub(1);
            drop(metrics);

            self.semaphore.add_permits(1);
            debug!("Closed stale connection");
            return;
        }

        available.push(connection);
        drop(available);

        let mut metrics = self.metrics.write().await;
        metrics.total_released += 1;
        metrics.active_connections = metrics.active_connections.saturating_sub(1);
        metrics.idle_connections += 1;
        drop(metrics);

        self.semaphore.add_permits(1);
        debug!("Released connection to pool");
    }

    /// Get current pool metrics
    pub async fn metrics(&self) -> PoolMetrics {
        self.metrics.read().await.clone()
    }

    /// Get current pool size
    pub async fn size(&self) -> usize {
        self.available.read().await.len()
    }

    /// Run health checks on idle connections
    pub async fn health_check<F, Fut>(&self, check_fn: F)
    where
        F: Fn(&T) -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        if !self.config.health_check_enabled {
            return;
        }

        let mut available = self.available.write().await;
        let mut healthy = Vec::new();
        let mut failures = 0;

        for conn in available.drain(..) {
            if check_fn(conn.get()).await {
                healthy.push(conn);
            } else {
                failures += 1;
                warn!("Connection failed health check");
            }
        }

        *available = healthy;
        drop(available);

        if failures > 0 {
            let mut metrics = self.metrics.write().await;
            metrics.health_check_failures += failures;
            metrics.total_closed += failures;
            metrics.idle_connections = metrics.idle_connections.saturating_sub(failures as usize);
            drop(metrics);

            self.semaphore.add_permits(failures as usize);
        }
    }

    /// Maintain minimum idle connections
    pub async fn maintain_idle<F, Fut>(&self, create_fn: F)
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, PoolError>>,
    {
        let current_idle = self.available.read().await.len();

        if current_idle < self.config.min_idle {
            let needed = self.config.min_idle - current_idle;

            for _ in 0..needed {
                if let Ok(permit) = self.semaphore.clone().try_acquire_owned() {
                    match create_fn().await {
                        Ok(conn) => {
                            let pooled = PooledConnection::new(conn);
                            self.available.write().await.push(pooled);

                            let mut metrics = self.metrics.write().await;
                            metrics.total_created += 1;
                            metrics.idle_connections += 1;

                            std::mem::forget(permit);
                        }
                        Err(e) => {
                            warn!("Failed to create idle connection: {:?}", e);
                            drop(permit);
                        }
                    }
                } else {
                    break;
                }
            }
        }
    }
}

/// Errors that can occur in connection pool operations
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    /// Connection acquisition timed out
    #[error("Connection pool timeout")]
    Timeout,

    /// Connection pool has been closed
    #[error("Connection pool closed")]
    Closed,

    /// Failed to create a new connection
    #[error("Failed to create connection: {0}")]
    CreateError(String),

    /// Error during connection operation
    #[error("Connection error: {0}")]
    ConnectionError(String),
}

/// HTTP client connection pool (example usage)
pub type HttpClientPool = ConnectionPool<reqwest::Client>;

impl HttpClientPool {
    /// Creates a new HTTP client pool with the given configuration
    pub fn new_http(config: PoolConfig) -> Self {
        Self::new(config)
    }

    /// Acquires an HTTP client from the pool
    pub async fn acquire_client(&self) -> Result<PooledConnection<reqwest::Client>, PoolError> {
        self.acquire(|| async {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .pool_max_idle_per_host(10)
                .build()
                .map_err(|e| PoolError::CreateError(e.to_string()))
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // PoolConfig tests
    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.min_idle, 10);
        assert_eq!(config.max_idle_time, Duration::from_secs(600));
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
        assert!(config.health_check_enabled);
        assert_eq!(config.health_check_interval, Duration::from_secs(60));
    }

    #[test]
    fn test_pool_config_clone() {
        let config = PoolConfig {
            max_connections: 50,
            min_idle: 5,
            ..Default::default()
        };
        let cloned = config.clone();
        assert_eq!(cloned.max_connections, config.max_connections);
        assert_eq!(cloned.min_idle, config.min_idle);
    }

    #[test]
    fn test_pool_config_debug() {
        let config = PoolConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("PoolConfig"));
        assert!(debug.contains("max_connections"));
    }

    // PooledConnection tests
    #[test]
    fn test_pooled_connection_new() {
        let conn = PooledConnection::new(42u32);
        assert_eq!(*conn.get(), 42);
    }

    #[test]
    fn test_pooled_connection_get() {
        let conn = PooledConnection::new("test".to_string());
        assert_eq!(*conn.get(), "test");
    }

    #[test]
    fn test_pooled_connection_get_mut() {
        let mut conn = PooledConnection::new(vec![1, 2, 3]);
        conn.get_mut().push(4);
        assert_eq!(*conn.get(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_pooled_connection_is_stale() {
        let conn = PooledConnection::new(42u32);
        // Just created, should not be stale
        assert!(!conn.is_stale(Duration::from_secs(1)));
        // With zero duration, should be stale immediately
        assert!(conn.is_stale(Duration::from_nanos(0)));
    }

    #[test]
    fn test_pooled_connection_age() {
        let conn = PooledConnection::new(42u32);
        let age = conn.age();
        // Should be very small (just created)
        assert!(age < Duration::from_secs(1));
    }

    // PoolMetrics tests
    #[test]
    fn test_pool_metrics_default() {
        let metrics = PoolMetrics::default();
        assert_eq!(metrics.active_connections, 0);
        assert_eq!(metrics.idle_connections, 0);
        assert_eq!(metrics.total_acquired, 0);
        assert_eq!(metrics.total_released, 0);
        assert_eq!(metrics.total_created, 0);
        assert_eq!(metrics.total_closed, 0);
        assert_eq!(metrics.acquire_timeouts, 0);
        assert_eq!(metrics.health_check_failures, 0);
    }

    #[test]
    fn test_pool_metrics_clone() {
        let mut metrics = PoolMetrics::default();
        metrics.total_acquired = 10;
        metrics.active_connections = 5;
        let cloned = metrics.clone();
        assert_eq!(cloned.total_acquired, 10);
        assert_eq!(cloned.active_connections, 5);
    }

    #[test]
    fn test_pool_metrics_debug() {
        let metrics = PoolMetrics::default();
        let debug = format!("{:?}", metrics);
        assert!(debug.contains("PoolMetrics"));
        assert!(debug.contains("active_connections"));
    }

    // PoolError tests
    #[test]
    fn test_pool_error_timeout() {
        let error = PoolError::Timeout;
        assert!(error.to_string().contains("timeout"));
    }

    #[test]
    fn test_pool_error_closed() {
        let error = PoolError::Closed;
        assert!(error.to_string().contains("closed"));
    }

    #[test]
    fn test_pool_error_create_error() {
        let error = PoolError::CreateError("connection failed".to_string());
        let msg = error.to_string();
        assert!(msg.contains("create connection"));
        assert!(msg.contains("connection failed"));
    }

    #[test]
    fn test_pool_error_connection_error() {
        let error = PoolError::ConnectionError("network issue".to_string());
        let msg = error.to_string();
        assert!(msg.contains("Connection error"));
        assert!(msg.contains("network issue"));
    }

    #[test]
    fn test_pool_error_debug() {
        let error = PoolError::Timeout;
        let debug = format!("{:?}", error);
        assert!(debug.contains("Timeout"));
    }

    // ConnectionPool tests
    #[tokio::test]
    async fn test_connection_pool() {
        let config = PoolConfig {
            max_connections: 5,
            min_idle: 2,
            ..Default::default()
        };

        let pool = ConnectionPool::<u32>::new(config);

        // Acquire connection
        let conn1 = pool.acquire(|| async { Ok(42) }).await.unwrap();

        assert_eq!(*conn1.get(), 42);

        // Release connection
        pool.release(conn1).await;

        // Verify metrics
        let metrics = pool.metrics().await;
        assert_eq!(metrics.total_created, 1);
        assert_eq!(metrics.total_acquired, 1);
        assert_eq!(metrics.total_released, 1);
    }

    #[tokio::test]
    async fn test_connection_pool_new() {
        let config = PoolConfig {
            max_connections: 10,
            min_idle: 2,
            ..Default::default()
        };
        let pool = ConnectionPool::<u32>::new(config);

        // Pool should start empty
        assert_eq!(pool.size().await, 0);
    }

    #[tokio::test]
    async fn test_connection_pool_acquire_creates_connection() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::<String>::new(config);

        let conn = pool.acquire(|| async { Ok("test-connection".to_string()) }).await.unwrap();

        assert_eq!(*conn.get(), "test-connection");

        let metrics = pool.metrics().await;
        assert_eq!(metrics.total_created, 1);
        assert_eq!(metrics.total_acquired, 1);
    }

    #[tokio::test]
    async fn test_connection_pool_reuses_connection() {
        let config = PoolConfig {
            max_connections: 5,
            min_idle: 1,
            ..Default::default()
        };
        let pool = ConnectionPool::<u32>::new(config);
        let create_count = Arc::new(AtomicUsize::new(0));

        // First acquire - creates connection
        let create_count_clone = create_count.clone();
        let conn1 = pool
            .acquire(move || {
                let count = create_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(42u32)
                }
            })
            .await
            .unwrap();

        // Release it
        pool.release(conn1).await;

        // Second acquire - should reuse
        let create_count_clone = create_count.clone();
        let conn2 = pool
            .acquire(move || {
                let count = create_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(100u32)
                }
            })
            .await
            .unwrap();

        // Should still be the original connection (value 42), not a new one
        assert_eq!(*conn2.get(), 42);
        assert_eq!(create_count.load(Ordering::SeqCst), 1); // Only created once

        let metrics = pool.metrics().await;
        assert_eq!(metrics.total_created, 1);
        assert_eq!(metrics.total_acquired, 2);
    }

    #[tokio::test]
    async fn test_connection_pool_release() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::<u32>::new(config);

        let conn = pool.acquire(|| async { Ok(42) }).await.unwrap();
        assert_eq!(pool.size().await, 0); // Connection in use, not in pool

        pool.release(conn).await;
        assert_eq!(pool.size().await, 1); // Connection returned to pool

        let metrics = pool.metrics().await;
        assert_eq!(metrics.total_released, 1);
        assert_eq!(metrics.idle_connections, 1);
    }

    #[tokio::test]
    async fn test_connection_pool_metrics() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::<u32>::new(config);

        // Initial metrics
        let metrics = pool.metrics().await;
        assert_eq!(metrics.total_created, 0);

        // Acquire and release
        let conn = pool.acquire(|| async { Ok(1) }).await.unwrap();
        pool.release(conn).await;

        let metrics = pool.metrics().await;
        assert_eq!(metrics.total_created, 1);
        assert_eq!(metrics.total_acquired, 1);
        assert_eq!(metrics.total_released, 1);
    }

    #[tokio::test]
    async fn test_connection_pool_size() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::<u32>::new(config);

        assert_eq!(pool.size().await, 0);

        let conn1 = pool.acquire(|| async { Ok(1) }).await.unwrap();
        let conn2 = pool.acquire(|| async { Ok(2) }).await.unwrap();

        // Connections in use, pool is empty
        assert_eq!(pool.size().await, 0);

        pool.release(conn1).await;
        assert_eq!(pool.size().await, 1);

        pool.release(conn2).await;
        assert_eq!(pool.size().await, 2);
    }

    #[tokio::test]
    async fn test_connection_pool_multiple_concurrent_acquires() {
        let config = PoolConfig {
            max_connections: 10,
            ..Default::default()
        };
        let pool = Arc::new(ConnectionPool::<u32>::new(config));

        let mut handles = vec![];
        for i in 0..5 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                let conn = pool_clone.acquire(move || async move { Ok(i as u32) }).await.unwrap();
                // Simulate some work
                tokio::time::sleep(Duration::from_millis(10)).await;
                pool_clone.release(conn).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let metrics = pool.metrics().await;
        assert_eq!(metrics.total_acquired, 5);
        assert_eq!(metrics.total_released, 5);
    }

    #[tokio::test]
    async fn test_connection_pool_acquire_error() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::<u32>::new(config);

        let result = pool
            .acquire(|| async { Err(PoolError::CreateError("test error".to_string())) })
            .await;

        assert!(result.is_err());
        if let Err(PoolError::CreateError(msg)) = result {
            assert_eq!(msg, "test error");
        }
    }

    #[tokio::test]
    async fn test_connection_pool_health_check() {
        let config = PoolConfig {
            max_connections: 5,
            min_idle: 0,
            health_check_enabled: true,
            ..Default::default()
        };
        let pool = ConnectionPool::<u32>::new(config);

        // Add some connections to pool
        let conn1 = pool.acquire(|| async { Ok(1) }).await.unwrap();
        let conn2 = pool.acquire(|| async { Ok(2) }).await.unwrap();
        pool.release(conn1).await;
        pool.release(conn2).await;

        // Health check - all connections pass
        pool.health_check(|_| async { true }).await;

        assert_eq!(pool.size().await, 2);

        // Health check - all connections fail
        pool.health_check(|_| async { false }).await;

        assert_eq!(pool.size().await, 0);

        let metrics = pool.metrics().await;
        assert_eq!(metrics.health_check_failures, 2);
    }

    #[tokio::test]
    async fn test_connection_pool_health_check_disabled() {
        let config = PoolConfig {
            health_check_enabled: false,
            ..Default::default()
        };
        let pool = ConnectionPool::<u32>::new(config);

        let conn = pool.acquire(|| async { Ok(1) }).await.unwrap();
        pool.release(conn).await;

        // Health check should do nothing when disabled
        pool.health_check(|_| async { false }).await;

        // Connection should still be there
        assert_eq!(pool.size().await, 1);
    }

    #[tokio::test]
    async fn test_connection_pool_maintain_idle() {
        let config = PoolConfig {
            max_connections: 10,
            min_idle: 3,
            ..Default::default()
        };
        let pool = ConnectionPool::<u32>::new(config);

        // Pool starts empty
        assert_eq!(pool.size().await, 0);

        // Maintain idle should create min_idle connections
        pool.maintain_idle(|| async { Ok(42u32) }).await;

        assert_eq!(pool.size().await, 3);

        let metrics = pool.metrics().await;
        assert_eq!(metrics.total_created, 3);
        assert_eq!(metrics.idle_connections, 3);
    }

    #[tokio::test]
    async fn test_connection_pool_maintain_idle_already_sufficient() {
        let config = PoolConfig {
            max_connections: 10,
            min_idle: 2,
            ..Default::default()
        };
        let pool = ConnectionPool::<u32>::new(config);

        // Manually add 3 connections
        let conn1 = pool.acquire(|| async { Ok(1) }).await.unwrap();
        let conn2 = pool.acquire(|| async { Ok(2) }).await.unwrap();
        let conn3 = pool.acquire(|| async { Ok(3) }).await.unwrap();
        pool.release(conn1).await;
        pool.release(conn2).await;
        pool.release(conn3).await;

        let initial_created = pool.metrics().await.total_created;

        // Maintain idle should not create more since we have 3 > min_idle(2)
        pool.maintain_idle(|| async { Ok(100u32) }).await;

        let final_created = pool.metrics().await.total_created;
        assert_eq!(initial_created, final_created);
    }

    #[tokio::test]
    async fn test_connection_pool_maintain_idle_error() {
        let config = PoolConfig {
            max_connections: 10,
            min_idle: 3,
            ..Default::default()
        };
        let pool = ConnectionPool::<u32>::new(config);

        // maintain_idle with failing create function
        pool.maintain_idle(|| async { Err(PoolError::CreateError("test".to_string())) })
            .await;

        // Pool should still be empty
        assert_eq!(pool.size().await, 0);
    }

    // HttpClientPool tests
    #[tokio::test]
    async fn test_http_client_pool_new() {
        let config = PoolConfig::default();
        let pool = HttpClientPool::new_http(config);
        assert_eq!(pool.size().await, 0);
    }

    #[tokio::test]
    async fn test_http_client_pool_acquire() {
        let config = PoolConfig::default();
        let pool = HttpClientPool::new_http(config);

        let result = pool.acquire_client().await;
        assert!(result.is_ok());

        let conn = result.unwrap();
        // Verify it's a valid reqwest client
        let _client: &reqwest::Client = conn.get();
    }

    // Edge cases
    #[tokio::test]
    async fn test_connection_pool_stale_connection_not_returned() {
        let config = PoolConfig {
            max_connections: 5,
            min_idle: 0, // Set to 0 so stale connections get closed
            max_idle_time: Duration::from_millis(1), // Very short idle time
            ..Default::default()
        };
        let pool = ConnectionPool::<u32>::new(config);

        let conn = pool.acquire(|| async { Ok(42) }).await.unwrap();

        // Wait for connection to become stale
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Release stale connection
        pool.release(conn).await;

        // Stale connection should be closed, not returned to pool
        let metrics = pool.metrics().await;
        assert_eq!(metrics.total_closed, 1);
    }

    #[tokio::test]
    async fn test_connection_pool_with_complex_type() {
        #[derive(Debug, Clone)]
        struct ComplexConnection {
            id: u32,
            data: Vec<String>,
        }

        let config = PoolConfig::default();
        let pool = ConnectionPool::<ComplexConnection>::new(config);

        let conn = pool
            .acquire(|| async {
                Ok(ComplexConnection {
                    id: 123,
                    data: vec!["test".to_string()],
                })
            })
            .await
            .unwrap();

        assert_eq!(conn.get().id, 123);
        assert_eq!(conn.get().data, vec!["test".to_string()]);
    }

    #[tokio::test]
    async fn test_pooled_connection_updates_last_used() {
        let mut conn = PooledConnection::new(42u32);
        let initial_time = conn.last_used;

        // Sleep a tiny bit
        tokio::time::sleep(Duration::from_millis(1)).await;

        // get_mut should update last_used
        let _ = conn.get_mut();

        assert!(conn.last_used > initial_time);
    }

    #[tokio::test]
    async fn test_connection_pool_partial_health_check() {
        let config = PoolConfig {
            max_connections: 10,
            min_idle: 0,
            health_check_enabled: true,
            ..Default::default()
        };
        let pool = ConnectionPool::<u32>::new(config);

        // Add connections with different values
        let conn1 = pool.acquire(|| async { Ok(1) }).await.unwrap();
        let conn2 = pool.acquire(|| async { Ok(2) }).await.unwrap();
        let conn3 = pool.acquire(|| async { Ok(3) }).await.unwrap();
        pool.release(conn1).await;
        pool.release(conn2).await;
        pool.release(conn3).await;

        // Health check that fails only even numbers
        pool.health_check(|val| {
            let v = *val;
            async move { v % 2 != 0 }
        })
        .await;

        // Only odd-valued connections should remain
        assert_eq!(pool.size().await, 2); // 1 and 3

        let metrics = pool.metrics().await;
        assert_eq!(metrics.health_check_failures, 1); // Connection with value 2
    }
}
