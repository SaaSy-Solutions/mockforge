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
    client: reqwest::Client,
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
            client: reqwest::Client::new(),
            core_bridge,
            workspace_service,
        }
    }

    /// Create a backup of a workspace
    ///
    /// Exports the workspace to the specified storage backend.
    /// Supports local filesystem, Azure Blob Storage, and Google Cloud Storage.
    /// For cloud storage, use `backup_workspace_with_config` to provide credentials.
    pub async fn backup_workspace(
        &self,
        workspace_id: Uuid,
        user_id: Uuid,
        storage_backend: StorageBackend,
        format: Option<String>,
        commit_id: Option<Uuid>,
    ) -> Result<WorkspaceBackup> {
        self.backup_workspace_with_config(
            workspace_id,
            user_id,
            storage_backend,
            format,
            commit_id,
            None,
        )
        .await
    }

    /// Create a backup of a workspace with storage configuration
    ///
    /// Exports the workspace to the specified storage backend.
    /// For Azure, storage_config should include:
    /// - `account_name`: Azure storage account name (required)
    /// - `container_name`: Container name (defaults to "mockforge-backups")
    /// - `account_key` or `sas_token`: Credentials (optional, uses DefaultAzureCredential if not provided)
    ///
    /// For GCS, storage_config should include:
    /// - `bucket_name`: GCS bucket name (defaults to "mockforge-backups")
    ///
    /// For local storage, storage_config is ignored.
    pub async fn backup_workspace_with_config(
        &self,
        workspace_id: Uuid,
        user_id: Uuid,
        storage_backend: StorageBackend,
        format: Option<String>,
        commit_id: Option<Uuid>,
        storage_config: Option<serde_json::Value>,
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
                self.save_to_s3(workspace_id, &serialized, &backup_format, storage_config.as_ref())
                    .await?
            }
            StorageBackend::Azure => {
                self.save_to_azure(
                    workspace_id,
                    &serialized,
                    &backup_format,
                    storage_config.as_ref(),
                )
                .await?
            }
            StorageBackend::Gcs => {
                self.save_to_gcs(workspace_id, &serialized, &backup_format, storage_config.as_ref())
                    .await?
            }
            StorageBackend::Custom => {
                self.save_to_custom(
                    workspace_id,
                    &serialized,
                    &backup_format,
                    storage_config.as_ref(),
                )
                .await?
            }
        };

        // Create backup record
        let mut backup =
            WorkspaceBackup::new(workspace_id, backup_url, storage_backend, size_bytes, user_id);
        backup.backup_format = backup_format;
        backup.storage_config = storage_config;
        backup.commit_id = commit_id;

        // Use lowercase enum name for storage_backend to match CHECK constraint
        let storage_backend_str = match backup.storage_backend {
            StorageBackend::Local => "local",
            StorageBackend::S3 => "s3",
            StorageBackend::Azure => "azure",
            StorageBackend::Gcs => "gcs",
            StorageBackend::Custom => "custom",
        };
        let storage_config_str = backup.storage_config.as_ref().map(|v| v.to_string());
        let created_at_str = backup.created_at.to_rfc3339();
        let expires_at_str = backup.expires_at.map(|dt| dt.to_rfc3339());

        // Save to database - use Uuid directly to match how users/workspaces are stored
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
            storage_backend_str,
            storage_config_str,
            backup.size_bytes,
            backup.backup_format,
            backup.encrypted,
            backup.commit_id,
            created_at_str,
            backup.created_by,
            expires_at_str
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
        _user_id: Uuid,
    ) -> Result<Uuid> {
        // Get backup record
        let backup = self.get_backup(backup_id).await?;

        // Load backup data
        let backup_data = match backup.storage_backend {
            StorageBackend::Local => self.load_from_local(&backup.backup_url).await?,
            StorageBackend::Custom => {
                self.load_from_custom(&backup.backup_url, backup.storage_config.as_ref())
                    .await?
            }
            _ => {
                return Err(CollabError::Internal(
                    "Only local and custom backups are supported for restore".to_string(),
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

        let rows = sqlx::query!(
            r#"
            SELECT
                id as "id: Uuid",
                workspace_id as "workspace_id: Uuid",
                backup_url,
                storage_backend,
                storage_config,
                size_bytes,
                backup_format,
                encrypted,
                commit_id as "commit_id: Uuid",
                created_at,
                created_by as "created_by: Uuid",
                expires_at
            FROM workspace_backups
            WHERE workspace_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
            workspace_id,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        let backups: Result<Vec<WorkspaceBackup>> = rows
            .into_iter()
            .map(|row| {
                let storage_backend = match row.storage_backend.as_str() {
                    "local" => StorageBackend::Local,
                    "s3" => StorageBackend::S3,
                    "azure" => StorageBackend::Azure,
                    "gcs" => StorageBackend::Gcs,
                    "custom" => StorageBackend::Custom,
                    other => {
                        return Err(CollabError::Internal(format!(
                            "Invalid storage_backend: {other}"
                        )))
                    }
                };
                Ok(WorkspaceBackup {
                    id: row.id,
                    workspace_id: row.workspace_id,
                    backup_url: row.backup_url,
                    storage_backend,
                    storage_config: row
                        .storage_config
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok()),
                    size_bytes: row.size_bytes,
                    backup_format: row.backup_format,
                    encrypted: row.encrypted != 0,
                    commit_id: row.commit_id,
                    created_at: DateTime::parse_from_rfc3339(&row.created_at)
                        .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {e}")))?
                        .with_timezone(&Utc),
                    created_by: row.created_by,
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
        let row = sqlx::query!(
            r#"
            SELECT
                id as "id: Uuid",
                workspace_id as "workspace_id: Uuid",
                backup_url,
                storage_backend,
                storage_config,
                size_bytes,
                backup_format,
                encrypted,
                commit_id as "commit_id: Uuid",
                created_at,
                created_by as "created_by: Uuid",
                expires_at
            FROM workspace_backups
            WHERE id = ?
            "#,
            backup_id
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| CollabError::Internal(format!("Backup not found: {backup_id}")))?;

        let storage_backend = match row.storage_backend.as_str() {
            "local" => StorageBackend::Local,
            "s3" => StorageBackend::S3,
            "azure" => StorageBackend::Azure,
            "gcs" => StorageBackend::Gcs,
            "custom" => StorageBackend::Custom,
            other => {
                return Err(CollabError::Internal(format!("Invalid storage_backend: {other}")))
            }
        };

        Ok(WorkspaceBackup {
            id: row.id,
            workspace_id: row.workspace_id,
            backup_url: row.backup_url,
            storage_backend,
            storage_config: row.storage_config.as_ref().and_then(|s| serde_json::from_str(s).ok()),
            size_bytes: row.size_bytes,
            backup_format: row.backup_format,
            encrypted: row.encrypted != 0,
            commit_id: row.commit_id,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {e}")))?
                .with_timezone(&Utc),
            created_by: row.created_by,
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
                self.delete_from_custom(&backup.backup_url, backup.storage_config.as_ref())
                    .await?;
            }
        }

        // Delete from database
        sqlx::query!(
            r#"
            DELETE FROM workspace_backups
            WHERE id = ?
            "#,
            backup_id
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

    /// Save backup to a custom HTTP storage backend.
    ///
    /// Expected config:
    /// - `upload_url` (required): target URL for PUT uploads.
    ///   Supports `{filename}` placeholder.
    /// - `backup_url_base` (optional): base URL used to build persisted backup URL.
    /// - `headers` (optional): object map of HTTP headers.
    async fn save_to_custom(
        &self,
        workspace_id: Uuid,
        data: &str,
        format: &str,
        storage_config: Option<&serde_json::Value>,
    ) -> Result<String> {
        let config = storage_config.ok_or_else(|| {
            CollabError::Internal("Custom storage configuration required".to_string())
        })?;

        let upload_url = config.get("upload_url").and_then(|v| v.as_str()).ok_or_else(|| {
            CollabError::Internal("Custom storage config must include 'upload_url'".to_string())
        })?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("workspace_{workspace_id}_{timestamp}.{format}");
        let resolved_upload_url = upload_url.replace("{filename}", &filename);

        let mut request = self.client.put(&resolved_upload_url).body(data.to_string()).header(
            "content-type",
            match format {
                "yaml" => "application/x-yaml",
                "json" => "application/json",
                _ => "application/octet-stream",
            },
        );

        if let Some(headers) = config.get("headers").and_then(|h| h.as_object()) {
            for (key, value) in headers {
                if let Some(value) = value.as_str() {
                    request = request.header(key, value);
                }
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| CollabError::Internal(format!("Custom upload request failed: {e}")))?;

        if !response.status().is_success() {
            return Err(CollabError::Internal(format!(
                "Custom upload failed with status {}",
                response.status()
            )));
        }

        if let Some(location) = response.headers().get("location").and_then(|v| v.to_str().ok()) {
            return Ok(location.to_string());
        }

        if let Ok(body_json) = response.json::<serde_json::Value>().await {
            if let Some(url) = body_json
                .get("backup_url")
                .or_else(|| body_json.get("url"))
                .and_then(|v| v.as_str())
            {
                return Ok(url.to_string());
            }
        }

        if let Some(base) = config.get("backup_url_base").and_then(|v| v.as_str()) {
            return Ok(format!("{}/{}", base.trim_end_matches('/'), filename));
        }

        Ok(resolved_upload_url)
    }

    /// Load backup from custom HTTP storage backend.
    async fn load_from_custom(
        &self,
        backup_url: &str,
        storage_config: Option<&serde_json::Value>,
    ) -> Result<String> {
        let mut request = self.client.get(backup_url);
        if let Some(config) = storage_config {
            if let Some(headers) = config.get("headers").and_then(|h| h.as_object()) {
                for (key, value) in headers {
                    if let Some(value) = value.as_str() {
                        request = request.header(key, value);
                    }
                }
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| CollabError::Internal(format!("Custom download request failed: {e}")))?;
        if !response.status().is_success() {
            return Err(CollabError::Internal(format!(
                "Custom download failed with status {}",
                response.status()
            )));
        }

        response
            .text()
            .await
            .map_err(|e| CollabError::Internal(format!("Failed to read custom backup body: {e}")))
    }

    /// Save backup to Azure Blob Storage
    #[allow(unused_variables)]
    async fn save_to_s3(
        &self,
        workspace_id: Uuid,
        data: &str,
        format: &str,
        storage_config: Option<&serde_json::Value>,
    ) -> Result<String> {
        #[cfg(feature = "s3")]
        {
            use aws_config::SdkConfig;
            use aws_sdk_s3::config::{Credentials, Region};
            use aws_sdk_s3::primitives::ByteStream;
            use aws_sdk_s3::Client as S3Client;

            let config = storage_config.ok_or_else(|| {
                CollabError::Internal("S3 storage configuration required".to_string())
            })?;

            let bucket_name =
                config.get("bucket_name").and_then(|v| v.as_str()).ok_or_else(|| {
                    CollabError::Internal("S3 bucket_name not found in storage_config".to_string())
                })?;

            let prefix = config.get("prefix").and_then(|v| v.as_str()).unwrap_or("backups");
            let region_str = config.get("region").and_then(|v| v.as_str()).unwrap_or("us-east-1");

            let aws_config: SdkConfig = if let (Some(access_key_id), Some(secret_access_key)) = (
                config.get("access_key_id").and_then(|v| v.as_str()),
                config.get("secret_access_key").and_then(|v| v.as_str()),
            ) {
                let credentials =
                    Credentials::new(access_key_id, secret_access_key, None, None, "mockforge");
                aws_config::ConfigLoader::default()
                    .credentials_provider(credentials)
                    .region(Region::new(region_str.to_string()))
                    .load()
                    .await
            } else {
                aws_config::load_from_env().await
            };

            let client = S3Client::new(&aws_config);

            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let object_key =
                format!("{}/workspace_{}_{}.{}", prefix, workspace_id, timestamp, format);
            let content_type = match format {
                "yaml" => "application/x-yaml",
                "json" => "application/json",
                _ => "application/octet-stream",
            };

            client
                .put_object()
                .bucket(bucket_name)
                .key(&object_key)
                .content_type(content_type)
                .body(ByteStream::from(data.as_bytes().to_vec()))
                .send()
                .await
                .map_err(|e| CollabError::Internal(format!("Failed to upload to S3: {}", e)))?;

            let backup_url = format!("s3://{}/{}", bucket_name, object_key);
            tracing::info!("Successfully uploaded backup to S3: {}", backup_url);
            Ok(backup_url)
        }

        #[cfg(not(feature = "s3"))]
        {
            Err(CollabError::Internal(
                "S3 backup requires 's3' feature to be enabled. Add 's3' feature to mockforge-collab in Cargo.toml.".to_string(),
            ))
        }
    }

    /// Save backup to Azure Blob Storage
    #[allow(unused_variables)]
    async fn save_to_azure(
        &self,
        workspace_id: Uuid,
        data: &str,
        format: &str,
        storage_config: Option<&serde_json::Value>,
    ) -> Result<String> {
        #[cfg(feature = "azure")]
        {
            use azure_identity::{DefaultAzureCredential, TokenCredentialOptions};
            use azure_storage::StorageCredentials;
            use azure_storage_blobs::prelude::*;
            use std::sync::Arc;

            // Get storage configuration
            let config = storage_config.ok_or_else(|| {
                CollabError::Internal("Azure storage configuration required".to_string())
            })?;

            let account_name = config
                .get("account_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    CollabError::Internal(
                        "Azure account_name required in storage config".to_string(),
                    )
                })?;

            let container_name = config
                .get("container_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "mockforge-backups".to_string());

            // Build storage credentials
            let storage_credentials = if let Some(account_key) =
                config.get("account_key").and_then(|v| v.as_str()).map(|s| s.to_string())
            {
                StorageCredentials::access_key(account_name.clone(), account_key)
            } else if let Some(sas_token) =
                config.get("sas_token").and_then(|v| v.as_str()).map(|s| s.to_string())
            {
                StorageCredentials::sas_token(sas_token)
                    .map_err(|e| CollabError::Internal(format!("Invalid SAS token: {}", e)))?
            } else {
                let credential = Arc::new(
                    DefaultAzureCredential::create(TokenCredentialOptions::default()).map_err(
                        |e| {
                            CollabError::Internal(format!(
                                "Failed to create Azure credentials: {}",
                                e
                            ))
                        },
                    )?,
                );
                StorageCredentials::token_credential(credential)
            };

            // Create blob name with timestamp
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let blob_name = format!("workspace_{workspace_id}_{timestamp}.{format}");

            // Create blob client and upload
            let blob_client = ClientBuilder::new(account_name.clone(), storage_credentials)
                .blob_client(&container_name, &blob_name);

            blob_client
                .put_block_blob(data.as_bytes().to_vec())
                .content_type(match format {
                    "yaml" => "application/x-yaml",
                    "json" => "application/json",
                    _ => "application/octet-stream",
                })
                .await
                .map_err(|e| CollabError::Internal(format!("Failed to upload to Azure: {}", e)))?;

            let backup_url = format!(
                "https://{}.blob.core.windows.net/{}/{}",
                account_name, container_name, blob_name
            );
            tracing::info!("Successfully uploaded backup to Azure: {}", backup_url);
            Ok(backup_url)
        }

        #[cfg(not(feature = "azure"))]
        {
            Err(CollabError::Internal(
                "Azure backup requires 'azure' feature to be enabled. Add 'azure' feature to mockforge-collab in Cargo.toml.".to_string(),
            ))
        }
    }

    /// Save backup to Google Cloud Storage
    #[allow(unused_variables)]
    async fn save_to_gcs(
        &self,
        workspace_id: Uuid,
        data: &str,
        format: &str,
        storage_config: Option<&serde_json::Value>,
    ) -> Result<String> {
        #[cfg(feature = "gcs")]
        {
            use bytes::Bytes;
            use google_cloud_storage::client::Storage;

            // Get storage configuration
            let config = storage_config.ok_or_else(|| {
                CollabError::Internal("GCS storage configuration required".to_string())
            })?;

            let bucket_name = config
                .get("bucket_name")
                .and_then(|v| v.as_str())
                .unwrap_or("mockforge-backups");

            // Initialize GCS client using the new builder API
            let client = Storage::builder().build().await.map_err(|e| {
                CollabError::Internal(format!("Failed to create GCS client: {}", e))
            })?;

            // Create object name with timestamp
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let object_name = format!("workspace_{workspace_id}_{timestamp}.{format}");

            // Upload object using the new write_object API
            // Convert to bytes::Bytes which implements Into<Payload<BytesSource>>
            let payload = Bytes::from(data.as_bytes().to_vec());
            client
                .write_object(bucket_name, &object_name, payload)
                .send_unbuffered()
                .await
                .map_err(|e| CollabError::Internal(format!("Failed to upload to GCS: {}", e)))?;

            let backup_url = format!("gs://{}/{}", bucket_name, object_name);
            tracing::info!("Successfully uploaded backup to GCS: {}", backup_url);
            Ok(backup_url)
        }

        #[cfg(not(feature = "gcs"))]
        {
            Err(CollabError::Internal(
                "GCS backup requires 'gcs' feature to be enabled. Add 'gcs' feature to mockforge-collab in Cargo.toml.".to_string(),
            ))
        }
    }

    /// Delete backup from S3
    async fn delete_from_s3(
        &self,
        backup_url: &str,
        storage_config: Option<&serde_json::Value>,
    ) -> Result<()> {
        #[cfg(not(feature = "s3"))]
        let _ = (backup_url, storage_config);
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

            let url_parts: Vec<&str> = backup_url
                .strip_prefix("s3://")
                .ok_or_else(|| {
                    CollabError::Internal(format!("Invalid S3 URL format: {}", backup_url))
                })?
                .splitn(2, '/')
                .collect();
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
            use azure_identity::{DefaultAzureCredential, TokenCredentialOptions};
            use azure_storage::StorageCredentials;
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

            let container_name = path_parts[0].to_string();
            let blob_name = path_parts[1..].join("/");
            let account_name = account_name.to_string();

            // Helper function to create default credentials
            let create_default_creds = || -> Result<StorageCredentials> {
                let credential = Arc::new(
                    DefaultAzureCredential::create(TokenCredentialOptions::default()).map_err(
                        |e| {
                            CollabError::Internal(format!(
                                "Failed to create Azure credentials: {}",
                                e
                            ))
                        },
                    )?,
                );
                Ok(StorageCredentials::token_credential(credential))
            };

            // Build storage credentials from config or use DefaultAzureCredential
            let storage_credentials = if let Some(config) = storage_config {
                if let Some(account_key) =
                    config.get("account_key").and_then(|v| v.as_str()).map(|s| s.to_string())
                {
                    // Use account key authentication
                    StorageCredentials::access_key(account_name.clone(), account_key)
                } else if let Some(sas_token) =
                    config.get("sas_token").and_then(|v| v.as_str()).map(|s| s.to_string())
                {
                    // Use SAS token authentication
                    StorageCredentials::sas_token(sas_token)
                        .map_err(|e| CollabError::Internal(format!("Invalid SAS token: {}", e)))?
                } else {
                    // Use DefaultAzureCredential for managed identity, environment vars, etc.
                    create_default_creds()?
                }
            } else {
                // Use DefaultAzureCredential
                create_default_creds()?
            };

            // Create blob client and delete
            let blob_client = ClientBuilder::new(account_name, storage_credentials)
                .blob_client(&container_name, &blob_name);

            blob_client.delete().await.map_err(|e| {
                CollabError::Internal(format!("Failed to delete Azure blob: {}", e))
            })?;

            tracing::info!("Successfully deleted Azure blob: {}", backup_url);
            Ok(())
        }

        #[cfg(not(feature = "azure"))]
        {
            let _ = (backup_url, storage_config); // Suppress unused warnings
            Err(CollabError::Internal(
                "Azure deletion requires 'azure' feature to be enabled. Add 'azure' feature to mockforge-collab in Cargo.toml.".to_string(),
            ))
        }
    }

    /// Delete backup from Google Cloud Storage
    async fn delete_from_gcs(
        &self,
        backup_url: &str,
        _storage_config: Option<&serde_json::Value>,
    ) -> Result<()> {
        #[cfg(feature = "gcs")]
        {
            use google_cloud_storage::client::StorageControl;

            // Parse GCS URL (format: gs://bucket-name/path/to/file)
            if !backup_url.starts_with("gs://") {
                return Err(CollabError::Internal(format!(
                    "Invalid GCS URL format: {}",
                    backup_url
                )));
            }

            let url_parts: Vec<&str> = backup_url
                .strip_prefix("gs://")
                .ok_or_else(|| {
                    CollabError::Internal(format!("Invalid GCS URL format: {}", backup_url))
                })?
                .splitn(2, '/')
                .collect();
            if url_parts.len() != 2 {
                return Err(CollabError::Internal(format!(
                    "Invalid GCS URL format (expected gs://bucket/object): {}",
                    backup_url
                )));
            }

            let bucket_name = url_parts[0];
            let object_name = url_parts[1];

            // Initialize GCS StorageControl client using the new API
            // Uses default credentials from environment (GOOGLE_APPLICATION_CREDENTIALS)
            // or metadata server when running on GCP
            let client = StorageControl::builder().build().await.map_err(|e| {
                CollabError::Internal(format!("Failed to create GCS client: {}", e))
            })?;

            // Delete object using google-cloud-storage 1.5 API
            client
                .delete_object()
                .set_bucket(format!("projects/_/buckets/{}", bucket_name))
                .set_object(object_name)
                .send()
                .await
                .map_err(|e| {
                    CollabError::Internal(format!("Failed to delete GCS object: {}", e))
                })?;

            tracing::info!("Successfully deleted GCS object: {}", backup_url);
            Ok(())
        }

        #[cfg(not(feature = "gcs"))]
        {
            let _ = backup_url; // Suppress unused warning
            Err(CollabError::Internal(
                "GCS deletion requires 'gcs' feature to be enabled. Add 'gcs' feature to mockforge-collab in Cargo.toml.".to_string(),
            ))
        }
    }

    /// Delete backup from custom HTTP storage backend.
    async fn delete_from_custom(
        &self,
        backup_url: &str,
        storage_config: Option<&serde_json::Value>,
    ) -> Result<()> {
        let mut request = self.client.delete(backup_url);
        if let Some(config) = storage_config {
            if let Some(headers) = config.get("headers").and_then(|h| h.as_object()) {
                for (key, value) in headers {
                    if let Some(value) = value.as_str() {
                        request = request.header(key, value);
                    }
                }
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| CollabError::Internal(format!("Custom delete request failed: {e}")))?;
        if !response.status().is_success() {
            return Err(CollabError::Internal(format!(
                "Custom delete failed with status {}",
                response.status()
            )));
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_backend_equality() {
        assert_eq!(StorageBackend::Local, StorageBackend::Local);
        assert_eq!(StorageBackend::S3, StorageBackend::S3);
        assert_eq!(StorageBackend::Azure, StorageBackend::Azure);
        assert_eq!(StorageBackend::Gcs, StorageBackend::Gcs);
        assert_eq!(StorageBackend::Custom, StorageBackend::Custom);

        assert_ne!(StorageBackend::Local, StorageBackend::S3);
    }

    #[test]
    fn test_storage_backend_serialization() {
        let backend = StorageBackend::S3;
        let json = serde_json::to_string(&backend).unwrap();
        let deserialized: StorageBackend = serde_json::from_str(&json).unwrap();

        assert_eq!(backend, deserialized);
    }

    #[test]
    fn test_storage_backend_all_variants() {
        let backends = vec![
            StorageBackend::Local,
            StorageBackend::S3,
            StorageBackend::Azure,
            StorageBackend::Gcs,
            StorageBackend::Custom,
        ];

        for backend in backends {
            let json = serde_json::to_string(&backend).unwrap();
            let deserialized: StorageBackend = serde_json::from_str(&json).unwrap();
            assert_eq!(backend, deserialized);
        }
    }

    #[test]
    fn test_workspace_backup_new() {
        let workspace_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();
        let backup_url = "s3://bucket/backup.yaml".to_string();
        let size_bytes = 1024;

        let backup = WorkspaceBackup::new(
            workspace_id,
            backup_url.clone(),
            StorageBackend::S3,
            size_bytes,
            created_by,
        );

        assert_eq!(backup.workspace_id, workspace_id);
        assert_eq!(backup.backup_url, backup_url);
        assert_eq!(backup.storage_backend, StorageBackend::S3);
        assert_eq!(backup.size_bytes, size_bytes);
        assert_eq!(backup.created_by, created_by);
        assert_eq!(backup.backup_format, "yaml");
        assert!(!backup.encrypted);
        assert!(backup.commit_id.is_none());
        assert!(backup.expires_at.is_none());
        assert!(backup.storage_config.is_none());
    }

    #[test]
    fn test_workspace_backup_clone() {
        let backup = WorkspaceBackup::new(
            Uuid::new_v4(),
            "backup.yaml".to_string(),
            StorageBackend::Local,
            512,
            Uuid::new_v4(),
        );

        let cloned = backup.clone();

        assert_eq!(backup.id, cloned.id);
        assert_eq!(backup.workspace_id, cloned.workspace_id);
        assert_eq!(backup.backup_url, cloned.backup_url);
        assert_eq!(backup.size_bytes, cloned.size_bytes);
    }

    #[test]
    fn test_workspace_backup_serialization() {
        let backup = WorkspaceBackup::new(
            Uuid::new_v4(),
            "backup.yaml".to_string(),
            StorageBackend::Local,
            256,
            Uuid::new_v4(),
        );

        let json = serde_json::to_string(&backup).unwrap();
        let deserialized: WorkspaceBackup = serde_json::from_str(&json).unwrap();

        assert_eq!(backup.id, deserialized.id);
        assert_eq!(backup.workspace_id, deserialized.workspace_id);
        assert_eq!(backup.storage_backend, deserialized.storage_backend);
    }

    #[test]
    fn test_workspace_backup_with_commit() {
        let mut backup = WorkspaceBackup::new(
            Uuid::new_v4(),
            "backup.yaml".to_string(),
            StorageBackend::Local,
            128,
            Uuid::new_v4(),
        );

        let commit_id = Uuid::new_v4();
        backup.commit_id = Some(commit_id);

        assert_eq!(backup.commit_id, Some(commit_id));
    }

    #[test]
    fn test_workspace_backup_with_encryption() {
        let mut backup = WorkspaceBackup::new(
            Uuid::new_v4(),
            "backup.yaml".to_string(),
            StorageBackend::S3,
            2048,
            Uuid::new_v4(),
        );

        backup.encrypted = true;

        assert!(backup.encrypted);
    }

    #[test]
    fn test_workspace_backup_with_expiration() {
        let mut backup = WorkspaceBackup::new(
            Uuid::new_v4(),
            "backup.yaml".to_string(),
            StorageBackend::Azure,
            512,
            Uuid::new_v4(),
        );

        let expires_at = Utc::now() + chrono::Duration::days(30);
        backup.expires_at = Some(expires_at);

        assert!(backup.expires_at.is_some());
    }

    #[test]
    fn test_workspace_backup_with_storage_config() {
        let mut backup = WorkspaceBackup::new(
            Uuid::new_v4(),
            "backup.yaml".to_string(),
            StorageBackend::S3,
            1024,
            Uuid::new_v4(),
        );

        let config = serde_json::json!({
            "region": "us-east-1",
            "bucket": "my-bucket"
        });
        backup.storage_config = Some(config.clone());

        assert_eq!(backup.storage_config, Some(config));
    }

    #[test]
    fn test_workspace_backup_different_formats() {
        let mut backup = WorkspaceBackup::new(
            Uuid::new_v4(),
            "backup.json".to_string(),
            StorageBackend::Local,
            256,
            Uuid::new_v4(),
        );

        assert_eq!(backup.backup_format, "yaml"); // Default

        backup.backup_format = "json".to_string();
        assert_eq!(backup.backup_format, "json");
    }

    #[test]
    fn test_storage_backend_debug() {
        let backend = StorageBackend::S3;
        let debug_str = format!("{:?}", backend);
        assert!(debug_str.contains("S3"));
    }

    #[test]
    fn test_workspace_backup_debug() {
        let backup = WorkspaceBackup::new(
            Uuid::new_v4(),
            "backup.yaml".to_string(),
            StorageBackend::Local,
            100,
            Uuid::new_v4(),
        );

        let debug_str = format!("{:?}", backup);
        assert!(debug_str.contains("WorkspaceBackup"));
    }
}
