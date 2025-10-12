//! Persistence layer for workspace configurations
//!
//! This module handles saving and loading workspace configurations to/from disk,
//! enabling persistent storage of workspace hierarchies and configurations.

use crate::config::AuthConfig as ConfigAuthConfig;
use crate::encryption::{utils, AutoEncryptionProcessor, WorkspaceKeyManager};
use crate::workspace::{EntityId, Folder, MockRequest, Workspace, WorkspaceRegistry};
use crate::{Error, Result};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

// Pre-compiled regex patterns for sensitive data detection
static CREDIT_CARD_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b")
        .expect("CREDIT_CARD_PATTERN regex is valid")
});

static SSN_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\d{3}[-\s]?\d{2}[-\s]?\d{4}\b").expect("SSN_PATTERN regex is valid")
});

/// Workspace persistence manager
#[derive(Debug)]
pub struct WorkspacePersistence {
    /// Base directory for workspace storage
    base_dir: PathBuf,
}

/// Serializable workspace registry for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableWorkspaceRegistry {
    workspaces: Vec<Workspace>,
    active_workspace: Option<EntityId>,
}

/// Sync state for tracking incremental syncs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// Last time a sync operation was performed
    pub last_sync_timestamp: DateTime<Utc>,
}

/// Sync strategy for workspace mirroring
#[derive(Debug, Clone, PartialEq)]
pub enum SyncStrategy {
    /// Sync all workspaces completely
    Full,
    /// Sync only changed workspaces (based on modification time)
    Incremental,
    /// Sync only specified workspace IDs
    Selective(Vec<String>),
}

/// Directory structure for synced workspaces
#[derive(Debug, Clone, PartialEq)]
pub enum DirectoryStructure {
    /// All workspaces in a flat structure: workspace-id.yaml
    Flat,
    /// Nested by workspace: workspaces/{name}/workspace.yaml + requests/
    Nested,
    /// Grouped by type: requests/, responses/, metadata/
    Grouped,
}

/// Result of a workspace sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Number of workspaces synced
    pub synced_workspaces: usize,
    /// Number of requests synced
    pub synced_requests: usize,
    /// Number of files created/updated
    pub files_created: usize,
    /// Target directory used
    pub target_dir: PathBuf,
}

/// Result of an encrypted workspace export
#[derive(Debug, Clone)]
pub struct EncryptedExportResult {
    /// Path to the encrypted export file
    pub output_path: PathBuf,
    /// Backup key for importing on other devices
    pub backup_key: String,
    /// When the export was created
    pub exported_at: DateTime<Utc>,
    /// Name of the exported workspace
    pub workspace_name: String,
    /// Whether encryption was successfully applied
    pub encryption_enabled: bool,
}

/// Result of an encrypted workspace import
#[derive(Debug, Clone)]
pub struct EncryptedImportResult {
    /// ID of the imported workspace
    pub workspace_id: String,
    /// Name of the imported workspace
    pub workspace_name: String,
    /// When the import was completed
    pub imported_at: DateTime<Utc>,
    /// Number of requests imported
    pub request_count: usize,
    /// Whether encryption was successfully restored
    pub encryption_restored: bool,
}

/// Result of a security check for sensitive data
#[derive(Debug, Clone)]
pub struct SecurityCheckResult {
    /// Workspace ID that was checked
    pub workspace_id: String,
    /// Workspace name that was checked
    pub workspace_name: String,
    /// Security warnings found
    pub warnings: Vec<SecurityWarning>,
    /// Security errors found (critical issues)
    pub errors: Vec<SecurityWarning>,
    /// Whether the workspace is considered secure
    pub is_secure: bool,
    /// Recommended actions to improve security
    pub recommended_actions: Vec<String>,
}

/// Security warning or error
#[derive(Debug, Clone)]
pub struct SecurityWarning {
    /// Type of field that contains sensitive data
    pub field_type: String,
    /// Name of the field
    pub field_name: String,
    /// Location where the sensitive data was found
    pub location: String,
    /// Severity of the issue
    pub severity: SecuritySeverity,
    /// Human-readable message
    pub message: String,
    /// Suggestion for fixing the issue
    pub suggestion: String,
}

/// Severity levels for security issues
#[derive(Debug, Clone, PartialEq)]
pub enum SecuritySeverity {
    /// Low risk - informational
    Low,
    /// Medium risk - should be reviewed
    Medium,
    /// High risk - requires attention
    High,
    /// Critical risk - blocks operations
    Critical,
}

/// Git-friendly workspace export format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceExport {
    /// Workspace metadata
    pub metadata: WorkspaceMetadata,
    /// Workspace configuration
    pub config: WorkspaceConfig,
    /// All requests organized by folder structure
    pub requests: HashMap<String, ExportedRequest>,
}

/// Metadata for exported workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
    /// Original workspace ID
    pub id: String,
    /// Workspace name
    pub name: String,
    /// Workspace description
    pub description: Option<String>,
    /// Export timestamp
    pub exported_at: DateTime<Utc>,
    /// Total number of requests
    pub request_count: usize,
    /// Total number of folders
    pub folder_count: usize,
}

/// Simplified workspace configuration for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
    /// Base URL for requests
    pub base_url: Option<String>,
    /// Environment variables
    pub variables: HashMap<String, String>,
}

/// Authentication configuration for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication type
    pub auth_type: String,
    /// Authentication parameters
    pub params: HashMap<String, String>,
}

impl AuthConfig {
    /// Convert from config AuthConfig to export AuthConfig
    pub fn from_config_auth(config_auth: &ConfigAuthConfig) -> Option<Self> {
        if let Some(jwt) = &config_auth.jwt {
            let mut params = HashMap::new();
            if let Some(secret) = &jwt.secret {
                params.insert("secret".to_string(), secret.clone());
            }
            if let Some(rsa_public_key) = &jwt.rsa_public_key {
                params.insert("rsa_public_key".to_string(), rsa_public_key.clone());
            }
            if let Some(ecdsa_public_key) = &jwt.ecdsa_public_key {
                params.insert("ecdsa_public_key".to_string(), ecdsa_public_key.clone());
            }
            if let Some(issuer) = &jwt.issuer {
                params.insert("issuer".to_string(), issuer.clone());
            }
            if let Some(audience) = &jwt.audience {
                params.insert("audience".to_string(), audience.clone());
            }
            if !jwt.algorithms.is_empty() {
                params.insert("algorithms".to_string(), jwt.algorithms.join(","));
            }
            Some(AuthConfig {
                auth_type: "jwt".to_string(),
                params,
            })
        } else if let Some(oauth2) = &config_auth.oauth2 {
            let mut params = HashMap::new();
            params.insert("client_id".to_string(), oauth2.client_id.clone());
            params.insert("client_secret".to_string(), oauth2.client_secret.clone());
            params.insert("introspection_url".to_string(), oauth2.introspection_url.clone());
            if let Some(auth_url) = &oauth2.auth_url {
                params.insert("auth_url".to_string(), auth_url.clone());
            }
            if let Some(token_url) = &oauth2.token_url {
                params.insert("token_url".to_string(), token_url.clone());
            }
            if let Some(token_type_hint) = &oauth2.token_type_hint {
                params.insert("token_type_hint".to_string(), token_type_hint.clone());
            }
            Some(AuthConfig {
                auth_type: "oauth2".to_string(),
                params,
            })
        } else if let Some(basic_auth) = &config_auth.basic_auth {
            let mut params = HashMap::new();
            for (user, pass) in &basic_auth.credentials {
                params.insert(user.clone(), pass.clone());
            }
            Some(AuthConfig {
                auth_type: "basic".to_string(),
                params,
            })
        } else if let Some(api_key) = &config_auth.api_key {
            let mut params = HashMap::new();
            params.insert("header_name".to_string(), api_key.header_name.clone());
            if let Some(query_name) = &api_key.query_name {
                params.insert("query_name".to_string(), query_name.clone());
            }
            if !api_key.keys.is_empty() {
                params.insert("keys".to_string(), api_key.keys.join(","));
            }
            Some(AuthConfig {
                auth_type: "api_key".to_string(),
                params,
            })
        } else {
            None
        }
    }
}

