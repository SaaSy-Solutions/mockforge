//! Bridge between mockforge-collab and mockforge-core workspace types
//!
//! This module provides conversion and synchronization between:
//! - `TeamWorkspace` (collaboration workspace with metadata)
//! - `Workspace` (full mockforge-core workspace with mocks, folders, etc.)

use crate::error::{CollabError, Result};
use crate::models::TeamWorkspace;
use mockforge_core::workspace::Workspace as CoreWorkspace;
use mockforge_core::workspace_persistence::WorkspacePersistence;
use serde_json::Value;
use std::path::Path;
use uuid::Uuid;

/// Bridge service for integrating collaboration workspaces with core workspaces
pub struct CoreBridge {
    persistence: WorkspacePersistence,
}

impl CoreBridge {
    /// Create a new core bridge
    pub fn new<P: AsRef<Path>>(workspace_dir: P) -> Self {
        Self {
            persistence: WorkspacePersistence::new(workspace_dir),
        }
    }

    /// Convert a `TeamWorkspace` to a Core Workspace
    ///
    /// Extracts the full workspace data from the TeamWorkspace.config field
    /// and reconstructs a Core Workspace object.
    pub fn team_to_core(&self, team_workspace: &TeamWorkspace) -> Result<CoreWorkspace> {
        // The full workspace data is stored in the config field as JSON
        let workspace_json = &team_workspace.config;

        // Deserialize the workspace from JSON
        let mut workspace: CoreWorkspace =
            serde_json::from_value(workspace_json.clone()).map_err(|e| {
                CollabError::Internal(format!("Failed to deserialize workspace from config: {e}"))
            })?;

        // Update the workspace ID to match the team workspace ID
        // (convert UUID to String)
        workspace.id = team_workspace.id.to_string();

        // Update metadata
        workspace.name = team_workspace.name.clone();
        workspace.description = team_workspace.description.clone();
        workspace.updated_at = team_workspace.updated_at;

        // Initialize default mock environments if they don't exist (for backward compatibility)
        workspace.initialize_default_mock_environments();

        Ok(workspace)
    }

    /// Convert a Core Workspace to a `TeamWorkspace`
    ///
    /// Serializes the full workspace data into the TeamWorkspace.config field
    /// and creates a `TeamWorkspace` with collaboration metadata.
    pub fn core_to_team(
        &self,
        core_workspace: &CoreWorkspace,
        owner_id: Uuid,
    ) -> Result<TeamWorkspace> {
        // Serialize the full workspace to JSON
        let workspace_json = serde_json::to_value(core_workspace).map_err(|e| {
            CollabError::Internal(format!("Failed to serialize workspace to JSON: {e}"))
        })?;

        // Create TeamWorkspace with the serialized workspace in config
        let mut team_workspace = TeamWorkspace::new(core_workspace.name.clone(), owner_id);
        team_workspace.id = Uuid::parse_str(&core_workspace.id).unwrap_or_else(|_| Uuid::new_v4()); // Fallback to new UUID if parse fails
        team_workspace.description = core_workspace.description.clone();
        team_workspace.config = workspace_json;
        team_workspace.created_at = core_workspace.created_at;
        team_workspace.updated_at = core_workspace.updated_at;

        Ok(team_workspace)
    }

    /// Get the full workspace state from a `TeamWorkspace`
    ///
    /// Returns the complete Core Workspace including all mocks, folders, and configuration.
    pub fn get_workspace_state(&self, team_workspace: &TeamWorkspace) -> Result<CoreWorkspace> {
        self.team_to_core(team_workspace)
    }

    /// Update the workspace state in a `TeamWorkspace`
    ///
    /// Serializes the Core Workspace and stores it in the TeamWorkspace.config field.
    pub fn update_workspace_state(
        &self,
        team_workspace: &mut TeamWorkspace,
        core_workspace: &CoreWorkspace,
    ) -> Result<()> {
        // Serialize the full workspace
        let workspace_json = serde_json::to_value(core_workspace)
            .map_err(|e| CollabError::Internal(format!("Failed to serialize workspace: {e}")))?;

        // Update the config field
        team_workspace.config = workspace_json;
        team_workspace.updated_at = chrono::Utc::now();

        Ok(())
    }

    /// Load workspace from disk using `WorkspacePersistence`
    ///
    /// This loads a workspace from the filesystem and converts it to a `TeamWorkspace`.
    pub async fn load_workspace_from_disk(
        &self,
        workspace_id: &str,
        owner_id: Uuid,
    ) -> Result<TeamWorkspace> {
        // Load from disk
        let core_workspace = self
            .persistence
            .load_workspace(workspace_id)
            .await
            .map_err(|e| CollabError::Internal(format!("Failed to load workspace: {e}")))?;

        // Convert to TeamWorkspace
        self.core_to_team(&core_workspace, owner_id)
    }

