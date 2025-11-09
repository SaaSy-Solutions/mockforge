//! Cloud backup and restore for workspaces

use crate::core_bridge::CoreBridge;
use crate::error::{CollabError, Result};
use crate::history::VersionControl;
use crate::workspace::WorkspaceService;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Storage backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "storage_backend", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    /// Local filesystem
    Local,
    /// Amazon S3
    S3,
    /// Azure Blob Storage
    Azure,
    /// Google Cloud Storage
    Gcs,
    /// Custom storage backend
    Custom,
}

/// Backup record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkspaceBackup {
    /// Unique backup ID
    pub id: Uuid,
    /// Workspace ID
    pub workspace_id: Uuid,
    /// Backup URL or path
    pub backup_url: String,
    /// Storage backend
    pub storage_backend: StorageBackend,
    /// Storage configuration (JSON)
    pub storage_config: Option<serde_json::Value>,
    /// Size in bytes
    pub size_bytes: i64,
    /// Backup format (yaml or json)
    pub backup_format: String,
    /// Whether backup is encrypted
    pub encrypted: bool,
    /// Commit ID this backup represents
    pub commit_id: Option<Uuid>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// User who created the backup
    pub created_by: Uuid,
    /// Optional expiration date
    pub expires_at: Option<DateTime<Utc>>,
}

impl WorkspaceBackup {
    /// Create a new backup record
    pub fn new(
        workspace_id: Uuid,
        backup_url: String,
        storage_backend: StorageBackend,
        size_bytes: i64,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            backup_url,
            storage_backend,
            storage_config: None,
            size_bytes,
            backup_format: "yaml".to_string(),
            encrypted: false,
            commit_id: None,
            created_at: Utc::now(),
            created_by,
            expires_at: None,
        }
    }
}

/// Backup service for managing workspace backups
pub struct BackupService {
    db: Pool<Sqlite>,
    version_control: VersionControl,
    local_backup_dir: Option<String>,
    core_bridge: Arc<CoreBridge>,
    workspace_service: Arc<WorkspaceService>,
}

impl BackupService {
    /// Create a new backup service
    pub fn new(
        db: Pool<Sqlite>,
        local_backup_dir: Option<String>,
        core_bridge: Arc<CoreBridge>,
        workspace_service: Arc<WorkspaceService>,
    ) -> Self {
        Self {
            db: db.clone(),
            version_control: VersionControl::new(db),
            local_backup_dir,
            core_bridge,
            workspace_service,
        }
    }

