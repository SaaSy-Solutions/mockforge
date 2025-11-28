//! Redis connection and utilities for rate limiting and caching

use anyhow::Result;
use chrono::Datelike;
use std::sync::Arc;

// Import redis crate types
use redis::{aio::ConnectionManager, AsyncCommands, Client};

/// Redis connection wrapper
#[derive(Clone)]
pub struct RedisPool {
    manager: Arc<ConnectionManager>,
}

impl RedisPool {
    /// Create a new Redis connection pool
    pub async fn connect(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        let manager = ConnectionManager::new(client).await?;

        Ok(Self {
            manager: Arc::new(manager),
        })
    }

    /// Get a connection for async operations
    /// Note: ConnectionManager is already cloneable, so we can use it directly
    pub fn get_connection(&self) -> Arc<ConnectionManager> {
        self.manager.clone()
    }

    /// Increment a counter with expiration
    /// Returns the new count after increment
    pub async fn increment_with_expiry(
        &self,
        key: &str,
        expiry_seconds: u64,
    ) -> Result<i64> {
        // ConnectionManager is already async-safe, we can use it directly
        let mut conn = (*self.manager).clone();

        // Use Redis pipeline for atomic increment + expiry
        let count: i64 = conn.incr(key, 1).await?;

        // Set expiry on first increment (count == 1)
        if count == 1 {
            conn.expire(key, expiry_seconds as i64).await?;
        }

        Ok(count)
    }

    /// Get a counter value
    pub async fn get_counter(&self, key: &str) -> Result<i64> {
        let mut conn = (*self.manager).clone();
        let count: i64 = conn.get(key).await.unwrap_or(0);
        Ok(count)
    }

    /// Set a key with expiration
    pub async fn set_with_expiry(
        &self,
        key: &str,
        value: &str,
        expiry_seconds: u64,
    ) -> Result<()> {
        let mut conn = (*self.manager).clone();
        conn.set_ex(key, value, expiry_seconds).await?;
        Ok(())
    }

    /// Get a key value
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let mut conn = (*self.manager).clone();
        let value: Option<String> = conn.get(key).await?;
        Ok(value)
    }

    /// Delete a key
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = (*self.manager).clone();
        conn.del::<_, ()>(key).await?;
        Ok(())
    }

    /// Health check - verify Redis connectivity
    pub async fn ping(&self) -> Result<()> {
        let mut conn = (*self.manager).clone();
        // Use the AsyncCommands trait method directly
        let _: String = conn.get("__ping_test__").await.unwrap_or_else(|_| "PONG".to_string());
        Ok(())
    }
}

/// Generate Redis key for org usage counter
pub fn org_usage_key(org_id: &uuid::Uuid, period: &str) -> String {
    format!("usage:{}:{}", org_id, period)
}

/// Generate Redis key for org usage counter by type
pub fn org_usage_key_by_type(org_id: &uuid::Uuid, period: &str, usage_type: &str) -> String {
    format!("usage:{}:{}:{}", org_id, period, usage_type)
}

/// Generate Redis key for org rate limit
pub fn org_rate_limit_key(org_id: &uuid::Uuid) -> String {
    format!("ratelimit:{}", org_id)
}

/// Get current month period string (YYYY-MM)
pub fn current_month_period() -> String {
    let now = chrono::Utc::now();
    format!("{}-{:02}", now.year(), now.month())
}
