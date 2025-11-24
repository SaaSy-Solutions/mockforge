//! Promotion workflow API handlers
//!
//! Provides REST endpoints for managing promotions between environments.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Json, Response},
};
use mockforge_collab::promotion::PromotionService;
use mockforge_core::workspace::{
    mock_environment::MockEnvironmentName,
    scenario_promotion::{
        ApprovalRules, PromotionEntityType, PromotionRequest, PromotionStatus,
        ScenarioPromotionWorkflow,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::handlers::workspaces::{ApiResponse, WorkspaceState};
use crate::rbac::extract_user_context;

/// Promotion state
#[derive(Clone)]
pub struct PromotionState {
    /// Promotion service
    pub promotion_service: Arc<PromotionService>,
    /// Workspace state for accessing workspace configs
    pub workspace_state: WorkspaceState,
}

impl PromotionState {
    /// Create a new promotion state
    pub fn new(promotion_service: Arc<PromotionService>, workspace_state: WorkspaceState) -> Self {
        Self {
            promotion_service,
            workspace_state,
        }
    }
}

/// Create promotion request
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePromotionRequest {
    /// Entity type (scenario, persona, config)
    pub entity_type: String,
    /// Entity ID
    pub entity_id: String,
    /// Entity version (optional)
    pub entity_version: Option<String>,
    /// Workspace ID
    pub workspace_id: String,
    /// Source environment
    pub from_environment: String,
    /// Target environment
    pub to_environment: String,
    /// Whether approval is required (optional, will be determined automatically if not provided)
    pub requires_approval: Option<bool>,
    /// Scenario tags (for scenarios only, used to determine approval requirements)
    #[serde(default)]
    pub scenario_tags: Option<Vec<String>>,
    /// Comments
    pub comments: Option<String>,
    /// Metadata (optional)
    pub metadata: Option<serde_json::Value>,
}

/// Promotion response
#[derive(Debug, Clone, Serialize)]
pub struct PromotionResponse {
    /// Promotion ID
    pub promotion_id: String,
    /// Entity type
    pub entity_type: String,
    /// Entity ID
    pub entity_id: String,
    /// Entity version
    pub entity_version: Option<String>,
    /// From environment
    pub from_environment: String,
    /// To environment
    pub to_environment: String,
    /// Status
    pub status: String,
    /// Promoted by user ID
    pub promoted_by: String,
    /// Approved by user ID (if approved)
    pub approved_by: Option<String>,
    /// Comments
    pub comments: Option<String>,
    /// PR URL (if GitOps enabled)
    pub pr_url: Option<String>,
    /// Timestamp
    pub timestamp: String,
}

/// Update promotion status request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePromotionStatusRequest {
    /// New status
    pub status: String,
    /// Approver user ID (if approving)
    pub approved_by: Option<String>,
}

/// List promotions query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct ListPromotionsQuery {
    /// Limit results
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Filter by status
    pub status: Option<String>,
    /// Filter by entity type
    pub entity_type: Option<String>,
}

fn default_limit() -> i64 {
    100
}

