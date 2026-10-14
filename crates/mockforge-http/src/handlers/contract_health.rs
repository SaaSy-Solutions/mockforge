//! Compatibility state for the contract timeline implemented in Intelligence.

pub use mockforge_intelligence::handlers::contract_health::*;
use std::sync::Arc;

/// State for contract health handlers. Existing fields and struct literals remain valid.
#[derive(Clone)]
pub struct ContractHealthState {
    /// Incident manager for structural incidents
    pub incident_manager: Arc<mockforge_core::incidents::IncidentManager>,
    /// Semantic incident manager
    pub semantic_manager: Arc<mockforge_core::incidents::SemanticIncidentManager>,
    /// Database connection (optional)
    #[cfg(feature = "database")]
    pub database: Option<crate::database::Database>,
}

impl ContractHealthStateProvider for ContractHealthState {
    type Incidents = mockforge_core::incidents::IncidentManager;
    fn incident_manager(&self) -> &Self::Incidents {
        self.incident_manager.as_ref()
    }
    fn semantic_manager(&self) -> &mockforge_intelligence::incidents::SemanticIncidentManager {
        self.semantic_manager.as_ref()
    }
    #[cfg(feature = "database")]
    fn database(&self) -> Option<&crate::database::Database> {
        self.database.as_ref()
    }
}
