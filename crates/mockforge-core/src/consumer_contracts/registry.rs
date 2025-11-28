//! Consumer registry for tracking API consumers
//!
//! This module provides functionality for registering and querying consumers.

use crate::consumer_contracts::types::{Consumer, ConsumerIdentifier};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Registry for tracking consumers
#[derive(Debug, Clone)]
pub struct ConsumerRegistry {
    /// Consumers indexed by ID
    consumers: Arc<RwLock<HashMap<String, Consumer>>>,
    /// Index by identifier for fast lookup
    identifier_index: Arc<RwLock<HashMap<ConsumerIdentifier, String>>>,
}

impl ConsumerRegistry {
    /// Create a new consumer registry
    pub fn new() -> Self {
        Self {
            consumers: Arc::new(RwLock::new(HashMap::new())),
            identifier_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a consumer
    pub async fn register(&self, consumer: Consumer) {
        let id = consumer.id.clone();
        let identifier = consumer.identifier.clone();

        // Store consumer
        {
            let mut consumers = self.consumers.write().await;
            consumers.insert(id.clone(), consumer);
        }

        // Update identifier index
        {
            let mut index = self.identifier_index.write().await;
            index.insert(identifier, id);
        }
    }

    /// Get consumer by ID
    pub async fn get_by_id(&self, id: &str) -> Option<Consumer> {
        let consumers = self.consumers.read().await;
        consumers.get(id).cloned()
    }

    /// Get consumer by identifier
    pub async fn get_by_identifier(&self, identifier: &ConsumerIdentifier) -> Option<Consumer> {
        let consumer_id = {
            let index = self.identifier_index.read().await;
            index.get(identifier).cloned()
        };

        if let Some(consumer_id) = consumer_id {
            self.get_by_id(&consumer_id).await
        } else {
            None
        }
    }

    /// List all consumers
    pub async fn list_all(&self) -> Vec<Consumer> {
        let consumers = self.consumers.read().await;
        consumers.values().cloned().collect()
    }

    /// Remove a consumer
    pub async fn remove(&self, id: &str) -> Option<Consumer> {
        let consumer = {
            let mut consumers = self.consumers.write().await;
            consumers.remove(id)
        };

        if let Some(ref consumer) = consumer {
            let mut index = self.identifier_index.write().await;
            index.remove(&consumer.identifier);
        }

        consumer
    }

    /// Create or get consumer by identifier
    pub async fn get_or_create(
        &self,
        identifier: ConsumerIdentifier,
        name: String,
        workspace_id: Option<String>,
    ) -> Consumer {
        // Try to get existing consumer
        if let Some(consumer) = self.get_by_identifier(&identifier).await {
            return consumer;
        }

        // Create new consumer
        let id = Uuid::new_v4().to_string();
        let consumer = Consumer::new(id, identifier, name, workspace_id);
        self.register(consumer.clone()).await;
        consumer
    }
}

impl Default for ConsumerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
