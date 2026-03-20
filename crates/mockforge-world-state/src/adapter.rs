//! Adapter implementing the `WorldStateMiddlewareTrait` from `mockforge-core`.
//!
//! This module bridges the concrete `WorldStateEngine` implementation
//! with the trait-based interface defined in `mockforge-core::middleware_traits`.

use crate::engine::WorldStateEngine;
use crate::query::WorldStateQuery;
use async_trait::async_trait;
use mockforge_core::middleware_traits::{self, MiddlewareResult};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Adapter that wraps [`WorldStateEngine`] and implements the core
/// [`WorldStateMiddlewareTrait`](middleware_traits::WorldStateMiddleware).
pub struct WorldStateMiddlewareAdapter {
    inner: Arc<RwLock<WorldStateEngine>>,
}

impl WorldStateMiddlewareAdapter {
    /// Create a new adapter wrapping the given world state engine.
    ///
    /// The engine is wrapped in `Arc<RwLock<>>` because `WorldStateEngine`
    /// requires `&self` for queries and `&mut self` for registration.
    pub fn new(engine: Arc<RwLock<WorldStateEngine>>) -> Self {
        Self { inner: engine }
    }

    /// Create a new adapter from a freshly constructed engine.
    pub fn from_engine(engine: WorldStateEngine) -> Self {
        Self {
            inner: Arc::new(RwLock::new(engine)),
        }
    }

    /// Get a reference to the inner engine lock.
    pub fn inner(&self) -> &Arc<RwLock<WorldStateEngine>> {
        &self.inner
    }
}

#[async_trait]
impl middleware_traits::WorldStateMiddleware for WorldStateMiddlewareAdapter {
    async fn snapshot(&self) -> MiddlewareResult<Value> {
        let engine = self.inner.read().await;
        let snapshot = engine
            .create_snapshot()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?;
        serde_json::to_value(&snapshot)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn query(&self, query: Value) -> MiddlewareResult<Value> {
        let ws_query: WorldStateQuery = serde_json::from_value(query)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let engine = self.inner.read().await;
        let result = engine
            .query(&ws_query)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?;

        serde_json::to_value(&result)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    fn layers(&self) -> Vec<String> {
        // We can't hold the async lock in a sync fn, so use try_read.
        match self.inner.try_read() {
            Ok(engine) => engine.get_layers().iter().map(|l| l.name().to_string()).collect(),
            Err(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::middleware_traits::WorldStateMiddleware as WorldStateMiddlewareTrait;

    #[tokio::test]
    async fn test_adapter_snapshot_empty() {
        let engine = WorldStateEngine::new();
        let adapter = WorldStateMiddlewareAdapter::from_engine(engine);

        let snapshot = adapter.snapshot().await.unwrap();
        assert!(snapshot["nodes"].is_array());
        assert!(snapshot["edges"].is_array());
    }

    #[tokio::test]
    async fn test_adapter_query() {
        let engine = WorldStateEngine::new();
        let adapter = WorldStateMiddlewareAdapter::from_engine(engine);

        let query = serde_json::json!({
            "include_edges": true
        });

        let result = adapter.query(query).await.unwrap();
        assert!(result["nodes"].is_array());
    }

    #[tokio::test]
    async fn test_adapter_layers_empty() {
        let engine = WorldStateEngine::new();
        let adapter = WorldStateMiddlewareAdapter::from_engine(engine);

        let layers = adapter.layers();
        assert!(layers.is_empty());
    }

    #[tokio::test]
    async fn test_adapter_as_trait_object() {
        let engine = WorldStateEngine::new();
        let adapter = WorldStateMiddlewareAdapter::from_engine(engine);

        let trait_obj: Arc<dyn WorldStateMiddlewareTrait> = Arc::new(adapter);

        let snapshot = trait_obj.snapshot().await.unwrap();
        assert!(snapshot["nodes"].is_array());
    }

    #[tokio::test]
    async fn test_adapter_invalid_query() {
        let engine = WorldStateEngine::new();
        let adapter = WorldStateMiddlewareAdapter::from_engine(engine);

        // Invalid query should fail deserialization gracefully
        // Actually, WorldStateQuery has all optional fields with defaults,
        // so even a number would fail but an empty object would work.
        let query = serde_json::json!(42);
        let result = adapter.query(query).await;
        assert!(result.is_err());
    }
}
