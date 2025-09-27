//! Synchronization functionality
//!
//! This module provides synchronization capabilities for workspaces,
//! including conflict resolution, merge strategies, and sync status tracking.

use crate::workspace::core::{EntityId, Workspace};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_yaml;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// Synchronization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Whether synchronization is enabled
    pub enabled: bool,
    /// Synchronization provider (git, cloud, etc.)
    pub provider: SyncProvider,
    /// Synchronization interval in seconds
    pub interval_seconds: u64,
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictResolutionStrategy,
    /// Whether to auto-commit changes
    pub auto_commit: bool,
    /// Whether to push changes automatically
    pub auto_push: bool,
    /// Directory structure preference
    pub directory_structure: SyncDirectoryStructure,
    /// Sync direction preference
    pub sync_direction: SyncDirection,
}

/// Synchronization provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncProvider {
    /// Git-based synchronization
    Git {
        /// Repository URL
        repo_url: String,
        /// Branch name
        branch: String,
        /// Authentication token (optional)
        auth_token: Option<String>,
    },
    /// Cloud-based synchronization
    Cloud {
        /// Service URL
        service_url: String,
        /// API key
        api_key: String,
        /// Project ID
        project_id: String,
    },
    /// Local file system synchronization
    Local {
        /// Directory path
        directory_path: String,
        /// Watch for changes
        watch_changes: bool,
    },
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    /// Always use local version
    LocalWins,
    /// Always use remote version
    RemoteWins,
    /// Manual resolution required
    Manual,
    /// Use last modified timestamp
    LastModified,
}

/// Directory structure for synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncDirectoryStructure {
    /// Single directory with all workspaces
    SingleDirectory,
    /// Separate directory per workspace
    PerWorkspace,
    /// Hierarchical structure based on folders
    Hierarchical,
}

/// Synchronization direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncDirection {
    /// Bidirectional sync
    Bidirectional,
    /// Local to remote only
    LocalToRemote,
    /// Remote to local only
    RemoteToLocal,
}

/// Synchronization status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// Last sync timestamp
    pub last_sync: Option<DateTime<Utc>>,
    /// Current sync state
    pub state: SyncState,
    /// Number of pending changes
    pub pending_changes: usize,
    /// Number of conflicts
    pub conflicts: usize,
    /// Last error message (if any)
    pub last_error: Option<String>,
}

/// Current synchronization state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncState {
    /// Not synchronized
    NotSynced,
    /// Currently syncing
    Syncing,
    /// Synchronized successfully
    Synced,
    /// Sync failed
    SyncFailed,
    /// Has conflicts
    HasConflicts,
}

/// Synchronization result
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Whether sync was successful
    pub success: bool,
    /// Number of files changed
    pub changes_count: usize,
    /// Conflicts that occurred
    pub conflicts: Vec<SyncConflict>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Conflict during synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    /// Entity ID that has conflict
    pub entity_id: EntityId,
    /// Entity type (workspace, request, etc.)
    pub entity_type: String,
    /// Local version
    pub local_version: serde_json::Value,
    /// Remote version
    pub remote_version: serde_json::Value,
    /// Resolution strategy used
    pub resolution: ConflictResolution,
}

/// Conflict resolution choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Use local version
    Local,
    /// Use remote version
    Remote,
    /// Manual resolution required
    Manual,
}

/// Workspace synchronization manager
#[derive(Debug, Clone)]
pub struct WorkspaceSyncManager {
    /// Synchronization configuration
    config: SyncConfig,
    /// Current sync status
    status: SyncStatus,
    /// Pending conflicts
    conflicts: Vec<SyncConflict>,
    /// Total number of sync operations performed
    total_syncs: usize,
    /// Number of successful syncs
    successful_syncs: usize,
    /// Number of failed syncs
    failed_syncs: usize,
    /// Total number of resolved conflicts
    resolved_conflicts: usize,
    /// Duration of last sync in milliseconds
    last_sync_duration_ms: Option<u64>,
}

