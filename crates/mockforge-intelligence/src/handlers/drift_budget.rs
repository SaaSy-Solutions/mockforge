//! Typed drift engine boundaries and stable public HTTP entrypoints.
//! Budget, incident, GitOps and metric operations live in focused submodules.

use super::structural_incidents::StructuralIncidents;
use async_trait::async_trait;
use mockforge_contracts::contract_drift::types::{DriftBudget, DriftBudgetConfig};
use mockforge_foundation::incidents_types::DriftIncident;

mod budgets;
mod gitops;
mod incidents;
mod metrics;
mod scoped_budgets;

pub use budgets::*;
pub use gitops::*;
pub use incidents::*;
pub use metrics::*;
pub use scoped_budgets::*;

/// Read access to the existing drift budget engine and its precedence rules.
pub trait DriftBudgetSource: Send + Sync {
    /// Current engine configuration.
    fn config(&self) -> &DriftBudgetConfig;
    /// Resolve the effective endpoint/workspace/service/tag budget.
    fn get_budget_for_endpoint(
        &self,
        endpoint: &str,
        method: &str,
        workspace_id: Option<&str>,
        service_name: Option<&str>,
        tags: Option<&[String]>,
    ) -> DriftBudget;
}

/// GitOps implementation supplied by the consuming application.
#[async_trait]
pub trait DriftGitOps: Send + Sync {
    /// Generate a pull request with the existing configured GitOps behavior.
    async fn generate_pr_from_incidents(
        &self,
        incidents: &[DriftIncident],
    ) -> mockforge_foundation::Result<Option<crate::pr_generation::PRResult>>;
}

/// Existing engine/manager instances supplied by downstream HTTP integrations.
pub trait DriftBudgetStateProvider: Clone + Send + Sync + 'static {
    /// Budget lookup implementation.
    type Engine: DriftBudgetSource;
    /// Structural incident manager implementation.
    type Incidents: StructuralIncidents;
    /// Optional configured GitOps implementation.
    type GitOps: DriftGitOps;
    /// Access the existing engine.
    fn engine(&self) -> &Self::Engine;
    /// Access the existing incident manager.
    fn incident_manager(&self) -> &Self::Incidents;
    /// Access the configured GitOps handler, if any.
    fn gitops_handler(&self) -> Option<&Self::GitOps>;
    /// Publish through the application's pipeline bus when enabled. The default
    /// keeps applications without pipeline support free of that dependency.
    fn publish_drift_threshold(
        &self,
        _workspace: uuid::Uuid,
        _endpoint: String,
        _exceeded: i32,
        _total: i32,
    ) {
    }
}

/// Create drift budget router
pub fn drift_budget_router<S: DriftBudgetStateProvider>(state: S) -> axum::Router {
    use axum::{
        routing::{get, patch, post},
        Router,
    };

    Router::new()
        .route("/api/v1/drift/budgets", post(create_budget::<S>))
        .route("/api/v1/drift/budgets", get(list_budgets::<S>))
        .route("/api/v1/drift/budgets/lookup", get(get_budget_for_endpoint::<S>))
        .route("/api/v1/drift/budgets/workspace", post(create_workspace_budget::<S>))
        .route("/api/v1/drift/budgets/service", post(create_service_budget::<S>))
        .route("/api/v1/drift/budgets/{id}", get(get_budget::<S>))
        .route("/api/v1/drift/incidents", get(list_incidents::<S>))
        .route("/api/v1/drift/incidents/stats", get(get_incident_stats::<S>))
        .route("/api/v1/drift/incidents/{id}", get(get_incident::<S>))
        .route("/api/v1/drift/incidents/{id}", patch(update_incident::<S>))
        .route("/api/v1/drift/incidents/{id}/resolve", post(resolve_incident::<S>))
        .route("/api/v1/drift/gitops/generate-pr", post(generate_gitops_pr::<S>))
        .route("/api/v1/drift/metrics", get(get_drift_metrics::<S>))
        .with_state(state)
}
