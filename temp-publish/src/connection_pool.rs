use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, RwLock};
use std::collections::HashMap;
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
    pub fn new(connection: T) -> Self {
        let now = Instant::now();
        Self {
            inner: connection,
            created_at: now,
            last_used: now,
        }
    }

    pub fn get(&self) -> &T {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.last_used = Instant::now();
        &mut self.inner
    }

    pub fn is_stale(&self, max_idle_time: Duration) -> bool {
        self.last_used.elapsed() > max_idle_time
    }

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

#[derive(Debug, Default, Clone)]
pub struct PoolMetrics {
    pub active_connections: usize,
    pub idle_connections: usize,
    pub total_acquired: u64,
    pub total_released: u64,
    pub total_created: u64,
    pub total_closed: u64,
    pub acquire_timeouts: u64,
    pub health_check_failures: u64,
}

impl<T> ConnectionPool<T>
where
    T: Send + 'static,
{
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
        if available.len() >= self.config.min_idle
            && connection.is_stale(self.config.max_idle_time)
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

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Connection pool timeout")]
    Timeout,

    #[error("Connection pool closed")]
    Closed,

    #[error("Failed to create connection: {0}")]
    CreateError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),
}

/// HTTP client connection pool (example usage)
pub type HttpClientPool = ConnectionPool<reqwest::Client>;

impl HttpClientPool {
    pub fn new_http(config: PoolConfig) -> Self {
        Self::new(config)
    }

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

    #[tokio::test]
    async fn test_connection_pool() {
        let config = PoolConfig {
            max_connections: 5,
            min_idle: 2,
            ..Default::default()
        };

        let pool = ConnectionPool::<u32>::new(config);

        // Acquire connection
        let conn1 = pool
            .acquire(|| async { Ok(42) })
            .await
            .unwrap();

        assert_eq!(*conn1.get(), 42);

        // Release connection
        pool.release(conn1).await;

        // Verify metrics
        let metrics = pool.metrics().await;
        assert_eq!(metrics.total_created, 1);
        assert_eq!(metrics.total_acquired, 1);
        assert_eq!(metrics.total_released, 1);
    }
}
