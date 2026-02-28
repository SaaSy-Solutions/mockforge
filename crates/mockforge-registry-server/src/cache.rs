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

    /// Delete multiple keys matching a glob pattern using Redis SCAN
    pub async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        let keys = self.redis.scan_keys(pattern).await?;
        for key in keys {
            let _ = self.redis.delete(&key).await;
        }
        Ok(())
    }

    /// Invalidate organization-related caches
    pub async fn invalidate_org(&self, org_id: &Uuid) -> Result<()> {
        let _ = self.delete(&format!("{}:{}", keys::ORG, org_id)).await;
        let _ = self.delete(&format!("{}:{}", keys::ORG_MEMBERS, org_id)).await;
        let _ = self.delete_pattern(&format!("{}:{}:*", keys::ORG_SETTING, org_id)).await;
        Ok(())
    }

    /// Invalidate user-related caches
    pub async fn invalidate_user(&self, user_id: &Uuid) -> Result<()> {
        let _ = self.delete(&format!("{}:{}", keys::USER, user_id)).await;
        let _ = self.delete_pattern(&format!("{}:{}:*", keys::USER_SETTING, user_id)).await;
        Ok(())
    }

    /// Invalidate subscription cache
    pub async fn invalidate_subscription(&self, org_id: &Uuid) -> Result<()> {
        let key = format!("{}:{}", keys::SUBSCRIPTION, org_id);
        self.delete(&key).await?;
        Ok(())
    }

    /// Invalidate marketplace content cache
    pub async fn invalidate_marketplace(
        &self,
        content_type: &str,
        content_id: &Uuid,
    ) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    // Cache key prefix constants tests
    #[test]
    fn test_key_constants() {
        assert_eq!(keys::ORG, "cache:org:");
        assert_eq!(keys::USER, "cache:user:");
        assert_eq!(keys::ORG_SETTING, "cache:org_setting:");
        assert_eq!(keys::USER_SETTING, "cache:user_setting:");
        assert_eq!(keys::SUBSCRIPTION, "cache:subscription:");
        assert_eq!(keys::PLUGIN, "cache:plugin:");
        assert_eq!(keys::TEMPLATE, "cache:template:");
        assert_eq!(keys::SCENARIO, "cache:scenario:");
        assert_eq!(keys::ORG_MEMBERS, "cache:org_members:");
    }

    // TTL constants tests
    #[test]
    fn test_ttl_short() {
        assert_eq!(ttl::SHORT, 60);
    }

    #[test]
    fn test_ttl_medium() {
        assert_eq!(ttl::MEDIUM, 300);
    }

    #[test]
    fn test_ttl_long() {
        assert_eq!(ttl::LONG, 900);
    }

    #[test]
    fn test_ttl_very_long() {
        assert_eq!(ttl::VERY_LONG, 3600);
    }

    #[test]
    fn test_ttl_org() {
        assert_eq!(ttl::ORG, ttl::MEDIUM);
    }

    #[test]
    fn test_ttl_user() {
        assert_eq!(ttl::USER, ttl::MEDIUM);
    }

    #[test]
    fn test_ttl_settings() {
        assert_eq!(ttl::SETTINGS, ttl::LONG);
    }

    #[test]
    fn test_ttl_subscription() {
        assert_eq!(ttl::SUBSCRIPTION, ttl::MEDIUM);
    }

    #[test]
    fn test_ttl_marketplace() {
        assert_eq!(ttl::MARKETPLACE, ttl::LONG);
    }

    #[test]
    fn test_ttl_org_members() {
        assert_eq!(ttl::ORG_MEMBERS, ttl::MEDIUM);
    }

    // Cache key helper function tests
    #[test]
    fn test_org_cache_key() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let key = org_cache_key(&id);
        assert_eq!(key, "cache:org::550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_user_cache_key() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
        let key = user_cache_key(&id);
        assert_eq!(key, "cache:user::550e8400-e29b-41d4-a716-446655440001");
    }

    #[test]
    fn test_org_setting_cache_key() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap();
        let key = org_setting_cache_key(&id, "theme");
        assert_eq!(key, "cache:org_setting::550e8400-e29b-41d4-a716-446655440002:theme");
    }

    #[test]
    fn test_user_setting_cache_key() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440003").unwrap();
        let key = user_setting_cache_key(&id, "notifications");
        assert_eq!(key, "cache:user_setting::550e8400-e29b-41d4-a716-446655440003:notifications");
    }

    #[test]
    fn test_subscription_cache_key() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440004").unwrap();
        let key = subscription_cache_key(&id);
        assert_eq!(key, "cache:subscription::550e8400-e29b-41d4-a716-446655440004");
    }

    #[test]
    fn test_org_members_cache_key() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440005").unwrap();
        let key = org_members_cache_key(&id);
        assert_eq!(key, "cache:org_members::550e8400-e29b-41d4-a716-446655440005");
    }

    #[test]
    fn test_plugin_cache_key() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440006").unwrap();
        let key = plugin_cache_key(&id);
        assert_eq!(key, "cache:plugin::550e8400-e29b-41d4-a716-446655440006");
    }

    #[test]
    fn test_template_cache_key() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440007").unwrap();
        let key = template_cache_key(&id);
        assert_eq!(key, "cache:template::550e8400-e29b-41d4-a716-446655440007");
    }

    #[test]
    fn test_scenario_cache_key() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440008").unwrap();
        let key = scenario_cache_key(&id);
        assert_eq!(key, "cache:scenario::550e8400-e29b-41d4-a716-446655440008");
    }

    // Test key uniqueness
    #[test]
    fn test_cache_keys_are_unique() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let org_key = org_cache_key(&id);
        let user_key = user_cache_key(&id);
        let plugin_key = plugin_cache_key(&id);
        let template_key = template_cache_key(&id);
        let scenario_key = scenario_cache_key(&id);
        let subscription_key = subscription_cache_key(&id);

        // All keys should be different even for same UUID
        assert_ne!(org_key, user_key);
        assert_ne!(org_key, plugin_key);
        assert_ne!(plugin_key, template_key);
        assert_ne!(template_key, scenario_key);
        assert_ne!(subscription_key, org_key);
    }

    // Test setting key variations
    #[test]
    fn test_setting_keys_with_different_settings() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let theme_key = org_setting_cache_key(&id, "theme");
        let lang_key = org_setting_cache_key(&id, "language");

        assert_ne!(theme_key, lang_key);
        assert!(theme_key.contains("theme"));
        assert!(lang_key.contains("language"));
    }
}
