//! Which configured drift budget applies to an endpoint.

use crate::contract_drift::types::{DriftBudget, DriftBudgetConfig};

/// Budget for an endpoint. Priority order: workspace > service/tag >
/// endpoint > default.
pub(super) fn budget_for(
    config: &DriftBudgetConfig,
    endpoint: &str,
    method: &str,
    workspace_id: Option<&str>,
    service_name: Option<&str>,
    tags: Option<&[String]>,
) -> DriftBudget {
    // Priority 1: Per-workspace budget
    if let Some(workspace_id) = workspace_id {
        if let Some(budget) = config.per_workspace_budgets.get(workspace_id) {
            return budget.clone();
        }
    }

    // Priority 2: Per-service budget (explicit service name)
    if let Some(service_name) = service_name {
        if let Some(budget) = config.per_service_budgets.get(service_name) {
            return budget.clone();
        }
    }

    // Priority 3: Per-tag budget (from OpenAPI tags)
    if let Some(tags) = tags {
        for tag in tags {
            if let Some(budget) = config.per_tag_budgets.get(tag) {
                return budget.clone();
            }
            // Also check per_service_budgets for tag matches
            if let Some(budget) = config.per_service_budgets.get(tag) {
                return budget.clone();
            }
        }
    }

    // Priority 4: Per-endpoint budget
    let key = format!("{} {}", method, endpoint);
    if let Some(budget) = config.per_endpoint_budgets.get(&key) {
        return budget.clone();
    }

    // Priority 5: Default budget
    config.default_budget.clone().unwrap_or_default()
}
