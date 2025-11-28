//! Conversation state management for multi-turn voice interactions
//!
//! This module manages conversation context and state for iterative API building
//! through voice commands.

use crate::openapi::OpenApiSpec;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Conversation manager for handling multi-turn voice interactions
pub struct ConversationManager {
    /// Active conversations indexed by ID
    conversations: HashMap<String, ConversationState>,
}

impl ConversationManager {
    /// Create a new conversation manager
    pub fn new() -> Self {
        Self {
            conversations: HashMap::new(),
        }
    }

    /// Start a new conversation
    pub fn start_conversation(&mut self) -> String {
        let id = Uuid::new_v4().to_string();
        let state = ConversationState {
            id: id.clone(),
            context: ConversationContext {
                conversation_id: id.clone(),
                current_spec: None,
                history: Vec::new(),
                metadata: HashMap::new(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.conversations.insert(id.clone(), state);
        id
    }

    /// Get conversation state by ID
    pub fn get_conversation(&self, id: &str) -> Option<&ConversationState> {
        self.conversations.get(id)
    }

    /// Get mutable conversation state by ID
    pub fn get_conversation_mut(&mut self, id: &str) -> Option<&mut ConversationState> {
        self.conversations.get_mut(id)
    }

    /// Update conversation with new command and resulting spec
    pub fn update_conversation(
        &mut self,
        id: &str,
        command: &str,
        spec: Option<OpenApiSpec>,
    ) -> Result<()> {
        let state = self
            .conversations
            .get_mut(id)
            .ok_or_else(|| crate::Error::generic(format!("Conversation {} not found", id)))?;

        // Add command to history
        state.context.history.push(ConversationEntry {
            timestamp: chrono::Utc::now(),
            command: command.to_string(),
            spec_snapshot: spec
                .as_ref()
                .map(|s| serde_json::to_value(s.spec.clone()).unwrap_or(serde_json::Value::Null)),
        });

        // Update current spec
        state.context.current_spec = spec;

        // Update timestamp
        state.updated_at = chrono::Utc::now();

        Ok(())
    }

    /// Remove a conversation
    pub fn remove_conversation(&mut self, id: &str) -> bool {
        self.conversations.remove(id).is_some()
    }

    /// List all active conversations
    pub fn list_conversations(&self) -> Vec<&ConversationState> {
        self.conversations.values().collect()
    }
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Conversation state for a single conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationState {
    /// Conversation ID
    pub id: String,
    /// Conversation context
    pub context: ConversationContext,
    /// When conversation was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Conversation context containing current state and history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    /// Conversation ID
    pub conversation_id: String,
    /// Current OpenAPI spec (if any)
    #[serde(skip)]
    pub current_spec: Option<OpenApiSpec>,
    /// Command history
    pub history: Vec<ConversationEntry>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Entry in conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEntry {
    /// Timestamp of the entry
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Command that was executed
    pub command: String,
    /// Snapshot of the spec after this command (as JSON)
    #[serde(default)]
    pub spec_snapshot: Option<serde_json::Value>,
}

impl ConversationContext {
    /// Create a new conversation context
    pub fn new(conversation_id: String) -> Self {
        Self {
            conversation_id,
            current_spec: None,
            history: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Update the current spec
    pub fn update_spec(&mut self, spec: OpenApiSpec) {
        self.current_spec = Some(spec);
    }

    /// Get the current spec
    pub fn get_spec(&self) -> Option<&OpenApiSpec> {
        self.current_spec.as_ref()
    }

    /// Add a command to history
    pub fn add_command(&mut self, command: String, spec_snapshot: Option<serde_json::Value>) {
        self.history.push(ConversationEntry {
            timestamp: chrono::Utc::now(),
            command,
            spec_snapshot,
        });
    }

    /// Get conversation summary for LLM context
    pub fn get_summary(&self) -> String {
        let mut summary = format!(
            "Conversation ID: {}\nHistory: {} commands\n",
            self.conversation_id,
            self.history.len()
        );

        if let Some(ref spec) = self.current_spec {
            summary.push_str(&format!(
                "Current API: {}\nVersion: {}\n",
                spec.title(),
                spec.version()
            ));
        } else {
            summary.push_str("Current API: None (new conversation)\n");
        }

        if !self.history.is_empty() {
            summary.push_str("\nRecent commands:\n");
            for entry in self.history.iter().rev().take(5) {
                summary.push_str(&format!("- {}\n", entry.command));
            }
        }

        summary
    }
}
