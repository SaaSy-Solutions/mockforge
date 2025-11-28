//! # MockForge Virtual Backend Reality (VBR) Engine
//!
//! The VBR engine creates stateful mock servers with persistent virtual databases,
//! auto-generated CRUD APIs, relationship constraints, session management, and
//! time-based data evolution.
//!
//! ## Overview
//!
//! VBR acts like a mini real backend with:
//! - Persistent virtual database (SQLite, JSON, in-memory options)
//! - CRUD APIs auto-generated from entity schemas
//! - Relationship modeling and constraint enforcement
//! - User session & auth emulation
//! - Time-based data evolution (data aging, expiring sessions)
//!
//! ## Example Usage
//!
//! ```no_run
//! use mockforge_vbr::{VbrEngine, VbrConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = VbrConfig::default()
//!     .with_storage_backend(mockforge_vbr::StorageBackend::Sqlite {
//!         path: "./data/vbr.db".into(),
//!     });
//!
//! let engine = VbrEngine::new(config).await?;
//! // Define entities and generate living API
//! # Ok(())
//! # }
//! ```

// Re-export error types from mockforge-core
pub use mockforge_core::{Error, Result};

use std::collections::HashMap;
use std::sync::Arc;

// Core modules
pub mod config;
pub mod database;
pub mod entities;
pub mod schema;

// Database and migration modules
pub mod constraints;
pub mod migration;

// API generation modules
pub mod api_generator;
pub mod handlers;

// Session and auth modules
pub mod auth;
pub mod session;

// Time-based features
pub mod aging;
pub mod mutation_rules;
pub mod scheduler;

// Integration module
pub mod integration;

// OpenAPI integration
pub mod openapi;

// Data seeding
pub mod seeding;

// ID generation
pub mod id_generation;

// Snapshots
pub mod snapshots;

// Re-export commonly used types
pub use config::{StorageBackend, VbrConfig};
pub use database::VirtualDatabase;
pub use entities::{Entity, EntityRegistry};
pub use mutation_rules::{
    ComparisonOperator, MutationOperation, MutationRule, MutationRuleManager, MutationTrigger,
};
pub use schema::{ManyToManyDefinition, VbrSchemaDefinition};
pub use snapshots::{SnapshotMetadata, TimeTravelSnapshotState};

/// Main VBR engine
pub struct VbrEngine {
    /// Configuration
    config: VbrConfig,
    /// Virtual database instance (stored in Arc for sharing)
    database: Arc<dyn VirtualDatabase + Send + Sync>,
    /// Entity registry
    registry: EntityRegistry,
}

impl VbrEngine {
    /// Create a new VBR engine with the given configuration
    pub async fn new(config: VbrConfig) -> Result<Self> {
        // Initialize virtual database (already in Arc)
        let database = database::create_database(&config.storage).await?;

        // Initialize entity registry
        let registry = EntityRegistry::new();

        Ok(Self {
            config,
            database,
            registry,
        })
    }

    /// Get the configuration
    pub fn config(&self) -> &VbrConfig {
        &self.config
    }

    /// Get the virtual database as Arc for sharing
    pub fn database_arc(&self) -> Arc<dyn VirtualDatabase + Send + Sync> {
        Arc::clone(&self.database)
    }

    /// Get a reference to the virtual database
    pub fn database(&self) -> &dyn VirtualDatabase {
        self.database.as_ref()
    }

    /// Get the entity registry
    pub fn registry(&self) -> &EntityRegistry {
        &self.registry
    }

    /// Get mutable access to the entity registry
    pub fn registry_mut(&mut self) -> &mut EntityRegistry {
        &mut self.registry
    }

