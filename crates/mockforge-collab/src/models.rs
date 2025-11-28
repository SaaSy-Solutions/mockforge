//! Core data models for collaboration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User role in a workspace
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Full access including workspace management
    Admin,
    /// Can create and edit mocks
    Editor,
    /// Read-only access
    Viewer,
}

impl UserRole {
    /// Check if this role can perform admin actions
    #[must_use]
    pub const fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }

    /// Check if this role can edit
    #[must_use]
    pub const fn can_edit(&self) -> bool {
        matches!(self, Self::Admin | Self::Editor)
    }

    /// Check if this role can view
    #[must_use]
    pub const fn can_view(&self) -> bool {
        true // All roles can view
    }
}

/// User account
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    /// Unique user ID
    pub id: Uuid,
    /// Username (unique)
    pub username: String,
    /// Email address (unique)
    pub email: String,
    /// Password hash (not serialized)
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// Display name
    pub display_name: Option<String>,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Account created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Whether the account is active
    pub is_active: bool,
}

impl User {
    /// Create a new user (for insertion)
    #[must_use]
    pub fn new(username: String, email: String, password_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            username,
            email,
            password_hash,
            display_name: None,
            avatar_url: None,
            created_at: now,
            updated_at: now,
            is_active: true,
        }
    }
}

/// Team workspace for collaboration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamWorkspace {
    /// Unique workspace ID
    pub id: Uuid,
    /// Workspace name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Owner user ID
    pub owner_id: Uuid,
    /// Workspace configuration (JSON)
    pub config: serde_json::Value,
    /// Current version number
    pub version: i64,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Whether the workspace is archived
    pub is_archived: bool,
}

impl TeamWorkspace {
    /// Create a new workspace
    #[must_use]
    pub fn new(name: String, owner_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            owner_id,
            config: serde_json::json!({}),
            version: 1,
            created_at: now,
            updated_at: now,
            is_archived: false,
        }
    }
}

/// Workspace membership
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkspaceMember {
    /// Unique membership ID
    pub id: Uuid,
    /// Workspace ID
    pub workspace_id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// Role in this workspace
    pub role: UserRole,
    /// When the user joined
    pub joined_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
}

impl WorkspaceMember {
    /// Create a new workspace member
    #[must_use]
    pub fn new(workspace_id: Uuid, user_id: Uuid, role: UserRole) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            user_id,
            role,
            joined_at: now,
            last_activity: now,
        }
    }
}

/// Workspace invitation
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkspaceInvitation {
    /// Unique invitation ID
    pub id: Uuid,
    /// Workspace ID
    pub workspace_id: Uuid,
    /// Email address to invite
    pub email: String,
    /// Role to assign
    pub role: UserRole,
    /// User who sent the invitation
    pub invited_by: Uuid,
    /// Invitation token
    pub token: String,
    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Whether the invitation was accepted
    pub accepted: bool,
}

/// Active user session in a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSession {
    /// User ID
    pub user_id: Uuid,
    /// Workspace ID
    pub workspace_id: Uuid,
    /// Session ID
    pub session_id: Uuid,
    /// When the session started
    pub connected_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Current cursor position (for presence)
    pub cursor: Option<CursorPosition>,
}

/// Cursor position for presence awareness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    /// File or resource being edited
    pub resource: String,
    /// Line number (if applicable)
    pub line: Option<u32>,
    /// Column number (if applicable)
    pub column: Option<u32>,
}

/// Workspace fork relationship
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkspaceFork {
    /// Unique fork ID
    pub id: Uuid,
    /// Source workspace ID (the original)
    pub source_workspace_id: Uuid,
    /// Forked workspace ID (the copy)
    pub forked_workspace_id: Uuid,
    /// When the fork was created
    pub forked_at: DateTime<Utc>,
    /// User who created the fork
    pub forked_by: Uuid,
    /// Commit ID at which fork was created (fork point)
    pub fork_point_commit_id: Option<Uuid>,
}

