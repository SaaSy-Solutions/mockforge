//! Session management for tracking state across requests

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::types::SessionState;
use crate::Result;

/// Session tracking method
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SessionTrackingMethod {
    /// Track via cookie
    Cookie,
    /// Track via HTTP header
    Header,
    /// Track via query parameter
    QueryParam,
}

impl Default for SessionTrackingMethod {
    fn default() -> Self {
        Self::Cookie
    }
}

/// Session tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionTracking {
    /// Tracking method
    #[serde(default)]
    pub method: SessionTrackingMethod,

    /// Cookie name (if method is Cookie)
    #[serde(default = "default_cookie_name")]
    pub cookie_name: String,

    /// Header name (if method is Header)
    #[serde(default = "default_header_name")]
    pub header_name: String,

    /// Query parameter name (if method is QueryParam)
    #[serde(default = "default_query_param")]
    pub query_param: String,

    /// Automatically create sessions if not present
    #[serde(default = "default_true")]
    pub auto_create: bool,
}

impl Default for SessionTracking {
    fn default() -> Self {
        Self {
            method: SessionTrackingMethod::Cookie,
            cookie_name: default_cookie_name(),
            header_name: default_header_name(),
            query_param: default_query_param(),
            auto_create: true,
        }
    }
}

fn default_cookie_name() -> String {
    "mockforge_session".to_string()
}

fn default_header_name() -> String {
    "X-Session-ID".to_string()
}

fn default_query_param() -> String {
    "session_id".to_string()
}

fn default_true() -> bool {
    true
}

/// Session manager for tracking and managing sessions
pub struct SessionManager {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, SessionState>>>,

    /// Session tracking configuration
    config: SessionTracking,

    /// Session timeout in seconds
    timeout_seconds: u64,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(config: SessionTracking, timeout_seconds: u64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
            timeout_seconds,
        }
    }

    /// Generate a new session ID
    pub fn generate_session_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Get or create a session
    pub async fn get_or_create_session(&self, session_id: Option<String>) -> Result<String> {
        let session_id = match session_id {
            Some(id) => {
                // Check if session exists
                let sessions = self.sessions.read().await;
                if sessions.contains_key(&id) {
                    id
                } else if self.config.auto_create {
                    drop(sessions); // Release read lock
                    let new_id = id.clone();
                    self.create_session(new_id.clone()).await?;
                    new_id
                } else {
                    return Err(crate::Error::generic(format!("Session '{}' not found", id)));
                }
            }
            None => {
                if self.config.auto_create {
                    let new_id = Self::generate_session_id();
                    self.create_session(new_id.clone()).await?;
                    new_id
                } else {
                    return Err(crate::Error::generic(
                        "No session ID provided and auto-create is disabled",
                    ));
                }
            }
        };

        Ok(session_id)
    }

    /// Create a new session
    pub async fn create_session(&self, session_id: String) -> Result<String> {
        let mut sessions = self.sessions.write().await;

        if sessions.contains_key(&session_id) {
            return Err(crate::Error::generic(format!("Session '{}' already exists", session_id)));
        }

        let state = SessionState::new(session_id.clone());
        sessions.insert(session_id.clone(), state);

        Ok(session_id)
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<SessionState> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Update a session
    pub async fn update_session(&self, session_id: &str, state: SessionState) -> Result<()> {
        let mut sessions = self.sessions.write().await;

        if !sessions.contains_key(session_id) {
            return Err(crate::Error::generic(format!("Session '{}' not found", session_id)));
        }

        sessions.insert(session_id.to_string(), state);
        Ok(())
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        Ok(())
    }

    /// List all active session IDs
    pub async fn list_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> usize {
        let timeout = chrono::Duration::seconds(self.timeout_seconds as i64);
        let mut sessions = self.sessions.write().await;

        let expired: Vec<String> = sessions
            .iter()
            .filter(|(_, state)| state.is_inactive(timeout))
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired.len();
        for id in expired {
            sessions.remove(&id);
        }

        count
    }

    /// Get the number of active sessions
    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Clear all sessions
    pub async fn clear_all(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.clear();
    }

    /// Get session tracking configuration
    pub fn config(&self) -> &SessionTracking {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_create_session() {
        let config = SessionTracking::default();
        let manager = SessionManager::new(config, 3600);

        let session_id = manager.create_session("test_session".to_string()).await.unwrap();
        assert_eq!(session_id, "test_session");

        let state = manager.get_session(&session_id).await;
        assert!(state.is_some());
    }

    #[tokio::test]
    async fn test_session_manager_get_or_create() {
        let config = SessionTracking::default();
        let manager = SessionManager::new(config, 3600);

        // Create with auto-create
        let session_id = manager.get_or_create_session(None).await.unwrap();
        assert!(!session_id.is_empty());

        // Get existing
        let same_id = manager.get_or_create_session(Some(session_id.clone())).await.unwrap();
        assert_eq!(session_id, same_id);
    }

    #[tokio::test]
    async fn test_session_manager_delete_session() {
        let config = SessionTracking::default();
        let manager = SessionManager::new(config, 3600);

        let session_id = manager.create_session("test_delete".to_string()).await.unwrap();
        assert!(manager.get_session(&session_id).await.is_some());

        manager.delete_session(&session_id).await.unwrap();
        assert!(manager.get_session(&session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_list_sessions() {
        let config = SessionTracking::default();
        let manager = SessionManager::new(config, 3600);

        manager.create_session("session1".to_string()).await.unwrap();
        manager.create_session("session2".to_string()).await.unwrap();

        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains(&"session1".to_string()));
        assert!(sessions.contains(&"session2".to_string()));
    }

    #[tokio::test]
    async fn test_session_manager_clear_all() {
        let config = SessionTracking::default();
        let manager = SessionManager::new(config, 3600);

        manager.create_session("session1".to_string()).await.unwrap();
        manager.create_session("session2".to_string()).await.unwrap();

        assert_eq!(manager.session_count().await, 2);

        manager.clear_all().await;
        assert_eq!(manager.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_cleanup_expired() {
        let config = SessionTracking::default();
        let manager = SessionManager::new(config, 1); // 1 second timeout

        let session_id = manager.create_session("test_expire".to_string()).await.unwrap();

        // Wait for session to expire
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let cleaned = manager.cleanup_expired_sessions().await;
        assert_eq!(cleaned, 1);
        assert!(manager.get_session(&session_id).await.is_none());
    }
}
