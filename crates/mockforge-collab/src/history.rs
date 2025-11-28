//! Version control and history tracking

use crate::error::{CollabError, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

/// A commit in the history
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Commit {
    /// Unique commit ID
    pub id: Uuid,
    /// Workspace ID
    pub workspace_id: Uuid,
    /// User who made the commit
    pub author_id: Uuid,
    /// Commit message
    pub message: String,
    /// Parent commit ID (None for initial commit)
    pub parent_id: Option<Uuid>,
    /// Workspace version at this commit
    pub version: i64,
    /// Full workspace state snapshot (JSON)
    pub snapshot: serde_json::Value,
    /// Changes made in this commit (diff)
    pub changes: serde_json::Value,
    /// Timestamp
    pub created_at: chrono::DateTime<Utc>,
}

impl Commit {
    /// Create a new commit
    #[must_use]
    pub fn new(
        workspace_id: Uuid,
        author_id: Uuid,
        message: String,
        parent_id: Option<Uuid>,
        version: i64,
        snapshot: serde_json::Value,
        changes: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            author_id,
            message,
            parent_id,
            version,
            snapshot,
            changes,
            created_at: Utc::now(),
        }
    }
}

/// A named snapshot (like a git tag)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Snapshot {
    /// Unique snapshot ID
    pub id: Uuid,
    /// Workspace ID
    pub workspace_id: Uuid,
    /// Snapshot name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Commit ID this snapshot points to
    pub commit_id: Uuid,
    /// Created by
    pub created_by: Uuid,
    /// Created timestamp
    pub created_at: chrono::DateTime<Utc>,
}

impl Snapshot {
    /// Create a new snapshot
    #[must_use]
    pub fn new(
        workspace_id: Uuid,
        name: String,
        description: Option<String>,
        commit_id: Uuid,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            name,
            description,
            commit_id,
            created_by,
            created_at: Utc::now(),
        }
    }
}

/// Version control system for workspaces
pub struct VersionControl {
    db: Pool<Sqlite>,
}