/// Synchronization event
#[derive(Debug, Clone)]
pub enum SyncEvent {
    /// Sync started
    Started,
    /// Sync progress update
    Progress { current: usize, total: usize },
    /// Sync completed successfully
    Completed(SyncResult),
    /// Sync failed
    Failed(String),
    /// Conflict detected
    ConflictDetected(SyncConflict),
}

impl WorkspaceSyncManager {
    /// Create a new sync manager
    pub fn new(config: SyncConfig) -> Self {
        let status = SyncStatus {
            last_sync: None,
            state: SyncState::NotSynced,
            pending_changes: 0,
            conflicts: 0,
            last_error: None,
        };

        Self {
            config,
            status,
            conflicts: Vec::new(),
            total_syncs: 0,
            successful_syncs: 0,
            failed_syncs: 0,
            resolved_conflicts: 0,
            last_sync_duration_ms: None,
        }
    }

    /// Get the current sync configuration
    pub fn get_config(&self) -> &SyncConfig {
        &self.config
    }

    /// Update sync configuration
    pub fn update_config(&mut self, config: SyncConfig) {
        self.config = config;
    }

    /// Get current sync status
    pub fn get_status(&self) -> &SyncStatus {
        &self.status
    }

    /// Get pending conflicts
    pub fn get_conflicts(&self) -> &[SyncConflict] {
        &self.conflicts
    }

    /// Check if sync is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Sync a workspace
    pub async fn sync_workspace(
        &mut self,
        workspace: &mut Workspace,
    ) -> Result<SyncResult, String> {
        if !self.config.enabled {
            return Err("Synchronization is disabled".to_string());
        }

        // Track sync count
        self.total_syncs += 1;

        // Start timing
        let start_time = std::time::Instant::now();

        self.status.state = SyncState::Syncing;
        self.status.last_error = None;

        let result = match &self.config.provider {
            SyncProvider::Git {
                repo_url,
                branch,
                auth_token,
            } => self.sync_with_git(workspace, repo_url, branch, auth_token.as_deref()).await,
            SyncProvider::Cloud {
                service_url,
                api_key,
                project_id,
            } => self.sync_with_cloud(workspace, service_url, api_key, project_id).await,
            SyncProvider::Local {
                directory_path,
                watch_changes,
            } => self.sync_with_local(workspace, directory_path, *watch_changes).await,
        };

        // Calculate duration
        let duration = start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;
        self.last_sync_duration_ms = Some(duration_ms);

        match &result {
            Ok(sync_result) => {
                if sync_result.success {
                    self.successful_syncs += 1;
                    self.status.state = SyncState::Synced;
                    self.status.last_sync = Some(Utc::now());
                    self.status.pending_changes = 0;
                    self.status.conflicts = sync_result.conflicts.len();
                } else {
                    self.failed_syncs += 1;
                    self.status.state = SyncState::SyncFailed;
                    self.status.last_error = sync_result.error.clone();
                }
            }
            Err(error) => {
                self.failed_syncs += 1;
                self.status.state = SyncState::SyncFailed;
                self.status.last_error = Some(error.clone());
            }
        }

        result
    }

    /// Sync with Git provider
    async fn sync_with_git(
        &self,
        workspace: &mut Workspace,
        repo_url: &str,
        branch: &str,
        auth_token: Option<&str>,
    ) -> Result<SyncResult, String> {
        // Create a temporary directory for the Git repository
        let temp_dir =
            tempfile::tempdir().map_err(|e| format!("Failed to create temp directory: {}", e))?;

        let repo_path = temp_dir.path().join("repo");

        match self.config.sync_direction {
            SyncDirection::LocalToRemote => {
                self.sync_local_to_git(workspace, repo_url, branch, auth_token, &repo_path)
                    .await
            }
            SyncDirection::RemoteToLocal => {
                self.sync_git_to_local(workspace, repo_url, branch, auth_token, &repo_path)
                    .await
            }
            SyncDirection::Bidirectional => {
                self.sync_bidirectional_git(workspace, repo_url, branch, auth_token, &repo_path)
                    .await
            }
        }
    }

