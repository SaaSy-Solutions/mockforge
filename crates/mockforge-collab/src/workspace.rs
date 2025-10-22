//! Workspace management and collaboration

use crate::error::{CollabError, Result};
use crate::models::{TeamWorkspace, UserRole, WorkspaceMember};
use crate::permissions::{Permission, PermissionChecker};
use chrono::Utc;
use parking_lot::RwLock;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Workspace service for managing collaborative workspaces
pub struct WorkspaceService {
    db: Pool<Sqlite>,
    cache: Arc<RwLock<HashMap<Uuid, TeamWorkspace>>>,
}

impl WorkspaceService {
    /// Create a new workspace service
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self {
            db,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new workspace
    pub async fn create_workspace(
        &self,
        name: String,
        description: Option<String>,
        owner_id: Uuid,
    ) -> Result<TeamWorkspace> {
        let mut workspace = TeamWorkspace::new(name, owner_id);
        workspace.description = description;

        // Insert into database
        sqlx::query!(
            r#"
            INSERT INTO workspaces (id, name, description, owner_id, config, version, created_at, updated_at, is_archived)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            workspace.id,
            workspace.name,
            workspace.description,
            workspace.owner_id,
            workspace.config,
            workspace.version,
            workspace.created_at,
            workspace.updated_at,
            workspace.is_archived
        )
        .execute(&self.db)
        .await?;

        // Add owner as admin member
        let member = WorkspaceMember::new(workspace.id, owner_id, UserRole::Admin);
        sqlx::query!(
            r#"
            INSERT INTO workspace_members (id, workspace_id, user_id, role, joined_at, last_activity)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            member.id,
            member.workspace_id,
            member.user_id,
            member.role,
            member.joined_at,
            member.last_activity
        )
        .execute(&self.db)
        .await?;

        // Update cache
        self.cache.write().insert(workspace.id, workspace.clone());

        Ok(workspace)
    }

    /// Get a workspace by ID
    pub async fn get_workspace(&self, workspace_id: Uuid) -> Result<TeamWorkspace> {
        // Check cache first
        if let Some(workspace) = self.cache.read().get(&workspace_id) {
            return Ok(workspace.clone());
        }

        // Query database
        let workspace = sqlx::query_as!(
            TeamWorkspace,
            r#"
            SELECT id, name, description, owner_id, config, version, created_at, updated_at, is_archived as "is_archived: bool"
            FROM workspaces
            WHERE id = ?
            "#,
            workspace_id
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| CollabError::WorkspaceNotFound(workspace_id.to_string()))?;

        // Update cache
        self.cache.write().insert(workspace_id, workspace.clone());

        Ok(workspace)
    }

    /// Update a workspace
    pub async fn update_workspace(
        &self,
        workspace_id: Uuid,
        user_id: Uuid,
        name: Option<String>,
        description: Option<String>,
        config: Option<serde_json::Value>,
    ) -> Result<TeamWorkspace> {
        // Check permissions
        let member = self.get_member(workspace_id, user_id).await?;
        PermissionChecker::check(member.role, Permission::WorkspaceUpdate)?;

        let mut workspace = self.get_workspace(workspace_id).await?;

        // Update fields
        if let Some(name) = name {
            workspace.name = name;
        }
        if let Some(description) = description {
            workspace.description = Some(description);
        }
        if let Some(config) = config {
            workspace.config = config;
        }
        workspace.updated_at = Utc::now();
        workspace.version += 1;

        // Save to database
        sqlx::query!(
            r#"
            UPDATE workspaces
            SET name = ?, description = ?, config = ?, version = ?, updated_at = ?
            WHERE id = ?
            "#,
            workspace.name,
            workspace.description,
            workspace.config,
            workspace.version,
            workspace.updated_at,
            workspace.id
        )
        .execute(&self.db)
        .await?;

        // Update cache
        self.cache.write().insert(workspace_id, workspace.clone());

        Ok(workspace)
    }

    /// Delete (archive) a workspace
    pub async fn delete_workspace(&self, workspace_id: Uuid, user_id: Uuid) -> Result<()> {
        // Check permissions
        let member = self.get_member(workspace_id, user_id).await?;
        PermissionChecker::check(member.role, Permission::WorkspaceDelete)?;

        sqlx::query!(
            r#"
            UPDATE workspaces
            SET is_archived = TRUE, updated_at = ?
            WHERE id = ?
            "#,
            Utc::now(),
            workspace_id
        )
        .execute(&self.db)
        .await?;

        // Remove from cache
        self.cache.write().remove(&workspace_id);

        Ok(())
    }

    /// Add a member to a workspace
    pub async fn add_member(
        &self,
        workspace_id: Uuid,
        user_id: Uuid,
        new_member_id: Uuid,
        role: UserRole,
    ) -> Result<WorkspaceMember> {
        // Check permissions
        let member = self.get_member(workspace_id, user_id).await?;
        PermissionChecker::check(member.role, Permission::InviteMembers)?;

        // Create new member
        let new_member = WorkspaceMember::new(workspace_id, new_member_id, role);

        sqlx::query!(
            r#"
            INSERT INTO workspace_members (id, workspace_id, user_id, role, joined_at, last_activity)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            new_member.id,
            new_member.workspace_id,
            new_member.user_id,
            new_member.role,
            new_member.joined_at,
            new_member.last_activity
        )
        .execute(&self.db)
        .await?;

        Ok(new_member)
    }

    /// Remove a member from a workspace
    pub async fn remove_member(
        &self,
        workspace_id: Uuid,
        user_id: Uuid,
        member_to_remove: Uuid,
    ) -> Result<()> {
        // Check permissions
        let member = self.get_member(workspace_id, user_id).await?;
        PermissionChecker::check(member.role, Permission::RemoveMembers)?;

        // Don't allow removing the owner
        let workspace = self.get_workspace(workspace_id).await?;
        if member_to_remove == workspace.owner_id {
            return Err(CollabError::InvalidInput("Cannot remove workspace owner".to_string()));
        }

        sqlx::query!(
            r#"
            DELETE FROM workspace_members
            WHERE workspace_id = ? AND user_id = ?
            "#,
            workspace_id,
            member_to_remove
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Change a member's role
    pub async fn change_role(
        &self,
        workspace_id: Uuid,
        user_id: Uuid,
        member_id: Uuid,
        new_role: UserRole,
    ) -> Result<WorkspaceMember> {
        // Check permissions
        let member = self.get_member(workspace_id, user_id).await?;
        PermissionChecker::check(member.role, Permission::ChangeRoles)?;

        // Don't allow changing the owner's role
        let workspace = self.get_workspace(workspace_id).await?;
        if member_id == workspace.owner_id {
            return Err(CollabError::InvalidInput(
                "Cannot change workspace owner's role".to_string(),
            ));
        }

        sqlx::query!(
            r#"
            UPDATE workspace_members
            SET role = ?
            WHERE workspace_id = ? AND user_id = ?
            "#,
            new_role,
            workspace_id,
            member_id
        )
        .execute(&self.db)
        .await?;

        self.get_member(workspace_id, member_id).await
    }

    /// Get a workspace member
    pub async fn get_member(&self, workspace_id: Uuid, user_id: Uuid) -> Result<WorkspaceMember> {
        sqlx::query_as!(
            WorkspaceMember,
            r#"
            SELECT id, workspace_id, user_id, role as "role: UserRole", joined_at, last_activity
            FROM workspace_members
            WHERE workspace_id = ? AND user_id = ?
            "#,
            workspace_id,
            user_id
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| CollabError::AuthorizationFailed("User is not a member".to_string()))
    }

    /// List all members of a workspace
    pub async fn list_members(&self, workspace_id: Uuid) -> Result<Vec<WorkspaceMember>> {
        let members = sqlx::query_as!(
            WorkspaceMember,
            r#"
            SELECT id, workspace_id, user_id, role as "role: UserRole", joined_at, last_activity
            FROM workspace_members
            WHERE workspace_id = ?
            ORDER BY joined_at
            "#,
            workspace_id
        )
        .fetch_all(&self.db)
        .await?;

        Ok(members)
    }

    /// List all workspaces for a user
    pub async fn list_user_workspaces(&self, user_id: Uuid) -> Result<Vec<TeamWorkspace>> {
        let workspaces = sqlx::query_as!(
            TeamWorkspace,
            r#"
            SELECT w.id, w.name, w.description, w.owner_id, w.config, w.version, w.created_at, w.updated_at, w.is_archived as "is_archived: bool"
            FROM workspaces w
            INNER JOIN workspace_members m ON w.id = m.workspace_id
            WHERE m.user_id = ? AND w.is_archived = FALSE
            ORDER BY w.updated_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.db)
        .await?;

        Ok(workspaces)
    }
}

/// Workspace manager (higher-level API)
pub struct WorkspaceManager {
    service: Arc<WorkspaceService>,
}

impl WorkspaceManager {
    /// Create a new workspace manager
    pub fn new(service: Arc<WorkspaceService>) -> Self {
        Self { service }
    }

    /// Create and setup a new workspace
    pub async fn create_workspace(
        &self,
        name: String,
        description: Option<String>,
        owner_id: Uuid,
    ) -> Result<TeamWorkspace> {
        self.service.create_workspace(name, description, owner_id).await
    }

    /// Get workspace with member check
    pub async fn get_workspace(&self, workspace_id: Uuid, user_id: Uuid) -> Result<TeamWorkspace> {
        // Verify user is a member
        self.service.get_member(workspace_id, user_id).await?;
        self.service.get_workspace(workspace_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would require a database setup
    // For now, they serve as documentation of the API
}
