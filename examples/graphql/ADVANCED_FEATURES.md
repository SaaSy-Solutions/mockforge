# Advanced GraphQL Features

This document covers the advanced features added to MockForge's GraphQL handler system.

## 🔄 Hot-Reloading Schema Files

MockForge can automatically reload GraphQL schemas when the file changes, eliminating the need to restart the server during development.

### Usage

```rust
use mockforge_graphql::SchemaWatcher;
use std::path::PathBuf;

// Create a schema watcher
let mut watcher = SchemaWatcher::new(PathBuf::from("./schema.graphql")).await?;

// Start watching for changes
watcher.start_watching()?;

// Get the current schema (always up-to-date)
let schema_sdl = watcher.get_schema().await;

// Manually reload if needed
watcher.reload().await?;
```

### Features

- **Automatic detection**: Watches the schema file for modifications
- **Zero-downtime**: Schema updates without restarting the server
- **Manual reload**: Force reload when needed
- **Error handling**: Graceful error handling for invalid schemas

### Example

```bash
# Start server with hot-reload enabled
mockforge serve --graphql ./schema.graphql

# In another terminal, edit the schema
vim schema.graphql  # Add new fields or types

# Server automatically reloads the schema
# ✓ Schema reloaded successfully
```

---

## 💾 Response Caching

Intelligent caching of GraphQL responses improves performance for frequently accessed queries.

### Basic Usage

```rust
use mockforge_graphql::{ResponseCache, CacheConfig, CacheKey};
use std::time::Duration;

// Create a cache with custom configuration
let config = CacheConfig {
    max_entries: 1000,                  // Maximum cached responses
    ttl: Duration::from_secs(300),      // 5 minute TTL
    enable_stats: true,                 // Track cache statistics
};

let cache = ResponseCache::new(config);

// Create a cache key
let key = CacheKey::from_request(
    Some("getUser".to_string()),
    "query { user(id: \"123\") { id name } }".to_string(),
    serde_json::json!({"id": "123"}),
);

// Try to get from cache
if let Some(response) = cache.get(&key) {
    println!("Cache hit!");
    return response;
}

// Execute query and cache the response
let response = execute_query().await;
cache.put(key, response.clone());
```

### Cache Statistics

```rust
// Get cache statistics
let stats = cache.stats();
println!("Hit rate: {:.2}%", stats.hit_rate());
println!("Total hits: {}", stats.hits);
println!("Total misses: {}", stats.misses);
println!("Evictions: {}", stats.evictions);
println!("Current size: {}", stats.size);

// Reset statistics
cache.reset_stats();
```

### Cache Middleware

```rust
use mockforge_graphql::CacheMiddleware;
use std::sync::Arc;

let cache = Arc::new(ResponseCache::default());

// Create middleware
let middleware = CacheMiddleware::new(cache);

// Only cache specific operations
let middleware = middleware.with_operations(vec![
    "getUser".to_string(),
    "getProduct".to_string(),
    "listProducts".to_string(),
]);

// Check if should cache
if middleware.should_cache(Some("getUser")) {
    if let Some(cached) = middleware.get_cached(&key) {
        return cached;
    }
}
```

### Cache Strategies

#### Time-To-Live (TTL)

```rust
// Short-lived cache for real-time data
let config = CacheConfig {
    ttl: Duration::from_secs(30),  // 30 seconds
    ..Default::default()
};
```

#### Size-Based Eviction

```rust
// Limit cache size
let config = CacheConfig {
    max_entries: 100,  // Only cache 100 responses
    ..Default::default()
};
```

#### Clearing Cache

```rust
// Clear all cached responses
cache.clear();

// Clear only expired entries
cache.clear_expired();
```

### Performance Tips

1. **Cache queries, not mutations**: Only cache idempotent operations
2. **Use appropriate TTL**: Balance freshness vs. performance
3. **Monitor statistics**: Adjust cache size based on hit rate
4. **Operation-specific caching**: Only cache frequently accessed queries

---

## 📡 Subscriptions

Real-time GraphQL subscriptions over WebSocket for live data updates.

### Subscription Manager