    /// Create VBR engine from an OpenAPI specification
    ///
    /// This method automatically:
    /// - Parses the OpenAPI 3.x specification
    /// - Extracts entity schemas from `components/schemas`
    /// - Auto-detects primary keys and foreign keys
    /// - Registers all entities in the engine
    /// - Creates database tables for all entities
    ///
    /// # Arguments
    /// * `config` - VBR configuration
    /// * `openapi_content` - OpenAPI specification content (JSON or YAML)
    ///
    /// # Returns
    /// VBR engine with entities registered and database initialized
    pub async fn from_openapi(
        config: VbrConfig,
        openapi_content: &str,
    ) -> Result<(Self, openapi::OpenApiConversionResult)> {
        // Parse OpenAPI content (JSON or YAML)
        let json_value: serde_json::Value = if openapi_content.trim_start().starts_with('{') {
            serde_json::from_str(openapi_content)
                .map_err(|e| Error::generic(format!("Failed to parse OpenAPI JSON: {}", e)))?
        } else {
            serde_yaml::from_str(openapi_content)
                .map_err(|e| Error::generic(format!("Failed to parse OpenAPI YAML: {}", e)))?
        };

        // Load OpenAPI spec
        let spec = mockforge_core::openapi::OpenApiSpec::from_json(json_value)
            .map_err(|e| Error::generic(format!("Failed to load OpenAPI spec: {}", e)))?;

        // Validate spec
        spec.validate()
            .map_err(|e| Error::generic(format!("Invalid OpenAPI specification: {}", e)))?;

        // Convert OpenAPI to VBR entities
        let conversion_result = openapi::convert_openapi_to_vbr(&spec)?;

        // Create engine
        let mut engine = Self::new(config).await?;

        // Register entities and create database tables
        for (entity_name, vbr_schema) in &conversion_result.entities {
            let entity = entities::Entity::new(entity_name.clone(), vbr_schema.clone());
            engine.registry_mut().register(entity.clone())?;

            // Create database table for this entity
            migration::create_table_for_entity(engine.database.as_ref(), &entity).await?;
        }

        Ok((engine, conversion_result))
    }

    /// Load VBR engine from an OpenAPI file
    ///
    /// # Arguments
    /// * `config` - VBR configuration
    /// * `file_path` - Path to OpenAPI specification file (JSON or YAML)
    ///
    /// # Returns
    /// VBR engine with entities registered and database initialized
    pub async fn from_openapi_file<P: AsRef<std::path::Path>>(
        config: VbrConfig,
        file_path: P,
    ) -> Result<(Self, openapi::OpenApiConversionResult)> {
        let content = tokio::fs::read_to_string(file_path.as_ref())
            .await
            .map_err(|e| Error::generic(format!("Failed to read OpenAPI file: {}", e)))?;

        Self::from_openapi(config, &content).await
    }

    /// Seed entity with data
    ///
    /// # Arguments
    /// * `entity_name` - Name of the entity to seed
    /// * `records` - Records to insert
    pub async fn seed_entity(
        &self,
        entity_name: &str,
        records: &[HashMap<String, serde_json::Value>],
    ) -> Result<usize> {
        seeding::seed_entity(self.database.as_ref(), &self.registry, entity_name, records).await
    }

    /// Seed all entities with data (respects dependencies)
    ///
    /// # Arguments
    /// * `seed_data` - Seed data organized by entity name
    pub async fn seed_all(&self, seed_data: &seeding::SeedData) -> Result<HashMap<String, usize>> {
        seeding::seed_all(self.database.as_ref(), &self.registry, seed_data).await
    }

    /// Load and seed from a file
    ///
    /// # Arguments
    /// * `file_path` - Path to seed file (JSON or YAML)
    pub async fn seed_from_file<P: AsRef<std::path::Path>>(
        &self,
        file_path: P,
    ) -> Result<HashMap<String, usize>> {
        let seed_data = seeding::load_seed_file(file_path).await?;
        self.seed_all(&seed_data).await
    }

    /// Clear all data from an entity
    ///
    /// # Arguments
    /// * `entity_name` - Name of the entity to clear
    pub async fn clear_entity(&self, entity_name: &str) -> Result<()> {
        seeding::clear_entity(self.database.as_ref(), &self.registry, entity_name).await
    }

    /// Clear all data from all entities
    pub async fn clear_all(&self) -> Result<()> {
        seeding::clear_all(self.database.as_ref(), &self.registry).await
    }

