//! Response caching and memoization for GraphQL operations
//!
//! Provides intelligent caching of GraphQL responses to improve performance.

use async_graphql::Response;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cache key for GraphQL operations
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CacheKey {
    /// Operation name
    pub operation_name: String,
    /// Query string
    pub query: String,
    /// Variables as JSON string for hashing
    pub variables: String,
}

impl CacheKey {
    /// Create a new cache key
    pub fn new(operation_name: String, query: String, variables: String) -> Self {
        Self {
            operation_name,
            query,
            variables,
        }
    }

    /// Create from GraphQL request components
    pub fn from_request(
        operation_name: Option<String>,
        query: String,
        variables: serde_json::Value,
    ) -> Self {
        Self {
            operation_name: operation_name.unwrap_or_default(),
            query,
            variables: variables.to_string(),
        }
    }
}

/// Cached response with metadata
pub struct CachedResponse {
    /// The GraphQL response data (as serde_json::Value for easy serialization)
    pub data: serde_json::Value,
    /// Any errors in the response
    pub errors: Vec<CachedError>,
    /// Extensions from the response
    pub extensions: Option<serde_json::Value>,
    /// When this was cached
    pub cached_at: Instant,
    /// Number of cache hits
    pub hit_count: usize,
}

/// Cached error representation
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CachedError {
    /// Error message
    pub message: String,
    /// Error locations in the query
    pub locations: Vec<CachedErrorLocation>,
    /// Path to the field that caused the error
    pub path: Option<Vec<serde_json::Value>>,
    /// Additional error extensions
    pub extensions: Option<serde_json::Value>,
}

/// Error location in the query
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CachedErrorLocation {
    /// Line number in the query (1-indexed)
    pub line: usize,
    /// Column number in the query (1-indexed)
    pub column: usize,
}

impl CachedResponse {
    /// Convert to GraphQL Response
    pub fn to_response(&self) -> Response {
        // Convert serde_json::Value back to async_graphql::Value
        let graphql_value = json_to_graphql_value(&self.data);
        let mut response = Response::new(graphql_value);

        // Restore errors
        for cached_error in &self.errors {
            let mut server_error =
                async_graphql::ServerError::new(cached_error.message.clone(), None);

            // Restore locations
            server_error.locations = cached_error
                .locations
                .iter()
                .map(|loc| async_graphql::Pos {
                    line: loc.line,
                    column: loc.column,
                })
                .collect();

            // Restore path
            if let Some(path) = &cached_error.path {
                server_error.path = path
                    .iter()
                    .filter_map(|v| match v {
                        serde_json::Value::String(s) => {
                            Some(async_graphql::PathSegment::Field(s.clone()))
                        }
                        serde_json::Value::Number(n) => {
                            n.as_u64().map(|i| async_graphql::PathSegment::Index(i as usize))
                        }
                        _ => None,
                    })
                    .collect();
            }

            response.errors.push(server_error);
        }

        // Restore extensions
        if let Some(ext) = &self.extensions {
            if let serde_json::Value::Object(map) = ext {
                for (key, value) in map {
                    response.extensions.insert(key.clone(), json_to_graphql_value(value));
                }
            }
        }

        response
    }

    /// Create from GraphQL Response
    pub fn from_response(response: &Response) -> Self {
        // Convert async_graphql::Value to serde_json::Value
        let data = graphql_value_to_json(&response.data);

        // Convert errors
        let errors: Vec<CachedError> = response
            .errors
            .iter()
            .map(|e| CachedError {
                message: e.message.clone(),
                locations: e
                    .locations
                    .iter()
                    .map(|loc| CachedErrorLocation {
                        line: loc.line,
                        column: loc.column,
                    })
                    .collect(),
                path: if e.path.is_empty() {
                    None
                } else {
                    Some(
                        e.path
                            .iter()
                            .map(|seg| match seg {
                                async_graphql::PathSegment::Field(s) => {
                                    serde_json::Value::String(s.clone())
                                }
                                async_graphql::PathSegment::Index(i) => {
                                    serde_json::Value::Number((*i as u64).into())
                                }
                            })
                            .collect(),
                    )
                },
                extensions: None, // ServerError extensions are not easily accessible
            })
            .collect();

        // Convert extensions
        let extensions = if response.extensions.is_empty() {
            None
        } else {
            let mut map = serde_json::Map::new();
            for (key, value) in &response.extensions {
                map.insert(key.clone(), graphql_value_to_json(value));
            }
            Some(serde_json::Value::Object(map))
        };

        Self {
            data,
            errors,
            extensions,
            cached_at: Instant::now(),
            hit_count: 0,
        }
    }
}

