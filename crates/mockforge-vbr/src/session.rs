//! Session integration
//!
//! This module integrates with existing SessionManager from mockforge-core/intelligent_behavior
//! for session-scoped data and session expiration handling.

use crate::config::StorageBackend;
use crate::database::{create_database, VirtualDatabase};
use crate::Result;
use mockforge_core::intelligent_behavior::session::SessionManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Session-scoped data manager
///
/// Manages per-session virtual databases when session-scoped data is enabled.
/// Each session gets its own isolated database instance.
pub struct SessionDataManager {
    /// Session manager from mockforge-core
    pub session_manager: Arc<SessionManager>,
    /// Storage backend configuration for session databases
    storage_backend: StorageBackend,
    /// Per-session database instances (in-memory by default)
    session_databases: Arc<RwLock<HashMap<String, Arc<dyn VirtualDatabase + Send + Sync>>>>,
}

impl SessionDataManager {
    /// Create a new session data manager
    pub fn new(session_manager: Arc<SessionManager>, storage_backend: StorageBackend) -> Self {
        Self {
            session_manager,
            storage_backend,
            session_databases: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create session-scoped database instance
    ///
    /// If session-scoped data is enabled, each session gets its own isolated
    /// database. This allows different users/sessions to have separate data.
    pub async fn get_session_database(
        &self,
        session_id: &str,
    ) -> Result<Arc<dyn VirtualDatabase + Send + Sync>> {
        // Check if session exists in SessionManager
        let session_state = self.session_manager.get_session(session_id).await;
        if session_state.is_none() {
            return Err(crate::Error::generic(format!("Session '{}' not found", session_id)));
        }

        // Check if we already have a database for this session
        let databases = self.session_databases.read().await;
        if let Some(db) = databases.get(session_id) {
            return Ok(Arc::clone(db));
        }
        drop(databases);

        // Create a new in-memory database for this session
        // Sessions always use in-memory storage for isolation
        let db_arc = create_database(&StorageBackend::Memory).await?;
        // Note: initialize() is called during create_database, so we don't need to call it again
        let db_clone = Arc::clone(&db_arc);

        // Store it for future use
        let mut databases = self.session_databases.write().await;
        databases.insert(session_id.to_string(), db_arc);

        Ok(db_clone)
    }

    /// Clean up database for a session
    pub async fn cleanup_session_database(&self, session_id: &str) -> Result<()> {
        let mut databases = self.session_databases.write().await;
        databases.remove(session_id);
        // Database will be dropped when Arc is dropped
        Ok(())
    }

    /// Clean up databases for expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<usize> {
        // Get expired sessions from SessionManager
        let expired_count = self.session_manager.cleanup_expired_sessions().await;

        // Get list of active sessions
        let active_sessions = self.session_manager.list_sessions().await;
        let active_set: std::collections::HashSet<String> = active_sessions.into_iter().collect();

        // Remove databases for sessions that are no longer active
        let mut databases = self.session_databases.write().await;
        let mut removed = 0;
        let expired_db_sessions: Vec<String> = databases
            .keys()
            .filter(|session_id| !active_set.contains(*session_id))
            .cloned()
            .collect();

        for session_id in expired_db_sessions {
            databases.remove(&session_id);
            removed += 1;
        }

        Ok(removed)
    }

    /// Get the session manager
    pub fn session_manager(&self) -> &Arc<SessionManager> {
        &self.session_manager
    }

    /// Get session state (returns session if it exists)
    pub async fn get_session_state(
        &self,
        session_id: &str,
    ) -> Option<mockforge_core::intelligent_behavior::types::SessionState> {
        self.session_manager.get_session(session_id).await
    }

    /// Update session state
    pub async fn update_session_state(
        &self,
        session_id: &str,
        state: mockforge_core::intelligent_behavior::types::SessionState,
    ) -> Result<()> {
        self.session_manager.update_session(session_id, state).await
    }
}
