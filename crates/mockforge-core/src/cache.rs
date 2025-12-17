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

/// Statistics for cache performance tracking
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// Number of cache hits (successful lookups)
    pub hits: u64,
    /// Number of cache misses (failed lookups)
    pub misses: u64,
    /// Number of entries evicted due to size limits
    pub evictions: u64,
    /// Number of entries expired due to TTL
    pub expirations: u64,
    /// Total number of insertions
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
    async fn cleanup_expired(
        &self,
        storage: &mut HashMap<K, CacheEntry<V>>,
        stats: &mut CacheStats,
    ) {
        let expired_keys: Vec<K> = storage
            .iter()
            .filter_map(|(k, v)| {
                if v.is_expired() {
                    Some(k.clone())
                } else {
                    None
                }
            })
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

/// Cached HTTP response data
#[derive(Debug, Clone)]
pub struct CachedResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body content
    pub body: String,
    /// Content-Type header value, if present
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
    pub fn generate_key(
        method: &str,
        path: &str,
        query: &str,
        headers: &HashMap<String, String>,
    ) -> String {
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

/// Compiled template with metadata for caching
#[derive(Debug, Clone)]
pub struct CompiledTemplate {
    /// The compiled template string
    pub template: String,
    /// List of variable names used in the template
    pub variables: Vec<String>,
    /// Timestamp when the template was compiled
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

    // ==================== Basic Cache Operations ====================

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
    async fn test_cache_new() {
        let cache: Cache<String, String> = Cache::new(100);
        assert!(cache.is_empty().await);
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn test_cache_with_ttl() {
        let cache: Cache<String, String> = Cache::with_ttl(100, Duration::from_secs(60));
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_cache_contains_key() {
        let cache = Cache::new(10);
        cache.insert("key1".to_string(), "value1".to_string(), None).await;

        assert!(cache.contains_key(&"key1".to_string()).await);
        assert!(!cache.contains_key(&"key2".to_string()).await);
    }

    #[tokio::test]
    async fn test_cache_remove() {
        let cache = Cache::new(10);
        cache.insert("key1".to_string(), "value1".to_string(), None).await;

        let removed = cache.remove(&"key1".to_string()).await;
        assert_eq!(removed, Some("value1".to_string()));
        assert!(!cache.contains_key(&"key1".to_string()).await);

        // Remove non-existent key
        let removed2 = cache.remove(&"key2".to_string()).await;
        assert_eq!(removed2, None);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = Cache::new(10);
        cache.insert("key1".to_string(), "value1".to_string(), None).await;
        cache.insert("key2".to_string(), "value2".to_string(), None).await;

        assert_eq!(cache.len().await, 2);
        cache.clear().await;
        assert_eq!(cache.len().await, 0);
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_cache_overwrite() {
        let cache = Cache::new(10);
        cache.insert("key1".to_string(), "value1".to_string(), None).await;
        cache.insert("key1".to_string(), "value2".to_string(), None).await;

        assert_eq!(cache.get(&"key1".to_string()).await, Some("value2".to_string()));
        assert_eq!(cache.len().await, 1);
    }

    // ==================== TTL Tests ====================

    #[tokio::test]
    async fn test_ttl_expiration() {
        let cache = Cache::with_ttl(10, Duration::from_millis(50));

        cache.insert("key1".to_string(), "value1".to_string(), None).await;
        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));

        sleep(Duration::from_millis(60)).await;
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_custom_ttl_per_entry() {
        let cache = Cache::new(10);

        // Insert with custom TTL
        cache
            .insert("short".to_string(), "short_lived".to_string(), Some(Duration::from_millis(30)))
            .await;
        cache
            .insert("long".to_string(), "long_lived".to_string(), Some(Duration::from_secs(60)))
            .await;

        assert_eq!(cache.get(&"short".to_string()).await, Some("short_lived".to_string()));
        assert_eq!(cache.get(&"long".to_string()).await, Some("long_lived".to_string()));

        // Wait for short TTL to expire
        sleep(Duration::from_millis(50)).await;

        assert_eq!(cache.get(&"short".to_string()).await, None);
        assert_eq!(cache.get(&"long".to_string()).await, Some("long_lived".to_string()));
    }

    #[tokio::test]
    async fn test_contains_key_respects_ttl() {
        let cache = Cache::with_ttl(10, Duration::from_millis(30));
        cache.insert("key".to_string(), "value".to_string(), None).await;

        assert!(cache.contains_key(&"key".to_string()).await);

        sleep(Duration::from_millis(50)).await;

        assert!(!cache.contains_key(&"key".to_string()).await);
    }

    // ==================== LRU Eviction Tests ====================

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
    async fn test_eviction_stats() {
        let cache = Cache::new(2);

        cache.insert("key1".to_string(), "value1".to_string(), None).await;
        cache.insert("key2".to_string(), "value2".to_string(), None).await;
        cache.insert("key3".to_string(), "value3".to_string(), None).await;

        let stats = cache.stats().await;
        assert_eq!(stats.evictions, 1);
    }

    #[tokio::test]
    async fn test_no_eviction_when_replacing() {
        let cache = Cache::new(2);

        cache.insert("key1".to_string(), "value1".to_string(), None).await;
        cache.insert("key2".to_string(), "value2".to_string(), None).await;
        // Replace existing key, shouldn't evict
        cache.insert("key1".to_string(), "updated".to_string(), None).await;

        let stats = cache.stats().await;
        assert_eq!(stats.evictions, 0);
        assert_eq!(cache.len().await, 2);
    }

    // ==================== Stats Tests ====================

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
    async fn test_reset_stats() {
        let cache = Cache::new(10);

        cache.insert("key1".to_string(), "value1".to_string(), None).await;
        cache.get(&"key1".to_string()).await;
        cache.get(&"key2".to_string()).await;

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);

        cache.reset_stats().await;

        let stats_after = cache.stats().await;
        assert_eq!(stats_after.hits, 0);
        assert_eq!(stats_after.misses, 0);
        assert_eq!(stats_after.insertions, 0);
    }

    #[tokio::test]
    async fn test_expiration_stats() {
        let cache = Cache::with_ttl(10, Duration::from_millis(20));

        cache.insert("key".to_string(), "value".to_string(), None).await;
        sleep(Duration::from_millis(30)).await;
        cache.get(&"key".to_string()).await; // Should trigger expiration

        let stats = cache.stats().await;
        assert_eq!(stats.expirations, 1);
    }

    // ==================== get_or_insert Tests ====================

    #[tokio::test]
    async fn test_get_or_insert_miss() {
        let cache = Cache::new(10);

        let value = cache
            .get_or_insert("key".to_string(), || async { "computed_value".to_string() })
            .await;

        assert_eq!(value, "computed_value".to_string());
        assert_eq!(cache.get(&"key".to_string()).await, Some("computed_value".to_string()));
    }

    #[tokio::test]
    async fn test_get_or_insert_hit() {
        let cache = Cache::new(10);
        cache.insert("key".to_string(), "existing_value".to_string(), None).await;

        let value = cache
            .get_or_insert("key".to_string(), || async { "should_not_be_used".to_string() })
            .await;

        assert_eq!(value, "existing_value".to_string());
    }

    #[tokio::test]
    async fn test_get_or_insert_with_ttl() {
        let cache = Cache::new(10);

        let value = cache
            .get_or_insert_with_ttl(
                "key".to_string(),
                || async { "computed".to_string() },
                Duration::from_millis(30),
            )
            .await;

        assert_eq!(value, "computed".to_string());

        // Value should exist before TTL expires
        assert!(cache.contains_key(&"key".to_string()).await);

        // Wait for TTL
        sleep(Duration::from_millis(50)).await;

        assert!(!cache.contains_key(&"key".to_string()).await);
    }

    // ==================== ResponseCache Tests ====================

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

    #[tokio::test]
    async fn test_response_cache_key_generation() {
        let headers1 = HashMap::new();
        let headers2 = HashMap::new();

        // Same request params should generate same key
        let key1 = ResponseCache::generate_key("GET", "/api/users", "page=1", &headers1);
        let key2 = ResponseCache::generate_key("GET", "/api/users", "page=1", &headers2);
        assert_eq!(key1, key2);

        // Different method should generate different key
        let key3 = ResponseCache::generate_key("POST", "/api/users", "page=1", &headers1);
        assert_ne!(key1, key3);

        // Different path should generate different key
        let key4 = ResponseCache::generate_key("GET", "/api/items", "page=1", &headers1);
        assert_ne!(key1, key4);

        // Different query should generate different key
        let key5 = ResponseCache::generate_key("GET", "/api/users", "page=2", &headers1);
        assert_ne!(key1, key5);
    }

    #[tokio::test]
    async fn test_response_cache_key_excludes_auth_headers() {
        let mut headers_without_auth = HashMap::new();
        headers_without_auth.insert("accept".to_string(), "application/json".to_string());

        let mut headers_with_auth = headers_without_auth.clone();
        headers_with_auth.insert("authorization".to_string(), "Bearer token123".to_string());

        // Authorization header should be excluded from key
        let key1 = ResponseCache::generate_key("GET", "/api/users", "", &headers_without_auth);
        let key2 = ResponseCache::generate_key("GET", "/api/users", "", &headers_with_auth);

        assert_eq!(key1, key2);
    }

    #[tokio::test]
    async fn test_response_cache_key_excludes_x_headers() {
        let mut headers1 = HashMap::new();
        headers1.insert("accept".to_string(), "application/json".to_string());

        let mut headers2 = headers1.clone();
        headers2.insert("x-request-id".to_string(), "unique-id-123".to_string());
        headers2.insert("x-correlation-id".to_string(), "corr-456".to_string());

        let key1 = ResponseCache::generate_key("GET", "/api/users", "", &headers1);
        let key2 = ResponseCache::generate_key("GET", "/api/users", "", &headers2);

        assert_eq!(key1, key2);
    }

    #[tokio::test]
    async fn test_response_cache_stats() {
        let response_cache = ResponseCache::new(10, Duration::from_secs(60));

        let response = CachedResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: "test".to_string(),
            content_type: None,
        };

        response_cache.cache_response("key1".to_string(), response).await;
        response_cache.get_response("key1").await; // Hit
        response_cache.get_response("key2").await; // Miss

        let stats = response_cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    // ==================== TemplateCache Tests ====================

    #[tokio::test]
    async fn test_template_cache_new() {
        let template_cache = TemplateCache::new(100);
        assert_eq!(template_cache.stats().await.insertions, 0);
    }

    #[tokio::test]
    async fn test_template_cache_operations() {
        let template_cache = TemplateCache::new(100);

        template_cache
            .cache_template(
                "greeting".to_string(),
                "Hello, {{name}}!".to_string(),
                vec!["name".to_string()],
            )
            .await;

        let cached = template_cache.get_template("greeting").await;
        assert!(cached.is_some());

        let template = cached.unwrap();
        assert_eq!(template.template, "Hello, {{name}}!");
        assert_eq!(template.variables, vec!["name".to_string()]);
    }

    #[tokio::test]
    async fn test_template_cache_miss() {
        let template_cache = TemplateCache::new(100);

        let cached = template_cache.get_template("nonexistent").await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_template_cache_stats() {
        let template_cache = TemplateCache::new(10);

        template_cache
            .cache_template("key".to_string(), "template".to_string(), vec![])
            .await;

        template_cache.get_template("key").await; // Hit
        template_cache.get_template("missing").await; // Miss

        let stats = template_cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.insertions, 1);
    }

    // ==================== CacheStats Tests ====================

    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.expirations, 0);
        assert_eq!(stats.insertions, 0);
    }

    #[test]
    fn test_cache_stats_clone() {
        let mut stats = CacheStats::default();
        stats.hits = 10;
        stats.misses = 5;

        let cloned = stats.clone();
        assert_eq!(cloned.hits, 10);
        assert_eq!(cloned.misses, 5);
    }

    #[test]
    fn test_cache_stats_debug() {
        let stats = CacheStats::default();
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("CacheStats"));
        assert!(debug_str.contains("hits"));
    }

    // ==================== CachedResponse Tests ====================

    #[test]
    fn test_cached_response_clone() {
        let response = CachedResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: "test".to_string(),
            content_type: Some("application/json".to_string()),
        };

        let cloned = response.clone();
        assert_eq!(cloned.status_code, 200);
        assert_eq!(cloned.body, "test");
        assert_eq!(cloned.content_type, Some("application/json".to_string()));
    }

    #[test]
    fn test_cached_response_debug() {
        let response = CachedResponse {
            status_code: 404,
            headers: HashMap::new(),
            body: "not found".to_string(),
            content_type: None,
        };

        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("CachedResponse"));
        assert!(debug_str.contains("404"));
    }

    // ==================== CompiledTemplate Tests ====================

    #[test]
    fn test_compiled_template_clone() {
        let template = CompiledTemplate {
            template: "Hello, {{name}}!".to_string(),
            variables: vec!["name".to_string()],
            compiled_at: Instant::now(),
        };

        let cloned = template.clone();
        assert_eq!(cloned.template, "Hello, {{name}}!");
        assert_eq!(cloned.variables, vec!["name".to_string()]);
    }

    #[test]
    fn test_compiled_template_debug() {
        let template = CompiledTemplate {
            template: "test".to_string(),
            variables: vec![],
            compiled_at: Instant::now(),
        };

        let debug_str = format!("{:?}", template);
        assert!(debug_str.contains("CompiledTemplate"));
        assert!(debug_str.contains("test"));
    }

    // ==================== Edge Cases ====================

    #[tokio::test]
    async fn test_cache_with_zero_size() {
        // Zero-size cache should handle gracefully
        let cache = Cache::new(0);
        cache.insert("key".to_string(), "value".to_string(), None).await;
        // May or may not be stored depending on implementation
    }

    #[tokio::test]
    async fn test_cache_with_numeric_keys() {
        let cache = Cache::new(10);
        cache.insert(1, "one".to_string(), None).await;
        cache.insert(2, "two".to_string(), None).await;

        assert_eq!(cache.get(&1).await, Some("one".to_string()));
        assert_eq!(cache.get(&2).await, Some("two".to_string()));
    }

    #[tokio::test]
    async fn test_cache_with_complex_values() {
        let cache: Cache<String, Vec<u8>> = Cache::new(10);
        cache.insert("bytes".to_string(), vec![1, 2, 3, 4, 5], None).await;

        let retrieved = cache.get(&"bytes".to_string()).await;
        assert_eq!(retrieved, Some(vec![1, 2, 3, 4, 5]));
    }

    #[tokio::test]
    async fn test_multiple_expirations_cleanup() {
        let cache = Cache::with_ttl(10, Duration::from_millis(20));

        cache.insert("key1".to_string(), "v1".to_string(), None).await;
        cache.insert("key2".to_string(), "v2".to_string(), None).await;
        cache.insert("key3".to_string(), "v3".to_string(), None).await;

        sleep(Duration::from_millis(30)).await;

        // All should be expired, but insert triggers cleanup
        cache.insert("new".to_string(), "new_val".to_string(), None).await;

        let stats = cache.stats().await;
        assert!(stats.expirations >= 3);
    }
}