/// Exported request format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedRequest {
    /// Request ID
    pub id: String,
    /// Request name
    pub name: String,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Folder path (for organization)
    pub folder_path: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Request body
    pub body: Option<String>,
    /// Response status code
    pub response_status: Option<u16>,
    /// Response body
    pub response_body: Option<String>,
    /// Response headers
    pub response_headers: HashMap<String, String>,
    /// Response delay (ms)
    pub delay: Option<u64>,
}

impl WorkspacePersistence {
    /// Create a new persistence manager
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Get the workspace directory path
    pub fn workspace_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Get the path for a specific workspace file
    pub fn workspace_file_path(&self, workspace_id: &str) -> PathBuf {
        self.base_dir.join(format!("{}.yaml", workspace_id))
    }

    /// Get the registry metadata file path
    pub fn registry_file_path(&self) -> PathBuf {
        self.base_dir.join("registry.yaml")
    }

    /// Get the sync state file path
    pub fn sync_state_file_path(&self) -> PathBuf {
        self.base_dir.join("sync_state.yaml")
    }

    /// Ensure the workspace directory exists
    pub async fn ensure_workspace_dir(&self) -> Result<()> {
        if !self.base_dir.exists() {
            fs::create_dir_all(&self.base_dir).await.map_err(|e| {
                Error::generic(format!("Failed to create workspace directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Save a workspace to disk
    pub async fn save_workspace(&self, workspace: &Workspace) -> Result<()> {
        self.ensure_workspace_dir().await?;

        let file_path = self.workspace_file_path(&workspace.id);
        let content = serde_yaml::to_string(workspace)
            .map_err(|e| Error::generic(format!("Failed to serialize workspace: {}", e)))?;

        fs::write(&file_path, content)
            .await
            .map_err(|e| Error::generic(format!("Failed to write workspace file: {}", e)))?;

        Ok(())
    }

    /// Load a workspace from disk
    pub async fn load_workspace(&self, workspace_id: &str) -> Result<Workspace> {
        let file_path = self.workspace_file_path(workspace_id);

        if !file_path.exists() {
            return Err(Error::generic(format!("Workspace file not found: {:?}", file_path)));
        }

        let content = fs::read_to_string(&file_path)
            .await
            .map_err(|e| Error::generic(format!("Failed to read workspace file: {}", e)))?;

        let workspace: Workspace = serde_yaml::from_str(&content)
            .map_err(|e| Error::generic(format!("Failed to deserialize workspace: {}", e)))?;

        Ok(workspace)
    }

    /// Delete a workspace from disk
    pub async fn delete_workspace(&self, workspace_id: &str) -> Result<()> {
        let file_path = self.workspace_file_path(workspace_id);

        if file_path.exists() {
            fs::remove_file(&file_path)
                .await
                .map_err(|e| Error::generic(format!("Failed to delete workspace file: {}", e)))?;
        }

        Ok(())
    }

    /// Save the workspace registry metadata
    pub async fn save_registry(&self, registry: &WorkspaceRegistry) -> Result<()> {
        self.ensure_workspace_dir().await?;

        let serializable = SerializableWorkspaceRegistry {
            workspaces: registry.get_workspaces().into_iter().cloned().collect(),
            active_workspace: registry.get_active_workspace_id().map(|s| s.to_string()),
        };

        let file_path = self.registry_file_path();
        let content = serde_yaml::to_string(&serializable)
            .map_err(|e| Error::generic(format!("Failed to serialize registry: {}", e)))?;

        fs::write(&file_path, content)
            .await
            .map_err(|e| Error::generic(format!("Failed to write registry file: {}", e)))?;

        Ok(())
    }

    /// Load the workspace registry metadata
    pub async fn load_registry(&self) -> Result<WorkspaceRegistry> {
        let file_path = self.registry_file_path();

        if !file_path.exists() {
            // Return empty registry if no registry file exists
            return Ok(WorkspaceRegistry::new());
        }

        let content = fs::read_to_string(&file_path)
            .await
            .map_err(|e| Error::generic(format!("Failed to read registry file: {}", e)))?;

        let serializable: SerializableWorkspaceRegistry = serde_yaml::from_str(&content)
            .map_err(|e| Error::generic(format!("Failed to deserialize registry: {}", e)))?;

        let mut registry = WorkspaceRegistry::new();

        // Load individual workspaces
        for workspace_meta in &serializable.workspaces {
            match self.load_workspace(&workspace_meta.id).await {
                Ok(workspace) => {
                    registry.add_workspace(workspace)?;
                }
                Err(e) => {
                    tracing::warn!("Failed to load workspace {}: {}", workspace_meta.id, e);
                }
            }
        }

        // Set active workspace
        if let Some(active_id) = &serializable.active_workspace {
            if let Err(e) = registry.set_active_workspace(Some(active_id.clone())) {
                tracing::warn!("Failed to set active workspace {}: {}", active_id, e);
            }
        }

        Ok(registry)
    }

    /// Save the sync state
    pub async fn save_sync_state(&self, sync_state: &SyncState) -> Result<()> {
        self.ensure_workspace_dir().await?;

        let file_path = self.sync_state_file_path();
        let content = serde_yaml::to_string(sync_state)
            .map_err(|e| Error::generic(format!("Failed to serialize sync state: {}", e)))?;

        fs::write(&file_path, content)
            .await
            .map_err(|e| Error::generic(format!("Failed to write sync state file: {}", e)))?;

        Ok(())
    }

    /// Load the sync state
    pub async fn load_sync_state(&self) -> Result<SyncState> {
        let file_path = self.sync_state_file_path();

        if !file_path.exists() {
            // Return default sync state if no sync state file exists
            return Ok(SyncState {
                last_sync_timestamp: Utc::now(),
            });
        }

        let content = fs::read_to_string(&file_path)
            .await
            .map_err(|e| Error::generic(format!("Failed to read sync state file: {}", e)))?;

        let sync_state: SyncState = serde_yaml::from_str(&content)
            .map_err(|e| Error::generic(format!("Failed to deserialize sync state: {}", e)))?;

        Ok(sync_state)
    }

    /// List all workspace IDs from disk
    pub async fn list_workspace_ids(&self) -> Result<Vec<EntityId>> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }

        let mut workspace_ids = Vec::new();

        let mut entries = fs::read_dir(&self.base_dir)
            .await
            .map_err(|e| Error::generic(format!("Failed to read workspace directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| Error::generic(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name != "registry.yaml" && file_name.ends_with(".yaml") {
                        if let Some(id) = file_name.strip_suffix(".yaml") {
                            workspace_ids.push(id.to_string());
                        }
                    }
                }
            }
        }

        Ok(workspace_ids)
    }

    /// Save the entire registry and all workspaces
    pub async fn save_full_registry(&self, registry: &WorkspaceRegistry) -> Result<()> {
        // Save registry metadata
        self.save_registry(registry).await?;

        // Save all workspaces
        for workspace in registry.get_workspaces() {
            self.save_workspace(workspace).await?;
        }

        Ok(())
    }

    /// Load the entire registry and all workspaces
    pub async fn load_full_registry(&self) -> Result<WorkspaceRegistry> {
        self.load_registry().await
    }

    /// Backup workspace data
    pub async fn backup_workspace(&self, workspace_id: &str, backup_dir: &Path) -> Result<PathBuf> {
        let workspace_file = self.workspace_file_path(workspace_id);

        if !workspace_file.exists() {
            return Err(Error::generic(format!("Workspace {} does not exist", workspace_id)));
        }

        // Ensure backup directory exists
        if !backup_dir.exists() {
            fs::create_dir_all(backup_dir)
                .await
                .map_err(|e| Error::generic(format!("Failed to create backup directory: {}", e)))?;
        }

        // Create backup filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_filename = format!("{}_{}.yaml", workspace_id, timestamp);
        let backup_path = backup_dir.join(backup_filename);

        // Copy workspace file
        fs::copy(&workspace_file, &backup_path)
            .await
            .map_err(|e| Error::generic(format!("Failed to create backup: {}", e)))?;

        Ok(backup_path)
    }

    /// Restore workspace from backup
    pub async fn restore_workspace(&self, backup_path: &Path) -> Result<EntityId> {
        if !backup_path.exists() {
            return Err(Error::generic(format!("Backup file does not exist: {:?}", backup_path)));
        }

        // Load workspace from backup
        let content = fs::read_to_string(backup_path)
            .await
            .map_err(|e| Error::generic(format!("Failed to read backup file: {}", e)))?;

        let workspace: Workspace = serde_yaml::from_str(&content)
            .map_err(|e| Error::generic(format!("Failed to deserialize backup: {}", e)))?;

        // Save restored workspace
        self.save_workspace(&workspace).await?;

        Ok(workspace.id)
    }

    /// Clean up old backups
    pub async fn cleanup_old_backups(&self, backup_dir: &Path, keep_count: usize) -> Result<usize> {
        if !backup_dir.exists() {
            return Ok(0);
        }

        let mut backup_files = Vec::new();

        let mut entries = fs::read_dir(backup_dir)
            .await
            .map_err(|e| Error::generic(format!("Failed to read backup directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| Error::generic(format!("Failed to read backup entry: {}", e)))?
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.ends_with(".yaml") {
                        if let Ok(metadata) = entry.metadata().await {
                            if let Ok(modified) = metadata.modified() {
                                backup_files.push((path, modified));
                            }
                        }
                    }
                }
            }
        }

        // Sort by modification time (newest first)
        backup_files.sort_by(|a, b| b.1.cmp(&a.1));

        // Remove old backups
        let mut removed_count = 0;
        for (path, _) in backup_files.iter().skip(keep_count) {
            if fs::remove_file(path).await.is_ok() {
                removed_count += 1;
            }
        }

        Ok(removed_count)
    }

    /// Advanced sync with additional configuration options
    pub async fn sync_to_directory_advanced(
        &self,
        target_dir: &str,
        strategy: &str,
        workspace_ids: Option<&str>,
        structure: &str,
        include_meta: bool,
        force: bool,
        filename_pattern: &str,
        exclude_pattern: Option<&str>,
        dry_run: bool,
    ) -> Result<SyncResult> {
        let target_path = PathBuf::from(target_dir);

        // Ensure target directory exists (unless dry run)
        if !dry_run && !target_path.exists() {
            fs::create_dir_all(&target_path)
                .await
                .map_err(|e| Error::generic(format!("Failed to create target directory: {}", e)))?;
        }

        // Parse strategy
        let sync_strategy = match strategy {
            "full" => SyncStrategy::Full,
            "incremental" => SyncStrategy::Incremental,
            "selective" => {
                if let Some(ids) = workspace_ids {
                    let workspace_list = ids.split(',').map(|s| s.trim().to_string()).collect();
                    SyncStrategy::Selective(workspace_list)
                } else {
                    return Err(Error::generic("Selective strategy requires workspace IDs"));
                }
            }
            _ => return Err(Error::generic(format!("Unknown sync strategy: {}", strategy))),
        };

        // Parse directory structure
        let dir_structure = match structure {
            "flat" => DirectoryStructure::Flat,
            "nested" => DirectoryStructure::Nested,
            "grouped" => DirectoryStructure::Grouped,
            _ => return Err(Error::generic(format!("Unknown directory structure: {}", structure))),
        };

        // Get workspaces to sync based on strategy
        let mut workspaces_to_sync = self.get_workspaces_for_sync(&sync_strategy).await?;

        // Apply exclusion filter if provided
        if let Some(exclude) = exclude_pattern {
            if let Ok(regex) = regex::Regex::new(exclude) {
                workspaces_to_sync.retain(|id| !regex.is_match(id));
            }
        }

        let mut result = SyncResult {
            synced_workspaces: 0,
            synced_requests: 0,
            files_created: 0,
            target_dir: target_path.clone(),
        };

        // Sync each workspace
        for workspace_id in workspaces_to_sync {
            if let Ok(workspace) = self.load_workspace(&workspace_id).await {
                let workspace_result = self
                    .sync_workspace_to_directory_advanced(
                        &workspace,
                        &target_path,
                        &dir_structure,
                        include_meta,
                        force,
                        filename_pattern,
                        dry_run,
                    )
                    .await?;

                result.synced_workspaces += 1;
                result.synced_requests += workspace_result.requests_count;
                result.files_created += workspace_result.files_created;
            }
        }

        // Update sync state for incremental syncs
        if let SyncStrategy::Incremental = sync_strategy {
            let new_sync_state = SyncState {
                last_sync_timestamp: Utc::now(),
            };
            if let Err(e) = self.save_sync_state(&new_sync_state).await {
                tracing::warn!("Failed to save sync state: {}", e);
            }
        }

        Ok(result)
    }

    /// Advanced sync for a single workspace with custom filename patterns
    async fn sync_workspace_to_directory_advanced(
        &self,
        workspace: &Workspace,
        target_dir: &Path,
        structure: &DirectoryStructure,
        include_meta: bool,
        force: bool,
        filename_pattern: &str,
        dry_run: bool,
    ) -> Result<WorkspaceSyncResult> {
        let mut result = WorkspaceSyncResult {
            requests_count: 0,
            files_created: 0,
        };

        match structure {
            DirectoryStructure::Flat => {
                let export = self.create_workspace_export(workspace).await?;
                let filename = self.generate_filename(filename_pattern, workspace);
                let file_path = target_dir.join(format!("{}.yaml", filename));

                if force || !file_path.exists() {
                    if !dry_run {
                        let content = serde_yaml::to_string(&export).map_err(|e| {
                            Error::generic(format!("Failed to serialize workspace: {}", e))
                        })?;

                        fs::write(&file_path, content).await.map_err(|e| {
                            Error::generic(format!("Failed to write workspace file: {}", e))
                        })?;
                    }
                    result.files_created += 1;
                }
            }

            DirectoryStructure::Nested => {
                let workspace_dir =
                    target_dir.join(self.generate_filename(filename_pattern, workspace));
                if !dry_run && !workspace_dir.exists() {
                    fs::create_dir_all(&workspace_dir).await.map_err(|e| {
                        Error::generic(format!("Failed to create workspace directory: {}", e))
                    })?;
                }

                // Export main workspace file
                let export = self.create_workspace_export(workspace).await?;
                let workspace_file = workspace_dir.join("workspace.yaml");

                if force || !workspace_file.exists() {
                    if !dry_run {
                        let content = serde_yaml::to_string(&export).map_err(|e| {
                            Error::generic(format!("Failed to serialize workspace: {}", e))
                        })?;

                        fs::write(&workspace_file, content).await.map_err(|e| {
                            Error::generic(format!("Failed to write workspace file: {}", e))
                        })?;
                    }
                    result.files_created += 1;
                }

                // Export individual requests
                let requests_dir = workspace_dir.join("requests");
                if !dry_run && !requests_dir.exists() {
                    fs::create_dir_all(&requests_dir).await.map_err(|e| {
                        Error::generic(format!("Failed to create requests directory: {}", e))
                    })?;
                }

                result.requests_count += self
                    .export_workspace_requests_advanced(workspace, &requests_dir, force, dry_run)
                    .await?;
            }

            DirectoryStructure::Grouped => {
                // Create grouped directories
                let requests_dir = target_dir.join("requests");
                let workspaces_dir = target_dir.join("workspaces");

                if !dry_run {
                    for dir in [&requests_dir, &workspaces_dir] {
                        if !dir.exists() {
                            fs::create_dir_all(dir).await.map_err(|e| {
                                Error::generic(format!("Failed to create directory: {}", e))
                            })?;
                        }
                    }
                }

                // Export workspace metadata
                let export = self.create_workspace_export(workspace).await?;
                let filename = self.generate_filename(filename_pattern, workspace);
                let workspace_file = workspaces_dir.join(format!("{}.yaml", filename));

                if force || !workspace_file.exists() {
                    if !dry_run {
                        let content = serde_yaml::to_string(&export).map_err(|e| {
                            Error::generic(format!("Failed to serialize workspace: {}", e))
                        })?;

                        fs::write(&workspace_file, content).await.map_err(|e| {
                            Error::generic(format!("Failed to write workspace file: {}", e))
                        })?;
                    }
                    result.files_created += 1;
                }

                // Export requests to requests directory
                result.requests_count += self
                    .export_workspace_requests_grouped_advanced(
                        workspace,
                        &requests_dir,
                        force,
                        dry_run,
                    )
                    .await?;
            }
        }

        // Create metadata file if requested
        if include_meta && !dry_run {
            self.create_metadata_file(workspace, target_dir, structure).await?;
            result.files_created += 1;
        }

        Ok(result)
    }

    /// Generate filename from pattern
    fn generate_filename(&self, pattern: &str, workspace: &Workspace) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");

        pattern
            .replace("{name}", &self.sanitize_filename(&workspace.name))
            .replace("{id}", &workspace.id)
            .replace("{timestamp}", &timestamp.to_string())
    }

    /// Advanced request export with dry run support
    async fn export_workspace_requests_advanced(
        &self,
        workspace: &Workspace,
        requests_dir: &Path,
        force: bool,
        dry_run: bool,
    ) -> Result<usize> {
        let mut count = 0;

        for request in &workspace.requests {
            let file_path =
                requests_dir.join(format!("{}.yaml", self.sanitize_filename(&request.name)));
            if force || !file_path.exists() {
                if !dry_run {
                    let exported = self.convert_request_to_exported(request, "");
                    let content = serde_yaml::to_string(&exported).map_err(|e| {
                        Error::generic(format!("Failed to serialize request: {}", e))
                    })?;

                    fs::write(&file_path, content).await.map_err(|e| {
                        Error::generic(format!("Failed to write request file: {}", e))
                    })?;
                }
                count += 1;
            }
        }

        // Export folder requests
        for folder in &workspace.folders {
            count += self
                .export_folder_requests_advanced(folder, requests_dir, force, &folder.name, dry_run)
                .await?;
        }

        Ok(count)
    }

    /// Advanced folder request export
    async fn export_folder_requests_advanced(
        &self,
        folder: &Folder,
        requests_dir: &Path,
        force: bool,
        folder_path: &str,
        dry_run: bool,
    ) -> Result<usize> {
        use std::collections::VecDeque;

        let mut count = 0;
        let mut queue = VecDeque::new();

        // Start with the root folder
        queue.push_back((folder, folder_path.to_string()));

        while let Some((current_folder, current_path)) = queue.pop_front() {
            // Export requests in current folder
            for request in &current_folder.requests {
                let file_path =
                    requests_dir.join(format!("{}.yaml", self.sanitize_filename(&request.name)));
                if force || !file_path.exists() {
                    if !dry_run {
                        let exported = self.convert_request_to_exported(request, &current_path);
                        let content = serde_yaml::to_string(&exported).map_err(|e| {
                            Error::generic(format!("Failed to serialize request: {}", e))
                        })?;

                        fs::write(&file_path, content).await.map_err(|e| {
                            Error::generic(format!("Failed to write request file: {}", e))
                        })?;
                    }
                    count += 1;
                }
            }

            // Add subfolders to queue with updated paths
            for subfolder in &current_folder.folders {
                let subfolder_path = if current_path.is_empty() {
                    subfolder.name.clone()
                } else {
                    format!("{}/{}", current_path, subfolder.name)
                };
                queue.push_back((subfolder, subfolder_path));
            }
        }

        Ok(count)
    }

    /// Advanced grouped request export
    async fn export_workspace_requests_grouped_advanced(
        &self,
        workspace: &Workspace,
        requests_dir: &Path,
        force: bool,
        dry_run: bool,
    ) -> Result<usize> {
        let mut count = 0;
        let workspace_requests_dir = requests_dir.join(self.sanitize_filename(&workspace.name));

        if !dry_run && !workspace_requests_dir.exists() {
            fs::create_dir_all(&workspace_requests_dir).await.map_err(|e| {
                Error::generic(format!("Failed to create workspace requests directory: {}", e))
            })?;
        }

        count += self
            .export_workspace_requests_advanced(workspace, &workspace_requests_dir, force, dry_run)
            .await?;
        Ok(count)
    }

    /// Sync workspaces to an external directory for Git/Dropbox integration
    pub async fn sync_to_directory(
        &self,
        target_dir: &str,
        strategy: &str,
        workspace_ids: Option<&str>,
        structure: &str,
        include_meta: bool,
        force: bool,
    ) -> Result<SyncResult> {
        let target_path = PathBuf::from(target_dir);

        // Ensure target directory exists
        if !target_path.exists() {
            fs::create_dir_all(&target_path)
                .await
                .map_err(|e| Error::generic(format!("Failed to create target directory: {}", e)))?;
        }

        // Parse strategy
        let sync_strategy = match strategy {
            "full" => SyncStrategy::Full,
            "incremental" => SyncStrategy::Incremental,
            "selective" => {
                if let Some(ids) = workspace_ids {
                    let workspace_list = ids.split(',').map(|s| s.trim().to_string()).collect();
                    SyncStrategy::Selective(workspace_list)
                } else {
                    return Err(Error::generic("Selective strategy requires workspace IDs"));
                }
            }
            _ => return Err(Error::generic(format!("Unknown sync strategy: {}", strategy))),
        };

        // Parse directory structure
        let dir_structure = match structure {
            "flat" => DirectoryStructure::Flat,
            "nested" => DirectoryStructure::Nested,
            "grouped" => DirectoryStructure::Grouped,
            _ => return Err(Error::generic(format!("Unknown directory structure: {}", structure))),
        };

        // Get workspaces to sync based on strategy
        let workspaces_to_sync = self.get_workspaces_for_sync(&sync_strategy).await?;

        let mut result = SyncResult {
            synced_workspaces: 0,
            synced_requests: 0,
            files_created: 0,
            target_dir: target_path.clone(),
        };

        // Sync each workspace
        for workspace_id in workspaces_to_sync {
            if let Ok(workspace) = self.load_workspace(&workspace_id).await {
                let workspace_result = self
                    .sync_workspace_to_directory(
                        &workspace,
                        &target_path,
                        &dir_structure,
                        include_meta,
                        force,
                    )
                    .await?;

                result.synced_workspaces += 1;
                result.synced_requests += workspace_result.requests_count;
                result.files_created += workspace_result.files_created;
            }
        }

        // Update sync state for incremental syncs
        if let SyncStrategy::Incremental = sync_strategy {
            let new_sync_state = SyncState {
                last_sync_timestamp: Utc::now(),
            };
            if let Err(e) = self.save_sync_state(&new_sync_state).await {
                tracing::warn!("Failed to save sync state: {}", e);
            }
        }

        Ok(result)
    }

    /// Get list of workspace IDs to sync based on strategy
    async fn get_workspaces_for_sync(&self, strategy: &SyncStrategy) -> Result<Vec<String>> {
        match strategy {
            SyncStrategy::Full => self.list_workspace_ids().await,
            SyncStrategy::Incremental => {
                // Load sync state to get last sync timestamp
                let sync_state = self.load_sync_state().await?;
                let last_sync = sync_state.last_sync_timestamp;

                // Get all workspace IDs
                let all_workspace_ids = self.list_workspace_ids().await?;

                // Filter workspaces that have been modified since last sync
                let mut modified_workspaces = Vec::new();
                for workspace_id in all_workspace_ids {
                    let file_path = self.workspace_file_path(&workspace_id);
                    if let Ok(metadata) = fs::metadata(&file_path).await {
                        if let Ok(modified_time) = metadata.modified() {
                            let modified_datetime = DateTime::<Utc>::from(modified_time);
                            if modified_datetime > last_sync {
                                modified_workspaces.push(workspace_id);
                            }
                        }
                    }
                }

                Ok(modified_workspaces)
            }
            SyncStrategy::Selective(ids) => Ok(ids.clone()),
        }
    }

    /// Sync a single workspace to the target directory
    async fn sync_workspace_to_directory(
        &self,
        workspace: &Workspace,
        target_dir: &Path,
        structure: &DirectoryStructure,
        include_meta: bool,
        force: bool,
    ) -> Result<WorkspaceSyncResult> {
        let mut result = WorkspaceSyncResult {
            requests_count: 0,
            files_created: 0,
        };

        match structure {
            DirectoryStructure::Flat => {
                let export = self.create_workspace_export(workspace).await?;
                let file_path =
                    target_dir.join(format!("{}.yaml", self.sanitize_filename(&workspace.name)));

                if force || !file_path.exists() {
                    let content = serde_yaml::to_string(&export).map_err(|e| {
                        Error::generic(format!("Failed to serialize workspace: {}", e))
                    })?;

                    fs::write(&file_path, content).await.map_err(|e| {
                        Error::generic(format!("Failed to write workspace file: {}", e))
                    })?;

                    result.files_created += 1;
                }
            }

            DirectoryStructure::Nested => {
                let workspace_dir = target_dir.join(self.sanitize_filename(&workspace.name));
                if !workspace_dir.exists() {
                    fs::create_dir_all(&workspace_dir).await.map_err(|e| {
                        Error::generic(format!("Failed to create workspace directory: {}", e))
                    })?;
                }

                // Export main workspace file
                let export = self.create_workspace_export(workspace).await?;
                let workspace_file = workspace_dir.join("workspace.yaml");

                if force || !workspace_file.exists() {
                    let content = serde_yaml::to_string(&export).map_err(|e| {
                        Error::generic(format!("Failed to serialize workspace: {}", e))
                    })?;

                    fs::write(&workspace_file, content).await.map_err(|e| {
                        Error::generic(format!("Failed to write workspace file: {}", e))
                    })?;

                    result.files_created += 1;
                }

                // Export individual requests
                let requests_dir = workspace_dir.join("requests");
                if !requests_dir.exists() {
                    fs::create_dir_all(&requests_dir).await.map_err(|e| {
                        Error::generic(format!("Failed to create requests directory: {}", e))
                    })?;
                }

                result.requests_count +=
                    self.export_workspace_requests(workspace, &requests_dir, force).await?;
            }

            DirectoryStructure::Grouped => {
                // Create grouped directories
                let requests_dir = target_dir.join("requests");
                let workspaces_dir = target_dir.join("workspaces");

                for dir in [&requests_dir, &workspaces_dir] {
                    if !dir.exists() {
                        fs::create_dir_all(dir).await.map_err(|e| {
                            Error::generic(format!("Failed to create directory: {}", e))
                        })?;
                    }
                }

                // Export workspace metadata
                let export = self.create_workspace_export(workspace).await?;
                let workspace_file = workspaces_dir
                    .join(format!("{}.yaml", self.sanitize_filename(&workspace.name)));

                if force || !workspace_file.exists() {
                    let content = serde_yaml::to_string(&export).map_err(|e| {
                        Error::generic(format!("Failed to serialize workspace: {}", e))
                    })?;

                    fs::write(&workspace_file, content).await.map_err(|e| {
                        Error::generic(format!("Failed to write workspace file: {}", e))
                    })?;

                    result.files_created += 1;
                }

                // Export requests to requests directory
                result.requests_count +=
                    self.export_workspace_requests_grouped(workspace, &requests_dir, force).await?;
            }
        }

        // Create metadata file if requested
        if include_meta {
            self.create_metadata_file(workspace, target_dir, structure).await?;
            result.files_created += 1;
        }

        Ok(result)
    }

    /// Create a Git-friendly workspace export
    async fn create_workspace_export(&self, workspace: &Workspace) -> Result<WorkspaceExport> {
        let mut requests = HashMap::new();

        // Collect all requests from workspace
        self.collect_requests_from_workspace(workspace, &mut requests, "".to_string());

        let metadata = WorkspaceMetadata {
            id: workspace.id.clone(),
            name: workspace.name.clone(),
            description: workspace.description.clone(),
            exported_at: Utc::now(),
            request_count: requests.len(),
            folder_count: workspace.folders.len(),
        };

        let config = WorkspaceConfig {
            auth: workspace.config.auth.as_ref().and_then(AuthConfig::from_config_auth),
            base_url: workspace.config.base_url.clone(),
            variables: workspace.config.global_environment.variables.clone(),
        };

        Ok(WorkspaceExport {
            metadata,
            config,
            requests,
        })
    }

    /// Collect all requests from workspace into a hashmap
    fn collect_requests_from_workspace(
        &self,
        workspace: &Workspace,
        requests: &mut HashMap<String, ExportedRequest>,
        folder_path: String,
    ) {
        // Add root-level requests
        for request in &workspace.requests {
            let exported = self.convert_request_to_exported(request, &folder_path);
            requests.insert(request.id.clone(), exported);
        }

        // Add folder requests recursively
        for folder in &workspace.folders {
            let current_path = if folder_path.is_empty() {
                folder.name.clone()
            } else {
                format!("{}/{}", folder_path, folder.name)
            };

            for request in &folder.requests {
                let exported = self.convert_request_to_exported(request, &current_path);
                requests.insert(request.id.clone(), exported);
            }

            // Recursively process subfolders
            self.collect_requests_from_folders(folder, requests, current_path);
        }
    }

    /// Recursively collect requests from folders
    fn collect_requests_from_folders(
        &self,
        folder: &Folder,
        requests: &mut HashMap<String, ExportedRequest>,
        folder_path: String,
    ) {
        for subfolder in &folder.folders {
            let current_path = format!("{}/{}", folder_path, subfolder.name);

            for request in &subfolder.requests {
                let exported = self.convert_request_to_exported(request, &current_path);
                requests.insert(request.id.clone(), exported);
            }

            self.collect_requests_from_folders(subfolder, requests, current_path);
        }
    }

    /// Convert a MockRequest to ExportedRequest
    fn convert_request_to_exported(
        &self,
        request: &MockRequest,
        folder_path: &str,
    ) -> ExportedRequest {
        ExportedRequest {
            id: request.id.clone(),
            name: request.name.clone(),
            method: format!("{:?}", request.method),
            path: request.path.clone(),
            folder_path: folder_path.to_string(),
            headers: request.headers.clone(),
            query_params: request.query_params.clone(),
            body: request.body.clone(),
            response_status: Some(request.response.status_code),
            response_body: request.response.body.clone(),
            response_headers: request.response.headers.clone(),
            delay: request.response.delay_ms,
        }
    }

    /// Export workspace with encryption for secure sharing
    pub async fn export_workspace_encrypted(
        &self,
        workspace: &Workspace,
        output_path: &Path,
    ) -> Result<EncryptedExportResult> {
        // Check if encryption is enabled for this workspace
        if !workspace.config.auto_encryption.enabled {
            return Err(Error::generic("Encryption is not enabled for this workspace. Enable encryption in workspace settings first."));
        }

        // Get auto-encryption config
        let encryption_config = workspace.config.auto_encryption.clone();
        let processor = AutoEncryptionProcessor::new(&workspace.id, encryption_config);

        // Create filtered workspace copy for export
        let mut filtered_workspace = workspace.to_filtered_for_sync();

        // Apply automatic encryption to the filtered workspace
        self.encrypt_workspace_data(&mut filtered_workspace, &processor)?;

        // Create standard export
        let export = self.create_workspace_export(&filtered_workspace).await?;

        // Encrypt the entire export
        let export_json = serde_json::to_string_pretty(&export)
            .map_err(|e| Error::generic(format!("Failed to serialize export: {}", e)))?;

        let encrypted_data = utils::encrypt_for_workspace(&workspace.id, &export_json)?;

        // Generate backup key for sharing
        let key_manager = WorkspaceKeyManager::new();
        let backup_key = key_manager.generate_workspace_key_backup(&workspace.id)?;

        // Write encrypted data to file
        fs::write(output_path, &encrypted_data)
            .await
            .map_err(|e| Error::generic(format!("Failed to write encrypted export: {}", e)))?;

        Ok(EncryptedExportResult {
            output_path: output_path.to_path_buf(),
            backup_key,
            exported_at: Utc::now(),
            workspace_name: workspace.name.clone(),
            encryption_enabled: true,
        })
    }

    /// Import encrypted workspace
    pub async fn import_workspace_encrypted(
        &self,
        encrypted_file: &Path,
        _workspace_name: Option<&str>,
        _registry: &mut WorkspaceRegistry,
    ) -> Result<EncryptedImportResult> {
        // Read encrypted data
        let _encrypted_data = fs::read_to_string(encrypted_file)
            .await
            .map_err(|e| Error::generic(format!("Failed to read encrypted file: {}", e)))?;

        // For import, we need the workspace ID and backup key
        // This would typically be provided by the user or extracted from metadata
        Err(Error::generic("Encrypted import requires workspace ID and backup key. Use import_workspace_encrypted_with_key instead."))
    }

    /// Import encrypted workspace with specific workspace ID and backup key
    pub async fn import_workspace_encrypted_with_key(
        &self,
        encrypted_file: &Path,
        workspace_id: &str,
        backup_key: &str,
        workspace_name: Option<&str>,
        registry: &mut WorkspaceRegistry,
    ) -> Result<EncryptedImportResult> {
        // Ensure workspace key exists or restore from backup
        let key_manager = WorkspaceKeyManager::new();
        if !key_manager.has_workspace_key(workspace_id) {
            key_manager.restore_workspace_key_from_backup(workspace_id, backup_key)?;
        }

        // Read and decrypt the data
        let encrypted_data = fs::read_to_string(encrypted_file)
            .await
            .map_err(|e| Error::generic(format!("Failed to read encrypted file: {}", e)))?;

        let decrypted_json = utils::decrypt_for_workspace(workspace_id, &encrypted_data)?;

        // Parse the export data
        let export: WorkspaceExport = serde_json::from_str(&decrypted_json)
            .map_err(|e| Error::generic(format!("Failed to parse decrypted export: {}", e)))?;

        // Convert export to workspace
        let workspace = self.convert_export_to_workspace(&export, workspace_name)?;

        // Add to registry
        let imported_id = registry.add_workspace(workspace)?;

        Ok(EncryptedImportResult {
            workspace_id: imported_id,
            workspace_name: export.metadata.name.clone(),
            imported_at: Utc::now(),
            request_count: export.requests.len(),
            encryption_restored: true,
        })
    }

    /// Apply encryption to workspace data before export
    fn encrypt_workspace_data(
        &self,
        workspace: &mut Workspace,
        processor: &AutoEncryptionProcessor,
    ) -> Result<()> {
        // Encrypt environment variables
        for env in &mut workspace.config.environments {
            processor.process_env_vars(&mut env.variables)?;
        }
        processor.process_env_vars(&mut workspace.config.global_environment.variables)?;

        // Note: Headers and request bodies would be encrypted here when implemented
        // For now, we rely on the filtering done by to_filtered_for_sync()

        Ok(())
    }

    /// Convert WorkspaceExport back to Workspace
    fn convert_export_to_workspace(
        &self,
        export: &WorkspaceExport,
        name_override: Option<&str>,
    ) -> Result<Workspace> {
        let mut workspace =
            Workspace::new(name_override.unwrap_or(&export.metadata.name).to_string());

        // Set description if provided
        if let Some(desc) = &export.metadata.description {
            workspace.description = Some(desc.clone());
        }

        // Restore requests from export
        for exported_request in export.requests.values() {
            // Convert exported request back to MockRequest
            let method = self.parse_http_method(&exported_request.method)?;
            let mut request = MockRequest::new(
                method,
                exported_request.path.clone(),
                exported_request.name.clone(),
            );

            // Set additional properties
            if let Some(status) = exported_request.response_status {
                request.response.status_code = status;
            }

            // Set other response properties if available
            if let Some(body) = &exported_request.response_body {
                request.response.body = Some(body.clone());
            }
            request.response.headers = exported_request.response_headers.clone();
            if let Some(delay) = exported_request.delay {
                request.response.delay_ms = Some(delay);
            }

            workspace.add_request(request)?;
        }

        // Restore configuration
        workspace.config.global_environment.variables = export.config.variables.clone();

        Ok(workspace)
    }

    /// Parse HTTP method string to enum
    fn parse_http_method(&self, method_str: &str) -> Result<crate::routing::HttpMethod> {
        match method_str.to_uppercase().as_str() {
            "GET" => Ok(crate::routing::HttpMethod::GET),
            "POST" => Ok(crate::routing::HttpMethod::POST),
            "PUT" => Ok(crate::routing::HttpMethod::PUT),
            "DELETE" => Ok(crate::routing::HttpMethod::DELETE),
            "PATCH" => Ok(crate::routing::HttpMethod::PATCH),
            "HEAD" => Ok(crate::routing::HttpMethod::HEAD),
            "OPTIONS" => Ok(crate::routing::HttpMethod::OPTIONS),
            _ => Err(Error::generic(format!("Unknown HTTP method: {}", method_str))),
        }
    }

    /// Check workspace for unencrypted sensitive data before export
    pub fn check_workspace_for_unencrypted_secrets(
        &self,
        workspace: &Workspace,
    ) -> Result<SecurityCheckResult> {
        let mut warnings = Vec::new();
        let errors = Vec::new();

        // Check environment variables
        self.check_environment_variables(workspace, &mut warnings)?;

        // Check for sensitive patterns in request data (when implemented)
        // This would check headers, bodies, etc.

        let has_warnings = !warnings.is_empty();
        let has_errors = !errors.is_empty();

        Ok(SecurityCheckResult {
            workspace_id: workspace.id.clone(),
            workspace_name: workspace.name.clone(),
            warnings,
            errors,
            is_secure: !has_warnings && !has_errors,
            recommended_actions: self.generate_security_recommendations(has_warnings, has_errors),
        })
    }

    /// Check environment variables for sensitive data
    fn check_environment_variables(
        &self,
        workspace: &Workspace,
        warnings: &mut Vec<SecurityWarning>,
    ) -> Result<()> {
        let sensitive_keys = [
            "password",
            "secret",
            "key",
            "token",
            "credential",
            "api_key",
            "apikey",
            "api_secret",
            "db_password",
            "database_password",
            "aws_secret_key",
            "aws_session_token",
            "private_key",
            "authorization",
            "auth_token",
            "access_token",
            "refresh_token",
            "cookie",
            "session",
            "csrf",
            "jwt",
            "bearer",
        ];

        // Check global environment
        for (key, value) in &workspace.config.global_environment.variables {
            if self.is_potentially_sensitive(key, value, &sensitive_keys) {
                warnings.push(SecurityWarning {
                    field_type: "environment_variable".to_string(),
                    field_name: key.clone(),
                    location: "global_environment".to_string(),
                    severity: SecuritySeverity::High,
                    message: format!(
                        "Potentially sensitive environment variable '{}' detected",
                        key
                    ),
                    suggestion: "Consider encrypting this value or excluding it from exports"
                        .to_string(),
                });
            }
        }

        // Check workspace environments
        for env in &workspace.config.environments {
            for (key, value) in &env.variables {
                if self.is_potentially_sensitive(key, value, &sensitive_keys) {
                    warnings.push(SecurityWarning {
                        field_type: "environment_variable".to_string(),
                        field_name: key.clone(),
                        location: format!("environment '{}'", env.name),
                        severity: SecuritySeverity::High,
                        message: format!("Potentially sensitive environment variable '{}' detected in environment '{}'", key, env.name),
                        suggestion: "Consider encrypting this value or excluding it from exports".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Check if a key-value pair is potentially sensitive
    fn is_potentially_sensitive(&self, key: &str, value: &str, sensitive_keys: &[&str]) -> bool {
        let key_lower = key.to_lowercase();

        // Check if key contains sensitive keywords
        if sensitive_keys.iter().any(|&sensitive| key_lower.contains(sensitive)) {
            return true;
        }

        // Check for patterns that indicate sensitive data
        self.contains_sensitive_patterns(value)
    }

    /// Check if value contains sensitive patterns
    fn contains_sensitive_patterns(&self, value: &str) -> bool {
        // Credit card pattern
        if CREDIT_CARD_PATTERN.is_match(value) {
            return true;
        }

        // SSN pattern
        if SSN_PATTERN.is_match(value) {
            return true;
        }

        // Long random-looking strings (potential API keys)
        if value.len() > 20 && value.chars().any(|c| c.is_alphanumeric()) {
            let alphanumeric_count = value.chars().filter(|c| c.is_alphanumeric()).count();
            let total_count = value.len();
            if alphanumeric_count as f64 / total_count as f64 > 0.8 {
                return true;
            }
        }

        false
    }

    /// Generate security recommendations based on findings
    fn generate_security_recommendations(
        &self,
        has_warnings: bool,
        has_errors: bool,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if has_warnings || has_errors {
            recommendations.push("Enable encryption for this workspace in settings".to_string());
            recommendations.push("Review and encrypt sensitive environment variables".to_string());
            recommendations.push("Use encrypted export for sharing workspaces".to_string());
        }

        if has_errors {
            recommendations
                .push("CRITICAL: Remove or encrypt sensitive data before proceeding".to_string());
        }

        recommendations
    }

    /// Export individual requests for nested structure
    async fn export_workspace_requests(
        &self,
        workspace: &Workspace,
        requests_dir: &Path,
        force: bool,
    ) -> Result<usize> {
        let mut count = 0;

        for request in &workspace.requests {
            let file_path =
                requests_dir.join(format!("{}.yaml", self.sanitize_filename(&request.name)));
            if force || !file_path.exists() {
                let exported = self.convert_request_to_exported(request, "");
                let content = serde_yaml::to_string(&exported)
                    .map_err(|e| Error::generic(format!("Failed to serialize request: {}", e)))?;

                fs::write(&file_path, content)
                    .await
                    .map_err(|e| Error::generic(format!("Failed to write request file: {}", e)))?;

                count += 1;
            }
        }

        // Export folder requests
        for folder in &workspace.folders {
            count += self.export_folder_requests(folder, requests_dir, force, &folder.name).await?;
        }

        Ok(count)
    }

    /// Export requests from folders recursively
    async fn export_folder_requests(
        &self,
        folder: &Folder,
        requests_dir: &Path,
        force: bool,
        folder_path: &str,
    ) -> Result<usize> {
        use std::collections::VecDeque;

        let mut count = 0;
        let mut queue = VecDeque::new();

        // Start with the root folder
        queue.push_back((folder, folder_path.to_string()));

        while let Some((current_folder, current_path)) = queue.pop_front() {
            // Export requests in current folder
            for request in &current_folder.requests {
                let file_path =
                    requests_dir.join(format!("{}.yaml", self.sanitize_filename(&request.name)));
                if force || !file_path.exists() {
                    let exported = self.convert_request_to_exported(request, &current_path);
                    let content = serde_yaml::to_string(&exported).map_err(|e| {
                        Error::generic(format!("Failed to serialize request: {}", e))
                    })?;

                    fs::write(&file_path, content).await.map_err(|e| {
                        Error::generic(format!("Failed to write request file: {}", e))
                    })?;

                    count += 1;
                }
            }

            // Add subfolders to queue with updated paths
            for subfolder in &current_folder.folders {
                let subfolder_path = if current_path.is_empty() {
                    subfolder.name.clone()
                } else {
                    format!("{}/{}", current_path, subfolder.name)
                };
                queue.push_back((subfolder, subfolder_path));
            }
        }

        Ok(count)
    }

    /// Export requests for grouped structure
    async fn export_workspace_requests_grouped(
        &self,
        workspace: &Workspace,
        requests_dir: &Path,
        force: bool,
    ) -> Result<usize> {
        let mut count = 0;
        let workspace_requests_dir = requests_dir.join(self.sanitize_filename(&workspace.name));

        if !workspace_requests_dir.exists() {
            fs::create_dir_all(&workspace_requests_dir).await.map_err(|e| {
                Error::generic(format!("Failed to create workspace requests directory: {}", e))
            })?;
        }

        count += self
            .export_workspace_requests(workspace, &workspace_requests_dir, force)
            .await?;
        Ok(count)
    }

    /// Create metadata file for Git integration
    async fn create_metadata_file(
        &self,
        workspace: &Workspace,
        target_dir: &Path,
        structure: &DirectoryStructure,
    ) -> Result<()> {
        let metadata = serde_json::json!({
            "workspace_id": workspace.id,
            "workspace_name": workspace.name,
            "description": workspace.description,
            "exported_at": Utc::now().to_rfc3339(),
            "structure": format!("{:?}", structure),
            "version": "1.0",
            "source": "mockforge"
        });

        let metadata_file = target_dir.join(".mockforge-meta.json");
        let content = serde_json::to_string_pretty(&metadata)
            .map_err(|e| Error::generic(format!("Failed to serialize metadata: {}", e)))?;

        fs::write(&metadata_file, content)
            .await
            .map_err(|e| Error::generic(format!("Failed to write metadata file: {}", e)))?;

        Ok(())
    }

    /// Sanitize filename for filesystem compatibility
    fn sanitize_filename(&self, name: &str) -> String {
        name.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c if c.is_whitespace() => '_',
                c => c,
            })
            .collect::<String>()
            .to_lowercase()
    }
}

/// Result of syncing a single workspace
#[derive(Debug)]
struct WorkspaceSyncResult {
    /// Number of requests exported
    requests_count: usize,
    /// Number of files created
    files_created: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{MockRequest, Workspace};
    use crate::HttpMethod;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_workspace_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = WorkspacePersistence::new(temp_dir.path());

        // Create a test workspace
        let mut workspace = Workspace::new("Test Workspace".to_string());
        let request =
            MockRequest::new(HttpMethod::GET, "/test".to_string(), "Test Request".to_string());
        workspace.add_request(request).unwrap();

        // Save workspace
        persistence.save_workspace(&workspace).await.unwrap();

        // Load workspace
        let loaded = persistence.load_workspace(&workspace.id).await.unwrap();
        assert_eq!(loaded.name, workspace.name);
        assert_eq!(loaded.requests.len(), 1);

        // List workspaces
        let ids = persistence.list_workspace_ids().await.unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], workspace.id);
    }

    #[tokio::test]
    async fn test_registry_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = WorkspacePersistence::new(temp_dir.path());

        let mut registry = WorkspaceRegistry::new();

        // Add workspaces
        let workspace1 = Workspace::new("Workspace 1".to_string());
        let workspace2 = Workspace::new("Workspace 2".to_string());

        let id1 = registry.add_workspace(workspace1).unwrap();
        let _id2 = registry.add_workspace(workspace2).unwrap();

        // Set active workspace
        registry.set_active_workspace(Some(id1.clone())).unwrap();

        // Save registry
        persistence.save_full_registry(&registry).await.unwrap();

        // Load registry
        let loaded_registry = persistence.load_full_registry().await.unwrap();

        assert_eq!(loaded_registry.get_workspaces().len(), 2);
        assert_eq!(loaded_registry.get_active_workspace().unwrap().name, "Workspace 1");
    }

    #[tokio::test]
    async fn test_backup_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let persistence = WorkspacePersistence::new(temp_dir.path());

        // Create and save workspace
        let workspace = Workspace::new("Test Workspace".to_string());
        persistence.save_workspace(&workspace).await.unwrap();

        // Create backup
        let backup_path = persistence.backup_workspace(&workspace.id, &backup_dir).await.unwrap();
        assert!(backup_path.exists());

        // Delete original
        persistence.delete_workspace(&workspace.id).await.unwrap();
        assert!(persistence.load_workspace(&workspace.id).await.is_err());

        // Restore from backup
        let restored_id = persistence.restore_workspace(&backup_path).await.unwrap();

        // Verify restored workspace
        let restored = persistence.load_workspace(&restored_id).await.unwrap();
        assert_eq!(restored.name, "Test Workspace");
    }
}