    /// Sync local workspace to Git repository
    async fn sync_local_to_git(
        &self,
        workspace: &Workspace,
        repo_url: &str,
        branch: &str,
        auth_token: Option<&str>,
        repo_path: &std::path::Path,
    ) -> Result<SyncResult, String> {
        // Clone or ensure repository exists
        self.ensure_git_repo(repo_url, branch, auth_token, repo_path).await?;

        // Serialize workspace to YAML file
        let workspace_file = repo_path.join(format!("{}.yaml", workspace.id));
        let workspace_yaml = serde_yaml::to_string(workspace)
            .map_err(|e| format!("Failed to serialize workspace: {}", e))?;

        tokio::fs::write(&workspace_file, &workspace_yaml)
            .await
            .map_err(|e| format!("Failed to write workspace file: {}", e))?;

        // Add, commit, and push changes
        self.git_add_commit_push(repo_path, &workspace_file, auth_token).await?;

        Ok(SyncResult {
            success: true,
            changes_count: 1,
            conflicts: vec![],
            error: None,
        })
    }

    /// Sync Git repository to local workspace
    async fn sync_git_to_local(
        &self,
        workspace: &mut Workspace,
        repo_url: &str,
        branch: &str,
        auth_token: Option<&str>,
        repo_path: &std::path::Path,
    ) -> Result<SyncResult, String> {
        // Clone or pull repository
        self.ensure_git_repo(repo_url, branch, auth_token, repo_path).await?;

        // Read workspace from Git repository
        let workspace_file = repo_path.join(format!("{}.yaml", workspace.id));

        if !workspace_file.exists() {
            return Ok(SyncResult {
                success: true,
                changes_count: 0,
                conflicts: vec![],
                error: None,
            });
        }

        let workspace_yaml = tokio::fs::read_to_string(&workspace_file)
            .await
            .map_err(|e| format!("Failed to read workspace file: {}", e))?;

        let remote_workspace: Workspace = serde_yaml::from_str(&workspace_yaml)
            .map_err(|e| format!("Failed to deserialize workspace: {}", e))?;

        // Check for conflicts
        let conflicts = self.detect_conflicts(workspace, &remote_workspace);

        if conflicts.is_empty() {
            // No conflicts, update local workspace
            *workspace = remote_workspace;
            Ok(SyncResult {
                success: true,
                changes_count: 1,
                conflicts: vec![],
                error: None,
            })
        } else {
            // Conflicts exist
            Ok(SyncResult {
                success: true,
                changes_count: 0,
                conflicts,
                error: None,
            })
        }
    }

    /// Bidirectional sync with Git repository
    async fn sync_bidirectional_git(
        &self,
        workspace: &mut Workspace,
        repo_url: &str,
        branch: &str,
        auth_token: Option<&str>,
        repo_path: &std::path::Path,
    ) -> Result<SyncResult, String> {
        // First sync from Git to local
        let pull_result = self
            .sync_git_to_local(workspace, repo_url, branch, auth_token, repo_path)
            .await?;

        if !pull_result.conflicts.is_empty() {
            // Conflicts detected, return them
            return Ok(pull_result);
        }

        // No conflicts, sync local to Git
        self.sync_local_to_git(workspace, repo_url, branch, auth_token, repo_path).await
    }

