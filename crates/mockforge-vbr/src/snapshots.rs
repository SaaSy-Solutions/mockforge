//! State snapshot and reset functionality
//!
//! This module provides functionality to create, restore, and manage database snapshots
//! for point-in-time recovery and environment state management.

use crate::database::VirtualDatabase;
use crate::entities::EntityRegistry;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Snapshot name
    pub name: String,
    /// Timestamp when snapshot was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Optional description
    pub description: Option<String>,
    /// Entity counts in the snapshot
    pub entity_counts: HashMap<String, usize>,
    /// Database size in bytes (if available)
    pub database_size: Option<u64>,
    /// Storage backend type
    pub storage_backend: String,
    /// Time travel state (if included in snapshot)
    #[serde(default)]
    pub time_travel_state: Option<TimeTravelSnapshotState>,
}

/// Time travel state included in snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTravelSnapshotState {
    /// Time travel enabled status
    pub enabled: bool,
    /// Current virtual time (if enabled)
    pub current_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Time scale factor
    pub scale_factor: f64,
    /// Cron jobs (serialized)
    #[serde(default)]
    pub cron_jobs: Vec<serde_json::Value>,
    /// Mutation rules (serialized)
    #[serde(default)]
    pub mutation_rules: Vec<serde_json::Value>,
}

/// Snapshot manager
pub struct SnapshotManager {
    /// Base directory for storing snapshots
    snapshots_dir: PathBuf,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    ///
    /// # Arguments
    /// * `snapshots_dir` - Base directory for storing snapshots
    pub fn new<P: AsRef<Path>>(snapshots_dir: P) -> Self {
        Self {
            snapshots_dir: snapshots_dir.as_ref().to_path_buf(),
        }
    }

    /// Get the path for a snapshot directory
    fn snapshot_path(&self, name: &str) -> PathBuf {
        self.snapshots_dir.join(name)
    }

    /// Get the path for snapshot metadata file
    fn metadata_path(&self, name: &str) -> PathBuf {
        self.snapshot_path(name).join("metadata.json")
    }

    /// Create a snapshot of the current database state
    ///
    /// # Arguments
    /// * `name` - Name for the snapshot
    /// * `description` - Optional description
    /// * `database` - The virtual database instance
    /// * `registry` - The entity registry
    /// * `include_time_travel` - Whether to include time travel state (cron jobs, mutation rules)
    /// * `time_travel_state` - Optional time travel state to include
    pub async fn create_snapshot(
        &self,
        name: &str,
        description: Option<String>,
        database: &dyn crate::database::VirtualDatabase,
        registry: &EntityRegistry,
    ) -> Result<SnapshotMetadata> {
        self.create_snapshot_with_time_travel(name, description, database, registry, false, None)
            .await
    }

