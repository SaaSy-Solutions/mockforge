//! File system watcher for bidirectional directory sync
//!
//! This module provides real-time file system monitoring for bidirectional
//! sync between workspaces and external directories.

use crate::workspace_persistence::WorkspacePersistence;
use crate::{Error, Result};
use notify::{Config, Event, RecommendedWatcher, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

/// File system watcher for workspace sync
pub struct SyncWatcher {
    /// Active watchers by workspace ID
    watchers: HashMap<String, RecommendedWatcher>,
    /// Running state
    running: Arc<Mutex<bool>>,
    /// Persistence layer
    persistence: Arc<WorkspacePersistence>,
}

/// File system synchronization events for workspace monitoring
#[derive(Debug, Clone)]
pub enum SyncEvent {
    /// A new file was created in the watched directory
    FileCreated {
        /// Workspace ID this file belongs to
        workspace_id: String,
        /// Path to the created file
        path: PathBuf,
        /// Contents of the created file
        content: String,
    },
    /// An existing file was modified in the watched directory
    FileModified {
        /// Workspace ID this file belongs to
        workspace_id: String,
        /// Path to the modified file
        path: PathBuf,
        /// Updated contents of the file
        content: String,
    },
    /// A file was deleted from the watched directory
    FileDeleted {
        /// Workspace ID this file belonged to
        workspace_id: String,
        /// Path to the deleted file
        path: PathBuf,
    },
    /// Multiple directory changes detected (batched summary)
    DirectoryChanged {
        /// Workspace ID where changes occurred
        workspace_id: String,
        /// List of file changes detected
        changes: Vec<FileChange>,
    },
}

/// Represents a single file change in the watched directory
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Path to the file that changed
    pub path: PathBuf,
    /// Type of change that occurred
    pub kind: ChangeKind,
    /// Optional file contents (for created/modified events)
    pub content: Option<String>,
}

/// Type of file system change detected
#[derive(Debug, Clone)]
pub enum ChangeKind {
    /// File was created
    Created,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
}

impl SyncWatcher {
    /// Create a new sync watcher
    pub fn new<P: AsRef<Path>>(workspace_dir: P) -> Self {
        let persistence = Arc::new(WorkspacePersistence::new(workspace_dir));

        Self {
            watchers: HashMap::new(),
            running: Arc::new(Mutex::new(false)),
            persistence,
        }
    }

