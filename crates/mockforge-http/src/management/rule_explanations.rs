// Uses deprecated `mockforge_core::intelligent_behavior::RuleGenerator`
// pending the eventual intelligent_behavior migration.
#![allow(deprecated)]

//! Rule-explanation storage and learn-from-examples endpoints
//! (`GET /__mockforge/mockai/rules/explanations`,
//! `GET /__mockforge/mockai/rules/{id}/explanation`,
//! `POST /__mockforge/mockai/learn`).
//!
//! Split out of the original `management/ai_gen.rs` under #656. These
//! three handlers share `ManagementState.rule_explanations` — the
//! learn endpoint writes, the list/get endpoints read.
//!
//! Stays in `mockforge-http` rather than moving to `mockforge-intelligence`:
//! the handlers are tightly coupled to `ManagementState`, and moving
//! them would require either relocating `ManagementState` (large blast
//! radius) or factoring out a slimmer state type just for these three.
//! Neither has been judged worth the churn yet.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;

use super::ManagementState;

/// List all rule explanations
pub(crate) async fn list_rule_explanations(
    State(state): State<ManagementState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    use mockforge_foundation::intelligent_behavior::rule_types::RuleType;

    let explanations = state.rule_explanations.read().await;
    let mut explanations_vec: Vec<_> = explanations.values().cloned().collect();

    // Filter by rule type if provided
    if let Some(rule_type_str) = params.get("rule_type") {
        if let Ok(rule_type) = serde_json::from_str::<RuleType>(&format!("\"{}\"", rule_type_str)) {
            explanations_vec.retain(|e| e.rule_type == rule_type);
        }
    }

    // Filter by minimum confidence if provided
    if let Some(min_confidence_str) = params.get("min_confidence") {
        if let Ok(min_confidence) = min_confidence_str.parse::<f64>() {
            explanations_vec.retain(|e| e.confidence >= min_confidence);
        }
    }

    // Sort by confidence (descending) and then by generated_at (descending)
    explanations_vec.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.generated_at.cmp(&a.generated_at))
    });

    Json(serde_json::json!({
        "explanations": explanations_vec,
        "total": explanations_vec.len(),
    }))
    .into_response()
}

/// Get a specific rule explanation by ID
pub(crate) async fn get_rule_explanation(
    State(state): State<ManagementState>,
    Path(rule_id): Path<String>,
) -> impl IntoResponse {
    let explanations = state.rule_explanations.read().await;

    match explanations.get(&rule_id) {
        Some(explanation) => Json(serde_json::json!({
            "explanation": explanation,
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Rule explanation not found",
                "message": format!("No explanation found for rule ID: {}", rule_id)
            })),
        )
            .into_response(),
    }
}

/// Request for learning from examples
#[derive(Debug, Deserialize)]
pub struct LearnFromExamplesRequest {
    /// Example request/response pairs to learn from
    pub examples: Vec<ExamplePairRequest>,
    /// Optional configuration override
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

/// Example pair request format
#[derive(Debug, Deserialize)]
pub struct ExamplePairRequest {
    /// Request data (method, path, body, etc.)
    pub request: serde_json::Value,
    /// Response data (status_code, body, etc.)
    pub response: serde_json::Value,
}

/// Learn behavioral rules from example pairs
///
/// This endpoint accepts example request/response pairs, generates behavioral rules
/// with explanations, and stores the explanations for later retrieval.
pub(crate) async fn learn_from_examples(
    State(state): State<ManagementState>,
    Json(request): Json<LearnFromExamplesRequest>,
) -> impl IntoResponse {
    use mockforge_core::intelligent_behavior::{
        config::{BehaviorModelConfig, IntelligentBehaviorConfig},
        rule_generator::{ExamplePair, RuleGenerator},
    };

    if request.examples.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "No examples provided",
                "message": "At least one example pair is required"
            })),
        )
            .into_response();
    }

    // Convert request examples to ExamplePair format
    let example_pairs: Result<Vec<ExamplePair>, String> = request
        .examples
        .into_iter()
        .enumerate()
        .map(|(idx, ex)| {
            // Parse request JSON to extract method, path, body, etc.
            let method = ex
                .request
                .get("method")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "GET".to_string());
            let path = ex
                .request
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "/".to_string());
            let request_body = ex.request.get("body").cloned();
            let query_params = ex
                .request
                .get("query_params")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            let headers = ex
                .request
                .get("headers")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();

            // Parse response JSON to extract status, body, etc.
            let status = ex
                .response
                .get("status_code")
                .or_else(|| ex.response.get("status"))
                .and_then(|v| v.as_u64())
                .map(|n| n as u16)
                .unwrap_or(200);
            let response_body = ex.response.get("body").cloned();

            Ok(ExamplePair {
                method,
                path,
                request: request_body,
                status,
                response: response_body,
                query_params,
                headers,
                metadata: {
                    let mut meta = std::collections::HashMap::new();
                    meta.insert("source".to_string(), "api".to_string());
                    meta.insert("example_index".to_string(), idx.to_string());
                    meta
                },
            })
        })
        .collect();

    let example_pairs = match example_pairs {
        Ok(pairs) => pairs,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid examples",
                    "message": e
                })),
            )
                .into_response();
        }
    };

    // Create behavior config (use provided config or default)
    let behavior_config = if let Some(config_json) = request.config {
        // Try to deserialize custom config, fall back to default
        serde_json::from_value(config_json)
            .unwrap_or_else(|_| IntelligentBehaviorConfig::default())
            .behavior_model
    } else {
        BehaviorModelConfig::default()
    };

    // Create rule generator
    let generator = RuleGenerator::new(behavior_config);

    // Generate rules with explanations
    let (rules, explanations) =
        match generator.generate_rules_with_explanations(example_pairs).await {
            Ok(result) => result,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Rule generation failed",
                        "message": format!("Failed to generate rules: {}", e)
                    })),
                )
                    .into_response();
            }
        };

    // Store explanations in ManagementState
    {
        let mut stored_explanations = state.rule_explanations.write().await;
        for explanation in &explanations {
            stored_explanations.insert(explanation.rule_id.clone(), explanation.clone());
        }
    }

    // Prepare response
    let response = serde_json::json!({
        "success": true,
        "rules_generated": {
            "consistency_rules": rules.consistency_rules.len(),
            "schemas": rules.schemas.len(),
            "state_machines": rules.state_transitions.len(),
            "system_prompt": !rules.system_prompt.is_empty(),
        },
        "explanations": explanations.iter().map(|e| serde_json::json!({
            "rule_id": e.rule_id,
            "rule_type": e.rule_type,
            "confidence": e.confidence,
            "reasoning": e.reasoning,
        })).collect::<Vec<_>>(),
        "total_explanations": explanations.len(),
    });

    Json(response).into_response()
}
