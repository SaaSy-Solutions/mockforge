//! Compatibility state for drift handlers implemented in Intelligence.
#![allow(deprecated)]

use mockforge_core::contract_drift::DriftBudgetEngine;
use mockforge_core::incidents::IncidentManager;
pub use mockforge_intelligence::handlers::drift_budget::*;
use std::sync::Arc;

/// State for drift budget handlers. Existing fields and struct literals remain valid.
#[derive(Clone)]
pub struct DriftBudgetState {
    /// Drift budget engine
    pub engine: Arc<DriftBudgetEngine>,
    /// Incident manager
    pub incident_manager: Arc<IncidentManager>,
    /// GitOps handler (optional)
    pub gitops_handler: Option<Arc<mockforge_core::drift_gitops::DriftGitOpsHandler>>,
}

impl DriftBudgetStateProvider for DriftBudgetState {
    type Engine = DriftBudgetEngine;
    type Incidents = IncidentManager;
    type GitOps = mockforge_core::drift_gitops::DriftGitOpsHandler;
    fn engine(&self) -> &Self::Engine {
        self.engine.as_ref()
    }
    fn incident_manager(&self) -> &Self::Incidents {
        self.incident_manager.as_ref()
    }
    fn gitops_handler(&self) -> Option<&Self::GitOps> {
        self.gitops_handler.as_deref()
    }
    fn publish_drift_threshold(
        &self,
        _workspace: uuid::Uuid,
        _endpoint: String,
        _exceeded: i32,
        _total: i32,
    ) {
        #[cfg(feature = "pipelines")]
        {
            let event = mockforge_pipelines::PipelineEvent::drift_threshold_exceeded(
                _workspace, _endpoint, _exceeded, _total,
            );
            if let Err(error) = mockforge_pipelines::publish_event(event) {
                tracing::warn!("Failed to publish drift threshold exceeded event: {}", error);
            }
        }
    }
}