    /// Create a backup of a workspace
    ///
    /// Exports the workspace to the specified storage backend.
    /// For now, we support local filesystem backups. Cloud storage
    /// backends (S3, Azure, GCS) can be added later.
    pub async fn backup_workspace(
        &self,
        workspace_id: Uuid,
        user_id: Uuid,
        storage_backend: StorageBackend,
        format: Option<String>,
        commit_id: Option<Uuid>,
    ) -> Result<WorkspaceBackup> {
        // Get workspace data
        // TODO: Integrate with mockforge-core to get full workspace state
        // For now, we'll use the workspace config from the database
        let workspace_data = self.get_workspace_data(workspace_id).await?;

        // Serialize workspace data
        let backup_format = format.unwrap_or_else(|| "yaml".to_string());
        let serialized = match backup_format.as_str() {
            "yaml" => serde_yaml::to_string(&workspace_data).map_err(|e| {
                CollabError::Internal(format!("Failed to serialize to YAML: {}", e))
            })?,
            "json" => serde_json::to_string_pretty(&workspace_data).map_err(|e| {
                CollabError::Internal(format!("Failed to serialize to JSON: {}", e))
            })?,
            _ => {
                return Err(CollabError::InvalidInput(format!(
                    "Unsupported backup format: {}",
                    backup_format
                )));
            }
        };

        let size_bytes = serialized.len() as i64;

        // Save to storage backend
        let backup_url = match storage_backend {
            StorageBackend::Local => {
                self.save_to_local(workspace_id, &serialized, &backup_format).await?
            }
            StorageBackend::S3 => {
                return Err(CollabError::Internal("S3 backup not yet implemented".to_string()));
            }
            StorageBackend::Azure => {
                return Err(CollabError::Internal("Azure backup not yet implemented".to_string()));
            }
            StorageBackend::Gcs => {
                return Err(CollabError::Internal("GCS backup not yet implemented".to_string()));
            }
            StorageBackend::Custom => {
                return Err(CollabError::Internal(
                    "Custom storage backend not yet implemented".to_string(),
                ));
            }
        };

        // Create backup record
        let mut backup =
            WorkspaceBackup::new(workspace_id, backup_url, storage_backend, size_bytes, user_id);
        backup.backup_format = backup_format;
        backup.commit_id = commit_id;

        // Save to database
        sqlx::query!(
            r#"
            INSERT INTO workspace_backups (
                id, workspace_id, backup_url, storage_backend, storage_config,
                size_bytes, backup_format, encrypted, commit_id, created_at, created_by, expires_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            backup.id,
            backup.workspace_id,
            backup.backup_url,
            backup.storage_backend,
            backup.storage_config,
            backup.size_bytes,
            backup.backup_format,
            backup.encrypted,
            backup.commit_id,
            backup.created_at,
            backup.created_by,
            backup.expires_at
        )
        .execute(&self.db)
        .await?;

        Ok(backup)
    }

    /// Restore a workspace from a backup
    pub async fn restore_workspace(
        &self,
        backup_id: Uuid,
        target_workspace_id: Option<Uuid>,
        user_id: Uuid,
    ) -> Result<Uuid> {
        // Get backup record
        let backup = self.get_backup(backup_id).await?;

        // Load backup data
        let backup_data = match backup.storage_backend {
            StorageBackend::Local => self.load_from_local(&backup.backup_url).await?,
            _ => {
                return Err(CollabError::Internal(
                    "Only local backups are supported for restore".to_string(),
                ));
            }
        };

        // Deserialize workspace data
        let workspace_data: serde_json::Value = match backup.backup_format.as_str() {
            "yaml" => serde_yaml::from_str(&backup_data)
                .map_err(|e| CollabError::Internal(format!("Failed to deserialize YAML: {}", e)))?,
            "json" => serde_json::from_str(&backup_data)
                .map_err(|e| CollabError::Internal(format!("Failed to deserialize JSON: {}", e)))?,
            _ => {
                return Err(CollabError::Internal(format!(
                    "Unsupported backup format: {}",
                    backup.backup_format
                )));
            }
        };

        // Get the user who created the backup (or use a default - this should be passed in)
        // For now, we'll need to get it from the backup record
        let backup_record = self.get_backup(backup_id).await?;
        let owner_id = backup_record.created_by;

        // Import workspace from backup using CoreBridge
        let restored_team_workspace = self
            .core_bridge
            .import_workspace_from_backup(&workspace_data, owner_id, None)
            .await?;

        // Determine target workspace ID
        let restored_workspace_id = target_workspace_id.unwrap_or(backup.workspace_id);

        // If restoring to a different workspace, update the ID
        let mut team_workspace = if restored_workspace_id != backup.workspace_id {
            // Create new workspace with the restored data
            let mut new_workspace = restored_team_workspace;
            new_workspace.id = restored_workspace_id;
            new_workspace
        } else {
            // Update existing workspace
            restored_team_workspace
        };

        // Update the workspace in the database
        // This is a simplified version - in production, you'd want to use WorkspaceService
        // For now, we'll save it to disk and let the system pick it up
        self.core_bridge.save_workspace_to_disk(&team_workspace).await?;

        // Create restore commit if specified
        if let Some(commit_id) = backup.commit_id {
            // Restore to specific commit
            let _ =
                self.version_control.restore_to_commit(restored_workspace_id, commit_id).await?;
        }

        Ok(restored_workspace_id)
    }

    /// List all backups for a workspace
    pub async fn list_backups(
        &self,
        workspace_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<WorkspaceBackup>> {
        let limit = limit.unwrap_or(100);
        let workspace_id_str = workspace_id.to_string();

        let rows = sqlx::query!(
            r#"
            SELECT
                id,
                workspace_id,
                backup_url,
                storage_backend,
                storage_config,
                size_bytes,
                backup_format,
                encrypted,
                commit_id,
                created_at,
                created_by,
                expires_at
            FROM workspace_backups
            WHERE workspace_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
            workspace_id_str,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        let backups: Result<Vec<WorkspaceBackup>> = rows
            .into_iter()
            .map(|row| {
                Ok(WorkspaceBackup {
                    id: Uuid::parse_str(&row.id)
                        .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
                    workspace_id: Uuid::parse_str(&row.workspace_id)
                        .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
                    backup_url: row.backup_url,
                    storage_backend: serde_json::from_str(&row.storage_backend).map_err(|e| {
                        CollabError::Internal(format!("Invalid storage_backend: {}", e))
                    })?,
                    storage_config: row.storage_config.and_then(|s| serde_json::from_str(&s).ok()),
                    size_bytes: row.size_bytes,
                    backup_format: row.backup_format,
                    encrypted: row.encrypted != 0,
                    commit_id: row.commit_id.and_then(|s| Uuid::parse_str(&s).ok()),
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                        .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {}", e)))?
                        .with_timezone(&chrono::Utc),
                    created_by: Uuid::parse_str(&row.created_by)
                        .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
                    expires_at: row
                        .expires_at
                        .map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .map_err(|e| {
                                    CollabError::Internal(format!("Invalid timestamp: {}", e))
                                })
                        })
                        .transpose()?,
                })
            })
            .collect();
        let backups = backups?;

        Ok(backups)
    }

    /// Get a backup by ID
    pub async fn get_backup(&self, backup_id: Uuid) -> Result<WorkspaceBackup> {
        let backup_id_str = backup_id.to_string();
        let row = sqlx::query!(
            r#"
            SELECT
                id,
                workspace_id,
                backup_url,
                storage_backend,
                storage_config,
                size_bytes,
                backup_format,
                encrypted,
                commit_id,
                created_at,
                created_by,
                expires_at
            FROM workspace_backups
            WHERE id = ?
            "#,
            backup_id_str
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| CollabError::Internal(format!("Backup not found: {}", backup_id)))?;

        Ok(WorkspaceBackup {
            id: Uuid::parse_str(&row.id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
            workspace_id: Uuid::parse_str(&row.workspace_id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
            backup_url: row.backup_url,
            storage_backend: serde_json::from_str(&row.storage_backend)
                .map_err(|e| CollabError::Internal(format!("Invalid storage_backend: {}", e)))?,
            storage_config: row.storage_config.and_then(|s| serde_json::from_str(&s).ok()),
            size_bytes: row.size_bytes,
            backup_format: row.backup_format,
            encrypted: row.encrypted != 0,
            commit_id: row.commit_id.and_then(|s| Uuid::parse_str(&s).ok()),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&chrono::Utc),
            created_by: Uuid::parse_str(&row.created_by)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
            expires_at: row
                .expires_at
                .map(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {}", e)))
                })
                .transpose()?,
        })
    }

    /// Delete a backup
    pub async fn delete_backup(&self, backup_id: Uuid) -> Result<()> {
        // Get backup record to get the URL
        let backup = self.get_backup(backup_id).await?;

        // Delete from storage
        match backup.storage_backend {
            StorageBackend::Local => {
                if Path::new(&backup.backup_url).exists() {
                    tokio::fs::remove_file(&backup.backup_url).await.map_err(|e| {
                        CollabError::Internal(format!("Failed to delete backup file: {}", e))
                    })?;
                }
            }
            _ => {
                // TODO: Implement deletion for cloud backends
            }
        }

        // Delete from database
        let backup_id_str = backup_id.to_string();
        sqlx::query!(
            r#"
            DELETE FROM workspace_backups
            WHERE id = ?
            "#,
            backup_id_str
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Save backup to local filesystem
    async fn save_to_local(&self, workspace_id: Uuid, data: &str, format: &str) -> Result<String> {
        let backup_dir = self.local_backup_dir.as_ref().ok_or_else(|| {
            CollabError::Internal("Local backup directory not configured".to_string())
        })?;

        // Ensure backup directory exists
        tokio::fs::create_dir_all(backup_dir).await.map_err(|e| {
            CollabError::Internal(format!("Failed to create backup directory: {}", e))
        })?;

        // Create backup filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("workspace_{}_{}.{}", workspace_id, timestamp, format);
        let backup_path = Path::new(backup_dir).join(&filename);

        // Write backup file
        tokio::fs::write(&backup_path, data)
            .await
            .map_err(|e| CollabError::Internal(format!("Failed to write backup file: {}", e)))?;

        Ok(backup_path.to_string_lossy().to_string())
    }

    /// Load backup from local filesystem
    async fn load_from_local(&self, backup_url: &str) -> Result<String> {
        tokio::fs::read_to_string(backup_url)
            .await
            .map_err(|e| CollabError::Internal(format!("Failed to read backup file: {}", e)))
    }

    /// Get workspace data for backup
    ///
    /// Gets the full workspace state from the TeamWorkspace and converts it to JSON.
    async fn get_workspace_data(&self, workspace_id: Uuid) -> Result<serde_json::Value> {
        // Get the TeamWorkspace
        let team_workspace = self.workspace_service.get_workspace(workspace_id).await?;

        // Use CoreBridge to get the full workspace state as JSON
        self.core_bridge.get_workspace_state_json(&team_workspace)
    }
}
