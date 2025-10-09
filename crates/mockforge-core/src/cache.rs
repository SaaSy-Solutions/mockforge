//! High-performance caching utilities for MockForge
//!
//! This module provides various caching strategies to optimize
//! performance for frequently accessed data.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cache entry with expiration support
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    expires_at: Option<Instant>,
    access_count: u64,
    last_accessed: Instant,
}

impl<V> CacheEntry<V> {
    fn new(value: V, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        Self {
            value,
            expires_at: ttl.map(|duration| now + duration),
            access_count: 0,
            last_accessed: now,
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|expires_at| Instant::now() > expires_at)
    }

    fn access(&mut self) -> &V {
        self.access_count += 1;
        self.last_accessed = Instant::now();
        &self.value
    }
}

/// High-performance in-memory cache with TTL and LRU eviction
#[derive(Debug)]
pub struct Cache<K, V> {
    storage: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    max_size: usize,
    default_ttl: Option<Duration>,
    stats: Arc<RwLock<CacheStats>>,
}

#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub expirations: u64,
    pub insertions: u64,
}

impl<K: Hash + Eq + Clone, V: Clone> Cache<K, V> {
    /// Create a new cache with specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            default_ttl: None,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Create a new cache with TTL support
    pub fn with_ttl(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            default_ttl: Some(default_ttl),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Insert a value with optional custom TTL
    pub async fn insert(&self, key: K, value: V, ttl: Option<Duration>) {
        let mut storage = self.storage.write().await;
        let mut stats = self.stats.write().await;

        // Use provided TTL or default TTL
        let effective_ttl = ttl.or(self.default_ttl);

        // Clean up expired entries
        self.cleanup_expired(&mut storage, &mut stats).await;

        // Evict LRU entries if at capacity
        if storage.len() >= self.max_size && !storage.contains_key(&key) {
            self.evict_lru(&mut storage, &mut stats).await;
        }

        storage.insert(key, CacheEntry::new(value, effective_ttl));
        stats.insertions += 1;
    }

    /// Get a value from the cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut storage = self.storage.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = storage.get_mut(key) {
            if entry.is_expired() {
                storage.remove(key);
                stats.expirations += 1;
                stats.misses += 1;
                return None;
            }

            stats.hits += 1;
            Some(entry.access().clone())
        } else {
            stats.misses += 1;
            None
        }
    }

    /// Check if a key exists in the cache (without updating access stats)
    pub async fn contains_key(&self, key: &K) -> bool {
        let storage = self.storage.read().await;
        if let Some(entry) = storage.get(key) {
            !entry.is_expired()
        } else {
            false
        }
    }

    /// Remove a key from the cache
    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut storage = self.storage.write().await;
        storage.remove(key).map(|entry| entry.value)
    }

    /// Clear all entries from the cache
    pub async fn clear(&self) {
        let mut storage = self.storage.write().await;
        storage.clear();
    }

    /// Get current cache size
    pub async fn len(&self) -> usize {
        let storage = self.storage.read().await;
        storage.len()
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> bool {
        let storage = self.storage.read().await;
        storage.is_empty()
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Reset cache statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = CacheStats::default();
    }

    /// Get or insert a value using a closure
    pub async fn get_or_insert<F, Fut>(&self, key: K, f: F) -> V
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = V>,
    {
        if let Some(value) = self.get(&key).await {
            return value;
        }

        let value = f().await;
        self.insert(key, value.clone(), None).await;
        value
    }

    /// Get or insert a value with custom TTL using a closure
    pub async fn get_or_insert_with_ttl<F, Fut>(&self, key: K, f: F, ttl: Duration) -> V
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = V>,
    {
        if let Some(value) = self.get(&key).await {
            return value;
        }

        let value = f().await;
        self.insert(key, value.clone(), Some(ttl)).await;
        value
    }

    /// Cleanup expired entries (internal)
    async fn cleanup_expired(&self, storage: &mut HashMap<K, CacheEntry<V>>, stats: &mut CacheStats) {
        let expired_keys: Vec<K> = storage
            .iter()
            .filter_map(|(k, v)| if v.is_expired() { Some(k.clone()) } else { None })
            .collect();

        for key in expired_keys {
            storage.remove(&key);
            stats.expirations += 1;
        }
    }

    /// Evict least recently used entry (internal)
    async fn evict_lru(&self, storage: &mut HashMap<K, CacheEntry<V>>, stats: &mut CacheStats) {
        if let Some((lru_key, _)) = storage
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(k, v)| (k.clone(), v.clone()))
        {
            storage.remove(&lru_key);
            stats.evictions += 1;
        }
    }
}

