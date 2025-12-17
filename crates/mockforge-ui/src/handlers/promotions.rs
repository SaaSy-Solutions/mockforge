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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_promotion_request_deserialization() {
        let json = r#"{
            "entity_type": "scenario",
            "entity_id": "test-scenario-123",
            "entity_version": "v1.0",
            "workspace_id": "workspace-1",
            "from_environment": "dev",
            "to_environment": "prod",
            "requires_approval": true,
            "comments": "Promoting to prod"
        }"#;

        let request: CreatePromotionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.entity_type, "scenario");
        assert_eq!(request.entity_id, "test-scenario-123");
        assert_eq!(request.entity_version, Some("v1.0".to_string()));
        assert_eq!(request.workspace_id, "workspace-1");
        assert_eq!(request.from_environment, "dev");
        assert_eq!(request.to_environment, "prod");
        assert_eq!(request.requires_approval, Some(true));
        assert_eq!(request.comments, Some("Promoting to prod".to_string()));
    }

    #[test]
    fn test_create_promotion_request_with_tags() {
        let json = r#"{
            "entity_type": "scenario",
            "entity_id": "test-123",
            "workspace_id": "workspace-1",
            "from_environment": "dev",
            "to_environment": "staging",
            "scenario_tags": ["critical", "payment"]
        }"#;

        let request: CreatePromotionRequest = serde_json::from_str(json).unwrap();
        assert!(request.scenario_tags.is_some());
        let tags = request.scenario_tags.unwrap();
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"critical".to_string()));
        assert!(tags.contains(&"payment".to_string()));
    }

    #[test]
    fn test_create_promotion_request_without_optional_fields() {
        let json = r#"{
            "entity_type": "persona",
            "entity_id": "persona-456",
            "workspace_id": "workspace-2",
            "from_environment": "staging",
            "to_environment": "prod"
        }"#;

        let request: CreatePromotionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.entity_version, None);
        assert_eq!(request.requires_approval, None);
        assert_eq!(request.scenario_tags, None);
        assert_eq!(request.comments, None);
        assert_eq!(request.metadata, None);
    }

    #[test]
    fn test_create_promotion_request_with_metadata() {
        let json = r#"{
            "entity_type": "config",
            "entity_id": "config-789",
            "workspace_id": "workspace-3",
            "from_environment": "dev",
            "to_environment": "staging",
            "metadata": {"key": "value", "number": 123}
        }"#;

        let request: CreatePromotionRequest = serde_json::from_str(json).unwrap();
        assert!(request.metadata.is_some());
        let metadata = request.metadata.unwrap();
        assert_eq!(metadata.get("key").unwrap().as_str().unwrap(), "value");
        assert_eq!(metadata.get("number").unwrap().as_i64().unwrap(), 123);
    }

    #[test]
    fn test_promotion_response_serialization() {
        let response = PromotionResponse {
            promotion_id: "promo-123".to_string(),
            entity_type: "scenario".to_string(),
            entity_id: "scenario-456".to_string(),
            entity_version: Some("v2.0".to_string()),
            from_environment: "dev".to_string(),
            to_environment: "prod".to_string(),
            status: "pending".to_string(),
            promoted_by: "user-789".to_string(),
            approved_by: Some("admin-001".to_string()),
            comments: Some("Test promotion".to_string()),
            pr_url: Some("https://github.com/org/repo/pull/123".to_string()),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains("promo-123"));
        assert!(serialized.contains("scenario"));
        assert!(serialized.contains("pending"));
        assert!(serialized.contains("admin-001"));
    }

    #[test]
    fn test_promotion_response_without_optional_fields() {
        let response = PromotionResponse {
            promotion_id: "promo-999".to_string(),
            entity_type: "persona".to_string(),
            entity_id: "persona-111".to_string(),
            entity_version: None,
            from_environment: "staging".to_string(),
            to_environment: "prod".to_string(),
            status: "approved".to_string(),
            promoted_by: "user-222".to_string(),
            approved_by: None,
            comments: None,
            pr_url: None,
            timestamp: "2024-01-02T00:00:00Z".to_string(),
        };

        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: PromotionResponse = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.promotion_id, "promo-999");
        assert_eq!(deserialized.entity_version, None);
        assert_eq!(deserialized.approved_by, None);
        assert_eq!(deserialized.comments, None);
        assert_eq!(deserialized.pr_url, None);
    }

    #[test]
    fn test_update_promotion_status_request_deserialization() {
        let json = r#"{
            "status": "approved",
            "approved_by": "admin-123"
        }"#;

        let request: UpdatePromotionStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.status, "approved");
        assert_eq!(request.approved_by, Some("admin-123".to_string()));
    }

    #[test]
    fn test_update_promotion_status_request_without_approver() {
        let json = r#"{"status": "rejected"}"#;

        let request: UpdatePromotionStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.status, "rejected");
        assert_eq!(request.approved_by, None);
    }

    #[test]
    fn test_list_promotions_query_default() {
        let json = "{}";
        let query: ListPromotionsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, 100); // default value
        assert_eq!(query.status, None);
        assert_eq!(query.entity_type, None);
    }

    #[test]
    fn test_list_promotions_query_with_filters() {
        let json = r#"{
            "limit": 50,
            "status": "pending",
            "entity_type": "scenario"
        }"#;

        let query: ListPromotionsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, 50);
        assert_eq!(query.status, Some("pending".to_string()));
        assert_eq!(query.entity_type, Some("scenario".to_string()));
    }

    #[test]
    fn test_promotion_history_query_deserialization() {
        let json = r#"{"workspace_id": "workspace-abc"}"#;

        let query: PromotionHistoryQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.workspace_id, "workspace-abc");
    }

    #[test]
    fn test_default_limit_function() {
        assert_eq!(default_limit(), 100);
    }

    #[test]
    fn test_create_promotion_request_clone() {
        let request = CreatePromotionRequest {
            entity_type: "scenario".to_string(),
            entity_id: "test-123".to_string(),
            entity_version: Some("v1".to_string()),
            workspace_id: "ws-1".to_string(),
            from_environment: "dev".to_string(),
            to_environment: "prod".to_string(),
            requires_approval: Some(true),
            scenario_tags: Some(vec!["tag1".to_string()]),
            comments: Some("test".to_string()),
            metadata: Some(serde_json::json!({"key": "value"})),
        };

        let cloned = request.clone();
        assert_eq!(cloned.entity_type, request.entity_type);
        assert_eq!(cloned.entity_id, request.entity_id);
        assert_eq!(cloned.workspace_id, request.workspace_id);
    }

    #[test]
    fn test_promotion_response_clone() {
        let response = PromotionResponse {
            promotion_id: "promo-1".to_string(),
            entity_type: "scenario".to_string(),
            entity_id: "scenario-1".to_string(),
            entity_version: Some("v1".to_string()),
            from_environment: "dev".to_string(),
            to_environment: "prod".to_string(),
            status: "pending".to_string(),
            promoted_by: "user-1".to_string(),
            approved_by: None,
            comments: None,
            pr_url: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let cloned = response.clone();
        assert_eq!(cloned.promotion_id, response.promotion_id);
        assert_eq!(cloned.entity_type, response.entity_type);
        assert_eq!(cloned.status, response.status);
    }

    #[test]
    fn test_create_promotion_request_debug() {
        let request = CreatePromotionRequest {
            entity_type: "scenario".to_string(),
            entity_id: "test-123".to_string(),
            entity_version: None,
            workspace_id: "ws-1".to_string(),
            from_environment: "dev".to_string(),
            to_environment: "prod".to_string(),
            requires_approval: None,
            scenario_tags: None,
            comments: None,
            metadata: None,
        };

        let debug_str = format!("{:?}", request);
        assert!(debug_str.contains("test-123"));
        assert!(debug_str.contains("ws-1"));
    }

    #[test]
    fn test_promotion_response_debug() {
        let response = PromotionResponse {
            promotion_id: "promo-1".to_string(),
            entity_type: "scenario".to_string(),
            entity_id: "scenario-1".to_string(),
            entity_version: None,
            from_environment: "dev".to_string(),
            to_environment: "prod".to_string(),
            status: "pending".to_string(),
            promoted_by: "user-1".to_string(),
            approved_by: None,
            comments: None,
            pr_url: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("promo-1"));
        assert!(debug_str.contains("pending"));
    }

    #[test]
    fn test_all_entity_types() {
        let entity_types = vec!["scenario", "persona", "config"];
        for entity_type in entity_types {
            let json = format!(
                r#"{{
                "entity_type": "{}",
                "entity_id": "test-id",
                "workspace_id": "ws-1",
                "from_environment": "dev",
                "to_environment": "prod"
            }}"#,
                entity_type
            );

            let request: CreatePromotionRequest = serde_json::from_str(&json).unwrap();
            assert_eq!(request.entity_type, entity_type);
        }
    }

    #[test]
    fn test_all_status_types() {
        let statuses = vec!["pending", "approved", "rejected", "completed", "failed"];
        for status in statuses {
            let json = format!(r#"{{"status": "{}"}}"#, status);
            let request: UpdatePromotionStatusRequest = serde_json::from_str(&json).unwrap();
            assert_eq!(request.status, status);
        }
    }
}
