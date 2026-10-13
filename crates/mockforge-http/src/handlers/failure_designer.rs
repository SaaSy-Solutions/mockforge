//! Compatibility state and typed Chaos adapter for Intelligence handlers.
use axum::{extract::State, response::Json};
use mockforge_chaos::{
    failure_designer::{FailureDesignRule, FailureDesigner, FailureType},
    ChaosScenario,
};
use mockforge_intelligence::handlers::failure_designer::{
    self as handlers, FailureDesignerProvider,
};
use serde_json::Value;
use std::sync::Arc;

pub use handlers::ValidateRuleResponse;
/// Original rule validation request type.
pub type ValidateRuleRequest = handlers::ValidateRuleRequest<FailureDesignRule>;
/// Original scenario generation request type.
pub type GenerateScenarioRequest = handlers::GenerateScenarioRequest<FailureDesignRule>;
/// Original typed scenario generation response.
pub type GenerateScenarioResponse = handlers::GenerateScenarioResponse<ChaosScenario>;

/// Failure designer API state
#[derive(Clone)]
pub struct FailureDesignerState {
    /// Failure designer instance
    pub designer: Arc<FailureDesigner>,
}

impl FailureDesignerState {
    /// Create new failure designer state
    pub fn new() -> Self {
        Self {
            designer: Arc::new(FailureDesigner::new()),
        }
    }
}

impl Default for FailureDesignerState {
    fn default() -> Self {
        Self::new()
    }
}

impl FailureDesignerProvider for FailureDesignerState {
    type Rule = FailureDesignRule;
    type Scenario = ChaosScenario;
    fn validate_rule(&self, rule: &Self::Rule) -> Result<(), String> {
        self.designer.validate_rule(rule)
    }
    fn rule_to_scenario(&self, rule: &Self::Rule) -> Result<Self::Scenario, String> {
        self.designer.rule_to_scenario(rule)
    }
    fn generate_route_chaos_config(&self, rule: &Self::Rule) -> Result<Value, String> {
        self.designer.generate_route_chaos_config(rule)
    }
    fn webhook_hook(&self, rule: &Self::Rule) -> Result<Option<Value>, String> {
        if matches!(rule.failure_type, FailureType::WebhookFailure { .. }) {
            self.designer.generate_webhook_hook(rule).map(Some)
        } else {
            Ok(None)
        }
    }
}
/// Validate through the original state and request paths.
pub async fn validate_rule(
    state: State<FailureDesignerState>,
    request: Json<ValidateRuleRequest>,
) -> Json<Value> {
    handlers::validate_rule(state, request).await
}
/// Generate through the original state and request paths.
pub async fn generate_scenario(
    state: State<FailureDesignerState>,
    request: Json<GenerateScenarioRequest>,
) -> Result<Json<Value>, String> {
    handlers::generate_scenario(state, request).await
}
/// Preview through the original state and request paths.
pub async fn preview_config(
    state: State<FailureDesignerState>,
    request: Json<GenerateScenarioRequest>,
) -> Result<Json<Value>, String> {
    handlers::preview_config(state, request).await
}
