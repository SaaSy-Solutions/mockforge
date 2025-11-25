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
    #[must_use]
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
    #[must_use]
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
        // Get workspace data using CoreBridge to get full workspace state
        let workspace = self
            .workspace_service
            .get_workspace(workspace_id)
            .await
            .map_err(|e| CollabError::Internal(format!("Failed to get workspace: {e}")))?;

        // Use CoreBridge to get full workspace state from mockforge-core
        // This integrates with mockforge-core to get the complete workspace state
        // including all mocks, folders, and configuration
        let workspace_data = self
            .core_bridge
            .export_workspace_for_backup(&workspace)
            .await
            .map_err(|e| CollabError::Internal(format!("Failed to export workspace: {e}")))?;

        // Serialize workspace data
        let backup_format = format.unwrap_or_else(|| "yaml".to_string());
        let serialized = match backup_format.as_str() {
            "yaml" => serde_yaml::to_string(&workspace_data)
                .map_err(|e| CollabError::Internal(format!("Failed to serialize to YAML: {e}")))?,
            "json" => serde_json::to_string_pretty(&workspace_data)
                .map_err(|e| CollabError::Internal(format!("Failed to serialize to JSON: {e}")))?,
            _ => {
                return Err(CollabError::InvalidInput(format!(
                    "Unsupported backup format: {backup_format}"
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
                .map_err(|e| CollabError::Internal(format!("Failed to deserialize YAML: {e}")))?,
            "json" => serde_json::from_str(&backup_data)
                .map_err(|e| CollabError::Internal(format!("Failed to deserialize JSON: {e}")))?,
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
        let team_workspace = if restored_workspace_id == backup.workspace_id {
            // Update existing workspace
            restored_team_workspace
        } else {
            // Create new workspace with the restored data
            let mut new_workspace = restored_team_workspace;
            new_workspace.id = restored_workspace_id;
            new_workspace
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
                        .map_err(|e| CollabError::Internal(format!("Invalid UUID: {e}")))?,
                    workspace_id: Uuid::parse_str(&row.workspace_id)
                        .map_err(|e| CollabError::Internal(format!("Invalid UUID: {e}")))?,
                    backup_url: row.backup_url,
                    storage_backend: serde_json::from_str(&row.storage_backend).map_err(|e| {
                        CollabError::Internal(format!("Invalid storage_backend: {e}"))
                    })?,
                    storage_config: row
                        .storage_config
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok()),
                    size_bytes: row.size_bytes,
                    backup_format: row.backup_format,
                    encrypted: row.encrypted != 0,
                    commit_id: row.commit_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
                    created_at: DateTime::parse_from_rfc3339(&row.created_at)
                        .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {e}")))?
                        .with_timezone(&Utc),
                    created_by: Uuid::parse_str(&row.created_by)
                        .map_err(|e| CollabError::Internal(format!("Invalid UUID: {e}")))?,
                    expires_at: row
                        .expires_at
                        .as_ref()
                        .map(|s| {
                            DateTime::parse_from_rfc3339(s)
                                .map(|dt| dt.with_timezone(&Utc))
                                .map_err(|e| {
                                    CollabError::Internal(format!("Invalid timestamp: {e}"))
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
        .ok_or_else(|| CollabError::Internal(format!("Backup not found: {backup_id}")))?;

        Ok(WorkspaceBackup {
            id: Uuid::parse_str(&row.id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {e}")))?,
            workspace_id: Uuid::parse_str(&row.workspace_id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {e}")))?,
            backup_url: row.backup_url,
            storage_backend: serde_json::from_str(&row.storage_backend)
                .map_err(|e| CollabError::Internal(format!("Invalid storage_backend: {e}")))?,
            storage_config: row.storage_config.as_ref().and_then(|s| serde_json::from_str(s).ok()),
            size_bytes: row.size_bytes,
            backup_format: row.backup_format,
            encrypted: row.encrypted != 0,
            commit_id: row.commit_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {e}")))?
                .with_timezone(&Utc),
            created_by: Uuid::parse_str(&row.created_by)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {e}")))?,
            expires_at: row
                .expires_at
                .as_ref()
                .map(|s| {
                    DateTime::parse_from_rfc3339(s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {e}")))
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
                        CollabError::Internal(format!("Failed to delete backup file: {e}"))
                    })?;
                }
            }
            StorageBackend::S3 => {
                self.delete_from_s3(&backup.backup_url, backup.storage_config.as_ref()).await?;
            }
            StorageBackend::Azure => {
                self.delete_from_azure(&backup.backup_url, backup.storage_config.as_ref())
                    .await?;
            }
            StorageBackend::Gcs => {
                self.delete_from_gcs(&backup.backup_url, backup.storage_config.as_ref()).await?;
            }
            StorageBackend::Custom => {
                return Err(CollabError::Internal(
                    "Custom storage backend deletion not implemented".to_string(),
                ));
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
            CollabError::Internal(format!("Failed to create backup directory: {e}"))
        })?;

        // Create backup filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("workspace_{workspace_id}_{timestamp}.{format}");
        let backup_path = Path::new(backup_dir).join(&filename);

        // Write backup file
        tokio::fs::write(&backup_path, data)
            .await
            .map_err(|e| CollabError::Internal(format!("Failed to write backup file: {e}")))?;

        Ok(backup_path.to_string_lossy().to_string())
    }

    /// Load backup from local filesystem
    async fn load_from_local(&self, backup_url: &str) -> Result<String> {
        tokio::fs::read_to_string(backup_url)
            .await
            .map_err(|e| CollabError::Internal(format!("Failed to read backup file: {e}")))
    }

    /// Delete backup from S3
    async fn delete_from_s3(
        &self,
        backup_url: &str,
        storage_config: Option<&serde_json::Value>,
    ) -> Result<()> {
        #[cfg(feature = "s3")]
        {
            use aws_config::SdkConfig;
            use aws_sdk_s3::config::{Credentials, Region};
            use aws_sdk_s3::Client as S3Client;

            // Parse S3 URL (format: s3://bucket-name/path/to/file)
            if !backup_url.starts_with("s3://") {
                return Err(CollabError::Internal(format!(
                    "Invalid S3 URL format: {}",
                    backup_url
                )));
            }

            let url_parts: Vec<&str> =
                backup_url.strip_prefix("s3://").unwrap().splitn(2, '/').collect();
            if url_parts.len() != 2 {
                return Err(CollabError::Internal(format!(
                    "Invalid S3 URL format: {}",
                    backup_url
                )));
            }

            let bucket = url_parts[0];
            let key = url_parts[1];

            // Build AWS config with credentials from storage_config or environment
            let aws_config: SdkConfig = if let Some(config) = storage_config {
                // Extract S3 credentials from storage_config
                // Expected format: {"access_key_id": "...", "secret_access_key": "...", "region": "..."}
                let access_key_id =
                    config.get("access_key_id").and_then(|v| v.as_str()).ok_or_else(|| {
                        CollabError::Internal(
                            "S3 access_key_id not found in storage_config".to_string(),
                        )
                    })?;

                let secret_access_key =
                    config.get("secret_access_key").and_then(|v| v.as_str()).ok_or_else(|| {
                        CollabError::Internal(
                            "S3 secret_access_key not found in storage_config".to_string(),
                        )
                    })?;

                let region_str =
                    config.get("region").and_then(|v| v.as_str()).unwrap_or("us-east-1");

                // Create credentials provider
                let credentials = Credentials::new(
                    access_key_id,
                    secret_access_key,
                    None, // session token
                    None, // expiration
                    "mockforge",
                );

                // Build AWS config with custom credentials and region
                aws_config::ConfigLoader::default()
                    .credentials_provider(credentials)
                    .region(Region::new(region_str.to_string()))
                    .load()
                    .await
            } else {
                // Use default AWS config (from environment variables, IAM role, etc.)
                aws_config::load_from_env().await
            };

            // Create S3 client
            let client = S3Client::new(&aws_config);

            // Delete object from S3
            client
                .delete_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| CollabError::Internal(format!("Failed to delete S3 object: {}", e)))?;

            tracing::info!("Successfully deleted S3 object: {}", backup_url);
            Ok(())
        }

        #[cfg(not(feature = "s3"))]
        {
            Err(CollabError::Internal(
                "S3 deletion requires 's3' feature to be enabled. Add 's3' feature to mockforge-collab in Cargo.toml.".to_string(),
            ))
        }
    }

    /// Delete backup from Azure Blob Storage
    async fn delete_from_azure(
        &self,
        backup_url: &str,
        storage_config: Option<&serde_json::Value>,
    ) -> Result<()> {
        #[cfg(feature = "azure")]
        {
            use azure_identity::DefaultAzureCredential;
            use azure_storage_blobs::prelude::*;
            use std::sync::Arc;

            // Parse Azure URL (format: https://account.blob.core.windows.net/container/path)
            if !backup_url.contains("blob.core.windows.net") {
                return Err(CollabError::Internal(format!(
                    "Invalid Azure Blob URL format: {}",
                    backup_url
                )));
            }

            // Parse URL properly
            let url = url::Url::parse(backup_url)
                .map_err(|e| CollabError::Internal(format!("Invalid Azure URL: {}", e)))?;

            // Extract account name from hostname (e.g., "account.blob.core.windows.net" -> "account")
            let hostname = url
                .host_str()
                .ok_or_else(|| CollabError::Internal("Invalid Azure hostname".to_string()))?;
            let account_name = hostname.split('.').next().ok_or_else(|| {
                CollabError::Internal("Invalid Azure hostname format".to_string())
            })?;

            // Extract container and blob name from path
            let path = url.path();
            let path_parts: Vec<&str> = path.splitn(3, '/').filter(|s| !s.is_empty()).collect();
            if path_parts.len() < 2 {
                return Err(CollabError::Internal(format!("Invalid Azure blob path: {}", path)));
            }

            let container_name = path_parts[0];
            let blob_name = path_parts[1..].join("/");

            // Extract Azure credentials from storage_config
            // Expected format: {"account_name": "...", "account_key": "..."} or use DefaultAzureCredential
            //
            // NOTE: azure_storage_blobs 0.19 API has changed from previous versions.
            // The API structure requires review of the 0.19 documentation to properly implement
            // credential handling and client creation. The previous implementation used a different
            // API structure that is no longer compatible.
            //
            // TODO: Review azure_storage_blobs 0.19 API documentation and update implementation:
            // - StorageCredentials import path and usage
            // - BlobServiceClient::new() signature and credential types
            // - DefaultAzureCredential integration with BlobServiceClient
            return Err(CollabError::Internal(
                "Azure deletion implementation needs to be updated for azure_storage_blobs 0.19 API. \
                 The API structure has changed and requires review of the 0.19 documentation."
                    .to_string(),
            ));
        }

        #[cfg(not(feature = "azure"))]
        {
            Err(CollabError::Internal(
                "Azure deletion requires 'azure' feature to be enabled. Add 'azure' feature to mockforge-collab in Cargo.toml.".to_string(),
            ))
        }
    }

    /// Delete backup from Google Cloud Storage
    async fn delete_from_gcs(
        &self,
        backup_url: &str,
        storage_config: Option<&serde_json::Value>,
    ) -> Result<()> {
        #[cfg(feature = "gcs")]
        {
            // Note: google-cloud-storage 1.4.0 API has changed significantly
            // The API structure is different from previous versions
            // This implementation may need adjustment based on actual 1.4.0 API documentation
            return Err(CollabError::Internal(
                "GCS deletion implementation needs to be updated for google-cloud-storage 1.4.0 API. \
                 The API structure has changed significantly and requires review of the 1.4.0 documentation."
                    .to_string(),
            ));

            /* TODO: Update to google-cloud-storage 1.4.0 API
            // The 1.4.0 API uses a different structure. Example implementation:
            use google_cloud_storage::client::Client;
            use google_cloud_storage::http::objects::delete::DeleteObjectRequest;

            // Parse GCS URL (format: gs://bucket-name/path/to/file)
            if !backup_url.starts_with("gs://") {
                return Err(CollabError::Internal(format!(
                    "Invalid GCS URL format: {}",
                    backup_url
                )));
            }

            let url_parts: Vec<&str> =
                backup_url.strip_prefix("gs://").unwrap().splitn(2, '/').collect();
            if url_parts.len() != 2 {
                return Err(CollabError::Internal(format!(
                    "Invalid GCS URL format: {}",
                    backup_url
                )));
            }

            let bucket_name = url_parts[0];
            let object_name = url_parts[1];

            // Extract GCS credentials from storage_config
            // Expected format: {"service_account_key": "...", "project_id": "..."}
            let project_id = storage_config
                .and_then(|c| c.get("project_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    CollabError::Internal("GCS project_id not found in storage_config".to_string())
                })?;

            // Initialize GCS client with google-cloud-storage 1.4.0 API
            // Note: The 1.4.0 API uses a different structure. For now, we'll use default credentials
            // and handle service account keys through environment variables or metadata server
            let client = Client::default()
                .await
                .map_err(|e| {
                    CollabError::Internal(format!("Failed to initialize GCS client: {}", e))
                })?;

            // Delete object using google-cloud-storage 1.4.0 API
            let request = DeleteObjectRequest {
                bucket: bucket_name.to_string(),
                object: object_name.to_string(),
                ..Default::default()
            };

            client
                .delete_object(&request)
                .await
                .map_err(|e| {
                    CollabError::Internal(format!("Failed to delete GCS object: {}", e))
                })?;

            tracing::info!("Successfully deleted GCS object: {}", backup_url);
            Ok(())
            */
        }

        #[cfg(not(feature = "gcs"))]
        {
            Err(CollabError::Internal(
                "GCS deletion requires 'gcs' feature to be enabled. Add 'gcs' feature to mockforge-collab in Cargo.toml.".to_string(),
            ))
        }
    }

    /// Get workspace data for backup
    ///
    /// Gets the full workspace state from the `TeamWorkspace` and converts it to JSON.
    async fn get_workspace_data(&self, workspace_id: Uuid) -> Result<serde_json::Value> {
        // Get the TeamWorkspace
        let team_workspace = self.workspace_service.get_workspace(workspace_id).await?;

        // Use CoreBridge to get the full workspace state as JSON
        self.core_bridge.get_workspace_state_json(&team_workspace)
    }
}
