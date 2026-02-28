//! Snapshot manager for saving and restoring system states
//!
//! The snapshot manager provides functionality to save complete system states
//! to disk and restore them later, enabling time travel capabilities.

use crate::consistency::ConsistencyEngine;
use crate::snapshots::state_exporter::ProtocolStateExporter;
use crate::snapshots::types::{SnapshotComponents, SnapshotManifest, SnapshotMetadata};
use crate::workspace_persistence::WorkspacePersistence;
use crate::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing::{debug, info, warn};

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
    ///
    /// # Arguments
    /// * `name` - Name for the snapshot
    /// * `description` - Optional description
    /// * `workspace_id` - Workspace identifier
    /// * `components` - Which components to include
    /// * `consistency_engine` - Optional consistency engine for unified state
    /// * `workspace_persistence` - Optional workspace persistence for config
    /// * `vbr_state` - Optional VBR state (pre-extracted JSON)
    /// * `recorder_state` - Optional Recorder state (pre-extracted JSON)
    #[allow(clippy::too_many_arguments)]
    pub async fn save_snapshot(
        &self,
        name: String,
        description: Option<String>,
        workspace_id: String,
        components: SnapshotComponents,
        consistency_engine: Option<&ConsistencyEngine>,
        workspace_persistence: Option<&WorkspacePersistence>,
        vbr_state: Option<serde_json::Value>,
        recorder_state: Option<serde_json::Value>,
    ) -> Result<SnapshotManifest> {
        self.save_snapshot_with_exporters(
            name,
            description,
            workspace_id,
            components,
            consistency_engine,
            workspace_persistence,
            vbr_state,
            recorder_state,
            HashMap::new(),
        )
        .await
    }

    /// Save a snapshot with protocol state exporters
    ///
    /// Extended version that accepts a map of protocol state exporters
    /// for capturing state from multiple protocols.
    #[allow(clippy::too_many_arguments)]
    pub async fn save_snapshot_with_exporters(
        &self,
        name: String,
        description: Option<String>,
        workspace_id: String,
        components: SnapshotComponents,
        consistency_engine: Option<&ConsistencyEngine>,
        workspace_persistence: Option<&WorkspacePersistence>,
        vbr_state: Option<serde_json::Value>,
        recorder_state: Option<serde_json::Value>,
        protocol_exporters: HashMap<String, Arc<dyn ProtocolStateExporter>>,
    ) -> Result<SnapshotManifest> {
        info!("Saving snapshot '{}' for workspace '{}'", name, workspace_id);

        // Create snapshot directory
        let snapshot_dir = self.snapshot_dir(&workspace_id, &name);
        fs::create_dir_all(&snapshot_dir).await?;

        // Create temporary directory for atomic writes
        let temp_dir = snapshot_dir.join(".tmp");
        fs::create_dir_all(&temp_dir).await?;

        let mut manifest =
            SnapshotManifest::new(name.clone(), workspace_id.clone(), components.clone());

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
            let config_path = temp_dir.join("workspace_config.yaml");
            if let Some(persistence) = workspace_persistence {
                match persistence.load_workspace(&workspace_id).await {
                    Ok(workspace) => {
                        let config_yaml = serde_yaml::to_string(&workspace).map_err(|e| {
                            crate::Error::generic(format!("Failed to serialize workspace: {}", e))
                        })?;
                        fs::write(&config_path, config_yaml).await?;
                        debug!("Saved workspace config to {}", config_path.display());
                    }
                    Err(e) => {
                        warn!("Failed to load workspace config for snapshot: {}. Saving empty config.", e);
                        let empty_config = serde_yaml::to_string(&serde_json::json!({}))?;
                        fs::write(&config_path, empty_config).await?;
                    }
                }
            } else {
                warn!("Workspace persistence not provided, saving empty workspace config");
                let empty_config = serde_yaml::to_string(&serde_json::json!({}))?;
                fs::write(&config_path, empty_config).await?;
            }
        }

        // Save VBR state if requested
        if components.vbr_state {
            let vbr_path = temp_dir.join("vbr_state.json");
            if let Some(state) = vbr_state {
                let state_json = serde_json::to_string_pretty(&state)?;
                fs::write(&vbr_path, state_json).await?;
                debug!("Saved VBR state to {}", vbr_path.display());
            } else {
                warn!("VBR state requested but not provided, saving empty state");
                let empty_state = serde_json::json!({});
                fs::write(&vbr_path, serde_json::to_string_pretty(&empty_state)?).await?;
            }
        }

        // Save Recorder state if requested
        if components.recorder_state {
            let recorder_path = temp_dir.join("recorder_state.json");
            if let Some(state) = recorder_state {
                let state_json = serde_json::to_string_pretty(&state)?;
                fs::write(&recorder_path, state_json).await?;
                debug!("Saved Recorder state to {}", recorder_path.display());
            } else {
                warn!("Recorder state requested but not provided, saving empty state");
                let empty_state = serde_json::json!({});
                fs::write(&recorder_path, serde_json::to_string_pretty(&empty_state)?).await?;
            }
        }

        // Save protocol states if requested
        if !components.protocols.is_empty() || !protocol_exporters.is_empty() {
            let protocols_dir = temp_dir.join("protocols");
            fs::create_dir_all(&protocols_dir).await?;

            // Determine which protocols to save
            let protocols_to_save: Vec<String> = if components.protocols.is_empty() {
                // If no specific protocols requested, save all available exporters
                protocol_exporters.keys().cloned().collect()
            } else {
                components.protocols.clone()
            };

            for protocol_name in protocols_to_save {
                let protocol_path = protocols_dir.join(format!("{}.json", protocol_name));

                // Try to get state from exporter if available
                if let Some(exporter) = protocol_exporters.get(&protocol_name) {
                    match exporter.export_state().await {
                        Ok(state) => {
                            let summary = exporter.state_summary().await;
                            fs::write(&protocol_path, serde_json::to_string_pretty(&state)?)
                                .await?;
                            debug!(
                                "Saved {} protocol state to {}: {}",
                                protocol_name,
                                protocol_path.display(),
                                summary
                            );
                        }
                        Err(e) => {
                            warn!(
                                "Failed to export {} protocol state: {}. Saving empty state.",
                                protocol_name, e
                            );
                            let empty_state = serde_json::json!({
                                "error": format!("Failed to export state: {}", e),
                                "protocol": protocol_name
                            });
                            fs::write(&protocol_path, serde_json::to_string_pretty(&empty_state)?)
                                .await?;
                        }
                    }
                } else {
                    // No exporter available, save placeholder
                    debug!(
                        "No exporter available for protocol {}, saving placeholder",
                        protocol_name
                    );
                    let placeholder_state = serde_json::json!({
                        "protocol": protocol_name,
                        "state": "no_exporter_available"
                    });
                    fs::write(&protocol_path, serde_json::to_string_pretty(&placeholder_state)?)
                        .await?;
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
    /// Returns the manifest and optionally the VBR and Recorder state as JSON
    /// (caller is responsible for restoring them to their respective systems).
    pub async fn load_snapshot(
        &self,
        name: String,
        workspace_id: String,
        components: Option<SnapshotComponents>,
        consistency_engine: Option<&ConsistencyEngine>,
        workspace_persistence: Option<&WorkspacePersistence>,
    ) -> Result<(SnapshotManifest, Option<serde_json::Value>, Option<serde_json::Value>)> {
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
        let (_size, checksum) = self.calculate_snapshot_checksum(&snapshot_dir).await?;
        if checksum != manifest.checksum {
            warn!("Snapshot checksum mismatch: expected {}, got {}", manifest.checksum, checksum);
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
                if let Some(persistence) = workspace_persistence {
                    let config_yaml = fs::read_to_string(&config_path).await?;
                    let workspace: crate::workspace::Workspace = serde_yaml::from_str(&config_yaml)
                        .map_err(|e| {
                            crate::Error::generic(format!("Failed to deserialize workspace: {}", e))
                        })?;
                    persistence.save_workspace(&workspace).await?;
                    debug!("Restored workspace config from {}", config_path.display());
                } else {
                    warn!(
                        "Workspace persistence not provided, skipping workspace config restoration"
                    );
                }
            } else {
                warn!("Workspace config file not found in snapshot: {}", config_path.display());
            }
        }

        // Load VBR state if requested (return as JSON for caller to restore)
        let vbr_state = if components_to_restore.vbr_state && manifest.components.vbr_state {
            let vbr_path = snapshot_dir.join("vbr_state.json");
            if vbr_path.exists() {
                let vbr_json = fs::read_to_string(&vbr_path).await?;
                let state: serde_json::Value = serde_json::from_str(&vbr_json).map_err(|e| {
                    crate::Error::generic(format!("Failed to parse VBR state: {}", e))
                })?;
                debug!("Loaded VBR state from {}", vbr_path.display());
                Some(state)
            } else {
                warn!("VBR state file not found in snapshot: {}", vbr_path.display());
                None
            }
        } else {
            None
        };

        // Load Recorder state if requested (return as JSON for caller to restore)
        let recorder_state =
            if components_to_restore.recorder_state && manifest.components.recorder_state {
                let recorder_path = snapshot_dir.join("recorder_state.json");
                if recorder_path.exists() {
                    let recorder_json = fs::read_to_string(&recorder_path).await?;
                    let state: serde_json::Value =
                        serde_json::from_str(&recorder_json).map_err(|e| {
                            crate::Error::generic(format!("Failed to parse Recorder state: {}", e))
                        })?;
                    debug!("Loaded Recorder state from {}", recorder_path.display());
                    Some(state)
                } else {
                    warn!("Recorder state file not found in snapshot: {}", recorder_path.display());
                    None
                }
            } else {
                None
            };

        info!("Snapshot '{}' loaded successfully", name);
        Ok((manifest, vbr_state, recorder_state))
    }

    /// Load a snapshot and restore system state with protocol exporters
    ///
    /// Extended version that accepts protocol state exporters to restore
    /// protocol-specific state from snapshots.
    ///
    /// # Arguments
    /// * `name` - Snapshot name
    /// * `workspace_id` - Workspace identifier
    /// * `components` - Which components to restore (uses manifest if None)
    /// * `consistency_engine` - Optional consistency engine for unified state
    /// * `workspace_persistence` - Optional workspace persistence for config
    /// * `protocol_exporters` - Map of protocol exporters for restoring protocol state
    pub async fn load_snapshot_with_exporters(
        &self,
        name: String,
        workspace_id: String,
        components: Option<SnapshotComponents>,
        consistency_engine: Option<&ConsistencyEngine>,
        workspace_persistence: Option<&WorkspacePersistence>,
        protocol_exporters: HashMap<String, Arc<dyn ProtocolStateExporter>>,
    ) -> Result<(SnapshotManifest, Option<serde_json::Value>, Option<serde_json::Value>)> {
        // First load the base snapshot
        let (manifest, vbr_state, recorder_state) = self
            .load_snapshot(
                name.clone(),
                workspace_id.clone(),
                components.clone(),
                consistency_engine,
                workspace_persistence,
            )
            .await?;

        // Determine which components to restore
        let components_to_restore = components.unwrap_or_else(|| manifest.components.clone());

        // Restore protocol states if any exporters provided and protocols were saved
        if !protocol_exporters.is_empty()
            && (!components_to_restore.protocols.is_empty()
                || !manifest.components.protocols.is_empty())
        {
            let snapshot_dir = self.snapshot_dir(&workspace_id, &name);
            let protocols_dir = snapshot_dir.join("protocols");

            if protocols_dir.exists() {
                // Determine which protocols to restore
                let protocols_to_restore: Vec<String> =
                    if components_to_restore.protocols.is_empty() {
                        // If no specific protocols requested, restore all available
                        manifest.components.protocols.clone()
                    } else {
                        components_to_restore.protocols.clone()
                    };

                for protocol_name in protocols_to_restore {
                    let protocol_path = protocols_dir.join(format!("{}.json", protocol_name));

                    if protocol_path.exists() {
                        if let Some(exporter) = protocol_exporters.get(&protocol_name) {
                            match fs::read_to_string(&protocol_path).await {
                                Ok(state_json) => {
                                    match serde_json::from_str::<serde_json::Value>(&state_json) {
                                        Ok(state) => {
                                            // Skip placeholder/error states
                                            if state.get("state")
                                                == Some(&serde_json::json!("no_exporter_available"))
                                            {
                                                debug!(
                                                    "Skipping {} protocol restore - no exporter was available during save",
                                                    protocol_name
                                                );
                                                continue;
                                            }
                                            if state.get("error").is_some() {
                                                warn!(
                                                    "Skipping {} protocol restore - state contains error from save",
                                                    protocol_name
                                                );
                                                continue;
                                            }

                                            match exporter.import_state(state).await {
                                                Ok(_) => {
                                                    debug!(
                                                        "Restored {} protocol state from {}",
                                                        protocol_name,
                                                        protocol_path.display()
                                                    );
                                                }
                                                Err(e) => {
                                                    warn!(
                                                        "Failed to restore {} protocol state: {}",
                                                        protocol_name, e
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Failed to parse {} protocol state: {}",
                                                protocol_name, e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to read {} protocol state file: {}",
                                        protocol_name, e
                                    );
                                }
                            }
                        } else {
                            debug!(
                                "No exporter provided for protocol {}, skipping restore",
                                protocol_name
                            );
                        }
                    } else {
                        debug!(
                            "Protocol state file not found for {}: {}",
                            protocol_name,
                            protocol_path.display()
                        );
                    }
                }
            }
        }

        Ok((manifest, vbr_state, recorder_state))
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
                                warn!(
                                    "Failed to parse manifest for snapshot {}: {}",
                                    snapshot_name, e
                                );
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
    pub async fn validate_snapshot(&self, name: String, workspace_id: String) -> Result<bool> {
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
                    if path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.starts_with('.'))
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    stack.push(path);
                } else if metadata.is_file() {
                    // Skip manifest.json from checksum calculation (it contains the checksum)
                    if path
                        .file_name()
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
                    hasher
                        .update(path.file_name().unwrap_or_default().to_string_lossy().as_bytes());
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
