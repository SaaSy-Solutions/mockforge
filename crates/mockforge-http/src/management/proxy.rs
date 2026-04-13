use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_proxy::config::{BodyTransform, BodyTransformRule, TransformOperation};
use serde::{Deserialize, Serialize};

use super::{default_true, ManagementState};

/// Request body for creating/updating proxy replacement rules
#[derive(Debug, Deserialize, Serialize)]
pub struct ProxyRuleRequest {
    /// URL pattern to match (supports wildcards like "/api/users/*")
    pub pattern: String,
    /// Rule type: "request" or "response"
    #[serde(rename = "type")]
    pub rule_type: String,
    /// Optional status code filter for response rules
    #[serde(default)]
    pub status_codes: Vec<u16>,
    /// Body transformations to apply
    pub body_transforms: Vec<BodyTransformRequest>,
    /// Whether this rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Request body for individual body transformations
#[derive(Debug, Deserialize, Serialize)]
pub struct BodyTransformRequest {
    /// JSONPath expression to target (e.g., "$.userId", "$.email")
    pub path: String,
    /// Replacement value (supports template expansion like "{{uuid}}", "{{faker.email}}")
    pub replace: String,
    /// Operation to perform: "replace", "add", or "remove"
    #[serde(default)]
    pub operation: String,
}

/// Response format for proxy rules
#[derive(Debug, Serialize)]
pub struct ProxyRuleResponse {
    /// Rule ID (index in the array)
    pub id: usize,
    /// URL pattern
    pub pattern: String,
    /// Rule type
    #[serde(rename = "type")]
    pub rule_type: String,
    /// Status codes (for response rules)
    pub status_codes: Vec<u16>,
    /// Body transformations
    pub body_transforms: Vec<BodyTransformRequest>,
    /// Whether enabled
    pub enabled: bool,
}

/// List all proxy replacement rules
pub(crate) async fn list_proxy_rules(
    State(state): State<ManagementState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Proxy not configured. Proxy config not available."
            })));
        }
    };

    let config = proxy_config.read().await;

    let mut rules: Vec<ProxyRuleResponse> = Vec::new();

    // Add request replacement rules
    for (idx, rule) in config.request_replacements.iter().enumerate() {
        rules.push(ProxyRuleResponse {
            id: idx,
            pattern: rule.pattern.clone(),
            rule_type: "request".to_string(),
            status_codes: Vec::new(),
            body_transforms: rule
                .body_transforms
                .iter()
                .map(|t| BodyTransformRequest {
                    path: t.path.clone(),
                    replace: t.replace.clone(),
                    operation: format!("{:?}", t.operation).to_lowercase(),
                })
                .collect(),
            enabled: rule.enabled,
        });
    }

    // Add response replacement rules
    let request_count = config.request_replacements.len();
    for (idx, rule) in config.response_replacements.iter().enumerate() {
        rules.push(ProxyRuleResponse {
            id: request_count + idx,
            pattern: rule.pattern.clone(),
            rule_type: "response".to_string(),
            status_codes: rule.status_codes.clone(),
            body_transforms: rule
                .body_transforms
                .iter()
                .map(|t| BodyTransformRequest {
                    path: t.path.clone(),
                    replace: t.replace.clone(),
                    operation: format!("{:?}", t.operation).to_lowercase(),
                })
                .collect(),
            enabled: rule.enabled,
        });
    }

    Ok(Json(serde_json::json!({
        "rules": rules
    })))
}

