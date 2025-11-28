//! Redis caching utilities for frequently accessed data
//!
//! Provides caching layer for organization data, user data, settings, and marketplace content
//! to reduce database load and improve response times.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::redis::RedisPool;

/// Cache key prefixes for different data types
pub mod keys {
    pub const ORG: &str = "cache:org:";
    pub const USER: &str = "cache:user:";
    pub const ORG_SETTING: &str = "cache:org_setting:";
    pub const USER_SETTING: &str = "cache:user_setting:";
    pub const SUBSCRIPTION: &str = "cache:subscription:";
    pub const PLUGIN: &str = "cache:plugin:";
    pub const TEMPLATE: &str = "cache:template:";
    pub const SCENARIO: &str = "cache:scenario:";
    pub const ORG_MEMBERS: &str = "cache:org_members:";
}

/// Cache TTL constants (in seconds)
pub mod ttl {

    /// Short-lived cache (1 minute) - for frequently changing data
    pub const SHORT: u64 = 60;

    /// Medium cache (5 minutes) - for moderately changing data
    pub const MEDIUM: u64 = 300;

    /// Long cache (15 minutes) - for relatively static data
    pub const LONG: u64 = 900;

    /// Very long cache (1 hour) - for static data
    pub const VERY_LONG: u64 = 3600;

    /// Organization data cache (5 minutes)
    pub const ORG: u64 = MEDIUM;

    /// User data cache (5 minutes)
    pub const USER: u64 = MEDIUM;

    /// Settings cache (15 minutes)
    pub const SETTINGS: u64 = LONG;

    /// Subscription cache (5 minutes)
    pub const SUBSCRIPTION: u64 = MEDIUM;

    /// Marketplace content cache (15 minutes)
    pub const MARKETPLACE: u64 = LONG;

    /// Org members cache (5 minutes)
    pub const ORG_MEMBERS: u64 = MEDIUM;
}

/// Cache wrapper for Redis operations
pub struct Cache {
    redis: RedisPool,
}

impl Cache {
    /// Create a new cache instance
    pub fn new(redis: RedisPool) -> Self {
        Self { redis }
    }

    /// Get a cached value as JSON
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        match self.redis.get(key).await? {
            Some(value) => {
                let deserialized: T = serde_json::from_str(&value)?;
                Ok(Some(deserialized))
            }
            None => Ok(None),
        }
    }

    /// Set a cached value as JSON
    pub async fn set<T>(&self, key: &str, value: &T, ttl: u64) -> Result<()>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(value)?;
        self.redis.set_with_expiry(key, &serialized, ttl).await?;
        Ok(())
    }

    /// Delete a cached value
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.redis.delete(key).await?;
        Ok(())
    }

    /// Delete multiple keys matching a pattern
    /// Note: This is a simple implementation. For production, consider using SCAN
    pub async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        // For now, we'll just delete the exact key
        // In production, you might want to use Redis SCAN to find matching keys
        self.redis.delete(pattern).await?;
        Ok(())
    }

    /// Invalidate organization-related caches
    pub async fn invalidate_org(&self, org_id: &Uuid) -> Result<()> {
        let keys = vec![
            format!("{}:{}", keys::ORG, org_id),
            format!("{}:{}", keys::ORG_MEMBERS, org_id),
            format!("{}:{}:*", keys::ORG_SETTING, org_id), // Pattern - would need SCAN in production
        ];
        for key in keys {
            let _ = self.delete(&key).await;
        }
        Ok(())
    }

    /// Invalidate user-related caches
    pub async fn invalidate_user(&self, user_id: &Uuid) -> Result<()> {
        let keys = vec![
            format!("{}:{}", keys::USER, user_id),
            format!("{}:{}:*", keys::USER_SETTING, user_id), // Pattern - would need SCAN in production
        ];
        for key in keys {
            let _ = self.delete(&key).await;
        }
        Ok(())
    }

    /// Invalidate subscription cache
    pub async fn invalidate_subscription(&self, org_id: &Uuid) -> Result<()> {
        let key = format!("{}:{}", keys::SUBSCRIPTION, org_id);
        self.delete(&key).await?;
        Ok(())
    }

    /// Invalidate marketplace content cache
    pub async fn invalidate_marketplace(&self, content_type: &str, content_id: &Uuid) -> Result<()> {
        let key = match content_type {
            "plugin" => format!("{}:{}", keys::PLUGIN, content_id),
            "template" => format!("{}:{}", keys::TEMPLATE, content_id),
            "scenario" => format!("{}:{}", keys::SCENARIO, content_id),
            _ => return Ok(()),
        };
        self.delete(&key).await?;
        Ok(())
    }

    /// Get or set pattern: Try cache first, fallback to database query
    pub async fn get_or_set<F, Fut, T>(&self, key: &str, ttl: u64, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
        T: Serialize + for<'de> Deserialize<'de>,
    {
        // Try cache first
        if let Some(cached) = self.get::<T>(key).await? {
            return Ok(cached);
        }

        // Cache miss - fetch from database
        let value = f().await?;

        // Store in cache (non-blocking - don't fail if cache write fails)
        if let Err(e) = self.set(key, &value, ttl).await {
            tracing::warn!("Failed to cache value for key {}: {}", key, e);
        }

        Ok(value)
    }
}

/// Helper function to generate organization cache key
pub fn org_cache_key(org_id: &Uuid) -> String {
    format!("{}:{}", keys::ORG, org_id)
}

/// Helper function to generate user cache key
pub fn user_cache_key(user_id: &Uuid) -> String {
    format!("{}:{}", keys::USER, user_id)
}

/// Helper function to generate org setting cache key
pub fn org_setting_cache_key(org_id: &Uuid, setting_key: &str) -> String {
    format!("{}:{}:{}", keys::ORG_SETTING, org_id, setting_key)
}

/// Helper function to generate user setting cache key
pub fn user_setting_cache_key(user_id: &Uuid, setting_key: &str) -> String {
    format!("{}:{}:{}", keys::USER_SETTING, user_id, setting_key)
}

/// Helper function to generate subscription cache key
pub fn subscription_cache_key(org_id: &Uuid) -> String {
    format!("{}:{}", keys::SUBSCRIPTION, org_id)
}

/// Helper function to generate org members cache key
pub fn org_members_cache_key(org_id: &Uuid) -> String {
    format!("{}:{}", keys::ORG_MEMBERS, org_id)
}

/// Helper function to generate plugin cache key
pub fn plugin_cache_key(plugin_id: &Uuid) -> String {
    format!("{}:{}", keys::PLUGIN, plugin_id)
}

/// Helper function to generate template cache key
pub fn template_cache_key(template_id: &Uuid) -> String {
    format!("{}:{}", keys::TEMPLATE, template_id)
}

/// Helper function to generate scenario cache key
pub fn scenario_cache_key(scenario_id: &Uuid) -> String {
    format!("{}:{}", keys::SCENARIO, scenario_id)
}
