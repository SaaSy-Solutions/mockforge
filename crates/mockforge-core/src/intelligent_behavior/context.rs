//! Stateful AI context management
//!
//! This module provides the StatefulAiContext which maintains conversation-like
//! state across multiple API requests.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::config::IntelligentBehaviorConfig;
use super::memory::VectorMemoryStore;
use super::types::{InteractionRecord, SessionState};
use crate::Result;

/// Stateful AI context manager
///
/// Tracks state across multiple requests within a session, maintaining
/// conversation history and enabling intelligent, context-aware responses.
pub struct StatefulAiContext {
    /// Session ID
    session_id: String,

    /// Current session state
    state: Arc<RwLock<SessionState>>,

    /// Vector memory store for long-term semantic memory
    memory_store: Option<Arc<VectorMemoryStore>>,

    /// Configuration
    config: IntelligentBehaviorConfig,
}

impl StatefulAiContext {
    /// Create a new stateful AI context
    pub fn new(session_id: impl Into<String>, config: IntelligentBehaviorConfig) -> Self {
        let session_id = session_id.into();
        let state = Arc::new(RwLock::new(SessionState::new(session_id.clone())));

        Self {
            session_id,
            state,
            memory_store: None,
            config,
        }
    }

    /// Create with vector memory store
    pub fn with_memory_store(mut self, store: Arc<VectorMemoryStore>) -> Self {
        self.memory_store = Some(store);
        self
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Record an interaction
    pub async fn record_interaction(
        &mut self,
        method: impl Into<String>,
        path: impl Into<String>,
        request: Option<serde_json::Value>,
        response: Option<serde_json::Value>,
    ) -> Result<()> {
        let interaction = InteractionRecord::new(
            method,
            path,
            request,
            200, // Default status
            response,
        );

        // Store in session state
        let mut state = self.state.write().await;
        state.record_interaction(interaction.clone());

        // Trim history if needed
        let max_history = self.config.performance.max_history_length;
        let history_len = state.history.len();
        if history_len > max_history {
            state.history.drain(0..history_len - max_history);
        }

        drop(state);

        // Store in vector memory if enabled
        if let Some(ref store) = self.memory_store {
            if self.config.vector_store.enabled {
                store.store_interaction(&self.session_id, &interaction).await?;
            }
        }

        Ok(())
    }

    /// Get current session state
    pub async fn get_state(&self) -> SessionState {
        let state = self.state.read().await;
        state.clone()
    }

    /// Set a state value
    pub async fn set_value(&self, key: impl Into<String>, value: serde_json::Value) {
        let mut state = self.state.write().await;
        state.set(key, value);
    }

    /// Get a state value
    pub async fn get_value(&self, key: &str) -> Option<serde_json::Value> {
        let state = self.state.read().await;
        state.get(key).cloned()
    }

    /// Remove a state value
    pub async fn remove_value(&self, key: &str) -> Option<serde_json::Value> {
        let mut state = self.state.write().await;
        state.remove(key)
    }

    /// Get interaction history
    pub async fn get_history(&self) -> Vec<InteractionRecord> {
        let state = self.state.read().await;
        state.history.clone()
    }

    /// Get relevant past interactions using semantic search
    pub async fn get_relevant_context(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<InteractionRecord>> {
        if let Some(ref store) = self.memory_store {
            if self.config.vector_store.enabled {
                return store.retrieve_context(&self.session_id, query, limit).await;
            }
        }

        // Fallback to recent history
        let state = self.state.read().await;
        let history = state.history.clone();
        Ok(history.into_iter().rev().take(limit).collect())
    }

    /// Build context summary for LLM prompt
    pub async fn build_context_summary(&self) -> String {
        let state = self.state.read().await;

        let mut summary = String::new();
        summary.push_str("# Session Context\n\n");

        // Current state
        if !state.state.is_empty() {
            summary.push_str("## Current State\n");
            for (key, value) in &state.state {
                summary.push_str(&format!("- {}: {}\n", key, value));
            }
            summary.push('\n');
        }

        // Recent interactions
        if !state.history.is_empty() {
            summary.push_str("## Recent Interactions\n");
            let recent = state.history.iter().rev().take(5);
            for interaction in recent {
                summary.push_str(&format!(
                    "- {} {} (status {})\n",
                    interaction.method, interaction.path, interaction.status
                ));
            }
        }

        summary
    }

    /// Clear all state
    pub async fn clear(&self) {
        let mut state = self.state.write().await;
        *state = SessionState::new(self.session_id.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_creation() {
        let config = IntelligentBehaviorConfig::default();
        let context = StatefulAiContext::new("test_session", config);

        assert_eq!(context.session_id(), "test_session");
    }

    #[tokio::test]
    async fn test_record_interaction() {
        let config = IntelligentBehaviorConfig::default();
        let mut context = StatefulAiContext::new("test_session", config);

        context.record_interaction(
            "POST",
            "/api/users",
            Some(serde_json::json!({"name": "Alice"})),
            Some(serde_json::json!({"id": "user_1", "name": "Alice"})),
        ).await.unwrap();

        let history = context.get_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].method, "POST");
        assert_eq!(history[0].path, "/api/users");
    }

    #[tokio::test]
    async fn test_state_management() {
        let config = IntelligentBehaviorConfig::default();
        let context = StatefulAiContext::new("test_session", config);

        // Set values
        context.set_value("user_id", serde_json::json!("user_123")).await;
        context.set_value("logged_in", serde_json::json!(true)).await;

        // Get values
        assert_eq!(
            context.get_value("user_id").await,
            Some(serde_json::json!("user_123"))
        );
        assert_eq!(
            context.get_value("logged_in").await,
            Some(serde_json::json!(true))
        );

        // Remove value
        let removed = context.remove_value("logged_in").await;
        assert_eq!(removed, Some(serde_json::json!(true)));
        assert_eq!(context.get_value("logged_in").await, None);
    }

    #[tokio::test]
    async fn test_context_summary() {
        let config = IntelligentBehaviorConfig::default();
        let mut context = StatefulAiContext::new("test_session", config);

        context.set_value("user_id", serde_json::json!("user_1")).await;

        context.record_interaction(
            "POST",
            "/api/login",
            Some(serde_json::json!({"email": "test@example.com"})),
            Some(serde_json::json!({"token": "abc123"})),
        ).await.unwrap();

        let summary = context.build_context_summary().await;

        assert!(summary.contains("Session Context"));
        assert!(summary.contains("user_id"));
        assert!(summary.contains("POST /api/login"));
    }
}