```rust
use mockforge_graphql::{SubscriptionManager, SubscriptionEvent};
use async_graphql::Value;

// Create subscription manager
let manager = SubscriptionManager::new();

// Subscribe to a topic
let mut receiver = manager.subscribe(
    "sub-1".to_string(),
    "orderStatusChanged".to_string(),
    Some("OrderStatusSubscription".to_string()),
);

// Publish events
let event = SubscriptionEvent::new(
    "orderStatusChanged".to_string(),
    Value::String("SHIPPED".to_string()),
);

manager.publish(event);

// Receive events
tokio::spawn(async move {
    while let Ok(event) = receiver.recv().await {
        println!("Received: {:?}", event.data);
    }
});
```

### Subscription Handler Trait

```rust
use mockforge_graphql::SubscriptionHandler;
use async_trait::async_trait;
use std::collections::HashMap;

struct OrderStatusSubscription;

#[async_trait]
impl SubscriptionHandler for OrderStatusSubscription {
    async fn on_subscribe(
        &self,
        topic: &str,
        variables: &HashMap<String, Value>
    ) -> Result<(), String> {
        // Validate subscription parameters
        if !variables.contains_key("orderId") {
            return Err("orderId is required".to_string());
        }
        Ok(())
    }

    async fn initial_data(
        &self,
        topic: &str,
        variables: &HashMap<String, Value>
    ) -> Option<Value> {
        // Return initial state
        Some(Value::String("PENDING".to_string()))
    }

    fn handles_subscription(&self, operation_name: &str) -> bool {
        operation_name == "orderStatusChanged"
    }
}
```

### Example Subscription

```graphql
subscription OrderStatusChanged($orderId: ID!) {
  orderStatusChanged(orderId: $orderId) {
    id
    status
    updatedAt
  }
}
```

### Managing Subscriptions

```rust
// Get all active topics
let topics = manager.topics();
println!("Active topics: {:?}", topics);

// Get subscriber count for a topic
let count = manager.subscriber_count(&"orderStatusChanged".to_string());
println!("{} subscribers", count);

// Get all active subscriptions
let subscriptions = manager.active_subscriptions();
for sub in subscriptions {
    println!("ID: {}, Topic: {}, Age: {:?}",
        sub.id,
        sub.topic,
        sub.created_at.elapsed()
    );
}

// Unsubscribe
manager.unsubscribe(&"sub-1".to_string());

// Clear all subscriptions
manager.clear();
```

### Broadcasting to Multiple Subscribers

```rust
// Multiple clients subscribe to the same topic
let recv1 = manager.subscribe("client-1".to_string(), "product".to_string(), None);
let recv2 = manager.subscribe("client-2".to_string(), "product".to_string(), None);
let recv3 = manager.subscribe("client-3".to_string(), "product".to_string(), None);

// Publish event (broadcasts to all 3 subscribers)
let event = SubscriptionEvent::new(
    "product".to_string(),
    Value::String("New product added!".to_string()),
);

let subscriber_count = manager.publish(event);
println!("Event sent to {} subscribers", subscriber_count);
```

---

## 🎯 Performance Monitoring

Track performance metrics for GraphQL operations.

### Operation-Level Metrics

Each GraphQL operation automatically tracks:

- **Execution time**: Duration from request to response
- **Cache hits/misses**: Whether the response was cached
- **Error rate**: Number of failed operations
- **Throughput**: Operations per second

### Accessing Metrics

```rust
use mockforge_graphql::cache::CacheStats;

// Get cache statistics
let stats = cache.stats();

println!("Performance Metrics:");
println!("  Hit Rate: {:.2}%", stats.hit_rate());
println!("  Total Requests: {}", stats.hits + stats.misses);
println!("  Cache Size: {} entries", stats.size);
println!("  Evictions: {}", stats.evictions);
```

### Integration with Observability

MockForge's GraphQL module integrates with the observability system:

```rust
use mockforge_observability::get_global_registry;

let registry = get_global_registry();

// GraphQL-specific metrics are automatically recorded
// - mockforge_graphql_requests_total
// - mockforge_graphql_request_duration_seconds
// - mockforge_graphql_cache_hits_total
// - mockforge_graphql_cache_misses_total
```

---

## 🔧 Configuration Examples

### Complete Configuration File

```yaml
# mockforge.yaml
graphql:
  enabled: true
  port: 4000
  schema_path: ./schema.graphql

  # Hot-reloading
  hot_reload: true

  # Caching
  cache:
    enabled: true
    max_entries: 1000
    ttl_seconds: 300
    enable_stats: true
    cacheable_operations:
      - getUser
      - getProduct
      - listProducts

  # Subscriptions
  subscriptions:
    enabled: true
    max_connections: 1000

  # Performance
  observability:
    enable_metrics: true
    enable_tracing: true

  # Playground
  playground_enabled: true
  introspection_enabled: true

  # Upstream passthrough
  upstream_url: null  # Optional
```