impl VersionControl {
    /// Create a new version control system
    #[must_use]
    pub const fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    /// Create a commit
    pub async fn create_commit(
        &self,
        workspace_id: Uuid,
        author_id: Uuid,
        message: String,
        parent_id: Option<Uuid>,
        version: i64,
        snapshot: serde_json::Value,
        changes: serde_json::Value,
    ) -> Result<Commit> {
        let commit =
            Commit::new(workspace_id, author_id, message, parent_id, version, snapshot, changes);

        sqlx::query!(
            r#"
            INSERT INTO commits (id, workspace_id, author_id, message, parent_id, version, snapshot, changes, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            commit.id,
            commit.workspace_id,
            commit.author_id,
            commit.message,
            commit.parent_id,
            commit.version,
            commit.snapshot,
            commit.changes,
            commit.created_at
        )
        .execute(&self.db)
        .await?;

        Ok(commit)
    }

    /// Get a commit by ID
    pub async fn get_commit(&self, commit_id: Uuid) -> Result<Commit> {
        sqlx::query_as!(
            Commit,
            r#"
            SELECT
                id as "id: Uuid",
                workspace_id as "workspace_id: Uuid",
                author_id as "author_id: Uuid",
                message,
                parent_id as "parent_id: Uuid",
                version,
                snapshot,
                changes,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
            FROM commits
            WHERE id = ?
            "#,
            commit_id
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| CollabError::Internal(format!("Commit not found: {commit_id}")))
    }

    /// Get commit history for a workspace
    pub async fn get_history(&self, workspace_id: Uuid, limit: Option<i32>) -> Result<Vec<Commit>> {
        let limit = limit.unwrap_or(100);

        let commits = sqlx::query_as!(
            Commit,
            r#"
            SELECT
                id as "id: Uuid",
                workspace_id as "workspace_id: Uuid",
                author_id as "author_id: Uuid",
                message,
                parent_id as "parent_id: Uuid",
                version,
                snapshot,
                changes,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
            FROM commits
            WHERE workspace_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
            workspace_id,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        Ok(commits)
    }

    /// Get the latest commit for a workspace
    pub async fn get_latest_commit(&self, workspace_id: Uuid) -> Result<Option<Commit>> {
        let commit = sqlx::query_as!(
            Commit,
            r#"
            SELECT
                id as "id: Uuid",
                workspace_id as "workspace_id: Uuid",
                author_id as "author_id: Uuid",
                message,
                parent_id as "parent_id: Uuid",
                version,
                snapshot,
                changes,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
            FROM commits
            WHERE workspace_id = ?
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            workspace_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(commit)
    }

    /// Create a named snapshot
    pub async fn create_snapshot(
        &self,
        workspace_id: Uuid,
        name: String,
        description: Option<String>,
        commit_id: Uuid,
        created_by: Uuid,
    ) -> Result<Snapshot> {
        // Verify commit exists
        self.get_commit(commit_id).await?;

        let snapshot = Snapshot::new(workspace_id, name, description, commit_id, created_by);

        sqlx::query!(
            r#"
            INSERT INTO snapshots (id, workspace_id, name, description, commit_id, created_by, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            snapshot.id,
            snapshot.workspace_id,
            snapshot.name,
            snapshot.description,
            snapshot.commit_id,
            snapshot.created_by,
            snapshot.created_at
        )
        .execute(&self.db)
        .await?;

        Ok(snapshot)
    }

    /// Get a snapshot by name
    pub async fn get_snapshot(&self, workspace_id: Uuid, name: &str) -> Result<Snapshot> {
        sqlx::query_as!(
            Snapshot,
            r#"
            SELECT
                id as "id: Uuid",
                workspace_id as "workspace_id: Uuid",
                name,
                description,
                commit_id as "commit_id: Uuid",
                created_by as "created_by: Uuid",
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
            FROM snapshots
            WHERE workspace_id = ? AND name = ?
            "#,
            workspace_id,
            name
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| CollabError::Internal(format!("Snapshot not found: {name}")))
    }

    /// List all snapshots for a workspace
    pub async fn list_snapshots(&self, workspace_id: Uuid) -> Result<Vec<Snapshot>> {
        let snapshots = sqlx::query_as!(
            Snapshot,
            r#"
            SELECT
                id as "id: Uuid",
                workspace_id as "workspace_id: Uuid",
                name,
                description,
                commit_id as "commit_id: Uuid",
                created_by as "created_by: Uuid",
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
            FROM snapshots
            WHERE workspace_id = ?
            ORDER BY created_at DESC
            "#,
            workspace_id
        )
        .fetch_all(&self.db)
        .await?;

        Ok(snapshots)
    }

    /// Restore workspace to a specific commit
    pub async fn restore_to_commit(
        &self,
        workspace_id: Uuid,
        commit_id: Uuid,
    ) -> Result<serde_json::Value> {
        let commit = self.get_commit(commit_id).await?;

        if commit.workspace_id != workspace_id {
            return Err(CollabError::InvalidInput(
                "Commit does not belong to this workspace".to_string(),
            ));
        }

        Ok(commit.snapshot)
    }

    /// Compare two commits
    pub async fn diff(&self, from_commit: Uuid, to_commit: Uuid) -> Result<serde_json::Value> {
        let from = self.get_commit(from_commit).await?;
        let to = self.get_commit(to_commit).await?;

        // Simple diff - in production, use a proper diffing library
        let diff = serde_json::json!({
            "from": from.snapshot,
            "to": to.snapshot,
            "changes": to.changes
        });

        Ok(diff)
    }
}

/// History tracking with auto-commit
pub struct History {
    version_control: VersionControl,
    auto_commit: bool,
}

impl History {
    /// Create a new history tracker
    #[must_use]
    pub const fn new(db: Pool<Sqlite>) -> Self {
        Self {
            version_control: VersionControl::new(db),
            auto_commit: true,
        }
    }

    /// Enable/disable auto-commit
    pub const fn set_auto_commit(&mut self, enabled: bool) {
        self.auto_commit = enabled;
    }

    /// Track a change (auto-commit if enabled)
    pub async fn track_change(
        &self,
        workspace_id: Uuid,
        user_id: Uuid,
        message: String,
        new_state: serde_json::Value,
        changes: serde_json::Value,
    ) -> Result<Option<Commit>> {
        if !self.auto_commit {
            return Ok(None);
        }

        let latest = self.version_control.get_latest_commit(workspace_id).await?;
        let parent_id = latest.as_ref().map(|c| c.id);
        let version = latest.map_or(1, |c| c.version + 1);

        let commit = self
            .version_control
            .create_commit(workspace_id, user_id, message, parent_id, version, new_state, changes)
            .await?;

        Ok(Some(commit))
    }

    /// Get history
    pub async fn get_history(&self, workspace_id: Uuid, limit: Option<i32>) -> Result<Vec<Commit>> {
        self.version_control.get_history(workspace_id, limit).await
    }

    /// Create a snapshot
    pub async fn create_snapshot(
        &self,
        workspace_id: Uuid,
        name: String,
        description: Option<String>,
        user_id: Uuid,
    ) -> Result<Snapshot> {
        // Get the latest commit
        let latest = self
            .version_control
            .get_latest_commit(workspace_id)
            .await?
            .ok_or_else(|| CollabError::Internal("No commits found".to_string()))?;

        self.version_control
            .create_snapshot(workspace_id, name, description, latest.id, user_id)
            .await
    }

    /// Restore from snapshot
    pub async fn restore_snapshot(
        &self,
        workspace_id: Uuid,
        snapshot_name: &str,
    ) -> Result<serde_json::Value> {
        let snapshot = self.version_control.get_snapshot(workspace_id, snapshot_name).await?;
        self.version_control.restore_to_commit(workspace_id, snapshot.commit_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_creation() {
        let workspace_id = Uuid::new_v4();
        let author_id = Uuid::new_v4();
        let commit = Commit::new(
            workspace_id,
            author_id,
            "Initial commit".to_string(),
            None,
            1,
            serde_json::json!({}),
            serde_json::json!({}),
        );

        assert_eq!(commit.workspace_id, workspace_id);
        assert_eq!(commit.author_id, author_id);
        assert_eq!(commit.version, 1);
        assert!(commit.parent_id.is_none());
    }

    #[test]
    fn test_snapshot_creation() {
        let workspace_id = Uuid::new_v4();
        let commit_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();
        let snapshot = Snapshot::new(
            workspace_id,
            "v1.0.0".to_string(),
            Some("First release".to_string()),
            commit_id,
            created_by,
        );

        assert_eq!(snapshot.name, "v1.0.0");
        assert_eq!(snapshot.workspace_id, workspace_id);
        assert_eq!(snapshot.commit_id, commit_id);
    }
}