/// Convert async_graphql::Value to serde_json::Value
fn graphql_value_to_json(value: &async_graphql::Value) -> serde_json::Value {
    match value {
        async_graphql::Value::Null => serde_json::Value::Null,
        async_graphql::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                serde_json::Value::Number(u.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        async_graphql::Value::String(s) => serde_json::Value::String(s.clone()),
        async_graphql::Value::Boolean(b) => serde_json::Value::Bool(*b),
        async_graphql::Value::List(arr) => {
            serde_json::Value::Array(arr.iter().map(graphql_value_to_json).collect())
        }
        async_graphql::Value::Object(obj) => {
            let map: serde_json::Map<String, serde_json::Value> =
                obj.iter().map(|(k, v)| (k.to_string(), graphql_value_to_json(v))).collect();
            serde_json::Value::Object(map)
        }
        async_graphql::Value::Enum(e) => serde_json::Value::String(e.to_string()),
        async_graphql::Value::Binary(b) => {
            use base64::Engine;
            serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(b))
        }
    }
}

/// Convert serde_json::Value to async_graphql::Value
fn json_to_graphql_value(value: &serde_json::Value) -> async_graphql::Value {
    match value {
        serde_json::Value::Null => async_graphql::Value::Null,
        serde_json::Value::Bool(b) => async_graphql::Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                async_graphql::Value::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                async_graphql::Value::Number(u.into())
            } else if let Some(f) = n.as_f64() {
                async_graphql::Value::Number(
                    async_graphql::Number::from_f64(f).unwrap_or_else(|| 0.into()),
                )
            } else {
                async_graphql::Value::Null
            }
        }
        serde_json::Value::String(s) => async_graphql::Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            async_graphql::Value::List(arr.iter().map(json_to_graphql_value).collect())
        }
        serde_json::Value::Object(obj) => {
            let map: indexmap::IndexMap<async_graphql::Name, async_graphql::Value> = obj
                .iter()
                .filter_map(|(k, v)| {
                    // GraphQL names must match [_A-Za-z][_0-9A-Za-z]*
                    let is_valid =
                        k.chars().next().is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
                            && k.chars().all(|c| c == '_' || c.is_ascii_alphanumeric());
                    if is_valid {
                        Some((async_graphql::Name::new(k), json_to_graphql_value(v)))
                    } else {
                        None
                    }
                })
                .collect();
            async_graphql::Value::Object(map)
        }
    }
}

/// Cache configuration
#[derive(Clone, Debug)]
pub struct CacheConfig {
    /// Maximum number of entries
    pub max_entries: usize,
    /// Time-to-live for cached responses
    pub ttl: Duration,
    /// Enable cache statistics
    pub enable_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            ttl: Duration::from_secs(300), // 5 minutes
            enable_stats: true,
        }
    }
}

/// Cache statistics
#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Number of evictions
    pub evictions: u64,
    /// Current cache size
    pub size: usize,
}

impl CacheStats {
    /// Get hit rate as a percentage
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

/// Response cache for GraphQL operations
pub struct ResponseCache {
    /// Cache storage
    cache: Arc<RwLock<HashMap<CacheKey, CachedResponse>>>,
    /// Cache configuration
    config: CacheConfig,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
}

impl ResponseCache {
    /// Create a new response cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Get a cached response
    pub fn get(&self, key: &CacheKey) -> Option<Response> {
        let mut cache = self.cache.write();

        if let Some(cached) = cache.get_mut(key) {
            // Check if TTL expired
            if cached.cached_at.elapsed() > self.config.ttl {
                cache.remove(key);
                self.record_miss();
                return None;
            }

            // Update hit count
            cached.hit_count += 1;
            self.record_hit();

            // Convert cached response to GraphQL Response
            Some(cached.to_response())
        } else {
            self.record_miss();
            None
        }
    }

    /// Put a response in the cache
    pub fn put(&self, key: CacheKey, response: Response) {
        let mut cache = self.cache.write();

        // Evict oldest entry if at capacity
        if cache.len() >= self.config.max_entries {
            if let Some(oldest_key) = self.find_oldest_key(&cache) {
                cache.remove(&oldest_key);
                self.record_eviction();
            }
        }

        // Convert response to cached format
        let cached_response = CachedResponse::from_response(&response);

        cache.insert(key, cached_response);

        self.update_size(cache.len());
    }

    /// Clear all cached responses
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        cache.clear();
        self.update_size(0);
    }

    /// Clear expired entries
    pub fn clear_expired(&self) {
        let mut cache = self.cache.write();
        let ttl = self.config.ttl;

        cache.retain(|_, cached| cached.cached_at.elapsed() <= ttl);
        self.update_size(cache.len());
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.read().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.write();
        *stats = CacheStats::default();
    }

    // Private helper methods

    fn find_oldest_key(&self, cache: &HashMap<CacheKey, CachedResponse>) -> Option<CacheKey> {
        cache
            .iter()
            .min_by_key(|(_, cached)| cached.cached_at)
            .map(|(key, _)| key.clone())
    }

    fn record_hit(&self) {
        if self.config.enable_stats {
            let mut stats = self.stats.write();
            stats.hits += 1;
        }
    }

