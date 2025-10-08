//! Collaborative editing support for orchestrations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};

/// User in a collaboration session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub color: String,
    pub cursor: Option<CursorPosition>,
    pub active_field: Option<String>,
    pub joined_at: DateTime<Utc>,
}

/// Cursor position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub x: i32,
    pub y: i32,
}

/// Collaboration change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationChange {
    pub id: String,
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
    pub change_type: ChangeType,
    pub path: String,
    pub value: serde_json::Value,
    pub previous_value: Option<serde_json::Value>,
}

/// Type of change
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Insert,
    Update,
    Delete,
}

/// Collaboration session
#[derive(Debug)]
pub struct CollaborationSession {
    pub orchestration_id: String,
    pub users: Arc<RwLock<HashMap<String, CollaborationUser>>>,
    pub changes: Arc<RwLock<Vec<CollaborationChange>>>,
    pub broadcast_tx: broadcast::Sender<CollaborationMessage>,
    pub created_at: DateTime<Utc>,
}

/// Collaboration message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CollaborationMessage {
    #[serde(rename = "user_joined")]
    UserJoined {
        data: UserJoinedData,
    },
    #[serde(rename = "user_left")]
    UserLeft {
        data: UserLeftData,
    },
    #[serde(rename = "user_presence")]
    UserPresence {
        data: UserPresenceData,
    },
    #[serde(rename = "change")]
    Change {
        data: ChangeData,
    },
    #[serde(rename = "sync")]
    Sync {
        data: SyncData,
    },
    #[serde(rename = "conflict")]
    Conflict {
        data: ConflictData,
    },
    #[serde(rename = "users_list")]
    UsersList {
        data: UsersListData,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserJoinedData {
    pub user: CollaborationUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLeftData {
    pub user_id: String,
    pub user_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresenceData {
    pub user_id: String,
    pub cursor: Option<CursorPosition>,
    pub active_field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeData {
    pub change: CollaborationChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncData {
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictData {
    pub message: String,
    pub conflicting_changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsersListData {
    pub users: Vec<CollaborationUser>,
}

impl CollaborationSession {
    /// Create a new collaboration session
    pub fn new(orchestration_id: String) -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);

        Self {
            orchestration_id,
            users: Arc::new(RwLock::new(HashMap::new())),
            changes: Arc::new(RwLock::new(Vec::new())),
            broadcast_tx,
            created_at: Utc::now(),
        }
    }

    /// Add a user to the session
    pub fn add_user(&self, user: CollaborationUser) -> Result<(), String> {
        let mut users = self.users.write().map_err(|e| e.to_string())?;
        users.insert(user.id.clone(), user.clone());

        // Broadcast user joined
        let _ = self.broadcast_tx.send(CollaborationMessage::UserJoined {
            data: UserJoinedData { user },
        });

        Ok(())
    }

    /// Remove a user from the session
    pub fn remove_user(&self, user_id: &str) -> Result<(), String> {
        let mut users = self.users.write().map_err(|e| e.to_string())?;

        if let Some(user) = users.remove(user_id) {
            // Broadcast user left
            let _ = self.broadcast_tx.send(CollaborationMessage::UserLeft {
                data: UserLeftData {
                    user_id: user_id.to_string(),
                    user_name: user.name,
                },
            });
        }

        Ok(())
    }

    /// Update user presence
    pub fn update_presence(
        &self,
        user_id: &str,
        cursor: Option<CursorPosition>,
        active_field: Option<String>,
    ) -> Result<(), String> {
        let mut users = self.users.write().map_err(|e| e.to_string())?;

        if let Some(user) = users.get_mut(user_id) {
            user.cursor = cursor.clone();
            user.active_field = active_field.clone();

            // Broadcast presence update
            let _ = self.broadcast_tx.send(CollaborationMessage::UserPresence {
                data: UserPresenceData {
                    user_id: user_id.to_string(),
                    cursor,
                    active_field,
                },
            });
        }

        Ok(())
    }

    /// Apply a change
    pub fn apply_change(&self, change: CollaborationChange) -> Result<(), String> {
        // Check for conflicts
        let changes = self.changes.read().map_err(|e| e.to_string())?;
        let conflicts: Vec<_> = changes
            .iter()
            .filter(|c| {
                c.path == change.path
                    && c.user_id != change.user_id
                    && c.timestamp > change.timestamp.checked_sub_signed(chrono::Duration::seconds(5)).unwrap_or(change.timestamp)
            })
            .map(|c| c.id.clone())
            .collect();

        drop(changes);

        if !conflicts.is_empty() {
            // Broadcast conflict
            let _ = self.broadcast_tx.send(CollaborationMessage::Conflict {
                data: ConflictData {
                    message: format!("Conflict detected in path: {}", change.path),
                    conflicting_changes: conflicts,
                },
            });
        }

        // Store change
        let mut changes = self.changes.write().map_err(|e| e.to_string())?;
        changes.push(change.clone());

        // Broadcast change
        let _ = self.broadcast_tx.send(CollaborationMessage::Change {
            data: ChangeData { change },
        });

        Ok(())
    }

    /// Get all active users
    pub fn get_users(&self) -> Result<Vec<CollaborationUser>, String> {
        let users = self.users.read().map_err(|e| e.to_string())?;
        Ok(users.values().cloned().collect())
    }

    /// Get change history
    pub fn get_changes(&self) -> Result<Vec<CollaborationChange>, String> {
        let changes = self.changes.read().map_err(|e| e.to_string())?;
        Ok(changes.clone())
    }

    /// Subscribe to updates
    pub fn subscribe(&self) -> broadcast::Receiver<CollaborationMessage> {
        self.broadcast_tx.subscribe()
    }
}

/// Collaboration manager
pub struct CollaborationManager {
    sessions: Arc<RwLock<HashMap<String, Arc<CollaborationSession>>>>,
}

impl CollaborationManager {
    /// Create a new collaboration manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a session
    pub fn get_or_create_session(&self, orchestration_id: &str) -> Result<Arc<CollaborationSession>, String> {
        let mut sessions = self.sessions.write().map_err(|e| e.to_string())?;

        if let Some(session) = sessions.get(orchestration_id) {
            Ok(Arc::clone(session))
        } else {
            let session = Arc::new(CollaborationSession::new(orchestration_id.to_string()));
            sessions.insert(orchestration_id.to_string(), Arc::clone(&session));
            Ok(session)
        }
    }

    /// Remove a session
    pub fn remove_session(&self, orchestration_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().map_err(|e| e.to_string())?;
        sessions.remove(orchestration_id);
        Ok(())
    }

    /// Get active sessions count
    pub fn active_sessions_count(&self) -> usize {
        self.sessions.read().map(|s| s.len()).unwrap_or(0)
    }
}

impl Default for CollaborationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = CollaborationSession::new("test-orch".to_string());
        assert_eq!(session.orchestration_id, "test-orch");
        assert_eq!(session.get_users().unwrap().len(), 0);
    }

    #[test]
    fn test_add_user() {
        let session = CollaborationSession::new("test-orch".to_string());
        let user = CollaborationUser {
            id: "user1".to_string(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            color: "#FF0000".to_string(),
            cursor: None,
            active_field: None,
            joined_at: Utc::now(),
        };

        session.add_user(user).unwrap();
        assert_eq!(session.get_users().unwrap().len(), 1);
    }

    #[test]
    fn test_remove_user() {
        let session = CollaborationSession::new("test-orch".to_string());
        let user = CollaborationUser {
            id: "user1".to_string(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            color: "#FF0000".to_string(),
            cursor: None,
            active_field: None,
            joined_at: Utc::now(),
        };

        session.add_user(user).unwrap();
        session.remove_user("user1").unwrap();
        assert_eq!(session.get_users().unwrap().len(), 0);
    }

    #[test]
    fn test_manager() {
        let manager = CollaborationManager::new();
        let session1 = manager.get_or_create_session("orch1").unwrap();
        let session2 = manager.get_or_create_session("orch1").unwrap();

        assert_eq!(Arc::ptr_eq(&session1, &session2), true);
        assert_eq!(manager.active_sessions_count(), 1);
    }
}
