//! Vector memory store for long-term semantic memory
//!
//! This module provides persistent memory using vector embeddings for
//! semantic search over past interactions.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::config::VectorStoreConfig;
use super::embedding_client::{cosine_similarity, EmbeddingClient};
use super::types::InteractionRecord;
use crate::Result;

/// Vector memory store for persistent, searchable interaction history
pub struct VectorMemoryStore {
    /// In-memory storage (session_id -> interactions)
    storage: Arc<RwLock<HashMap<String, Vec<InteractionRecord>>>>,

    /// Embedding client
    embedding_client: Option<Arc<EmbeddingClient>>,

    /// Configuration
    config: VectorStoreConfig,
}

impl VectorMemoryStore {
    /// Create a new vector memory store
    pub fn new(config: VectorStoreConfig) -> Self {
        let embedding_client = if config.enabled {
            Some(Arc::new(EmbeddingClient::new(
                config.embedding_provider.clone(),
                config.embedding_model.clone(),
                None,
                None,
            )))
        } else {
            None
        };

        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            embedding_client,
            config,
        }
    }

    /// Store an interaction with semantic embedding
    ///
    /// # Arguments
    /// * `session_id` - Session identifier
    /// * `interaction` - Interaction to store
    pub async fn store_interaction(
        &self,
        session_id: &str,
        interaction: &InteractionRecord,
    ) -> Result<()> {
        let mut interaction_with_embedding = interaction.clone();

        // Generate embedding if enabled
        if let Some(ref client) = self.embedding_client {
            let summary = interaction.summary();
            match client.generate_embedding(&summary).await {
                Ok(embedding) => {
                    interaction_with_embedding.embedding = Some(embedding);
                }
                Err(e) => {
                    tracing::warn!("Failed to generate embedding: {}", e);
                    // Continue without embedding
                }
            }
        }

        let mut storage = self.storage.write().await;
        storage
            .entry(session_id.to_string())
            .or_insert_with(Vec::new)
            .push(interaction_with_embedding);

        Ok(())
    }

    /// Retrieve relevant past interactions using semantic search
    ///
    /// # Arguments
    /// * `session_id` - Session identifier
    /// * `query` - Search query
    /// * `limit` - Maximum number of results to return
    pub async fn retrieve_context(
        &self,
        session_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<InteractionRecord>> {
        let storage = self.storage.read().await;

        let interactions = storage.get(session_id).cloned().unwrap_or_default();

        // If no embedding client or no interactions, return recent ones
        if self.embedding_client.is_none() || interactions.is_empty() {
            return Ok(interactions.into_iter().rev().take(limit).collect());
        }

        // Generate embedding for query
        let query_embedding = match &self.embedding_client {
            Some(client) => match client.generate_embedding(query).await {
                Ok(emb) => emb,
                Err(e) => {
                    tracing::warn!("Failed to generate query embedding: {}", e);
                    return Ok(interactions.into_iter().rev().take(limit).collect());
                }
            },
            None => return Ok(interactions.into_iter().rev().take(limit).collect()),
        };

        // Calculate similarity scores
        let mut scored_interactions: Vec<(InteractionRecord, f32)> = interactions
            .into_iter()
            .filter_map(|interaction| {
                interaction.embedding.as_ref().map(|emb| {
                    let score = cosine_similarity(&query_embedding, emb);
                    (interaction.clone(), score)
                })
            })
            .collect();

        // Filter by threshold and sort by score
        scored_interactions.retain(|(_, score)| *score >= self.config.similarity_threshold);
        scored_interactions
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top-k results
        Ok(scored_interactions
            .into_iter()
            .take(limit)
            .map(|(interaction, _)| interaction)
            .collect())
    }

    /// Get all interactions for a session
    pub async fn get_session_interactions(
        &self,
        session_id: &str,
    ) -> Result<Vec<InteractionRecord>> {
        let storage = self.storage.read().await;

        Ok(storage.get(session_id).cloned().unwrap_or_default())
    }

    /// Clear all interactions for a session
    pub async fn clear_session(&self, session_id: &str) -> Result<()> {
        let mut storage = self.storage.write().await;
        storage.remove(session_id);
        Ok(())
    }

    /// Clear all stored interactions
    pub async fn clear_all(&self) -> Result<()> {
        let mut storage = self.storage.write().await;
        storage.clear();
        Ok(())
    }
}

impl Default for VectorMemoryStore {
    fn default() -> Self {
        Self::new(VectorStoreConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let store = VectorMemoryStore::new(VectorStoreConfig::default());

        let interaction = InteractionRecord::new(
            "POST",
            "/api/users",
            Some(serde_json::json!({"name": "Alice"})),
            201,
            Some(serde_json::json!({"id": "user_1", "name": "Alice"})),
        );

        store.store_interaction("session_1", &interaction).await.unwrap();

        let retrieved = store.retrieve_context("session_1", "user creation", 10).await.unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].method, "POST");
    }

    #[tokio::test]
    async fn test_clear_session() {
        let store = VectorMemoryStore::new(VectorStoreConfig::default());

        let interaction = InteractionRecord::new("GET", "/api/users", None, 200, None);
        store.store_interaction("session_1", &interaction).await.unwrap();

        store.clear_session("session_1").await.unwrap();

        let retrieved = store.get_session_interactions("session_1").await.unwrap();
        assert!(retrieved.is_empty());
    }
}