/// Response cache specifically optimized for HTTP responses
#[derive(Debug)]
pub struct ResponseCache {
    cache: Cache<String, CachedResponse>,
}

#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub content_type: Option<String>,
}

impl ResponseCache {
    /// Create a new response cache
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: Cache::with_ttl(max_size, ttl),
        }
    }

    /// Generate cache key from request parameters
    pub fn generate_key(method: &str, path: &str, query: &str, headers: &HashMap<String, String>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let mut hasher = DefaultHasher::new();
        hasher.write(method.as_bytes());
        hasher.write(path.as_bytes());
        hasher.write(query.as_bytes());
        
        // Include relevant headers in cache key
        let mut sorted_headers: Vec<_> = headers.iter().collect();
        sorted_headers.sort_by_key(|(k, _)| *k);
        for (key, value) in sorted_headers {
            if key.to_lowercase() != "authorization" && !key.to_lowercase().starts_with("x-") {
                hasher.write(key.as_bytes());
                hasher.write(value.as_bytes());
            }
        }

        format!("resp_{}_{}", hasher.finish(), path.len())
    }

    /// Cache a response
    pub async fn cache_response(&self, key: String, response: CachedResponse) {
        self.cache.insert(key, response, None).await;
    }

    /// Get cached response
    pub async fn get_response(&self, key: &str) -> Option<CachedResponse> {
        self.cache.get(&key.to_string()).await
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        self.cache.stats().await
    }
}

/// Template cache for compiled templates
#[derive(Debug)]
pub struct TemplateCache {
    cache: Cache<String, CompiledTemplate>,
}

#[derive(Debug, Clone)]
pub struct CompiledTemplate {
    pub template: String,
    pub variables: Vec<String>,
    pub compiled_at: Instant,
}

impl TemplateCache {
    /// Create a new template cache
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Cache::new(max_size),
        }
    }

    /// Cache a compiled template
    pub async fn cache_template(&self, key: String, template: String, variables: Vec<String>) {
        let compiled = CompiledTemplate {
            template,
            variables,
            compiled_at: Instant::now(),
        };
        self.cache.insert(key, compiled, None).await;
    }

    /// Get cached template
    pub async fn get_template(&self, key: &str) -> Option<CompiledTemplate> {
        self.cache.get(&key.to_string()).await
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        self.cache.stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_basic_cache_operations() {
        let cache = Cache::new(3);
        
        cache.insert("key1".to_string(), "value1".to_string(), None).await;
        cache.insert("key2".to_string(), "value2".to_string(), None).await;
        
        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));
        assert_eq!(cache.get(&"key2".to_string()).await, Some("value2".to_string()));
        assert_eq!(cache.get(&"key3".to_string()).await, None);
        
        assert_eq!(cache.len().await, 2);
        assert!(!cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let cache = Cache::with_ttl(10, Duration::from_millis(50));
        
        cache.insert("key1".to_string(), "value1".to_string(), None).await;
        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));
        
        sleep(Duration::from_millis(60)).await;
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let cache = Cache::new(2);
        
        cache.insert("key1".to_string(), "value1".to_string(), None).await;
        cache.insert("key2".to_string(), "value2".to_string(), None).await;
        
        // Access key1 to make it more recently used
        cache.get(&"key1".to_string()).await;
        
        // Insert key3, should evict key2 (least recently used)
        cache.insert("key3".to_string(), "value3".to_string(), None).await;
        
        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));
        assert_eq!(cache.get(&"key2".to_string()).await, None);
        assert_eq!(cache.get(&"key3".to_string()).await, Some("value3".to_string()));
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = Cache::new(10);
        
        cache.insert("key1".to_string(), "value1".to_string(), None).await;
        cache.get(&"key1".to_string()).await; // Hit
        cache.get(&"key2".to_string()).await; // Miss
        
        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.insertions, 1);
    }

    #[tokio::test]
    async fn test_response_cache() {
        let response_cache = ResponseCache::new(100, Duration::from_secs(300));
        
        let headers = HashMap::new();
        let key = ResponseCache::generate_key("GET", "/api/users", "", &headers);
        
        let response = CachedResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: "test response".to_string(),
            content_type: Some("application/json".to_string()),
        };
        
        response_cache.cache_response(key.clone(), response.clone()).await;
        let cached = response_cache.get_response(&key).await;
        
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().body, "test response");
    }
}