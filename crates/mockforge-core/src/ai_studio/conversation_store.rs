//! Conversation storage for AI Studio chat sessions
//!
//! This module provides persistent storage for conversation history, allowing
//! multi-turn conversations in the AI Studio. It supports both in-memory
//! (for development) and file-based (for production) storage.

use crate::ai_studio::chat_orchestrator::{ChatContext, ChatMessage};
use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Conversation storage backend
pub struct ConversationStore {
    /// In-memory cache of conversations
    cache: Arc<RwLock<HashMap<String, Conversation>>>,
    /// Storage path (if using file-based persistence)
    storage_path: Option<PathBuf>,
}

/// A conversation with its history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Unique conversation ID
    pub id: String,
    /// Workspace ID this conversation belongs to
    pub workspace_id: Option<String>,
    /// Conversation history
    pub messages: Vec<ChatMessage>,
    /// When the conversation was created
    pub created_at: DateTime<Utc>,
    /// When the conversation was last updated
    pub updated_at: DateTime<Utc>,
    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Conversation {
    /// Create a new conversation
    pub fn new(workspace_id: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            workspace_id,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// Convert to ChatContext
    pub fn to_context(&self) -> ChatContext {
        ChatContext {
            history: self.messages.clone(),
            workspace_id: self.workspace_id.clone(),
        }
    }
}

impl ConversationStore {
    /// Create a new in-memory conversation store
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            storage_path: None,
        }
    }

    /// Create a new conversation store with file-based persistence
    pub fn with_persistence<P: AsRef<Path>>(storage_path: P) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            storage_path: Some(storage_path.as_ref().to_path_buf()),
        }
    }

    /// Initialize the store (load from disk if using persistence)
    pub async fn initialize(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            // Ensure directory exists
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    crate::Error::generic(format!("Failed to create storage directory: {}", e))
                })?;
            }

            // Load existing conversations if file exists
            if path.exists() {
                let content = fs::read_to_string(path).await.map_err(|e| {
                    crate::Error::generic(format!("Failed to read conversation store: {}", e))
                })?;

                let conversations: Vec<Conversation> =
                    serde_json::from_str(&content).map_err(|e| {
                        crate::Error::generic(format!("Failed to parse conversation store: {}", e))
                    })?;

                let mut cache = self.cache.write().await;
                for conv in conversations {
                    cache.insert(conv.id.clone(), conv);
                }
            }
        }

        Ok(())
    }

    /// Save conversations to disk (if using persistence)
    async fn persist(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            let cache = self.cache.read().await;
            let conversations: Vec<&Conversation> = cache.values().collect();

            let content = serde_json::to_string_pretty(&conversations).map_err(|e| {
                crate::Error::generic(format!("Failed to serialize conversations: {}", e))
            })?;

            fs::write(path, content).await.map_err(|e| {
                crate::Error::generic(format!("Failed to write conversation store: {}", e))
            })?;
        }

        Ok(())
    }

    /// Create a new conversation
    pub async fn create_conversation(&self, workspace_id: Option<String>) -> Result<String> {
        let conversation = Conversation::new(workspace_id);
        let id = conversation.id.clone();

        {
            let mut cache = self.cache.write().await;
            cache.insert(id.clone(), conversation);
        }

        self.persist().await?;
        Ok(id)
    }

    /// Get a conversation by ID
    pub async fn get_conversation(&self, id: &str) -> Result<Option<Conversation>> {
        let cache = self.cache.read().await;
        Ok(cache.get(id).cloned())
    }

    /// Add a message to a conversation
    pub async fn add_message(&self, conversation_id: &str, message: ChatMessage) -> Result<()> {
        let mut cache = self.cache.write().await;
        if let Some(conversation) = cache.get_mut(conversation_id) {
            conversation.add_message(message);
            self.persist().await?;
            Ok(())
        } else {
            Err(crate::Error::generic(format!("Conversation not found: {}", conversation_id)))
        }
    }

    /// Get conversation context for chat
    pub async fn get_context(&self, conversation_id: &str) -> Result<Option<ChatContext>> {
        let conversation = self.get_conversation(conversation_id).await?;
        Ok(conversation.map(|c| c.to_context()))
    }

    /// List conversations for a workspace
    pub async fn list_conversations(
        &self,
        workspace_id: Option<&str>,
    ) -> Result<Vec<Conversation>> {
        let cache = self.cache.read().await;
        let conversations: Vec<Conversation> = cache
            .values()
            .filter(|conv| {
                if let Some(wid) = workspace_id {
                    conv.workspace_id.as_deref() == Some(wid)
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        Ok(conversations)
    }

    /// Delete a conversation
    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.remove(conversation_id);
        self.persist().await?;
        Ok(())
    }

    /// Clear old conversations (older than specified days)
    pub async fn clear_old_conversations(&self, days: u64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        let mut cache = self.cache.write().await;
        let mut removed = 0;

        cache.retain(|_, conv| {
            if conv.updated_at < cutoff {
                removed += 1;
                false
            } else {
                true
            }
        });

        if removed > 0 {
            self.persist().await?;
        }

        Ok(removed)
    }
}

impl Default for ConversationStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Global conversation store instance
static CONVERSATION_STORE: once_cell::sync::Lazy<Arc<ConversationStore>> =
    once_cell::sync::Lazy::new(|| {
        // Use file-based storage in .mockforge directory
        let storage_path = dirs::home_dir()
            .map(|home| home.join(".mockforge").join("conversations.json"))
            .or_else(|| Some(PathBuf::from(".mockforge/conversations.json")));

        if let Some(path) = storage_path {
            Arc::new(ConversationStore::with_persistence(path))
        } else {
            Arc::new(ConversationStore::new())
        }
    });

/// Get the global conversation store
pub fn get_conversation_store() -> Arc<ConversationStore> {
    CONVERSATION_STORE.clone()
}

/// Initialize the global conversation store
pub async fn initialize_conversation_store() -> Result<()> {
    CONVERSATION_STORE.initialize().await
}