/// Create a promotion request
///
/// POST /api/v2/promotions
pub async fn create_promotion(
    State(state): State<PromotionState>,
    headers: HeaderMap,
    Json(body): Json<CreatePromotionRequest>,
) -> Result<Json<ApiResponse<PromotionResponse>>, Response> {
    // Extract user context from headers (set by RBAC middleware)
    let user_context = extract_user_context(&headers).ok_or_else(|| {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body("User authentication required".into())
            .unwrap()
    })?;

    let user_id = Uuid::parse_str(&user_context.user_id).map_err(|_| {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("Invalid user ID".into())
            .unwrap()
    })?;
    // Parse entity type
    let entity_type = match body.entity_type.to_lowercase().as_str() {
        "scenario" => PromotionEntityType::Scenario,
        "persona" => PromotionEntityType::Persona,
        "config" => PromotionEntityType::Config,
        _ => {
            return Ok(Json(ApiResponse::error(format!(
                "Invalid entity type: {}",
                body.entity_type
            ))));
        }
    };

    // Parse environments
    let from_env = match MockEnvironmentName::from_str(&body.from_environment) {
        Some(env) => env,
        None => {
            return Ok(Json(ApiResponse::error(format!(
                "Invalid from_environment: {}",
                body.from_environment
            ))));
        }
    };

    let to_env = match MockEnvironmentName::from_str(&body.to_environment) {
        Some(env) => env,
        None => {
            return Ok(Json(ApiResponse::error(format!(
                "Invalid to_environment: {}",
                body.to_environment
            ))));
        }
    };

    // Determine if approval is required
    // If not explicitly set, check using approval workflow (for scenarios with tags)
    let (requires_approval, approval_reason) = if let Some(explicit_approval) =
        body.requires_approval
    {
        (
            explicit_approval,
            if explicit_approval {
                Some("Manual approval required for promotion".to_string())
            } else {
                None
            },
        )
    } else if entity_type == PromotionEntityType::Scenario {
        // For scenarios, check approval workflow if tags are provided
        let scenario_tags = body.scenario_tags.as_deref().unwrap_or(&[]);
        let approval_rules = ApprovalRules::default();
        let (requires, reason) =
            ScenarioPromotionWorkflow::requires_approval(scenario_tags, to_env, &approval_rules);
        (requires, reason)
    } else {
        // Default: require approval for safety
        (true, Some("Approval required for promotion".to_string()))
    };

    // Build metadata including scenario tags if provided
    let mut metadata: std::collections::HashMap<String, serde_json::Value> = body
        .metadata
        .as_ref()
        .and_then(|v| {
            if let serde_json::Value::Object(map) = v {
                Some(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            } else {
                None
            }
        })
        .unwrap_or_default();

    // Store scenario tags in metadata for preservation
    if let Some(tags) = &body.scenario_tags {
        metadata.insert(
            "scenario_tags".to_string(),
            serde_json::to_value(tags).unwrap_or(serde_json::Value::Array(vec![])),
        );
    }

    // Create promotion request
    let promotion_request = PromotionRequest {
        entity_type,
        entity_id: body.entity_id.clone(),
        entity_version: body.entity_version.clone(),
        workspace_id: body.workspace_id.clone(),
        from_environment: from_env,
        to_environment: to_env,
        requires_approval,
        approval_required_reason: approval_reason,
        comments: body.comments.clone(),
        metadata,
    };

    // Get workspace config for GitOps (if needed)
    let workspace_config = {
        let registry = state.workspace_state.registry.read().await;
        if let Ok(workspace) = registry.get_workspace(&body.workspace_id) {
            // Serialize workspace config to JSON
            serde_json::to_value(&workspace.workspace.config).ok()
        } else {
            None
        }
    };

    // Record promotion
    let promotion_id = match state
        .promotion_service
        .record_promotion(&promotion_request, user_id, PromotionStatus::Pending, workspace_config)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return Ok(Json(ApiResponse::error(format!("Failed to create promotion: {}", e))));
        }
    };

    // Build response
    let response = PromotionResponse {
        promotion_id: promotion_id.to_string(),
        entity_type: body.entity_type,
        entity_id: body.entity_id,
        entity_version: body.entity_version,
        from_environment: body.from_environment,
        to_environment: body.to_environment,
        status: "pending".to_string(),
        promoted_by: user_id.to_string(),
        approved_by: None,
        comments: body.comments,
        pr_url: None, // Will be populated if GitOps creates PR
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(ApiResponse::success(response)))
}

