//! Compatibility entrypoint for the Contracts-owned drift evaluation engine.
use super::{consumer_mapping::ConsumerImpactAnalyzer, fitness::FitnessFunctionRegistry};
use crate::{ai_contract_diff::ContractDiffResult, openapi::OpenApiSpec};
use mockforge_contracts::contract_drift::{
    budget_engine::DriftBudgetEngine as ContractEngine,
    field_tracking::FieldCountTracker,
    types::{DriftBudget, DriftBudgetConfig, DriftResult},
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Existing Core entrypoint retains concrete constructors and registry setters.
#[derive(Debug)]
pub struct DriftBudgetEngine {
    inner: ContractEngine,
}
impl DriftBudgetEngine {
    /// Create a new drift budget engine
    pub fn new(config: DriftBudgetConfig) -> Self {
        Self {
            inner: ContractEngine::new(config),
        }
    }
    /// Create a new drift budget engine with field count tracking
    pub fn new_with_tracker(
        config: DriftBudgetConfig,
        tracker: Arc<RwLock<FieldCountTracker>>,
    ) -> Self {
        Self {
            inner: ContractEngine::new_with_tracker(config, tracker),
        }
    }
    /// Set the fitness function registry
    pub fn set_fitness_registry(&mut self, registry: Arc<RwLock<FitnessFunctionRegistry>>) {
        self.inner.set_fitness_registry(registry);
    }
    /// Set the consumer impact analyzer
    pub fn set_consumer_analyzer(&mut self, analyzer: Arc<RwLock<ConsumerImpactAnalyzer>>) {
        self.inner.set_consumer_analyzer(analyzer);
    }
    /// Set the field count tracker
    pub fn set_field_tracker(&mut self, field_tracker: Arc<RwLock<FieldCountTracker>>) {
        self.inner.set_field_tracker(field_tracker)
    }
    /// Evaluate contract diff result against drift budget
    ///
    /// Returns a `DriftResult` indicating whether the budget is exceeded and
    /// which mismatches are considered breaking.
    pub async fn evaluate(
        &self,
        diff_result: &ContractDiffResult,
        endpoint: &str,
        method: &str,
    ) -> DriftResult {
        self.inner.evaluate(diff_result, endpoint, method).await
    }
    /// Evaluate contract diff result against drift budget with context
    ///
    /// Returns a `DriftResult` indicating whether the budget is exceeded and
    /// which mismatches are considered breaking.
    pub async fn evaluate_with_context(
        &self,
        diff_result: &ContractDiffResult,
        endpoint: &str,
        method: &str,
        workspace_id: Option<&str>,
        service_name: Option<&str>,
        tags: Option<&[String]>,
    ) -> DriftResult {
        self.inner
            .evaluate_with_context(diff_result, endpoint, method, workspace_id, service_name, tags)
            .await
    }
    /// Evaluate with old and new OpenAPI specs
    ///
    /// This method is similar to `evaluate_with_context` but accepts old and new OpenAPI specs
    /// to enable fitness function evaluation.
    #[allow(clippy::too_many_arguments)]
    pub async fn evaluate_with_specs(
        &self,
        old_spec: Option<&OpenApiSpec>,
        new_spec: &OpenApiSpec,
        diff_result: &ContractDiffResult,
        endpoint: &str,
        method: &str,
        workspace_id: Option<&str>,
        service_name: Option<&str>,
        tags: Option<&[String]>,
    ) -> DriftResult {
        self.inner
            .evaluate_with_specs(
                old_spec,
                new_spec,
                diff_result,
                endpoint,
                method,
                workspace_id,
                service_name,
                tags,
            )
            .await
    }
    /// Get budget for a specific endpoint
    ///
    /// Priority order: workspace > service/tag > endpoint > default
    pub fn get_budget_for_endpoint(
        &self,
        endpoint: &str,
        method: &str,
        workspace_id: Option<&str>,
        service_name: Option<&str>,
        tags: Option<&[String]>,
    ) -> DriftBudget {
        self.inner
            .get_budget_for_endpoint(endpoint, method, workspace_id, service_name, tags)
    }
    /// Get the drift budget configuration
    pub fn config(&self) -> &DriftBudgetConfig {
        self.inner.config()
    }
    /// Update the drift budget configuration
    pub fn update_config(&mut self, config: DriftBudgetConfig) {
        self.inner.update_config(config)
    }
}
impl mockforge_intelligence::handlers::drift_budget::DriftBudgetSource for DriftBudgetEngine {
    fn config(&self) -> &DriftBudgetConfig {
        DriftBudgetEngine::config(self)
    }
    fn get_budget_for_endpoint(
        &self,
        endpoint: &str,
        method: &str,
        workspace_id: Option<&str>,
        service_name: Option<&str>,
        tags: Option<&[String]>,
    ) -> DriftBudget {
        DriftBudgetEngine::get_budget_for_endpoint(
            self,
            endpoint,
            method,
            workspace_id,
            service_name,
            tags,
        )
    }
}
