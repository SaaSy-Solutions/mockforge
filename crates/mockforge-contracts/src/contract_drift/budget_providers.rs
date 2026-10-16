//! Typed inputs for optional fitness and consumer analysis, supplied by callers.
use mockforge_foundation::{
    contract_diff_types::ContractDiffResult,
    contract_drift_types::{ConsumerImpact, FitnessTestResult},
};
use mockforge_openapi::OpenApiSpec;
use std::fmt::Debug;

/// Optional fitness registry; failed evaluation retains the engine's historical
/// behavior of leaving the fitness result list unchanged.
pub trait FitnessSource: Debug + Send + Sync {
    /// Run fitness functions for one endpoint change; `None` when evaluation fails.
    #[allow(clippy::too_many_arguments)]
    fn evaluate_fitness(
        &self,
        old_spec: Option<&OpenApiSpec>,
        new_spec: &OpenApiSpec,
        diff: &ContractDiffResult,
        endpoint: &str,
        method: &str,
        workspace: Option<&str>,
        service: Option<&str>,
    ) -> Option<Vec<FitnessTestResult>>;
}

/// Optional consumer impact analysis without a dependency on Core registries.
pub trait ConsumerImpactSource: Debug + Send + Sync {
    /// Consumers affected by a change to this endpoint, if any are mapped.
    fn consumer_impact(&self, endpoint: &str, method: &str) -> Option<ConsumerImpact>;
}
