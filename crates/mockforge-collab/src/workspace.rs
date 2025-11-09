//! Workspace management and collaboration

use crate::core_bridge::CoreBridge;
use crate::error::{CollabError, Result};
use crate::models::{
    MergeConflict, MergeStatus, TeamWorkspace, UserRole, WorkspaceFork, WorkspaceMember,
    WorkspaceMerge,
};
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
    core_bridge: Option<Arc<CoreBridge>>,
}

impl WorkspaceService {
    /// Create a new workspace service
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self {
            db,
            cache: Arc::new(RwLock::new(HashMap::new())),
            core_bridge: None,
        }
    }

    /// Create a new workspace service with CoreBridge integration
    pub fn with_core_bridge(db: Pool<Sqlite>, core_bridge: Arc<CoreBridge>) -> Self {
        Self {
            db,
            cache: Arc::new(RwLock::new(HashMap::new())),
            core_bridge: Some(core_bridge),
        }
    }

    /// Create a new workspace
    pub async fn create_workspace(
        &self,
        name: String,
        description: Option<String>,
        owner_id: Uuid,
    ) -> Result<TeamWorkspace> {
        let mut workspace = TeamWorkspace::new(name.clone(), owner_id);
        workspace.description = description.clone();

        // If we have CoreBridge, create a proper core workspace and embed it
        if let Some(core_bridge) = &self.core_bridge {
            let core_workspace = core_bridge.create_empty_workspace(name, owner_id)?;
            workspace.config = core_workspace.config;
        } else {
            // Fallback: create minimal config
            workspace.config = serde_json::json!({
                "name": workspace.name,
                "description": workspace.description,
                "folders": [],
                "requests": []
            });
        }

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
            SELECT
                id as "id: Uuid",
                name,
                description,
                owner_id as "owner_id: Uuid",
                config,
                version,
                created_at as "created_at: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
                is_archived as "is_archived: bool"
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

        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE workspaces
            SET is_archived = TRUE, updated_at = ?
            WHERE id = ?
            "#,
            now,
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
            SELECT
                id as "id: Uuid",
                workspace_id as "workspace_id: Uuid",
                user_id as "user_id: Uuid",
                role as "role: UserRole",
                joined_at as "joined_at: chrono::DateTime<chrono::Utc>",
                last_activity as "last_activity: chrono::DateTime<chrono::Utc>"
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
            SELECT
                id as "id: Uuid",
                workspace_id as "workspace_id: Uuid",
                user_id as "user_id: Uuid",
                role as "role: UserRole",
                joined_at as "joined_at: chrono::DateTime<chrono::Utc>",
                last_activity as "last_activity: chrono::DateTime<chrono::Utc>"
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
            SELECT
                w.id as "id: Uuid",
                w.name,
                w.description,
                w.owner_id as "owner_id: Uuid",
                w.config,
                w.version,
                w.created_at as "created_at: chrono::DateTime<chrono::Utc>",
                w.updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
                w.is_archived as "is_archived: bool"
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

    /// Fork a workspace (create an independent copy)
    ///
    /// Creates a new workspace that is a copy of the source workspace.
    /// The forked workspace has its own ID and can be modified independently.
    pub async fn fork_workspace(
        &self,
        source_workspace_id: Uuid,
        new_name: Option<String>,
        new_owner_id: Uuid,
        fork_point_commit_id: Option<Uuid>,
    ) -> Result<TeamWorkspace> {
        // Verify user has access to source workspace
        self.get_member(source_workspace_id, new_owner_id).await?;

        // Get source workspace
        let source_workspace = self.get_workspace(source_workspace_id).await?;

        // Create new workspace with copied data
        let mut forked_workspace = TeamWorkspace::new(
            new_name.unwrap_or_else(|| format!("{} (Fork)", source_workspace.name)),
            new_owner_id,
        );
        forked_workspace.description = source_workspace.description.clone();

        // Deep copy the config (workspace data) to ensure independence
        // If we have CoreBridge, we can properly clone the core workspace
        if let Some(core_bridge) = &self.core_bridge {
            // Get the core workspace from source
            if let Ok(mut core_workspace) = core_bridge.team_to_core(&source_workspace) {
                // Generate new IDs for all entities in the forked workspace
                core_workspace.id = forked_workspace.id.to_string();
                core_workspace.name = forked_workspace.name.clone();
                core_workspace.description = forked_workspace.description.clone();
                core_workspace.created_at = forked_workspace.created_at;
                core_workspace.updated_at = forked_workspace.updated_at;

                // Regenerate IDs for folders and requests to ensure independence
                Self::regenerate_entity_ids(&mut core_workspace);

                // Convert back to TeamWorkspace
                if let Ok(team_ws) = core_bridge.core_to_team(&core_workspace, new_owner_id) {
                    forked_workspace.config = team_ws.config;
                } else {
                    // Fallback to shallow copy
                    forked_workspace.config = source_workspace.config.clone();
                }
            } else {
                // Fallback to shallow copy
                forked_workspace.config = source_workspace.config.clone();
            }
        } else {
            // Fallback to shallow copy
            forked_workspace.config = source_workspace.config.clone();
        }

        // Insert forked workspace into database
        sqlx::query!(
            r#"
            INSERT INTO workspaces (id, name, description, owner_id, config, version, created_at, updated_at, is_archived)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            forked_workspace.id,
            forked_workspace.name,
            forked_workspace.description,
            forked_workspace.owner_id,
            forked_workspace.config,
            forked_workspace.version,
            forked_workspace.created_at,
            forked_workspace.updated_at,
            forked_workspace.is_archived
        )
        .execute(&self.db)
        .await?;

        // Add owner as admin member
        let member = WorkspaceMember::new(forked_workspace.id, new_owner_id, UserRole::Admin);
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

        // Create fork relationship record
        let fork = WorkspaceFork::new(
            source_workspace_id,
            forked_workspace.id,
            new_owner_id,
            fork_point_commit_id,
        );
        sqlx::query!(
            r#"
            INSERT INTO workspace_forks (id, source_workspace_id, forked_workspace_id, forked_at, forked_by, fork_point_commit_id)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            fork.id,
            fork.source_workspace_id,
            fork.forked_workspace_id,
            fork.forked_at,
            fork.forked_by,
            fork.fork_point_commit_id
        )
        .execute(&self.db)
        .await?;

        // Update cache
        self.cache.write().insert(forked_workspace.id, forked_workspace.clone());

        Ok(forked_workspace)
    }

    /// List all forks of a workspace
    pub async fn list_forks(&self, workspace_id: Uuid) -> Result<Vec<WorkspaceFork>> {
        let forks = sqlx::query_as!(
            WorkspaceFork,
            r#"
            SELECT
                id as "id: Uuid",
                source_workspace_id as "source_workspace_id: Uuid",
                forked_workspace_id as "forked_workspace_id: Uuid",
                forked_at as "forked_at: chrono::DateTime<chrono::Utc>",
                forked_by as "forked_by: Uuid",
                fork_point_commit_id as "fork_point_commit_id: Uuid"
            FROM workspace_forks
            WHERE source_workspace_id = ?
            ORDER BY forked_at DESC
            "#,
            workspace_id
        )
        .fetch_all(&self.db)
        .await?;

        Ok(forks)
    }

    /// Get the source workspace for a fork
    pub async fn get_fork_source(
        &self,
        forked_workspace_id: Uuid,
    ) -> Result<Option<WorkspaceFork>> {
        let fork = sqlx::query_as!(
            WorkspaceFork,
            r#"
            SELECT
                id as "id: Uuid",
                source_workspace_id as "source_workspace_id: Uuid",
                forked_workspace_id as "forked_workspace_id: Uuid",
                forked_at as "forked_at: chrono::DateTime<chrono::Utc>",
                forked_by as "forked_by: Uuid",
                fork_point_commit_id as "fork_point_commit_id: Uuid"
            FROM workspace_forks
            WHERE forked_workspace_id = ?
            "#,
            forked_workspace_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(fork)
    }

    /// Regenerate entity IDs in a core workspace to ensure fork independence
    fn regenerate_entity_ids(core_workspace: &mut mockforge_core::workspace::Workspace) {
        use mockforge_core::workspace::{Folder, MockRequest};
        use uuid::Uuid;

        // Regenerate workspace ID
        core_workspace.id = Uuid::new_v4().to_string();

        // Helper to regenerate folder IDs recursively
        fn regenerate_folder_ids(folder: &mut Folder) {
            folder.id = Uuid::new_v4().to_string();
            for subfolder in &mut folder.folders {
                regenerate_folder_ids(subfolder);
            }
            for request in &mut folder.requests {
                request.id = Uuid::new_v4().to_string();
            }
        }

        // Regenerate IDs for root folders
        for folder in &mut core_workspace.folders {
            regenerate_folder_ids(folder);
        }

        // Regenerate IDs for root requests
        for request in &mut core_workspace.requests {
            request.id = Uuid::new_v4().to_string();
        }
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
