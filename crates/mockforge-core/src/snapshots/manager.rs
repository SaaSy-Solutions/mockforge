//! Snapshot manager for saving and restoring system states
//!
//! The snapshot manager provides functionality to save complete system states
//! to disk and restore them later, enabling time travel capabilities.

use crate::consistency::ConsistencyEngine;
use crate::snapshots::types::{SnapshotComponents, SnapshotManifest, SnapshotMetadata};
use crate::Result;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info, warn};

/// Snapshot manager for saving and restoring system states
///
/// Manages snapshots stored in a directory structure:
/// `~/.mockforge/snapshots/{workspace_id}/{snapshot_name}/`
pub struct SnapshotManager {
    /// Base directory for snapshots
    base_dir: PathBuf,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    ///
    /// Defaults to `~/.mockforge/snapshots` if no base directory is provided.
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        let base_dir = base_dir.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".mockforge")
                .join("snapshots")
        });

        Self { base_dir }
    }

    /// Get the snapshot directory for a workspace
    fn workspace_dir(&self, workspace_id: &str) -> PathBuf {
        self.base_dir.join(workspace_id)
    }

    /// Get the snapshot directory for a specific snapshot
    fn snapshot_dir(&self, workspace_id: &str, snapshot_name: &str) -> PathBuf {
        self.workspace_dir(workspace_id).join(snapshot_name)
    }

    /// Save a snapshot of the current system state
    ///
    /// This creates a snapshot directory and saves all specified components.
    pub async fn save_snapshot(
        &self,
        name: String,
        description: Option<String>,
        workspace_id: String,
        components: SnapshotComponents,
        consistency_engine: Option<&ConsistencyEngine>,
        // TODO: Add other component sources (VBR, Recorder, etc.) as they're integrated
    ) -> Result<SnapshotManifest> {
        info!("Saving snapshot '{}' for workspace '{}'", name, workspace_id);

        // Create snapshot directory
        let snapshot_dir = self.snapshot_dir(&workspace_id, &name);
        fs::create_dir_all(&snapshot_dir).await?;

        // Create temporary directory for atomic writes
        let temp_dir = snapshot_dir.join(".tmp");
        fs::create_dir_all(&temp_dir).await?;

        let mut manifest = SnapshotManifest::new(name.clone(), workspace_id.clone(), components.clone());

        // Save unified state if requested
        if components.unified_state {
            if let Some(engine) = consistency_engine {
                let unified_state = engine.get_state(&workspace_id).await;
                if let Some(state) = unified_state {
                    let state_path = temp_dir.join("unified_state.json");
                    let state_json = serde_json::to_string_pretty(&state)?;
                    fs::write(&state_path, &state_json).await?;
                    debug!("Saved unified state to {}", state_path.display());
                } else {
                    warn!("No unified state found for workspace {}", workspace_id);
                }
            }
        }

        // Save workspace config if requested
        if components.workspace_config {
            // TODO: Load and save workspace config when workspace persistence is integrated
            let config_path = temp_dir.join("workspace_config.yaml");
            let empty_config = serde_yaml::to_string(&serde_json::json!({}))?;
            fs::write(&config_path, empty_config).await?;
            debug!("Saved workspace config placeholder to {}", config_path.display());
        }

        // Save protocol states if requested
        if !components.protocols.is_empty() || components.protocols.is_empty() {
            let protocols_dir = temp_dir.join("protocols");
            fs::create_dir_all(&protocols_dir).await?;

            if let Some(_engine) = consistency_engine {
                // Save all protocol states
                let protocols: Vec<String> = if components.protocols.is_empty() {
                    vec!["http".to_string(), "graphql".to_string(), "grpc".to_string(), "websocket".to_string(), "tcp".to_string()]
                } else {
                    components.protocols.clone()
                };

                for protocol_name in protocols {
                    // TODO: Get protocol state from engine when protocol adapters are integrated
                    let protocol_path = protocols_dir.join(format!("{}.json", protocol_name));
                    let empty_state = serde_json::json!({});
                    fs::write(&protocol_path, serde_json::to_string_pretty(&empty_state)?).await?;
                }
            }
        }

        // Calculate checksum and size
        let (size, checksum) = self.calculate_snapshot_checksum(&temp_dir).await?;
        manifest.size_bytes = size;
        manifest.checksum = checksum;
        manifest.description = description;

        // Write manifest
        let manifest_path = temp_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        fs::write(&manifest_path, &manifest_json).await?;

        // Atomically move temp directory to final location
        // Remove old snapshot if it exists
        if snapshot_dir.exists() && snapshot_dir != temp_dir {
            let old_backup = snapshot_dir.with_extension("old");
            if old_backup.exists() {
                fs::remove_dir_all(&old_backup).await?;
            }
            fs::rename(&snapshot_dir, &old_backup).await?;
        }

        // Move temp to final location
        if temp_dir.exists() {
            // Move contents from temp_dir to snapshot_dir
            let mut entries = fs::read_dir(&temp_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let dest = snapshot_dir.join(entry.file_name());
                fs::rename(entry.path(), &dest).await?;
            }
            fs::remove_dir(&temp_dir).await?;
        }

        info!("Snapshot '{}' saved successfully ({} bytes)", name, size);
        Ok(manifest)
    }

    /// Load a snapshot and restore system state
    ///
    /// Restores the specified components from a snapshot.
    pub async fn load_snapshot(
        &self,
        name: String,
        workspace_id: String,
        components: Option<SnapshotComponents>,
        consistency_engine: Option<&ConsistencyEngine>,
    ) -> Result<SnapshotManifest> {
        info!("Loading snapshot '{}' for workspace '{}'", name, workspace_id);

        let snapshot_dir = self.snapshot_dir(&workspace_id, &name);
        if !snapshot_dir.exists() {
            return Err(crate::Error::from(format!(
                "Snapshot '{}' not found for workspace '{}'",
                name, workspace_id
            )));
        }

        // Load manifest
        let manifest_path = snapshot_dir.join("manifest.json");
        let manifest_json = fs::read_to_string(&manifest_path).await?;
        let manifest: SnapshotManifest = serde_json::from_str(&manifest_json)?;

        // Validate checksum
        let (size, checksum) = self.calculate_snapshot_checksum(&snapshot_dir).await?;
        if checksum != manifest.checksum {
            warn!(
                "Snapshot checksum mismatch: expected {}, got {}",
                manifest.checksum, checksum
            );
            // Continue anyway, but log warning
        }

        // Determine which components to restore
        let components_to_restore = components.unwrap_or_else(|| manifest.components.clone());

        // Restore unified state if requested
        if components_to_restore.unified_state && manifest.components.unified_state {
            if let Some(engine) = consistency_engine {
                let state_path = snapshot_dir.join("unified_state.json");
                if state_path.exists() {
                    let state_json = fs::read_to_string(&state_path).await?;
                    let unified_state: crate::consistency::UnifiedState =
                        serde_json::from_str(&state_json)?;
                    engine.restore_state(unified_state).await?;
                    debug!("Restored unified state from {}", state_path.display());
                }
            }
        }

        // Restore workspace config if requested
        if components_to_restore.workspace_config && manifest.components.workspace_config {
            let config_path = snapshot_dir.join("workspace_config.yaml");
            if config_path.exists() {
                // TODO: Restore workspace config when workspace persistence is integrated
                debug!("Loaded workspace config from {}", config_path.display());
            }
        }

        info!("Snapshot '{}' loaded successfully", name);
        Ok(manifest)
    }

    /// List all snapshots for a workspace
    pub async fn list_snapshots(&self, workspace_id: &str) -> Result<Vec<SnapshotMetadata>> {
        let workspace_dir = self.workspace_dir(workspace_id);
        if !workspace_dir.exists() {
            return Ok(Vec::new());
        }

        let mut snapshots = Vec::new();
        let mut entries = fs::read_dir(&workspace_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let snapshot_name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden directories and temp directories
            if snapshot_name.starts_with('.') {
                continue;
            }

            let manifest_path = entry.path().join("manifest.json");
            if manifest_path.exists() {
                match fs::read_to_string(&manifest_path).await {
                    Ok(manifest_json) => {
                        match serde_json::from_str::<SnapshotManifest>(&manifest_json) {
                            Ok(manifest) => {
                                snapshots.push(SnapshotMetadata::from(manifest));
                            }
                            Err(e) => {
                                warn!("Failed to parse manifest for snapshot {}: {}", snapshot_name, e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read manifest for snapshot {}: {}", snapshot_name, e);
                    }
                }
            }
        }

        // Sort by creation date (newest first)
        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(snapshots)
    }

    /// Delete a snapshot
    pub async fn delete_snapshot(&self, name: String, workspace_id: String) -> Result<()> {
        info!("Deleting snapshot '{}' for workspace '{}'", name, workspace_id);
        let snapshot_dir = self.snapshot_dir(&workspace_id, &name);
        if snapshot_dir.exists() {
            fs::remove_dir_all(&snapshot_dir).await?;
            info!("Snapshot '{}' deleted successfully", name);
        } else {
            return Err(crate::Error::from(format!(
                "Snapshot '{}' not found for workspace '{}'",
                name, workspace_id
            )));
        }
        Ok(())
    }

    /// Get snapshot information
    pub async fn get_snapshot_info(
        &self,
        name: String,
        workspace_id: String,
    ) -> Result<SnapshotManifest> {
        let snapshot_dir = self.snapshot_dir(&workspace_id, &name);
        let manifest_path = snapshot_dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(crate::Error::from(format!(
                "Snapshot '{}' not found for workspace '{}'",
                name, workspace_id
            )));
        }

        let manifest_json = fs::read_to_string(&manifest_path).await?;
        let manifest: SnapshotManifest = serde_json::from_str(&manifest_json)?;
        Ok(manifest)
    }

    /// Validate snapshot integrity
    pub async fn validate_snapshot(
        &self,
        name: String,
        workspace_id: String,
    ) -> Result<bool> {
        let snapshot_dir = self.snapshot_dir(&workspace_id, &name);
        let manifest_path = snapshot_dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(crate::Error::from(format!(
                "Snapshot '{}' not found for workspace '{}'",
                name, workspace_id
            )));
        }

        let manifest_json = fs::read_to_string(&manifest_path).await?;
        let manifest: SnapshotManifest = serde_json::from_str(&manifest_json)?;

        let (_, checksum) = self.calculate_snapshot_checksum(&snapshot_dir).await?;
        Ok(checksum == manifest.checksum)
    }

    /// Calculate checksum and size of snapshot directory
    async fn calculate_snapshot_checksum(&self, dir: &Path) -> Result<(u64, String)> {
        let mut hasher = Sha256::new();
        let mut total_size = 0u64;

        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            let mut entries = fs::read_dir(&current).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let metadata = fs::metadata(&path).await?;

                if metadata.is_dir() {
                    // Skip temp directories
                    if path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.starts_with('.'))
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    stack.push(path);
                } else if metadata.is_file() {
                    // Skip manifest.json from checksum calculation (it contains the checksum)
                    if path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s == "manifest.json")
                        .unwrap_or(false)
                    {
                        continue;
                    }

                    let file_size = metadata.len();
                    total_size += file_size;

                    let content = fs::read(&path).await?;
                    hasher.update(&content);
                    hasher.update(path.file_name().unwrap_or_default().to_string_lossy().as_bytes());
                }
            }
        }

        let checksum = format!("sha256:{:x}", hasher.finalize());
        Ok((total_size, checksum))
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new(None)
    }
}

