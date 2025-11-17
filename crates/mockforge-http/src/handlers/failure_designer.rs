//! Failure Designer API handlers

use axum::extract::{Path, Query, State};
use axum::response::Json;
use mockforge_chaos::failure_designer::{FailureDesignRule, FailureDesigner};
use mockforge_chaos::ChaosScenario;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

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

/// Request to validate a failure design rule
#[derive(Debug, Deserialize)]
pub struct ValidateRuleRequest {
    /// Rule to validate
    pub rule: FailureDesignRule,
}

/// Request to generate chaos scenario from rule
#[derive(Debug, Deserialize)]
pub struct GenerateScenarioRequest {
    /// Rule to convert
    pub rule: FailureDesignRule,
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
pub struct GenerateScenarioResponse {
    /// Success flag
    pub success: bool,
    /// Generated chaos scenario
    pub scenario: ChaosScenario,
    /// Route chaos configuration (JSON)
    pub route_chaos_config: Value,
    /// Webhook hook configuration (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_hook: Option<Value>,
}

/// Validate a failure design rule
///
/// POST /api/v1/chaos/failure-designer/validate
pub async fn validate_rule(
    State(state): State<FailureDesignerState>,
    Json(request): Json<ValidateRuleRequest>,
) -> Json<Value> {
    let designer = &state.designer;
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
pub async fn generate_scenario(
    State(state): State<FailureDesignerState>,
    Json(request): Json<GenerateScenarioRequest>,
) -> Result<Json<Value>, String> {
    let designer = &state.designer;

    // Generate chaos scenario
    let scenario = designer
        .rule_to_scenario(&request.rule)
        .map_err(|e| format!("Failed to generate scenario: {}", e))?;

    // Generate route chaos config
    let route_chaos_config = designer
        .generate_route_chaos_config(&request.rule)
        .map_err(|e| format!("Failed to generate route chaos config: {}", e))?;

    // Generate webhook hook if applicable
    let webhook_hook = if matches!(
        request.rule.failure_type,
        mockforge_chaos::failure_designer::FailureType::WebhookFailure { .. }
    ) {
        Some(
            designer
                .generate_webhook_hook(&request.rule)
                .map_err(|e| format!("Failed to generate webhook hook: {}", e))?,
        )
    } else {
        None
    };

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
pub async fn preview_config(
    State(state): State<FailureDesignerState>,
    Json(request): Json<GenerateScenarioRequest>,
) -> Result<Json<Value>, String> {
    let designer = &state.designer;

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

    let webhook_hook = if matches!(
        request.rule.failure_type,
        mockforge_chaos::failure_designer::FailureType::WebhookFailure { .. }
    ) {
        Some(
            designer
                .generate_webhook_hook(&request.rule)
                .map_err(|e| format!("Failed to generate webhook hook: {}", e))?,
        )
    } else {
        None
    };

    Ok(Json(json!({
        "success": true,
        "rule": request.rule,
        "scenario": scenario,
        "route_chaos_config": route_chaos_config,
        "webhook_hook": webhook_hook,
    })))
}