### Environment Variables

```bash
# Enable hot-reload
export MOCKFORGE_GRAPHQL_HOT_RELOAD=true

# Cache configuration
export MOCKFORGE_GRAPHQL_CACHE_ENABLED=true
export MOCKFORGE_GRAPHQL_CACHE_MAX_ENTRIES=1000
export MOCKFORGE_GRAPHQL_CACHE_TTL=300

# Subscriptions
export MOCKFORGE_GRAPHQL_SUBSCRIPTIONS_ENABLED=true
export MOCKFORGE_GRAPHQL_MAX_CONNECTIONS=1000
```

---

## 📊 Best Practices

### 1. Schema Organization

```graphql
# Split schemas into modules
# schema/query.graphql
type Query {
  user(id: ID!): User
}

# schema/mutation.graphql
type Mutation {
  createUser(input: CreateUserInput!): User
}

# schema/subscription.graphql
type Subscription {
  userChanged(id: ID!): User
}
```

### 2. Cache Strategy

```rust
// Cache read-heavy operations
let cacheable = vec![
    "getUser",
    "getProduct",
    "listProducts",
];

// Don't cache mutations or real-time data
let non_cacheable = vec![
    "createUser",
    "updateOrder",
    "orderStatusChanged",  // Subscription
];
```

### 3. Subscription Cleanup

```rust
// Clean up subscriptions on disconnect
impl Drop for SubscriptionConnection {
    fn drop(&mut self) {
        manager.unsubscribe(&self.id);
    }
}
```

### 4. Performance Optimization

```rust
// Use appropriate TTL based on data freshness requirements
let config = CacheConfig {
    ttl: match operation_type {
        "user" => Duration::from_secs(60),      // 1 minute
        "product" => Duration::from_secs(300),  // 5 minutes
        "analytics" => Duration::from_secs(3600), // 1 hour
        _ => Duration::from_secs(60),
    },
    ..Default::default()
};
```

---

## 🧪 Testing

### Testing with Cache

```rust
#[tokio::test]
async fn test_cached_response() {
    let cache = ResponseCache::default();
    let key = CacheKey::from_request(
        Some("getUser".to_string()),
        "query { user { id } }".to_string(),
        serde_json::json!({"id": "123"}),
    );

    // First request (cache miss)
    let response1 = execute_query(&key).await;
    cache.put(key.clone(), response1.clone());

    // Second request (cache hit)
    let response2 = cache.get(&key);
    assert!(response2.is_some());

    let stats = cache.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 0);
}
```

### Testing Subscriptions

```rust
#[tokio::test]
async fn test_subscription_broadcast() {
    let manager = SubscriptionManager::new();

    let mut recv1 = manager.subscribe("1".to_string(), "test".to_string(), None);
    let mut recv2 = manager.subscribe("2".to_string(), "test".to_string(), None);

    let event = SubscriptionEvent::new(
        "test".to_string(),
        Value::String("data".to_string()),
    );

    manager.publish(event);

    assert!(recv1.try_recv().is_ok());
    assert!(recv2.try_recv().is_ok());
}
```

---

## 📚 API Reference

### SchemaWatcher

- `new(path)` - Create watcher for schema file
- `start_watching()` - Start file monitoring
- `get_schema()` - Get current schema SDL
- `reload()` - Manually reload schema

### ResponseCache

- `new(config)` - Create with configuration
- `get(key)` - Get cached response
- `put(key, response)` - Cache a response
- `clear()` - Clear all entries
- `clear_expired()` - Remove expired entries
- `stats()` - Get cache statistics

### SubscriptionManager

- `new()` - Create manager
- `subscribe(id, topic, operation)` - Subscribe to topic
- `unsubscribe(id)` - Remove subscription
- `publish(event)` - Broadcast event
- `topics()` - List active topics
- `subscriber_count(topic)` - Count subscribers
- `active_subscriptions()` - List all subscriptions

---

## 🎓 Learn More

- [GraphQL Specification](https://spec.graphql.org/)
- [MockForge Documentation](https://docs.mockforge.dev/)
- [async-graphql Guide](https://async-graphql.github.io/async-graphql/)
