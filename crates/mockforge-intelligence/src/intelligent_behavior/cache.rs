//! Response caching for intelligent behavior
//!
//! This module provides a simple cache for LLM responses to improve performance
//! and reduce API costs.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cache entry with TTL
#[derive(Clone)]
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
    ttl: Duration,
}

impl<V> CacheEntry<V> {
    fn new(value: V, ttl: Duration) -> Self {
        Self {
            value,
            inserted_at: Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

/// Simple TTL-based cache for responses
pub struct ResponseCache {
    /// Cache storage
    storage: Arc<RwLock<HashMap<String, CacheEntry<serde_json::Value>>>>,

    /// Default TTL
    default_ttl: Duration,
}

impl ResponseCache {
    /// Create a new response cache
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Get a value from cache
    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        let storage = self.storage.read().await;

        if let Some(entry) = storage.get(key) {
            if !entry.is_expired() {
                return Some(entry.value.clone());
            }
        }

        None
    }

    /// Put a value in cache
    pub async fn put(&self, key: String, value: serde_json::Value) {
        let mut storage = self.storage.write().await;
        storage.insert(key, CacheEntry::new(value, self.default_ttl));
    }

    /// Put a value with custom TTL
    pub async fn put_with_ttl(&self, key: String, value: serde_json::Value, ttl: Duration) {
        let mut storage = self.storage.write().await;
        storage.insert(key, CacheEntry::new(value, ttl));
    }

    /// Remove a value from cache
    pub async fn remove(&self, key: &str) -> Option<serde_json::Value> {
        let mut storage = self.storage.write().await;
        storage.remove(key).map(|entry| entry.value)
    }

    /// Clear all expired entries
    pub async fn cleanup_expired(&self) -> usize {
        let mut storage = self.storage.write().await;

        let expired_keys: Vec<String> = storage
            .iter()
            .filter(|(_, entry)| entry.is_expired())
            .map(|(key, _)| key.clone())
            .collect();

        let count = expired_keys.len();
        for key in expired_keys {
            storage.remove(&key);
        }

        count
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        let mut storage = self.storage.write().await;
        storage.clear();
    }

    /// Get cache size
    pub async fn size(&self) -> usize {
        let storage = self.storage.read().await;
        storage.len()
    }
}

/// Generate a cache key from method, path, and request body
pub fn generate_cache_key(method: &str, path: &str, body: Option<&serde_json::Value>) -> String {
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    method.hash(&mut hasher);
    path.hash(&mut hasher);

    if let Some(body) = body {
        if let Ok(json_str) = serde_json::to_string(body) {
            json_str.hash(&mut hasher);
        }
    }

    format!("{}:{}:{:x}", method, path, hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_cache_get_put() {
        let cache = ResponseCache::new(60);

        let value = json!({"message": "test"});
        cache.put("test_key".to_string(), value.clone()).await;

        let retrieved = cache.get("test_key").await;
        assert_eq!(retrieved, Some(value));
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = ResponseCache::new(1); // 1 second TTL

        let value = json!({"message": "test"});
        cache.put("test_key".to_string(), value.clone()).await;

        // Should be present initially
        assert!(cache.get("test_key").await.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be expired
        assert!(cache.get("test_key").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let cache = ResponseCache::new(1);

        cache.put("key1".to_string(), json!("value1")).await;
        cache.put("key2".to_string(), json!("value2")).await;

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        let cleaned = cache.cleanup_expired().await;
        assert_eq!(cleaned, 2);
        assert_eq!(cache.size().await, 0);
    }

    #[test]
    fn test_cache_key_generation() {
        let key1 = generate_cache_key("GET", "/api/users", None);
        let key2 = generate_cache_key("GET", "/api/users", None);
        let key3 = generate_cache_key("POST", "/api/users", None);

        // Same request should generate same key
        assert_eq!(key1, key2);

        // Different method should generate different key
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_key_with_body() {
        let body1 = json!({"name": "Alice"});
        let body2 = json!({"name": "Bob"});

        let key1 = generate_cache_key("POST", "/api/users", Some(&body1));
        let key2 = generate_cache_key("POST", "/api/users", Some(&body2));

        // Different body should generate different key
        assert_ne!(key1, key2);
    }
}