/// Create a new proxy replacement rule
pub(crate) async fn create_proxy_rule(
    State(state): State<ManagementState>,
    Json(request): Json<ProxyRuleRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Proxy not configured. Proxy config not available."
            })));
        }
    };

    // Validate request
    if request.body_transforms.is_empty() {
        return Ok(Json(serde_json::json!({
            "error": "At least one body transform is required"
        })));
    }

    let body_transforms: Vec<BodyTransform> = request
        .body_transforms
        .iter()
        .map(|t| {
            let op = match t.operation.as_str() {
                "replace" => TransformOperation::Replace,
                "add" => TransformOperation::Add,
                "remove" => TransformOperation::Remove,
                _ => TransformOperation::Replace,
            };
            BodyTransform {
                path: t.path.clone(),
                replace: t.replace.clone(),
                operation: op,
            }
        })
        .collect();

    let new_rule = BodyTransformRule {
        pattern: request.pattern.clone(),
        status_codes: request.status_codes.clone(),
        body_transforms,
        enabled: request.enabled,
    };

    let mut config = proxy_config.write().await;

    let rule_id = if request.rule_type == "request" {
        config.request_replacements.push(new_rule);
        config.request_replacements.len() - 1
    } else if request.rule_type == "response" {
        config.response_replacements.push(new_rule);
        config.request_replacements.len() + config.response_replacements.len() - 1
    } else {
        return Ok(Json(serde_json::json!({
            "error": format!("Invalid rule type: {}. Must be 'request' or 'response'", request.rule_type)
        })));
    };

    Ok(Json(serde_json::json!({
        "id": rule_id,
        "message": "Rule created successfully"
    })))
}

/// Get a specific proxy replacement rule
pub(crate) async fn get_proxy_rule(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Proxy not configured. Proxy config not available."
            })));
        }
    };

    let config = proxy_config.read().await;
    let rule_id: usize = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(serde_json::json!({
                "error": format!("Invalid rule ID: {}", id)
            })));
        }
    };

    let request_count = config.request_replacements.len();

    if rule_id < request_count {
        // Request rule
        let rule = &config.request_replacements[rule_id];
        Ok(Json(serde_json::json!({
            "id": rule_id,
            "pattern": rule.pattern,
            "type": "request",
            "status_codes": [],
            "body_transforms": rule.body_transforms.iter().map(|t| serde_json::json!({
                "path": t.path,
                "replace": t.replace,
                "operation": format!("{:?}", t.operation).to_lowercase()
            })).collect::<Vec<_>>(),
            "enabled": rule.enabled
        })))
    } else if rule_id < request_count + config.response_replacements.len() {
        // Response rule
        let response_idx = rule_id - request_count;
        let rule = &config.response_replacements[response_idx];
        Ok(Json(serde_json::json!({
            "id": rule_id,
            "pattern": rule.pattern,
            "type": "response",
            "status_codes": rule.status_codes,
            "body_transforms": rule.body_transforms.iter().map(|t| serde_json::json!({
                "path": t.path,
                "replace": t.replace,
                "operation": format!("{:?}", t.operation).to_lowercase()
            })).collect::<Vec<_>>(),
            "enabled": rule.enabled
        })))
    } else {
        Ok(Json(serde_json::json!({
            "error": format!("Rule ID {} not found", rule_id)
        })))
    }
}

/// Update a proxy replacement rule
pub(crate) async fn update_proxy_rule(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    Json(request): Json<ProxyRuleRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Proxy not configured. Proxy config not available."
            })));
        }
    };

    let mut config = proxy_config.write().await;
    let rule_id: usize = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(serde_json::json!({
                "error": format!("Invalid rule ID: {}", id)
            })));
        }
    };

    let body_transforms: Vec<BodyTransform> = request
        .body_transforms
        .iter()
        .map(|t| {
            let op = match t.operation.as_str() {
                "replace" => TransformOperation::Replace,
                "add" => TransformOperation::Add,
                "remove" => TransformOperation::Remove,
                _ => TransformOperation::Replace,
            };
            BodyTransform {
                path: t.path.clone(),
                replace: t.replace.clone(),
                operation: op,
            }
        })
        .collect();

    let updated_rule = BodyTransformRule {
        pattern: request.pattern.clone(),
        status_codes: request.status_codes.clone(),
        body_transforms,
        enabled: request.enabled,
    };

    let request_count = config.request_replacements.len();

    if rule_id < request_count {
        // Update request rule
        config.request_replacements[rule_id] = updated_rule;
    } else if rule_id < request_count + config.response_replacements.len() {
        // Update response rule
        let response_idx = rule_id - request_count;
        config.response_replacements[response_idx] = updated_rule;
    } else {
        return Ok(Json(serde_json::json!({
            "error": format!("Rule ID {} not found", rule_id)
        })));
    }

    Ok(Json(serde_json::json!({
        "id": rule_id,
        "message": "Rule updated successfully"
    })))
}