/// Get promotion details
///
/// GET /api/v2/promotions/{promotion_id}
pub async fn get_promotion(
    State(state): State<PromotionState>,
    Path(promotion_id): Path<String>,
) -> Result<Json<ApiResponse<PromotionResponse>>, Response> {
    let promotion_uuid = match Uuid::parse_str(&promotion_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(Json(ApiResponse::error("Invalid promotion ID".to_string())));
        }
    };

    // Get promotion by ID
    match state.promotion_service.get_promotion_by_id(promotion_uuid).await {
        Ok(Some(promotion)) => {
            let response = PromotionResponse {
                promotion_id: promotion.promotion_id,
                entity_type: promotion.entity_type.to_string(),
                entity_id: promotion.entity_id,
                entity_version: promotion.entity_version,
                from_environment: promotion.from_environment.as_str().to_string(),
                to_environment: promotion.to_environment.as_str().to_string(),
                status: promotion.status.to_string(),
                promoted_by: promotion.promoted_by,
                approved_by: promotion.approved_by,
                comments: promotion.comments,
                pr_url: promotion.pr_url,
                timestamp: promotion.timestamp.to_rfc3339(),
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Ok(None) => Ok(Json(ApiResponse::error("Promotion not found".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to get promotion: {}", e)))),
    }
}

/// Update promotion status
///
/// PUT /api/v2/promotions/{promotion_id}/status
pub async fn update_promotion_status(
    State(state): State<PromotionState>,
    headers: HeaderMap,
    Path(promotion_id): Path<String>,
    Json(body): Json<UpdatePromotionStatusRequest>,
) -> Result<Json<ApiResponse<PromotionResponse>>, Response> {
    // Extract user context from headers (set by RBAC middleware)
    let user_context = extract_user_context(&headers).ok_or_else(|| {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body("User authentication required".into())
            .unwrap()
    })?;
    let promotion_uuid = match Uuid::parse_str(&promotion_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(Json(ApiResponse::error("Invalid promotion ID".to_string())));
        }
    };

    // Parse status
    let status = match body.status.to_lowercase().as_str() {
        "pending" => PromotionStatus::Pending,
        "approved" => PromotionStatus::Approved,
        "rejected" => PromotionStatus::Rejected,
        "completed" => PromotionStatus::Completed,
        "failed" => PromotionStatus::Failed,
        _ => {
            return Ok(Json(ApiResponse::error(format!("Invalid status: {}", body.status))));
        }
    };

    // Use authenticated user as approver if approving/rejecting
    let approver_id = if matches!(status, PromotionStatus::Approved | PromotionStatus::Rejected) {
        Some(Uuid::parse_str(&user_context.user_id).map_err(|_| {
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Invalid user ID".into())
                .unwrap()
        })?)
    } else {
        body.approved_by.and_then(|s| Uuid::parse_str(&s).ok())
    };

    // Update status
    match state
        .promotion_service
        .update_promotion_status(promotion_uuid, status, approver_id)
        .await
    {
        Ok(_) => {
            // Get updated promotion
            match state.promotion_service.get_promotion_by_id(promotion_uuid).await {
                Ok(Some(promotion)) => {
                    let response = PromotionResponse {
                        promotion_id: promotion.promotion_id,
                        entity_type: promotion.entity_type.to_string(),
                        entity_id: promotion.entity_id,
                        entity_version: promotion.entity_version,
                        from_environment: promotion.from_environment.as_str().to_string(),
                        to_environment: promotion.to_environment.as_str().to_string(),
                        status: promotion.status.to_string(),
                        promoted_by: promotion.promoted_by,
                        approved_by: promotion.approved_by,
                        comments: promotion.comments,
                        pr_url: promotion.pr_url,
                        timestamp: promotion.timestamp.to_rfc3339(),
                    };
                    Ok(Json(ApiResponse::success(response)))
                }
                Ok(None) => {
                    Ok(Json(ApiResponse::error("Promotion not found after update".to_string())))
                }
                Err(e) => {
                    Ok(Json(ApiResponse::error(format!("Failed to get updated promotion: {}", e))))
                }
            }
        }
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to update promotion status: {}", e)))),
    }
}

/// List promotions for a workspace
///
/// GET /api/v2/promotions/workspace/{workspace_id}
pub async fn list_workspace_promotions(
    State(state): State<PromotionState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<ListPromotionsQuery>,
) -> Result<Json<ApiResponse<Vec<PromotionResponse>>>, Response> {
    match state
        .promotion_service
        .get_workspace_promotions(&workspace_id, Some(query.limit))
        .await
    {
        Ok(promotions) => {
            let responses: Vec<PromotionResponse> = promotions
                .into_iter()
                .filter(|p| {
                    // Filter by status if provided
                    if let Some(ref status_filter) = query.status {
                        p.status.to_string() == *status_filter
                    } else {
                        true
                    }
                })
                .filter(|p| {
                    // Filter by entity type if provided
                    if let Some(ref entity_type_filter) = query.entity_type {
                        p.entity_type.to_string() == *entity_type_filter
                    } else {
                        true
                    }
                })
                .map(|p| PromotionResponse {
                    promotion_id: p.promotion_id,
                    entity_type: p.entity_type.to_string(),
                    entity_id: p.entity_id,
                    entity_version: p.entity_version,
                    from_environment: p.from_environment.as_str().to_string(),
                    to_environment: p.to_environment.as_str().to_string(),
                    status: p.status.to_string(),
                    promoted_by: p.promoted_by,
                    approved_by: p.approved_by,
                    comments: p.comments,
                    pr_url: p.pr_url,
                    timestamp: p.timestamp.to_rfc3339(),
                })
                .collect();

            Ok(Json(ApiResponse::success(responses)))
        }
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to list promotions: {}", e)))),
    }
}

/// List pending promotions
///
/// GET /api/v2/promotions/pending
pub async fn list_pending_promotions(
    State(state): State<PromotionState>,
    Query(query): Query<ListPromotionsQuery>,
) -> Result<Json<ApiResponse<Vec<PromotionResponse>>>, Response> {
    match state.promotion_service.get_pending_promotions(None).await {
        Ok(promotions) => {
            let responses: Vec<PromotionResponse> = promotions
                .into_iter()
                .take(query.limit as usize)
                .map(|p| PromotionResponse {
                    promotion_id: p.promotion_id,
                    entity_type: p.entity_type.to_string(),
                    entity_id: p.entity_id,
                    entity_version: p.entity_version,
                    from_environment: p.from_environment.as_str().to_string(),
                    to_environment: p.to_environment.as_str().to_string(),
                    status: p.status.to_string(),
                    promoted_by: p.promoted_by,
                    approved_by: p.approved_by,
                    comments: p.comments,
                    pr_url: p.pr_url,
                    timestamp: p.timestamp.to_rfc3339(),
                })
                .collect();

            Ok(Json(ApiResponse::success(responses)))
        }
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to list pending promotions: {}", e)))),
    }
}

/// Get promotion history query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct PromotionHistoryQuery {
    /// Workspace ID
    pub workspace_id: String,
}

/// Get promotion history for an entity
///
/// GET /api/v2/promotions/entity/{entity_type}/{entity_id}
pub async fn get_entity_promotion_history(
    State(state): State<PromotionState>,
    Path((entity_type, entity_id)): Path<(String, String)>,
    Query(query): Query<PromotionHistoryQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, Response> {
    // Parse entity type
    let entity_type_enum = match entity_type.to_lowercase().as_str() {
        "scenario" => PromotionEntityType::Scenario,
        "persona" => PromotionEntityType::Persona,
        "config" => PromotionEntityType::Config,
        _ => {
            return Ok(Json(ApiResponse::error(format!("Invalid entity type: {}", entity_type))));
        }
    };

    match state
        .promotion_service
        .get_promotion_history(&query.workspace_id, entity_type_enum, &entity_id)
        .await
    {
        Ok(history) => {
            let history_json = serde_json::to_value(history).unwrap_or(serde_json::json!({}));
            Ok(Json(ApiResponse::success(history_json)))
        }
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to get promotion history: {}", e)))),
    }
}