    fn record_miss(&self) {
        if self.config.enable_stats {
            let mut stats = self.stats.write();
            stats.misses += 1;
        }
    }

    fn record_eviction(&self) {
        if self.config.enable_stats {
            let mut stats = self.stats.write();
            stats.evictions += 1;
        }
    }

    fn update_size(&self, size: usize) {
        if self.config.enable_stats {
            let mut stats = self.stats.write();
            stats.size = size;
        }
    }
}

/// Cache middleware for automatic caching
pub struct CacheMiddleware {
    cache: Arc<ResponseCache>,
    /// Operations to cache (None = cache all)
    cacheable_operations: Option<Vec<String>>,
}

impl CacheMiddleware {
    /// Create new cache middleware
    pub fn new(cache: Arc<ResponseCache>) -> Self {
        Self {
            cache,
            cacheable_operations: None,
        }
    }

    /// Set specific operations to cache
    pub fn with_operations(mut self, operations: Vec<String>) -> Self {
        self.cacheable_operations = Some(operations);
        self
    }

    /// Check if an operation should be cached
    pub fn should_cache(&self, operation_name: Option<&str>) -> bool {
        match &self.cacheable_operations {
            None => true, // Cache everything
            Some(ops) => {
                operation_name.map(|name| ops.contains(&name.to_string())).unwrap_or(false)
            }
        }
    }

    /// Get cached response if available
    pub fn get_cached(&self, key: &CacheKey) -> Option<Response> {
        self.cache.get(key)
    }

    /// Cache a response
    pub fn cache_response(&self, key: CacheKey, response: Response) {
        self.cache.put(key, response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::Value;

    #[test]
    fn test_cache_key_creation() {
        let key = CacheKey::new(
            "getUser".to_string(),
            "query { user { id } }".to_string(),
            r#"{"id": "123"}"#.to_string(),
        );

        assert_eq!(key.operation_name, "getUser");
    }

    #[test]
    fn test_cache_key_from_request() {
        let key = CacheKey::from_request(
            Some("getUser".to_string()),
            "query { user { id } }".to_string(),
            serde_json::json!({"id": "123"}),
        );

        assert_eq!(key.operation_name, "getUser");
        assert!(key.variables.contains("123"));
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.max_entries, 1000);
        assert_eq!(config.ttl, Duration::from_secs(300));
        assert!(config.enable_stats);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::default();
        stats.hits = 80;
        stats.misses = 20;

        assert_eq!(stats.hit_rate(), 80.0);
    }

    #[test]
    fn test_cache_put_and_get() {
        let cache = ResponseCache::default();
        let key = CacheKey::new("test".to_string(), "query".to_string(), "{}".to_string());
        let response = Response::new(Value::Null);

        cache.put(key.clone(), response);
        let cached = cache.get(&key);

        assert!(cached.is_some());
    }

    #[test]
    fn test_cache_miss() {
        let cache = ResponseCache::default();
        let key = CacheKey::new("nonexistent".to_string(), "query".to_string(), "{}".to_string());

        let cached = cache.get(&key);
        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_clear() {
        let cache = ResponseCache::default();
        let key = CacheKey::new("test".to_string(), "query".to_string(), "{}".to_string());
        let response = Response::new(Value::Null);

        cache.put(key.clone(), response);
        assert!(cache.get(&key).is_some());

        cache.clear();
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = ResponseCache::default();
        let key = CacheKey::new("test".to_string(), "query".to_string(), "{}".to_string());

        // Miss
        let _ = cache.get(&key);

        // Put and hit
        let response = Response::new(Value::Null);
        cache.put(key.clone(), response);
        let _ = cache.get(&key);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.size, 1);
    }

    #[test]
    fn test_cache_middleware_should_cache() {
        let cache = Arc::new(ResponseCache::default());
        let middleware = CacheMiddleware::new(cache);

        assert!(middleware.should_cache(Some("getUser")));
        assert!(middleware.should_cache(None));
    }

    #[test]
    fn test_cache_middleware_with_specific_operations() {
        let cache = Arc::new(ResponseCache::default());
        let middleware = CacheMiddleware::new(cache)
            .with_operations(vec!["getUser".to_string(), "getProduct".to_string()]);

        assert!(middleware.should_cache(Some("getUser")));
        assert!(!middleware.should_cache(Some("createUser")));
    }

    #[test]
    fn test_cache_eviction() {
        let config = CacheConfig {
            max_entries: 2,
            ttl: Duration::from_secs(300),
            enable_stats: true,
        };
        let cache = ResponseCache::new(config);

        // Add 3 entries (should evict the oldest)
        for i in 0..3 {
            let key = CacheKey::new(format!("op{}", i), "query".to_string(), "{}".to_string());
            cache.put(key, Response::new(Value::Null));
            std::thread::sleep(Duration::from_millis(10)); // Ensure different timestamps
        }

        let stats = cache.stats();
        assert_eq!(stats.size, 2);
        assert_eq!(stats.evictions, 1);
    }
}
