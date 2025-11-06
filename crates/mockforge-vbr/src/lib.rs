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

use std::sync::Arc;

// Core modules
pub mod config;
pub mod database;
pub mod entities;
pub mod schema;

// Database and migration modules
pub mod migration;
pub mod constraints;

// API generation modules
pub mod api_generator;
pub mod handlers;

// Session and auth modules
pub mod session;
pub mod auth;

// Time-based features
pub mod aging;
pub mod scheduler;

// Integration module
pub mod integration;

// Re-export commonly used types
pub use config::{StorageBackend, VbrConfig};
pub use database::VirtualDatabase;
pub use entities::{Entity, EntityRegistry};
pub use schema::VbrSchemaDefinition;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vbr_engine_creation() {
        let config = VbrConfig::default();
        let engine = VbrEngine::new(config).await;
        assert!(engine.is_ok());
    }
}
