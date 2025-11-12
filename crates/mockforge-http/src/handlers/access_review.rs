//! Access review API handlers
//!
//! Provides HTTP endpoints for managing access reviews, including:
//! - Listing reviews
//! - Approving/revoking access
//! - Getting review reports
//! - Starting reviews

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_core::security::{
    access_review::{
        AccessReview, ReviewType, UserReviewItem,
    },
    access_review_service::AccessReviewService,
    emit_security_event, EventActor, EventOutcome, EventTarget, SecurityEvent, SecurityEventType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

/// State for access review handlers
#[derive(Clone)]
pub struct AccessReviewState {
    /// Access review service
    pub service: Arc<RwLock<AccessReviewService>>,
}

/// Request to approve access in a review
#[derive(Debug, Deserialize)]
pub struct ApproveAccessRequest {
    /// User ID to approve
    pub user_id: Uuid,
    /// Whether access is approved
    pub approved: bool,
    /// Justification for approval
    pub justification: Option<String>,
}

/// Request to revoke access in a review
#[derive(Debug, Deserialize)]
pub struct RevokeAccessRequest {
    /// User ID to revoke
    pub user_id: Uuid,
    /// Reason for revocation
    pub reason: String,
}

/// Response for review list
#[derive(Debug, Serialize)]
pub struct ReviewListResponse {
    /// List of reviews
    pub reviews: Vec<ReviewSummary>,
}

/// Review summary (simplified review for list view)
#[derive(Debug, Serialize)]
pub struct ReviewSummary {
    /// Review ID
    pub review_id: String,
    /// Review type
    pub review_type: String,
    /// Review status
    pub status: String,
    /// Due date
    pub due_date: chrono::DateTime<chrono::Utc>,
    /// Total items count
    pub items_count: u32,
    /// Pending approvals count
    pub pending_approvals: u32,
}

/// Response for review details
#[derive(Debug, Serialize)]
pub struct ReviewDetailResponse {
    /// Review details
    #[serde(flatten)]
    pub review: AccessReview,
    /// Review items (if user access review)
    pub items: Option<Vec<UserReviewItem>>,
}

/// Response for approve/revoke operations
#[derive(Debug, Serialize)]
pub struct ReviewActionResponse {
    /// Review ID
    pub review_id: String,
    /// User ID
    pub user_id: Uuid,
    /// Action status
    pub status: String,
    /// Action timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Additional message
    pub message: Option<String>,
}

/// Get all access reviews
///
/// GET /api/v1/security/access-reviews
pub async fn list_reviews(
    State(state): State<AccessReviewState>,
) -> Result<Json<ReviewListResponse>, StatusCode> {
    let service = state.service.read().await;
    let reviews = service.get_all_reviews();

    let summaries: Vec<ReviewSummary> = reviews
        .iter()
        .map(|review| ReviewSummary {
            review_id: review.review_id.clone(),
            review_type: format!("{:?}", review.review_type),
            status: format!("{:?}", review.status),
            due_date: review.due_date,
            items_count: review.total_items,
            pending_approvals: review.pending_approvals,
        })
        .collect();

    Ok(Json(ReviewListResponse { reviews: summaries }))
}

/// Get a specific access review
///
/// GET /api/v1/security/access-reviews/{review_id}
pub async fn get_review(
    State(state): State<AccessReviewState>,
    Path(review_id): Path<String>,
) -> Result<Json<ReviewDetailResponse>, StatusCode> {
    let service = state.service.read().await;

    let review = service
        .get_review(&review_id)
        .ok_or_else(|| {
            error!("Review {} not found", review_id);
            StatusCode::NOT_FOUND
        })?
        .clone();

    // Get review items if available
    let items = service
        .engine()
        .get_review_items(&review_id)
        .map(|items_map| items_map.values().cloned().collect());

    Ok(Json(ReviewDetailResponse { review, items }))
}

/// Approve access in a review
///
/// POST /api/v1/security/access-reviews/{review_id}/approve
pub async fn approve_access(
    State(state): State<AccessReviewState>,
    Path(review_id): Path<String>,
    Json(request): Json<ApproveAccessRequest>,
    // In a real implementation, this would extract the authenticated user ID
    // For now, we'll use a placeholder
) -> Result<Json<ReviewActionResponse>, StatusCode> {
    let mut service = state.service.write().await;

    // TODO: Extract actual user ID from authentication
    let approver_id = Uuid::new_v4(); // Placeholder

    match service.approve_user_access(&review_id, request.user_id, approver_id, request.justification).await {
        Ok(()) => {
            info!(
                "Access approved for user {} in review {}",
                request.user_id, review_id
            );

            // Emit security event
            let event = SecurityEvent::new(SecurityEventType::AuthzAccessGranted, None, None)
                .with_actor(EventActor {
                    user_id: Some(approver_id.to_string()),
                    username: None,
                    ip_address: None,
                    user_agent: None,
                })
                .with_target(EventTarget {
                    resource_type: Some("access_review".to_string()),
                    resource_id: Some(review_id.clone()),
                    method: None,
                })
                .with_outcome(EventOutcome {
                    success: true,
                    reason: Some("Access approved in review".to_string()),
                })
                .with_metadata("user_id".to_string(), serde_json::json!(request.user_id));
            emit_security_event(event).await;

            Ok(Json(ReviewActionResponse {
                review_id,
                user_id: request.user_id,
                status: "approved".to_string(),
                timestamp: chrono::Utc::now(),
                message: Some("Access approved successfully".to_string()),
            }))
        }
        Err(e) => {
            error!("Failed to approve access: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Revoke access in a review
///
/// POST /api/v1/security/access-reviews/{review_id}/revoke
pub async fn revoke_access(
    State(state): State<AccessReviewState>,
    Path(review_id): Path<String>,
    Json(request): Json<RevokeAccessRequest>,
    // In a real implementation, this would extract the authenticated user ID
) -> Result<Json<ReviewActionResponse>, StatusCode> {
    let mut service = state.service.write().await;

    // TODO: Extract actual user ID from authentication
    let revoker_id = Uuid::new_v4(); // Placeholder

    match service.revoke_user_access(&review_id, request.user_id, revoker_id, request.reason.clone()).await {
        Ok(()) => {
            info!(
                "Access revoked for user {} in review {}",
                request.user_id, review_id
            );

            // Emit security event
            let event = SecurityEvent::new(SecurityEventType::AccessUserSuspended, None, None)
                .with_actor(EventActor {
                    user_id: Some(revoker_id.to_string()),
                    username: None,
                    ip_address: None,
                    user_agent: None,
                })
                .with_target(EventTarget {
                    resource_type: Some("access_review".to_string()),
                    resource_id: Some(review_id.clone()),
                    method: None,
                })
                .with_outcome(EventOutcome {
                    success: true,
                    reason: Some(request.reason.clone()),
                })
                .with_metadata("user_id".to_string(), serde_json::json!(request.user_id))
                .with_metadata("review_id".to_string(), serde_json::json!(review_id));
            emit_security_event(event).await;

            Ok(Json(ReviewActionResponse {
                review_id,
                user_id: request.user_id,
                status: "revoked".to_string(),
                timestamp: chrono::Utc::now(),
                message: Some(format!("Access revoked: {}", request.reason)),
            }))
        }
        Err(e) => {
            error!("Failed to revoke access: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Get review report
///
/// GET /api/v1/security/access-reviews/{review_id}/report
pub async fn get_review_report(
    State(state): State<AccessReviewState>,
    Path(review_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let service = state.service.read().await;

    let review = service
        .get_review(&review_id)
        .ok_or_else(|| {
            error!("Review {} not found", review_id);
            StatusCode::NOT_FOUND
        })?;

    // Convert review to JSON report format
    let report = serde_json::json!({
        "review_id": review.review_id,
        "review_date": review.review_date,
        "review_type": format!("{:?}", review.review_type),
        "status": format!("{:?}", review.status),
        "total_items": review.total_items,
        "items_reviewed": review.items_reviewed,
        "findings": review.findings,
        "actions_taken": review.actions_taken,
        "pending_reviews": review.pending_approvals,
        "next_review_date": review.next_review_date,
    });

    Ok(Json(report))
}

/// Start a new access review
///
/// POST /api/v1/security/access-reviews/start
pub async fn start_review(
    State(state): State<AccessReviewState>,
    Json(request): Json<StartReviewRequest>,
) -> Result<Json<ReviewDetailResponse>, StatusCode> {
    let mut service = state.service.write().await;

    // Start review based on type
    let review_id = match request.review_type {
        ReviewType::UserAccess => {
            service.start_user_access_review().await
                .map_err(|e| {
                    error!("Failed to start user access review: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
        }
        ReviewType::PrivilegedAccess => {
            service.start_privileged_access_review().await
                .map_err(|e| {
                    error!("Failed to start privileged access review: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
        }
        ReviewType::ApiToken => {
            service.start_token_review().await
                .map_err(|e| {
                    error!("Failed to start token review: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
        }
        ReviewType::ResourceAccess => {
            return Err(StatusCode::NOT_IMPLEMENTED);
        }
    };

    info!("Started access review: {}", review_id);

    // Get the review details
    let review = service.get_review(&review_id)
        .ok_or_else(|| {
            error!("Review {} not found after creation", review_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .clone();

    // Emit security event
    let event = SecurityEvent::new(SecurityEventType::ComplianceComplianceCheck, None, None)
        .with_target(EventTarget {
            resource_type: Some("access_review".to_string()),
            resource_id: Some(review_id.clone()),
            method: None,
        })
        .with_outcome(EventOutcome {
            success: true,
            reason: Some("Access review started".to_string()),
        });
    emit_security_event(event).await;

    let items = service
        .engine()
        .get_review_items(&review_id)
        .map(|items_map| items_map.values().cloned().collect());

    Ok(Json(ReviewDetailResponse {
        review,
        items,
    }))
}

/// Request to start a review
#[derive(Debug, Deserialize)]
pub struct StartReviewRequest {
    /// Review type to start
    pub review_type: ReviewType,
}

/// Create access review router
pub fn access_review_router(state: AccessReviewState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/", get(list_reviews))
        .route("/start", post(start_review))
        .route("/{review_id}", get(get_review))
        .route("/{review_id}/approve", post(approve_access))
        .route("/{review_id}/revoke", post(revoke_access))
        .route("/{review_id}/report", get(get_review_report))
        .with_state(state)
}