    /// Start monitoring a workspace directory
    pub async fn start_monitoring(&mut self, workspace_id: &str, directory: &str) -> Result<()> {
        let directory_path = PathBuf::from(directory);

        // Ensure directory exists
        if !directory_path.exists() {
            std::fs::create_dir_all(&directory_path)
                .map_err(|e| Error::generic(format!("Failed to create sync directory: {}", e)))?;
        }

        let (tx, mut rx) = mpsc::channel(100);
        let workspace_id_string = workspace_id.to_string();
        let workspace_id_for_watcher = workspace_id_string.clone();
        let workspace_id_for_processing = workspace_id_string.clone();
        let directory_path_clone = directory_path.clone();
        let directory_path_for_processing = directory_path.clone();
        let directory_str = directory.to_string();

        let config = Config::default()
            .with_poll_interval(Duration::from_secs(1))
            .with_compare_contents(true);

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    debug!("File system event: {:?}", event);
                    let tx_clone = tx.clone();
                    let workspace_id_clone = workspace_id_string.clone();
                    let dir_clone = directory_path_clone.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_fs_event(
                            &tx_clone,
                            &workspace_id_clone,
                            &dir_clone,
                            &event,
                        )
                        .await
                        {
                            error!("Failed to handle file system event: {}", e);
                        }
                    });
                }
            },
            config,
        )
        .map_err(|e| Error::generic(format!("Failed to create file watcher: {}", e)))?;

        // Watch the directory recursively
        watcher
            .watch(&directory_path, notify::RecursiveMode::Recursive)
            .map_err(|e| Error::generic(format!("Failed to watch directory: {}", e)))?;

        // Store the watcher
        self.watchers.insert(workspace_id_for_watcher, watcher);

        // Start processing events
        let persistence_clone = self.persistence.clone();
        let is_running = self.running.clone();

        tokio::spawn(async move {
            info!(
                "Started monitoring workspace {} in directory {}",
                workspace_id_for_processing, directory_str
            );
            info!(
                workspace_id = %workspace_id_for_processing,
                directory = %directory_str,
                "Monitoring workspace directory"
            );

            while *is_running.lock().await {
                match timeout(Duration::from_millis(100), rx.recv()).await {
                    Ok(Some(event)) => {
                        if let Err(e) = Self::process_sync_event(
                            &persistence_clone,
                            &workspace_id_for_processing,
                            &directory_path_for_processing,
                            event,
                        )
                        .await
                        {
                            error!("Failed to process sync event: {}", e);
                        }
                    }
                    Ok(None) => break,  // Channel closed
                    Err(_) => continue, // Timeout, continue monitoring
                }
            }

            info!(
                "Stopped monitoring workspace {} in directory {}",
                workspace_id_for_processing, directory_str
            );
            info!(
                workspace_id = %workspace_id_for_processing,
                directory = %directory_str,
                "Stopped monitoring workspace directory"
            );
        });

        Ok(())
    }

    /// Stop monitoring a workspace
    pub async fn stop_monitoring(&mut self, workspace_id: &str) -> Result<()> {
        if let Some(watcher) = self.watchers.remove(workspace_id) {
            // Dropping the watcher will stop it
            drop(watcher);
        }

        Ok(())
    }

    /// Stop all monitoring
    pub async fn stop_all(&mut self) -> Result<()> {
        *self.running.lock().await = false;
        self.watchers.clear();
        Ok(())
    }

    /// Handle a file system event
    async fn handle_fs_event(
        tx: &mpsc::Sender<SyncEvent>,
        workspace_id: &str,
        base_dir: &Path,
        event: &Event,
    ) -> Result<()> {
        let mut changes = Vec::new();

        for path in &event.paths {
            // Make path relative to the watched directory
            let relative_path = path.strip_prefix(base_dir).unwrap_or(path);

            // Skip metadata files and temporary files
            if relative_path.starts_with(".")
                || relative_path
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with("."))
                    .unwrap_or(false)
            {
                continue;
            }

            // Only process YAML files
            if let Some(extension) = path.extension() {
                if extension != "yaml" && extension != "yml" {
                    continue;
                }
            }

            match event.kind {
                notify::EventKind::Create(_) => {
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        changes.push(FileChange {
                            path: relative_path.to_path_buf(),
                            kind: ChangeKind::Created,
                            content: Some(content),
                        });
                    }
                }
                notify::EventKind::Modify(_) => {
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        changes.push(FileChange {
                            path: relative_path.to_path_buf(),
                            kind: ChangeKind::Modified,
                            content: Some(content),
                        });
                    }
                }
                notify::EventKind::Remove(_) => {
                    changes.push(FileChange {
                        path: relative_path.to_path_buf(),
                        kind: ChangeKind::Deleted,
                        content: None,
                    });
                }
                _ => {}
            }
        }

        if !changes.is_empty() {
            let _ = tx
                .send(SyncEvent::DirectoryChanged {
                    workspace_id: workspace_id.to_string(),
                    changes,
                })
                .await;
        }

        Ok(())
    }

    /// Process a sync event
    async fn process_sync_event(
        persistence: &WorkspacePersistence,
        _workspace_id: &str,
        _directory: &Path,
        event: SyncEvent,
    ) -> Result<()> {
        if let SyncEvent::DirectoryChanged {
            workspace_id,
            changes,
        } = event
        {
            info!("Processing {} file changes for workspace {}", changes.len(), workspace_id);

            if !changes.is_empty() {
                info!(
                    workspace_id = %workspace_id,
                    count = changes.len(),
                    "Detected file changes in workspace"
                );
            }

            for change in changes {
                match change.kind {
                    ChangeKind::Created => {
                        info!(path = %change.path.display(), "File created");
                        if let Some(content) = change.content {
                            if let Err(e) = Self::import_yaml_content(
                                persistence,
                                &workspace_id,
                                &change.path,
                                &content,
                            )
                            .await
                            {
                                warn!("Failed to import file {}: {}", change.path.display(), e);
                            } else {
                                info!(path = %change.path.display(), "Successfully imported");
                            }
                        }
                    }
                    ChangeKind::Modified => {
                        info!(path = %change.path.display(), "File modified");
                        if let Some(content) = change.content {
                            if let Err(e) = Self::import_yaml_content(
                                persistence,
                                &workspace_id,
                                &change.path,
                                &content,
                            )
                            .await
                            {
                                warn!("Failed to import file {}: {}", change.path.display(), e);
                            } else {
                                info!(path = %change.path.display(), "Successfully updated");
                            }
                        }
                    }
                    ChangeKind::Deleted => {
                        debug!("File deleted: {}", change.path.display());
                        debug!("Auto-deletion from workspace is disabled");
                        // For now, we don't auto-delete from workspace on file deletion
                        // This could be configurable in the future
                    }
                }
            }
        }

        Ok(())
    }

    /// Import YAML content into workspace
    async fn import_yaml_content(
        persistence: &WorkspacePersistence,
        workspace_id: &str,
        path: &Path,
        content: &str,
    ) -> Result<()> {
        // Load the workspace
        let workspace = persistence.load_workspace(workspace_id).await?;

        // Check sync direction before proceeding
        if !matches!(workspace.get_sync_direction(), crate::workspace::SyncDirection::Bidirectional)
        {
            debug!("Workspace {} is not configured for bidirectional sync", workspace_id);
            return Ok(());
        }

        // Try to parse as a workspace export
        if let Ok(_export) =
            serde_yaml::from_str::<crate::workspace_persistence::WorkspaceExport>(content)
        {
            // This is a full workspace export - we should be cautious about importing
            // For now, just log the intent
            info!(
                "Detected workspace export for {}, skipping full import to avoid conflicts",
                workspace_id
            );
            debug!("Skipping workspace export to avoid conflicts");
            return Ok(());
        }

        // Try to parse as a request
        if let Ok(request) = serde_yaml::from_str::<crate::workspace::MockRequest>(content) {
            // Import individual request
            debug!("Importing request {} from {}", request.name, path.display());

            let mut workspace = persistence.load_workspace(workspace_id).await?;
            // Add to root level
            workspace.add_request(request)?;
            persistence.save_workspace(&workspace).await?;

            info!(
                "Successfully imported request from {} into workspace {}",
                path.display(),
                workspace_id
            );
        } else {
            debug!("Content in {} is not a recognized format, skipping", path.display());
            return Err(Error::generic(
                "File is not a recognized format (expected MockRequest YAML)".to_string(),
            ));
        }

        Ok(())
    }

    /// Get monitoring status
    pub async fn is_monitoring(&self, workspace_id: &str) -> bool {
        self.watchers.contains_key(workspace_id)
    }

    /// Get list of monitored workspaces
    pub fn get_monitored_workspaces(&self) -> Vec<String> {
        self.watchers.keys().cloned().collect()
    }
}