    /// Save workspace to disk using `WorkspacePersistence`
    ///
    /// This saves a `TeamWorkspace` to the filesystem as a Core Workspace.
    pub async fn save_workspace_to_disk(&self, team_workspace: &TeamWorkspace) -> Result<()> {
        // Convert to Core Workspace
        let core_workspace = self.team_to_core(team_workspace)?;

        // Save to disk
        self.persistence
            .save_workspace(&core_workspace)
            .await
            .map_err(|e| CollabError::Internal(format!("Failed to save workspace: {e}")))?;

        Ok(())
    }

    /// Export workspace for backup
    ///
    /// Uses `WorkspacePersistence` to create a backup-compatible export.
    pub async fn export_workspace_for_backup(
        &self,
        team_workspace: &TeamWorkspace,
    ) -> Result<Value> {
        // Convert to Core Workspace
        let core_workspace = self.team_to_core(team_workspace)?;

        // Serialize to JSON for backup
        serde_json::to_value(&core_workspace)
            .map_err(|e| CollabError::Internal(format!("Failed to serialize for backup: {e}")))
    }

    /// Import workspace from backup
    ///
    /// Restores a workspace from a backup JSON value.
    pub async fn import_workspace_from_backup(
        &self,
        backup_data: &Value,
        owner_id: Uuid,
        new_name: Option<String>,
    ) -> Result<TeamWorkspace> {
        // Deserialize Core Workspace from backup
        let mut core_workspace: CoreWorkspace = serde_json::from_value(backup_data.clone())
            .map_err(|e| CollabError::Internal(format!("Failed to deserialize backup: {e}")))?;

        // Update name if provided
        if let Some(name) = new_name {
            core_workspace.name = name;
        }

        // Generate new ID for restored workspace
        core_workspace.id = Uuid::new_v4().to_string();
        core_workspace.created_at = chrono::Utc::now();
        core_workspace.updated_at = chrono::Utc::now();

        // Convert to TeamWorkspace
        self.core_to_team(&core_workspace, owner_id)
    }

    /// Get workspace state as JSON for sync
    ///
    /// Returns the full workspace state as a JSON value for real-time synchronization.
    pub fn get_workspace_state_json(&self, team_workspace: &TeamWorkspace) -> Result<Value> {
        let core_workspace = self.team_to_core(team_workspace)?;
        serde_json::to_value(&core_workspace)
            .map_err(|e| CollabError::Internal(format!("Failed to serialize state: {e}")))
    }

    /// Update workspace state from JSON
    ///
    /// Updates the `TeamWorkspace` with state from a JSON value (from sync).
    pub fn update_workspace_state_from_json(
        &self,
        team_workspace: &mut TeamWorkspace,
        state_json: &Value,
    ) -> Result<()> {
        // Deserialize Core Workspace from JSON
        let mut core_workspace: CoreWorkspace = serde_json::from_value(state_json.clone())
            .map_err(|e| CollabError::Internal(format!("Failed to deserialize state JSON: {e}")))?;

        // Preserve TeamWorkspace metadata
        core_workspace.id = team_workspace.id.to_string();
        core_workspace.name = team_workspace.name.clone();
        core_workspace.description = team_workspace.description.clone();

        // Update the TeamWorkspace
        self.update_workspace_state(team_workspace, &core_workspace)
    }

    /// Create a new empty workspace
    ///
    /// Creates a new Core Workspace and converts it to a `TeamWorkspace`.
    pub fn create_empty_workspace(&self, name: String, owner_id: Uuid) -> Result<TeamWorkspace> {
        let core_workspace = CoreWorkspace::new(name);
        self.core_to_team(&core_workspace, owner_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_to_core_conversion() {
        let bridge = CoreBridge::new("/tmp/test");
        let owner_id = Uuid::new_v4();

        // Create a simple core workspace
        let core_workspace = CoreWorkspace::new("Test Workspace".to_string());
        let team_workspace = bridge.core_to_team(&core_workspace, owner_id).unwrap();

        // Convert back
        let restored = bridge.team_to_core(&team_workspace).unwrap();

        assert_eq!(restored.name, core_workspace.name);
        assert_eq!(restored.folders.len(), core_workspace.folders.len());
        assert_eq!(restored.requests.len(), core_workspace.requests.len());
    }

    #[test]
    fn test_state_json_roundtrip() {
        let bridge = CoreBridge::new("/tmp/test");
        let owner_id = Uuid::new_v4();

        // Create workspace
        let core_workspace = CoreWorkspace::new("Test".to_string());
        let mut team_workspace = bridge.core_to_team(&core_workspace, owner_id).unwrap();

        // Get state as JSON
        let state_json = bridge.get_workspace_state_json(&team_workspace).unwrap();

        // Update from JSON
        bridge
            .update_workspace_state_from_json(&mut team_workspace, &state_json)
            .unwrap();

        // Verify it still works
        let restored = bridge.team_to_core(&team_workspace).unwrap();
        assert_eq!(restored.name, "Test");
    }
}