/// Delete a proxy replacement rule
pub(crate) async fn delete_proxy_rule(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let proxy_config = match &state.proxy_config {
        Some(config) => config,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Proxy not configured. Proxy config not available."
            })));
        }
    };

    let mut config = proxy_config.write().await;
    let rule_id: usize = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(serde_json::json!({
                "error": format!("Invalid rule ID: {}", id)
            })));
        }
    };

    let request_count = config.request_replacements.len();

    if rule_id < request_count {
        // Delete request rule
        config.request_replacements.remove(rule_id);
    } else if rule_id < request_count + config.response_replacements.len() {
        // Delete response rule
        let response_idx = rule_id - request_count;
        config.response_replacements.remove(response_idx);
    } else {
        return Ok(Json(serde_json::json!({
            "error": format!("Rule ID {} not found", rule_id)
        })));
    }

    Ok(Json(serde_json::json!({
        "id": rule_id,
        "message": "Rule deleted successfully"
    })))
}

/// Get proxy rules and transformation configuration for inspection
pub(crate) async fn get_proxy_inspect(
    State(state): State<ManagementState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit: usize = params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(50);
    let offset: usize = params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0);

    let proxy_config = match &state.proxy_config {
        Some(config) => config.read().await,
        None => {
            return Ok(Json(serde_json::json!({
                "error": "Proxy not configured. Proxy config not available."
            })));
        }
    };

    let mut rules = Vec::new();
    for (idx, rule) in proxy_config.request_replacements.iter().enumerate() {
        rules.push(serde_json::json!({
            "id": idx,
            "kind": "request",
            "pattern": rule.pattern,
            "enabled": rule.enabled,
            "status_codes": rule.status_codes,
            "transform_count": rule.body_transforms.len(),
            "transforms": rule.body_transforms.iter().map(|t| serde_json::json!({
                "path": t.path,
                "operation": t.operation,
                "replace": t.replace
            })).collect::<Vec<_>>()
        }));
    }
    let request_rule_count = rules.len();
    for (idx, rule) in proxy_config.response_replacements.iter().enumerate() {
        rules.push(serde_json::json!({
            "id": request_rule_count + idx,
            "kind": "response",
            "pattern": rule.pattern,
            "enabled": rule.enabled,
            "status_codes": rule.status_codes,
            "transform_count": rule.body_transforms.len(),
            "transforms": rule.body_transforms.iter().map(|t| serde_json::json!({
                "path": t.path,
                "operation": t.operation,
                "replace": t.replace
            })).collect::<Vec<_>>()
        }));
    }

    let total = rules.len();
    let paged_rules: Vec<_> = rules.into_iter().skip(offset).take(limit).collect();

    Ok(Json(serde_json::json!({
        "enabled": proxy_config.enabled,
        "target_url": proxy_config.target_url,
        "prefix": proxy_config.prefix,
        "timeout_seconds": proxy_config.timeout_seconds,
        "follow_redirects": proxy_config.follow_redirects,
        "passthrough_by_default": proxy_config.passthrough_by_default,
        "rules": paged_rules,
        "request_rule_count": request_rule_count,
        "response_rule_count": total.saturating_sub(request_rule_count),
        "limit": limit,
        "offset": offset,
        "total": total
    })))
}
