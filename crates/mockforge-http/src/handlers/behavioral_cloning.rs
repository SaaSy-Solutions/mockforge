//! Compatibility API and recorder-backed state for behavioral cloning.
//!
//! Handler logic lives in intelligence. These public constructors and fields
//! retain the HTTP crate's recorder path behavior without introducing an
//! intelligence-to-recorder dependency.

use async_trait::async_trait;
use mockforge_intelligence::behavioral_cloning::storage::BehavioralCloningStateProvider;
use mockforge_intelligence::behavioral_cloning::EdgeAmplifier;
use mockforge_recorder::database::RecorderDatabase;
use std::{path::PathBuf, sync::Arc};

pub use mockforge_intelligence::handlers::behavioral_cloning::*;

/// Behavioral cloning API state
#[derive(Clone)]
pub struct BehavioralCloningState {
    /// Edge amplifier instance
    pub edge_amplifier: Arc<EdgeAmplifier>,
    /// Optional recorder database path
    /// If provided, handlers will open the database as needed
    pub database_path: Option<PathBuf>,
}

impl BehavioralCloningState {
    /// Create new behavioral cloning state
    pub fn new() -> Self {
        Self {
            edge_amplifier: Arc::new(EdgeAmplifier::new()),
            database_path: None,
        }
    }

    /// Create new state with database path
    pub fn with_database_path(path: PathBuf) -> Self {
        Self {
            edge_amplifier: Arc::new(EdgeAmplifier::new()),
            database_path: Some(path),
        }
    }

    /// Open database connection
    async fn open_database(&self) -> Result<RecorderDatabase, String> {
        let db_path = self.database_path.as_ref().cloned().unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("recordings.db")
        });

        RecorderDatabase::new(&db_path)
            .await
            .map_err(|e| format!("Failed to open recorder database: {}", e))
    }
}

impl Default for BehavioralCloningState {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BehavioralCloningStateProvider for BehavioralCloningState {
    type Store = RecorderDatabase;

    async fn open_store(&self) -> Result<Self::Store, String> {
        self.open_database().await
    }
}
