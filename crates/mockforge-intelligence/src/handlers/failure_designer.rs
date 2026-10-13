//! Failure design HTTP handlers with a typed downstream Chaos provider.
use axum::{extract::State, response::Json};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};

/// Operations supplied by the application's existing failure designer.
pub trait FailureDesignerProvider: Clone + Send + Sync + 'static {
    /// Concrete rule accepted by the existing engine.
    type Rule: DeserializeOwned + Serialize + Send + Sync + 'static;
    /// Concrete scenario returned by that engine.
    type Scenario: Serialize;
    /// Validate without generating a scenario.
    fn validate_rule(&self, rule: &Self::Rule) -> Result<(), String>;
    /// Generate the original typed scenario.
    fn rule_to_scenario(&self, rule: &Self::Rule) -> Result<Self::Scenario, String>;
    /// Generate the route configuration consumed by the runtime.
    fn generate_route_chaos_config(&self, rule: &Self::Rule) -> Result<Value, String>;
    /// Generate a hook for webhook rules, or retain the original null for other rules.
    fn webhook_hook(&self, rule: &Self::Rule) -> Result<Option<Value>, String>;
}

/// Request to validate a failure design rule
#[derive(Debug, Deserialize)]
pub struct ValidateRuleRequest<R> {
    /// Rule to validate
    pub rule: R,
}

/// Request to generate chaos scenario from rule
#[derive(Debug, Deserialize)]
pub struct GenerateScenarioRequest<R> {
    /// Rule to convert
    pub rule: R,
}

/// Response for rule validation
#[derive(Debug, Serialize)]
pub struct ValidateRuleResponse {
    /// Success flag
    pub success: bool,
    /// Validation errors (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for scenario generation
#[derive(Debug, Serialize)]
pub struct GenerateScenarioResponse<C> {
    /// Success flag
    pub success: bool,
    /// Generated chaos scenario
    pub scenario: C,
    /// Route chaos configuration (JSON)
    pub route_chaos_config: Value,
    /// Webhook hook configuration (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_hook: Option<Value>,
}

/// Validate a failure design rule
///
/// POST /api/v1/chaos/failure-designer/validate
pub async fn validate_rule<S: FailureDesignerProvider>(
    State(state): State<S>,
    Json(request): Json<ValidateRuleRequest<S::Rule>>,
) -> Json<Value> {
    let designer = &state;
    match designer.validate_rule(&request.rule) {
        Ok(()) => Json(json!({
            "success": true,
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e,
        })),
    }
}

/// Generate chaos scenario from failure design rule
///
/// POST /api/v1/chaos/failure-designer/generate
pub async fn generate_scenario<S: FailureDesignerProvider>(
    State(state): State<S>,
    Json(request): Json<GenerateScenarioRequest<S::Rule>>,
) -> Result<Json<Value>, String> {
    let designer = &state;

    // Generate chaos scenario
    let scenario = designer
        .rule_to_scenario(&request.rule)
        .map_err(|e| format!("Failed to generate scenario: {}", e))?;

    // Generate route chaos config
    let route_chaos_config = designer
        .generate_route_chaos_config(&request.rule)
        .map_err(|e| format!("Failed to generate route chaos config: {}", e))?;

    // Generate webhook hook if applicable
    let webhook_hook = designer
        .webhook_hook(&request.rule)
        .map_err(|e| format!("Failed to generate webhook hook: {}", e))?;

    Ok(Json(json!({
        "success": true,
        "scenario": scenario,
        "route_chaos_config": route_chaos_config,
        "webhook_hook": webhook_hook,
    })))
}

/// Preview generated configuration
///
/// POST /api/v1/chaos/failure-designer/preview
pub async fn preview_config<S: FailureDesignerProvider>(
    State(state): State<S>,
    Json(request): Json<GenerateScenarioRequest<S::Rule>>,
) -> Result<Json<Value>, String> {
    let designer = &state;

    // Validate rule first
    designer
        .validate_rule(&request.rule)
        .map_err(|e| format!("Validation failed: {}", e))?;

    // Generate all configurations
    let scenario = designer
        .rule_to_scenario(&request.rule)
        .map_err(|e| format!("Failed to generate scenario: {}", e))?;

    let route_chaos_config = designer
        .generate_route_chaos_config(&request.rule)
        .map_err(|e| format!("Failed to generate route chaos config: {}", e))?;

    let webhook_hook = designer
        .webhook_hook(&request.rule)
        .map_err(|e| format!("Failed to generate webhook hook: {}", e))?;

    Ok(Json(json!({
        "success": true,
        "rule": request.rule,
        "scenario": scenario,
        "route_chaos_config": route_chaos_config,
        "webhook_hook": webhook_hook,
    })))
}