    /// Ensure Git repository exists and is up to date
    async fn ensure_git_repo(
        &self,
        repo_url: &str,
        branch: &str,
        auth_token: Option<&str>,
        repo_path: &std::path::Path,
    ) -> Result<(), String> {
        use std::process::Command;

        if repo_path.exists() {
            // Repository exists, pull latest changes
            let output = Command::new("git")
                .args(["-C", &repo_path.to_string_lossy(), "pull", "origin", branch])
                .output()
                .map_err(|e| format!("Failed to pull repository: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Git pull failed: {}", stderr));
            }
        } else {
            // Clone repository
            // If auth token provided, modify URL for authentication
            let clone_url = if let Some(token) = auth_token {
                self.inject_auth_token_into_url(repo_url, token)
            } else {
                repo_url.to_string()
            };

            let output = Command::new("git")
                .args([
                    "clone",
                    "--branch",
                    branch,
                    &clone_url,
                    &repo_path.to_string_lossy(),
                ])
                .output()
                .map_err(|e| format!("Failed to clone repository: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Git clone failed: {}", stderr));
            }
        }

        Ok(())
    }

    /// Add, commit, and push changes to Git repository
    async fn git_add_commit_push(
        &self,
        repo_path: &std::path::Path,
        workspace_file: &std::path::Path,
        _auth_token: Option<&str>,
    ) -> Result<(), String> {
        use std::process::Command;

        let repo_path_str = repo_path.to_string_lossy();
        let file_path_str = workspace_file
            .strip_prefix(repo_path)
            .unwrap_or(workspace_file)
            .to_string_lossy();

        // Add file
        let output = Command::new("git")
            .args(["-C", &repo_path_str, "add", &file_path_str])
            .output()
            .map_err(|e| format!("Failed to add file to git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Git add failed: {}", stderr));
        }

        // Check if there are changes to commit
        let status_output = Command::new("git")
            .args(["-C", &repo_path_str, "status", "--porcelain"])
            .output()
            .map_err(|e| format!("Failed to check git status: {}", e))?;

        if status_output.stdout.is_empty() {
            // No changes to commit
            return Ok(());
        }

        // Commit changes
        let output = Command::new("git")
            .args(["-C", &repo_path_str, "commit", "-m", "Update workspace"])
            .output()
            .map_err(|e| format!("Failed to commit changes: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Git commit failed: {}", stderr));
        }

        // Push changes
        let output = Command::new("git")
            .args(["-C", &repo_path_str, "push", "origin", "HEAD"])
            .output()
            .map_err(|e| format!("Failed to push changes: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Git push failed: {}", stderr));
        }

        Ok(())
    }

    /// Inject authentication token into Git URL
    fn inject_auth_token_into_url(&self, url: &str, token: &str) -> String {
        if let Some(https_pos) = url.find("https://") {
            let rest = &url[https_pos + "https://".len()..];
            format!("https://oauth2:{}@{}", token, rest)
        } else {
            // For SSH URLs or other formats, return as-is
            url.to_string()
        }
    }

    /// Sync with cloud provider
    async fn sync_with_cloud(
        &self,
        workspace: &mut Workspace,
        service_url: &str,
        api_key: &str,
        project_id: &str,
    ) -> Result<SyncResult, String> {
        // Create HTTP client
        let client = reqwest::Client::new();

        // Build API URLs
        let base_url = service_url.trim_end_matches('/');
        let workspace_url =
            format!("{}/api/v1/projects/{}/workspaces/{}", base_url, project_id, workspace.id);

        match self.config.sync_direction {
            SyncDirection::LocalToRemote => {
                // Only upload local workspace to cloud
                self.upload_workspace_to_cloud(&client, &workspace_url, api_key, workspace)
                    .await
            }
            SyncDirection::RemoteToLocal => {
                // Only download remote workspace and update local
                self.download_workspace_from_cloud(&client, &workspace_url, api_key, workspace)
                    .await
            }
            SyncDirection::Bidirectional => {
                // Fetch remote, compare, handle conflicts, then upload if needed
                self.bidirectional_sync(&client, &workspace_url, api_key, workspace).await
            }
        }
    }

    /// Upload workspace to cloud service
    async fn upload_workspace_to_cloud(
        &self,
        client: &reqwest::Client,
        workspace_url: &str,
        api_key: &str,
        workspace: &Workspace,
    ) -> Result<SyncResult, String> {
        // Serialize workspace to JSON
        let workspace_json = serde_json::to_string(workspace)
            .map_err(|e| format!("Failed to serialize workspace: {}", e))?;

        // Upload to cloud
        let response = client
            .put(workspace_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .body(workspace_json)
            .send()
            .await
            .map_err(|e| format!("Failed to upload workspace: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Cloud upload failed with status {}: {}", status, error_text));
        }

        Ok(SyncResult {
            success: true,
            changes_count: 1,
            conflicts: vec![],
            error: None,
        })
    }

    /// Download workspace from cloud service
    async fn download_workspace_from_cloud(
        &self,
        client: &reqwest::Client,
        workspace_url: &str,
        api_key: &str,
        workspace: &mut Workspace,
    ) -> Result<SyncResult, String> {
        // Download from cloud
        let response = client
            .get(workspace_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| format!("Failed to download workspace: {}", e))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // Workspace doesn't exist in cloud, nothing to sync
            return Ok(SyncResult {
                success: true,
                changes_count: 0,
                conflicts: vec![],
                error: None,
            });
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Cloud download failed with status {}: {}", status, error_text));
        }

        let remote_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse remote workspace: {}", e))?;

        // Deserialize remote workspace
        let remote_workspace: Workspace = serde_json::from_value(remote_json.clone())
            .map_err(|e| format!("Failed to deserialize remote workspace: {}", e))?;

        // Check for conflicts based on timestamps
        let conflicts = self.detect_conflicts(workspace, &remote_workspace);

        // Apply conflict resolution
        if conflicts.is_empty() {
            // No conflicts, update local workspace with remote
            *workspace = remote_workspace;
            Ok(SyncResult {
                success: true,
                changes_count: 1,
                conflicts: vec![],
                error: None,
            })
        } else {
            // Conflicts exist, return them for manual resolution
            Ok(SyncResult {
                success: true,
                changes_count: 0,
                conflicts,
                error: None,
            })
        }
    }

    /// Perform bidirectional synchronization
    async fn bidirectional_sync(
        &self,
        client: &reqwest::Client,
        workspace_url: &str,
        api_key: &str,
        workspace: &mut Workspace,
    ) -> Result<SyncResult, String> {
        // First try to download remote workspace
        let download_result = self
            .download_workspace_from_cloud(client, workspace_url, api_key, workspace)
            .await?;

        if !download_result.conflicts.is_empty() {
            // Conflicts detected, return them
            return Ok(download_result);
        }

        // No conflicts, upload local workspace
        self.upload_workspace_to_cloud(client, workspace_url, api_key, workspace).await
    }

    /// Detect conflicts between local and remote workspaces
    fn detect_conflicts(&self, local: &Workspace, remote: &Workspace) -> Vec<SyncConflict> {
        let mut conflicts = vec![];

        // Simple conflict detection based on updated_at timestamps
        if local.updated_at > remote.updated_at {
            // Local is newer, potential conflict
            let local_json = serde_json::to_value(local).unwrap_or_default();
            let remote_json = serde_json::to_value(remote).unwrap_or_default();

            if local_json != remote_json {
                conflicts.push(SyncConflict {
                    entity_id: local.id.clone(),
                    entity_type: "workspace".to_string(),
                    local_version: local_json,
                    remote_version: remote_json,
                    resolution: ConflictResolution::Manual,
                });
            }
        }

        conflicts
    }

    /// Sync with local filesystem
    async fn sync_with_local(
        &self,
        workspace: &mut Workspace,
        directory_path: &str,
        _watch_changes: bool,
    ) -> Result<SyncResult, String> {
        let dir_path = Path::new(directory_path);

        // Ensure directory exists
        if !dir_path.exists() {
            fs::create_dir_all(dir_path)
                .await
                .map_err(|e| format!("Failed to create directory {}: {}", directory_path, e))?;
        }

        match self.config.sync_direction {
            SyncDirection::LocalToRemote => {
                // Write workspace to file
                let file_path = dir_path.join(format!("{}.yaml", workspace.id));
                let content = serde_yaml::to_string(workspace)
                    .map_err(|e| format!("Failed to serialize workspace: {}", e))?;

                fs::write(&file_path, content)
                    .await
                    .map_err(|e| format!("Failed to write workspace file: {}", e))?;

                Ok(SyncResult {
                    success: true,
                    changes_count: 1,
                    conflicts: vec![],
                    error: None,
                })
            }
            SyncDirection::RemoteToLocal => {
                // Load workspace from file
                let file_path = dir_path.join(format!("{}.yaml", workspace.id));

                if !file_path.exists() {
                    return Err(format!("Workspace file not found: {:?}", file_path));
                }

                let content = fs::read_to_string(&file_path)
                    .await
                    .map_err(|e| format!("Failed to read workspace file: {}", e))?;

                let remote_workspace: Workspace = serde_yaml::from_str(&content)
                    .map_err(|e| format!("Failed to deserialize workspace: {}", e))?;

                // Compare workspaces and detect conflicts
                let conflicts = {
                    let mut conflicts = vec![];

                    // Check for conflicts
                    if workspace.updated_at > remote_workspace.updated_at {
                        // Local is newer, this is a conflict
                        let local_json = serde_json::to_value(&*workspace).unwrap_or_default();
                        let remote_json =
                            serde_json::to_value(&remote_workspace).unwrap_or_default();
                        conflicts.push(SyncConflict {
                            entity_id: workspace.id.clone(),
                            entity_type: "workspace".to_string(),
                            local_version: local_json,
                            remote_version: remote_json,
                            resolution: ConflictResolution::Manual,
                        });
                    } else if workspace.updated_at == remote_workspace.updated_at {
                        // Same timestamp, check if content differs
                        let local_json = serde_json::to_value(&*workspace).unwrap_or_default();
                        let remote_json =
                            serde_json::to_value(&remote_workspace).unwrap_or_default();
                        if local_json != remote_json {
                            // Content differs but timestamps are same, conflict
                            conflicts.push(SyncConflict {
                                entity_id: workspace.id.clone(),
                                entity_type: "workspace".to_string(),
                                local_version: local_json,
                                remote_version: remote_json,
                                resolution: ConflictResolution::Manual,
                            });
                        }
                    }

                    conflicts
                };

                // If no conflicts and remote is newer or equal, update local workspace with remote
                if conflicts.is_empty() && remote_workspace.updated_at >= workspace.updated_at {
                    *workspace = remote_workspace;
                    Ok(SyncResult {
                        success: true,
                        changes_count: 1,
                        conflicts: vec![],
                        error: None,
                    })
                } else {
                    Ok(SyncResult {
                        success: true,
                        changes_count: 0,
                        conflicts,
                        error: None,
                    })
                }
            }
            SyncDirection::Bidirectional => {
                // For bidirectional, first try to load remote, then write local
                let file_path = dir_path.join(format!("{}.yaml", workspace.id));

                let mut conflicts = vec![];

                if file_path.exists() {
                    let content = fs::read_to_string(&file_path)
                        .await
                        .map_err(|e| format!("Failed to read workspace file: {}", e))?;

                    let remote_workspace: Workspace = serde_yaml::from_str(&content)
                        .map_err(|e| format!("Failed to deserialize workspace: {}", e))?;

                    // Simple conflict detection based on updated_at
                    if remote_workspace.updated_at > workspace.updated_at {
                        // Remote is newer, this would be a conflict
                        let remote_version =
                            serde_json::to_value(&remote_workspace).unwrap_or_default();
                        conflicts.push(SyncConflict {
                            entity_id: workspace.id.clone(),
                            entity_type: "workspace".to_string(),
                            local_version: serde_json::to_value(&*workspace).unwrap_or_default(),
                            remote_version,
                            resolution: ConflictResolution::Manual,
                        });
                    }
                }

                // Write local workspace
                let content = serde_yaml::to_string(workspace)
                    .map_err(|e| format!("Failed to serialize workspace: {}", e))?;

                fs::write(&file_path, content)
                    .await
                    .map_err(|e| format!("Failed to write workspace file: {}", e))?;

                Ok(SyncResult {
                    success: true,
                    changes_count: 1,
                    conflicts,
                    error: None,
                })
            }
        }
    }

    /// Resolve conflicts
    pub fn resolve_conflicts(
        &mut self,
        resolutions: HashMap<EntityId, ConflictResolution>,
    ) -> Result<usize, String> {
        let mut resolved_count = 0;

        for conflict in &self.conflicts.clone() {
            if let Some(resolution) = resolutions.get(&conflict.entity_id) {
                match resolution {
                    ConflictResolution::Local => {
                        // Apply local version
                        resolved_count += 1;
                    }
                    ConflictResolution::Remote => {
                        // Apply remote version
                        resolved_count += 1;
                    }
                    ConflictResolution::Manual => {
                        // Mark for manual resolution
                        continue;
                    }
                }
            }
        }

        // Track resolved conflicts
        self.resolved_conflicts += resolved_count;

        // Remove resolved conflicts
        self.conflicts.retain(|conflict| {
            !resolutions.contains_key(&conflict.entity_id)
                || matches!(resolutions.get(&conflict.entity_id), Some(ConflictResolution::Manual))
        });

        self.status.conflicts = self.conflicts.len();
        if self.conflicts.is_empty() {
            self.status.state = SyncState::Synced;
        } else {
            self.status.state = SyncState::HasConflicts;
        }

        Ok(resolved_count)
    }

    /// Get sync statistics
    pub fn get_sync_stats(&self) -> SyncStats {
        SyncStats {
            total_syncs: self.total_syncs,
            successful_syncs: self.successful_syncs,
            failed_syncs: self.failed_syncs,
            total_conflicts: self.conflicts.len(),
            resolved_conflicts: self.resolved_conflicts,
            last_sync_duration_ms: self.last_sync_duration_ms,
        }
    }

    /// Export sync configuration
    pub fn export_config(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.config)
            .map_err(|e| format!("Failed to serialize sync config: {}", e))
    }

    /// Import sync configuration
    pub fn import_config(&mut self, json_data: &str) -> Result<(), String> {
        let config: SyncConfig = serde_json::from_str(json_data)
            .map_err(|e| format!("Failed to deserialize sync config: {}", e))?;

        self.config = config;
        Ok(())
    }

    /// Check if there are pending changes
    pub fn has_pending_changes(&self) -> bool {
        self.status.pending_changes > 0
    }

    /// Get conflicts that need manual resolution
    pub fn get_manual_conflicts(&self) -> Vec<&SyncConflict> {
        self.conflicts
            .iter()
            .filter(|_conflict| {
                // This would need to be determined based on the conflict resolution strategy
                true
            })
            .collect()
    }
}

/// Synchronization statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    /// Total number of sync operations
    pub total_syncs: usize,
    /// Number of successful syncs
    pub successful_syncs: usize,
    /// Number of failed syncs
    pub failed_syncs: usize,
    /// Total number of conflicts encountered
    pub total_conflicts: usize,
    /// Number of resolved conflicts
    pub resolved_conflicts: usize,
    /// Duration of last sync in milliseconds
    pub last_sync_duration_ms: Option<u64>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: SyncProvider::Local {
                directory_path: "./workspaces".to_string(),
                watch_changes: true,
            },
            interval_seconds: 300,
            conflict_strategy: ConflictResolutionStrategy::LocalWins,
            auto_commit: true,
            auto_push: false,
            directory_structure: SyncDirectoryStructure::PerWorkspace,
            sync_direction: SyncDirection::Bidirectional,
        }
    }
}

impl Default for WorkspaceSyncManager {
    fn default() -> Self {
        Self::new(SyncConfig::default())
    }
}
