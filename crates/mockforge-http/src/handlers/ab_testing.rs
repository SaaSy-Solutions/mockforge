//! A/B Testing API handlers
//!
//! This module provides HTTP handlers for managing A/B tests and mock variants.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_core::ab_testing::analytics::ABTestReport;
use mockforge_core::ab_testing::{
    ABTestConfig, VariantAnalytics, VariantComparison, VariantManager,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

/// State for A/B testing handlers
#[derive(Clone)]
pub struct ABTestingState {
    /// Variant manager
    pub variant_manager: Arc<VariantManager>,
}

impl ABTestingState {
    /// Create new A/B testing state
    pub fn new() -> Self {
        Self {
            variant_manager: Arc::new(VariantManager::new()),
        }
    }
}

/// Request to create or update an A/B test
#[derive(Debug, Deserialize)]
pub struct CreateABTestRequest {
    /// A/B test configuration
    pub test: ABTestConfig,
}

/// Request to update variant allocation
#[derive(Debug, Deserialize)]
pub struct UpdateAllocationRequest {
    /// New allocations
    pub allocations: Vec<mockforge_core::ab_testing::VariantAllocation>,
}

/// Query parameters for endpoint operations
#[derive(Debug, Deserialize)]
pub struct EndpointQuery {
    /// HTTP method
    pub method: String,
    /// Endpoint path
    pub path: String,
}

/// Create or update an A/B test
///
/// POST /api/v1/ab-tests
pub async fn create_ab_test(
    State(state): State<ABTestingState>,
    Json(req): Json<CreateABTestRequest>,
) -> Result<Json<ABTestConfig>, StatusCode> {
    info!("Creating A/B test: {}", req.test.test_name);

    // Validate allocations
    if let Err(e) = req.test.validate_allocations() {
        error!("Invalid A/B test configuration: {}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    match state.variant_manager.register_test(req.test.clone()).await {
        Ok(_) => {
            info!("A/B test created successfully: {}", req.test.test_name);
            Ok(Json(req.test))
        }
        Err(e) => {
            error!("Failed to create A/B test: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get A/B test configuration for an endpoint
///
/// GET /api/v1/ab-tests?method={method}&path={path}
pub async fn get_ab_test(
    State(state): State<ABTestingState>,
    Query(params): Query<EndpointQuery>,
) -> Result<Json<ABTestConfig>, StatusCode> {
    state
        .variant_manager
        .get_test(&params.method, &params.path)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// List all A/B tests
///
/// GET /api/v1/ab-tests
pub async fn list_ab_tests(
    State(state): State<ABTestingState>,
) -> Result<Json<Vec<ABTestConfig>>, StatusCode> {
    let tests = state.variant_manager.list_tests().await;
    Ok(Json(tests))
}

/// Delete an A/B test
///
/// DELETE /api/v1/ab-tests?method={method}&path={path}
pub async fn delete_ab_test(
    State(state): State<ABTestingState>,
    Query(params): Query<EndpointQuery>,
) -> Result<Json<Value>, StatusCode> {
    match state.variant_manager.remove_test(&params.method, &params.path).await {
        Ok(_) => {
            info!("A/B test deleted: {} {}", params.method, params.path);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("A/B test deleted for {} {}", params.method, params.path)
            })))
        }
        Err(e) => {
            error!("Failed to delete A/B test: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get analytics for an A/B test
///
/// GET /api/v1/ab-tests/analytics?method={method}&path={path}
pub async fn get_ab_test_analytics(
    State(state): State<ABTestingState>,
    Query(params): Query<EndpointQuery>,
) -> Result<Json<ABTestReport>, StatusCode> {
    let test_config = state
        .variant_manager
        .get_test(&params.method, &params.path)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let variant_analytics =
        state.variant_manager.get_endpoint_analytics(&params.method, &params.path).await;

    let report = ABTestReport::new(test_config, variant_analytics);
    Ok(Json(report))
}

/// Get analytics for a specific variant
///
/// GET /api/v1/ab-tests/variants/analytics?method={method}&path={path}&variant_id={variant_id}
pub async fn get_variant_analytics(
    State(state): State<ABTestingState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<VariantAnalytics>, StatusCode> {
    let method = params.get("method").ok_or(StatusCode::BAD_REQUEST)?;
    let path = params.get("path").ok_or(StatusCode::BAD_REQUEST)?;
    let variant_id = params.get("variant_id").ok_or(StatusCode::BAD_REQUEST)?;

    state
        .variant_manager
        .get_variant_analytics(method, path, variant_id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Compare two variants
///
/// GET /api/v1/ab-tests/variants/compare?method={method}&path={path}&variant_a={id}&variant_b={id}
pub async fn compare_variants(
    State(state): State<ABTestingState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<VariantComparison>, StatusCode> {
    let method = params.get("method").ok_or(StatusCode::BAD_REQUEST)?;
    let path = params.get("path").ok_or(StatusCode::BAD_REQUEST)?;
    let variant_a_id = params.get("variant_a").ok_or(StatusCode::BAD_REQUEST)?;
    let variant_b_id = params.get("variant_b").ok_or(StatusCode::BAD_REQUEST)?;

    let analytics_a = state
        .variant_manager
        .get_variant_analytics(method, path, variant_a_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let analytics_b = state
        .variant_manager
        .get_variant_analytics(method, path, variant_b_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let comparison = VariantComparison::new(&analytics_a, &analytics_b);
    Ok(Json(comparison))
}

/// Update variant allocations for an A/B test
///
/// PUT /api/v1/ab-tests/allocations?method={method}&path={path}
pub async fn update_allocations(
    State(state): State<ABTestingState>,
    Query(params): Query<EndpointQuery>,
    Json(req): Json<UpdateAllocationRequest>,
) -> Result<Json<ABTestConfig>, StatusCode> {
    let mut test_config = state
        .variant_manager
        .get_test(&params.method, &params.path)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate new allocations
    test_config.allocations = req.allocations;
    if let Err(e) = test_config.validate_allocations() {
        error!("Invalid allocations: {}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update the test
    match state.variant_manager.register_test(test_config.clone()).await {
        Ok(_) => {
            info!("Updated allocations for {} {}", params.method, params.path);
            Ok(Json(test_config))
        }
        Err(e) => {
            error!("Failed to update allocations: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Enable or disable an A/B test
///
/// PATCH /api/v1/ab-tests/enable?method={method}&path={path}&enabled={true|false}
pub async fn toggle_ab_test(
    State(state): State<ABTestingState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ABTestConfig>, StatusCode> {
    let method = params.get("method").ok_or(StatusCode::BAD_REQUEST)?;
    let path = params.get("path").ok_or(StatusCode::BAD_REQUEST)?;
    let enabled = params
        .get("enabled")
        .and_then(|v| v.parse::<bool>().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let mut test_config = state
        .variant_manager
        .get_test(method, path)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    test_config.enabled = enabled;

    match state.variant_manager.register_test(test_config.clone()).await {
        Ok(_) => {
            info!(
                "{} A/B test for {} {}",
                if enabled { "Enabled" } else { "Disabled" },
                method,
                path
            );
            Ok(Json(test_config))
        }
        Err(e) => {
            error!("Failed to toggle A/B test: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create A/B testing router
pub fn ab_testing_router(state: ABTestingState) -> axum::Router {
    use axum::routing::{delete, get, patch, post, put};
    use axum::Router;

    Router::new()
        .route("/api/v1/ab-tests", post(create_ab_test).get(list_ab_tests))
        .route("/api/v1/ab-tests/analytics", get(get_ab_test_analytics))
        .route("/api/v1/ab-tests/variants/analytics", get(get_variant_analytics))
        .route("/api/v1/ab-tests/variants/compare", get(compare_variants))
        .route("/api/v1/ab-tests/allocations", put(update_allocations))
        .route("/api/v1/ab-tests/enable", patch(toggle_ab_test))
        .route("/api/v1/ab-tests/delete", delete(delete_ab_test))
        .with_state(state)
}