    /// Create a snapshot of the current database state
    ///
    /// # Arguments
    /// * `name` - Name for the snapshot
    /// * `description` - Optional description
    /// * `snapshots_dir` - Directory to store snapshots
    pub async fn create_snapshot<P: AsRef<std::path::Path>>(
        &self,
        name: &str,
        description: Option<String>,
        snapshots_dir: P,
    ) -> Result<snapshots::SnapshotMetadata> {
        let manager = snapshots::SnapshotManager::new(snapshots_dir);
        manager
            .create_snapshot(name, description, self.database.as_ref(), &self.registry)
            .await
    }

    /// Create a snapshot with time travel state
    ///
    /// # Arguments
    /// * `name` - Name for the snapshot
    /// * `description` - Optional description
    /// * `snapshots_dir` - Directory to store snapshots
    /// * `include_time_travel` - Whether to include time travel state
    /// * `time_travel_state` - Optional time travel state to include
    pub async fn create_snapshot_with_time_travel<P: AsRef<std::path::Path>>(
        &self,
        name: &str,
        description: Option<String>,
        snapshots_dir: P,
        include_time_travel: bool,
        time_travel_state: Option<snapshots::TimeTravelSnapshotState>,
    ) -> Result<snapshots::SnapshotMetadata> {
        let manager = snapshots::SnapshotManager::new(snapshots_dir);
        manager
            .create_snapshot_with_time_travel(
                name,
                description,
                self.database.as_ref(),
                &self.registry,
                include_time_travel,
                time_travel_state,
            )
            .await
    }

    /// Restore a snapshot
    ///
    /// # Arguments
    /// * `name` - Name of the snapshot to restore
    /// * `snapshots_dir` - Directory where snapshots are stored
    pub async fn restore_snapshot<P: AsRef<std::path::Path>>(
        &self,
        name: &str,
        snapshots_dir: P,
    ) -> Result<()> {
        let manager = snapshots::SnapshotManager::new(snapshots_dir);
        manager.restore_snapshot(name, self.database.as_ref(), &self.registry).await
    }

    /// Restore a snapshot with time travel state
    ///
    /// # Arguments
    /// * `name` - Name of the snapshot to restore
    /// * `snapshots_dir` - Directory where snapshots are stored
    /// * `restore_time_travel` - Whether to restore time travel state
    /// * `time_travel_restore_callback` - Optional callback to restore time travel state
    pub async fn restore_snapshot_with_time_travel<P, F>(
        &self,
        name: &str,
        snapshots_dir: P,
        restore_time_travel: bool,
        time_travel_restore_callback: Option<F>,
    ) -> Result<()>
    where
        P: AsRef<std::path::Path>,
        F: FnOnce(snapshots::TimeTravelSnapshotState) -> Result<()>,
    {
        let manager = snapshots::SnapshotManager::new(snapshots_dir);
        manager
            .restore_snapshot_with_time_travel(
                name,
                self.database.as_ref(),
                &self.registry,
                restore_time_travel,
                time_travel_restore_callback,
            )
            .await
    }

    /// List all snapshots
    ///
    /// # Arguments
    /// * `snapshots_dir` - Directory where snapshots are stored
    pub async fn list_snapshots<P: AsRef<std::path::Path>>(
        snapshots_dir: P,
    ) -> Result<Vec<snapshots::SnapshotMetadata>> {
        let manager = snapshots::SnapshotManager::new(snapshots_dir);
        manager.list_snapshots().await
    }

    /// Delete a snapshot
    ///
    /// # Arguments
    /// * `name` - Name of the snapshot to delete
    /// * `snapshots_dir` - Directory where snapshots are stored
    pub async fn delete_snapshot<P: AsRef<std::path::Path>>(
        name: &str,
        snapshots_dir: P,
    ) -> Result<()> {
        let manager = snapshots::SnapshotManager::new(snapshots_dir);
        manager.delete_snapshot(name).await
    }

    /// Reset database to empty state
    pub async fn reset(&self) -> Result<()> {
        snapshots::reset_database(self.database.as_ref(), &self.registry).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vbr_engine_creation() {
        // Use Memory backend for tests to avoid file system issues
        let config = VbrConfig::default().with_storage_backend(StorageBackend::Memory);
        let engine = VbrEngine::new(config).await;
        assert!(engine.is_ok());
    }
}