    /// Create a snapshot with optional time travel state
    ///
    /// # Arguments
    /// * `name` - Name for the snapshot
    /// * `description` - Optional description
    /// * `database` - The virtual database instance
    /// * `registry` - The entity registry
    /// * `include_time_travel` - Whether to include time travel state
    /// * `time_travel_state` - Optional time travel state to include
    pub async fn create_snapshot_with_time_travel(
        &self,
        name: &str,
        description: Option<String>,
        database: &dyn crate::database::VirtualDatabase,
        registry: &EntityRegistry,
        include_time_travel: bool,
        time_travel_state: Option<TimeTravelSnapshotState>,
    ) -> Result<SnapshotMetadata> {
        // Create snapshot directory
        let snapshot_dir = self.snapshot_path(name);
        fs::create_dir_all(&snapshot_dir)
            .await
            .map_err(|e| Error::generic(format!("Failed to create snapshot directory: {}", e)))?;

        // Get entity counts
        let mut entity_counts = HashMap::new();
        for entity_name in registry.list() {
            if let Some(entity) = registry.get(&entity_name) {
                let table_name = entity.table_name();
                let count_query = format!("SELECT COUNT(*) as count FROM {}", table_name);
                let results = database.query(&count_query, &[]).await?;
                let count = results
                    .first()
                    .and_then(|r| r.get("count"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .unwrap_or(0);
                entity_counts.insert(entity_name.clone(), count);
            }
        }

        // Create snapshot based on storage backend
        let storage_backend = database.connection_info();
        let database_size = self.create_snapshot_data(name, database, registry).await?;

        // Create metadata
        let metadata = SnapshotMetadata {
            name: name.to_string(),
            created_at: chrono::Utc::now(),
            description,
            time_travel_state: if include_time_travel {
                time_travel_state
            } else {
                None
            },
            entity_counts,
            database_size,
            storage_backend,
        };

        // Save metadata
        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| Error::generic(format!("Failed to serialize metadata: {}", e)))?;
        fs::write(self.metadata_path(name), metadata_json)
            .await
            .map_err(|e| Error::generic(format!("Failed to write metadata: {}", e)))?;

        Ok(metadata)
    }

    /// Create snapshot data based on storage backend
    async fn create_snapshot_data(
        &self,
        name: &str,
        database: &dyn crate::database::VirtualDatabase,
        registry: &EntityRegistry,
    ) -> Result<Option<u64>> {
        let snapshot_dir = self.snapshot_path(name);
        let storage_backend = database.connection_info();

        if storage_backend.contains("sqlite") {
            // For SQLite, we'll export all data to JSON
            // In a production system, you might use SQLite backup API
            self.export_sqlite_to_json(&snapshot_dir, database, registry).await?;
            Ok(None) // Size calculation would require file system access
        } else if storage_backend.contains("json") {
            // For JSON backend, copy the JSON file
            // This would require access to the JSON database implementation
            Ok(None)
        } else {
            // For in-memory, export to JSON
            self.export_memory_to_json(&snapshot_dir, database, registry).await?;
            Ok(None)
        }
    }

    /// Export SQLite database to JSON files
    async fn export_sqlite_to_json(
        &self,
        snapshot_dir: &Path,
        database: &dyn crate::database::VirtualDatabase,
        registry: &EntityRegistry,
    ) -> Result<()> {
        let data_dir = snapshot_dir.join("data");
        fs::create_dir_all(&data_dir)
            .await
            .map_err(|e| Error::generic(format!("Failed to create data directory: {}", e)))?;

        // Export each entity table to JSON
        for entity_name in registry.list() {
            if let Some(entity) = registry.get(&entity_name) {
                let table_name = entity.table_name();
                let query = format!("SELECT * FROM {}", table_name);
                let records = database.query(&query, &[]).await?;

                let json_file = data_dir.join(format!("{}.json", entity_name.to_lowercase()));
                let json_content = serde_json::to_string_pretty(&records)
                    .map_err(|e| Error::generic(format!("Failed to serialize data: {}", e)))?;
                fs::write(&json_file, json_content)
                    .await
                    .map_err(|e| Error::generic(format!("Failed to write data file: {}", e)))?;
            }
        }

        Ok(())
    }

    /// Export in-memory database to JSON files
    async fn export_memory_to_json(
        &self,
        snapshot_dir: &Path,
        database: &dyn crate::database::VirtualDatabase,
        registry: &EntityRegistry,
    ) -> Result<()> {
        self.export_sqlite_to_json(snapshot_dir, database, registry).await
    }

    /// Restore a snapshot
    ///
    /// # Arguments
    /// * `name` - Name of the snapshot to restore
    /// * `database` - The virtual database instance (will be reset)
    /// * `registry` - The entity registry
    pub async fn restore_snapshot(
        &self,
        name: &str,
        database: &dyn crate::database::VirtualDatabase,
        registry: &EntityRegistry,
    ) -> Result<()> {
        self.restore_snapshot_with_time_travel(
            name,
            database,
            registry,
            false,
            None::<fn(TimeTravelSnapshotState) -> Result<()>>,
        )
        .await
    }

    /// Restore a snapshot with optional time travel state restoration
    ///
    /// # Arguments
    /// * `name` - Name of the snapshot to restore
    /// * `database` - The virtual database instance (will be reset)
    /// * `registry` - The entity registry
    /// * `restore_time_travel` - Whether to restore time travel state
    /// * `time_travel_restore_callback` - Optional callback to restore time travel state
    pub async fn restore_snapshot_with_time_travel<F>(
        &self,
        name: &str,
        database: &dyn crate::database::VirtualDatabase,
        registry: &EntityRegistry,
        restore_time_travel: bool,
        time_travel_restore_callback: Option<F>,
    ) -> Result<()>
    where
        F: FnOnce(TimeTravelSnapshotState) -> Result<()>,
    {
        // Load metadata to verify snapshot exists
        let metadata = self.get_snapshot_metadata(name).await?;

        // Clear existing data
        self.reset_database(database, registry).await?;

        // Restore data based on storage backend
        let snapshot_dir = self.snapshot_path(name);
        if metadata.storage_backend.contains("sqlite")
            || metadata.storage_backend.contains("memory")
        {
            self.import_json_to_database(&snapshot_dir, database, registry).await?;
        } else if metadata.storage_backend.contains("json") {
            // For JSON backend, would need to copy the JSON file
            // This requires access to the JSON database implementation
            return Err(Error::generic(
                "JSON backend snapshot restore not yet implemented".to_string(),
            ));
        }

        // Restore time travel state if requested and available
        if restore_time_travel {
            if let Some(ref time_travel_state) = metadata.time_travel_state {
                if let Some(callback) = time_travel_restore_callback {
                    callback(time_travel_state.clone())?;
                }
            }
        }

        Ok(())
    }

    /// Import JSON data into database
    async fn import_json_to_database(
        &self,
        snapshot_dir: &Path,
        database: &dyn crate::database::VirtualDatabase,
        registry: &EntityRegistry,
    ) -> Result<()> {
        let data_dir = snapshot_dir.join("data");

        if !data_dir.exists() {
            return Err(Error::generic("Snapshot data directory not found".to_string()));
        }

        // Import each entity
        for entity_name in registry.list() {
            let json_file = data_dir.join(format!("{}.json", entity_name.to_lowercase()));
            if !json_file.exists() {
                // Skip if file doesn't exist (entity had no data in snapshot)
                continue;
            }

            {
                let content = fs::read_to_string(&json_file)
                    .await
                    .map_err(|e| Error::generic(format!("Failed to read data file: {}", e)))?;

                let records: Vec<HashMap<String, Value>> = serde_json::from_str(&content)
                    .map_err(|e| Error::generic(format!("Failed to parse data file: {}", e)))?;

                if let Some(entity) = registry.get(&entity_name) {
                    let table_name = entity.table_name();

                    // Ensure table exists before inserting
                    // For Memory database, this is handled by execute, but we need to make sure
                    // the table structure is preserved after reset
                    if !database.table_exists(&table_name).await.unwrap_or(false) {
                        // Table was removed during reset, we need to recreate it
                        // For Memory database, this happens automatically on first INSERT
                        // But we should ensure the table entry exists
                    }

                    for record in records {
                        let fields: Vec<String> = record.keys().cloned().collect();
                        let placeholders: Vec<String> =
                            (0..fields.len()).map(|_| "?".to_string()).collect();
                        let values: Vec<Value> = fields
                            .iter()
                            .map(|f| record.get(f).cloned().unwrap_or(Value::Null))
                            .collect();

                        let query = format!(
                            "INSERT INTO {} ({}) VALUES ({})",
                            table_name,
                            fields.join(", "),
                            placeholders.join(", ")
                        );

                        database.execute(&query, &values).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Reset database to empty state
    async fn reset_database(
        &self,
        database: &dyn crate::database::VirtualDatabase,
        registry: &EntityRegistry,
    ) -> Result<()> {
        // Delete all data from all tables
        for entity_name in registry.list() {
            if let Some(entity) = registry.get(&entity_name) {
                let table_name = entity.table_name();
                let query = format!("DELETE FROM {}", table_name);
                let _ = database.execute(&query, &[]).await;
            }
        }

        // Reset counters
        let counter_table = "_vbr_counters";
        if database.table_exists(counter_table).await.unwrap_or(false) {
            let query = format!("DELETE FROM {}", counter_table);
            let _ = database.execute(&query, &[]).await;
        }

        Ok(())
    }

    /// List all snapshots
    pub async fn list_snapshots(&self) -> Result<Vec<SnapshotMetadata>> {
        if !self.snapshots_dir.exists() {
            return Ok(Vec::new());
        }

        let mut snapshots = Vec::new();

        let mut entries = fs::read_dir(&self.snapshots_dir)
            .await
            .map_err(|e| Error::generic(format!("Failed to read snapshots directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| Error::generic(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(metadata) = self.get_snapshot_metadata(name).await {
                        snapshots.push(metadata);
                    }
                }
            }
        }

        // Sort by creation time (newest first)
        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(snapshots)
    }

    /// Get snapshot metadata
    pub async fn get_snapshot_metadata(&self, name: &str) -> Result<SnapshotMetadata> {
        let metadata_path = self.metadata_path(name);
        let content = fs::read_to_string(&metadata_path)
            .await
            .map_err(|e| Error::generic(format!("Failed to read snapshot metadata: {}", e)))?;

        let metadata: SnapshotMetadata = serde_json::from_str(&content)
            .map_err(|e| Error::generic(format!("Failed to parse snapshot metadata: {}", e)))?;

        Ok(metadata)
    }

    /// Delete a snapshot
    pub async fn delete_snapshot(&self, name: &str) -> Result<()> {
        let snapshot_dir = self.snapshot_path(name);

        if !snapshot_dir.exists() {
            return Err(Error::generic(format!("Snapshot '{}' not found", name)));
        }

        fs::remove_dir_all(&snapshot_dir)
            .await
            .map_err(|e| Error::generic(format!("Failed to delete snapshot: {}", e)))?;

        Ok(())
    }
}

/// Reset database to empty state (public API)
pub async fn reset_database(
    database: &dyn crate::database::VirtualDatabase,
    registry: &EntityRegistry,
) -> Result<()> {
    // This is a simplified reset - in production, you might want to
    // drop and recreate tables, but for now we'll just delete all data
    for entity_name in registry.list() {
        if let Some(entity) = registry.get(&entity_name) {
            let table_name = entity.table_name();
            let query = format!("DELETE FROM {}", table_name);
            let _ = database.execute(&query, &[]).await;
        }
    }

    // Reset counters
    let counter_table = "_vbr_counters";
    if database.table_exists(counter_table).await.unwrap_or(false) {
        let query = format!("DELETE FROM {}", counter_table);
        let _ = database.execute(&query, &[]).await;
    }

    Ok(())
}