impl Drop for SyncWatcher {
    fn drop(&mut self) {
        // Note: We can't await in drop, so watchers will be stopped when they're dropped
        // The runtime will handle cleanup
    }
}

/// Background sync service
pub struct SyncService {
    watcher: Arc<Mutex<SyncWatcher>>,
    running: Arc<Mutex<bool>>,
}

impl SyncService {
    /// Create a new sync service
    pub fn new<P: AsRef<Path>>(workspace_dir: P) -> Self {
        let watcher = Arc::new(Mutex::new(SyncWatcher::new(workspace_dir)));

        Self {
            watcher,
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the sync service
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.lock().await;
        *running = true;
        info!("Sync service started");
        Ok(())
    }

    /// Stop the sync service
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.lock().await;
        *running = false;

        let mut watcher = self.watcher.lock().await;
        watcher.stop_all().await?;
        info!("Sync service stopped");
        Ok(())
    }

    /// Start monitoring a workspace
    pub async fn monitor_workspace(&self, workspace_id: &str, directory: &str) -> Result<()> {
        let mut watcher = self.watcher.lock().await;
        watcher.start_monitoring(workspace_id, directory).await?;
        Ok(())
    }

    /// Stop monitoring a workspace
    pub async fn stop_monitoring_workspace(&self, workspace_id: &str) -> Result<()> {
        let mut watcher = self.watcher.lock().await;
        watcher.stop_monitoring(workspace_id).await?;
        Ok(())
    }

    /// Get monitoring status
    pub async fn is_workspace_monitored(&self, workspace_id: &str) -> bool {
        let watcher = self.watcher.lock().await;
        watcher.is_monitoring(workspace_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_sync_service_creation() {
        let temp_dir = TempDir::new().unwrap();
        let service = SyncService::new(temp_dir.path());

        assert!(!*service.running.lock().await);
    }

    #[tokio::test]
    async fn test_sync_service_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let service = SyncService::new(temp_dir.path());

        // Start service
        service.start().await.unwrap();
        assert!(*service.running.lock().await);

        // Stop service
        service.stop().await.unwrap();
        assert!(!*service.running.lock().await);
    }
}