impl WorkspaceFork {
    /// Create a new fork record
    #[must_use]
    pub fn new(
        source_workspace_id: Uuid,
        forked_workspace_id: Uuid,
        forked_by: Uuid,
        fork_point_commit_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_workspace_id,
            forked_workspace_id,
            forked_by,
            fork_point_commit_id,
            forked_at: Utc::now(),
        }
    }
}

/// Merge status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "merge_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MergeStatus {
    /// Merge is pending
    Pending,
    /// Merge is in progress
    InProgress,
    /// Merge completed successfully
    Completed,
    /// Merge has conflicts that need resolution
    Conflict,
    /// Merge was cancelled
    Cancelled,
}

/// Workspace merge operation
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkspaceMerge {
    /// Unique merge ID
    pub id: Uuid,
    /// Source workspace ID (being merged FROM)
    pub source_workspace_id: Uuid,
    /// Target workspace ID (being merged INTO)
    pub target_workspace_id: Uuid,
    /// Common ancestor commit ID
    pub base_commit_id: Uuid,
    /// Latest commit from source workspace
    pub source_commit_id: Uuid,
    /// Latest commit from target workspace
    pub target_commit_id: Uuid,
    /// Resulting merge commit ID (None if not completed)
    pub merge_commit_id: Option<Uuid>,
    /// Merge status
    pub status: MergeStatus,
    /// Conflict data (JSON array of conflicts)
    pub conflict_data: Option<serde_json::Value>,
    /// User who performed the merge
    pub merged_by: Option<Uuid>,
    /// When the merge was completed
    pub merged_at: Option<DateTime<Utc>>,
    /// When the merge was created
    pub created_at: DateTime<Utc>,
}

impl WorkspaceMerge {
    /// Create a new merge operation
    #[must_use]
    pub fn new(
        source_workspace_id: Uuid,
        target_workspace_id: Uuid,
        base_commit_id: Uuid,
        source_commit_id: Uuid,
        target_commit_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_workspace_id,
            target_workspace_id,
            base_commit_id,
            source_commit_id,
            target_commit_id,
            merge_commit_id: None,
            status: MergeStatus::Pending,
            conflict_data: None,
            merged_by: None,
            merged_at: None,
            created_at: Utc::now(),
        }
    }
}

/// Conflict in a merge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeConflict {
    /// Path to the conflicting field
    pub path: String,
    /// Base value (common ancestor)
    pub base_value: Option<serde_json::Value>,
    /// Source value (from workspace being merged)
    pub source_value: Option<serde_json::Value>,
    /// Target value (from current workspace)
    pub target_value: Option<serde_json::Value>,
    /// Conflict type
    pub conflict_type: ConflictType,
}

/// Type of conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictType {
    /// Both sides modified the same field
    Modified,
    /// Field was deleted in one side, modified in the other
    DeletedModified,
    /// Field was added in both sides with different values
    BothAdded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_permissions() {
        assert!(UserRole::Admin.is_admin());
        assert!(UserRole::Admin.can_edit());
        assert!(UserRole::Admin.can_view());

        assert!(!UserRole::Editor.is_admin());
        assert!(UserRole::Editor.can_edit());
        assert!(UserRole::Editor.can_view());

        assert!(!UserRole::Viewer.is_admin());
        assert!(!UserRole::Viewer.can_edit());
        assert!(UserRole::Viewer.can_view());
    }

    #[test]
    fn test_user_creation() {
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hashed_password".to_string(),
        );

        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert!(user.is_active);
    }

    #[test]
    fn test_workspace_creation() {
        let owner_id = Uuid::new_v4();
        let workspace = TeamWorkspace::new("Test Workspace".to_string(), owner_id);

        assert_eq!(workspace.name, "Test Workspace");
        assert_eq!(workspace.owner_id, owner_id);
        assert_eq!(workspace.version, 1);
        assert!(!workspace.is_archived);
    }

    #[test]
    fn test_workspace_member_creation() {
        let workspace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let member = WorkspaceMember::new(workspace_id, user_id, UserRole::Editor);

        assert_eq!(member.workspace_id, workspace_id);
        assert_eq!(member.user_id, user_id);
        assert_eq!(member.role, UserRole::Editor);
    }
}
