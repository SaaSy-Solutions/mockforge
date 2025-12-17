//! Coverage Metrics API handlers (MockOps)
//!
//! Provides endpoints for scenario usage, persona CI hits, endpoint coverage,
//! reality level staleness, and drift percentage metrics.

use axum::{extract::Query, http::StatusCode, Json};
use mockforge_analytics::{
    AnalyticsDatabase, DriftPercentageMetrics, EndpointCoverage, PersonaCIHit,
    RealityLevelStaleness, ScenarioUsageMetrics,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, error};

use crate::models::ApiResponse;

/// Coverage metrics state
#[derive(Clone)]
pub struct CoverageMetricsState {
    pub db: Arc<tokio::sync::OnceCell<AnalyticsDatabase>>,
}

impl CoverageMetricsState {
    pub fn new(db: AnalyticsDatabase) -> Self {
        let cell = tokio::sync::OnceCell::new();
        let _ = cell.set(db);
        Self { db: Arc::new(cell) }
    }

    async fn get_db(&self) -> Result<&AnalyticsDatabase, StatusCode> {
        self.db.get().ok_or_else(|| {
            error!("Analytics database not initialized");
            StatusCode::SERVICE_UNAVAILABLE
        })
    }
}

/// Query parameters for coverage metrics endpoints
#[derive(Debug, Deserialize)]
pub struct CoverageQuery {
    /// Workspace ID filter
    pub workspace_id: Option<String>,
    /// Organization ID filter
    pub org_id: Option<String>,
    /// Limit results
    pub limit: Option<i64>,
    /// Minimum coverage percentage (for endpoint coverage)
    pub min_coverage: Option<f64>,
    /// Maximum staleness days (for reality level staleness)
    pub max_staleness_days: Option<i32>,
}

/// Get scenario usage metrics
///
/// GET /api/v2/analytics/scenarios/usage
pub async fn get_scenario_usage(
    axum::extract::Extension(state): axum::extract::Extension<CoverageMetricsState>,
    Query(params): Query<CoverageQuery>,
) -> Result<Json<ApiResponse<Vec<ScenarioUsageMetrics>>>, StatusCode> {
    debug!("Getting scenario usage metrics");

    let db = state.get_db().await?;
    match db
        .get_scenario_usage(params.workspace_id.as_deref(), params.org_id.as_deref(), params.limit)
        .await
    {
        Ok(metrics) => Ok(Json(ApiResponse::success(metrics))),
        Err(e) => {
            error!("Failed to get scenario usage metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get persona CI hits
///
/// GET /api/v2/analytics/personas/ci-hits
pub async fn get_persona_ci_hits(
    axum::extract::Extension(state): axum::extract::Extension<CoverageMetricsState>,
    Query(params): Query<CoverageQuery>,
) -> Result<Json<ApiResponse<Vec<PersonaCIHit>>>, StatusCode> {
    debug!("Getting persona CI hits");

    let db = state.get_db().await?;
    match db
        .get_persona_ci_hits(params.workspace_id.as_deref(), params.org_id.as_deref(), params.limit)
        .await
    {
        Ok(hits) => Ok(Json(ApiResponse::success(hits))),
        Err(e) => {
            error!("Failed to get persona CI hits: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get endpoint coverage
///
/// GET /api/v2/analytics/endpoints/coverage
pub async fn get_endpoint_coverage(
    axum::extract::Extension(state): axum::extract::Extension<CoverageMetricsState>,
    Query(params): Query<CoverageQuery>,
) -> Result<Json<ApiResponse<Vec<EndpointCoverage>>>, StatusCode> {
    debug!("Getting endpoint coverage");

    let db = state.get_db().await?;
    match db
        .get_endpoint_coverage(
            params.workspace_id.as_deref(),
            params.org_id.as_deref(),
            params.min_coverage,
        )
        .await
    {
        Ok(coverage) => Ok(Json(ApiResponse::success(coverage))),
        Err(e) => {
            error!("Failed to get endpoint coverage: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get reality level staleness
///
/// GET /api/v2/analytics/reality-levels/staleness
pub async fn get_reality_level_staleness(
    axum::extract::Extension(state): axum::extract::Extension<CoverageMetricsState>,
    Query(params): Query<CoverageQuery>,
) -> Result<Json<ApiResponse<Vec<RealityLevelStaleness>>>, StatusCode> {
    debug!("Getting reality level staleness");

    let db = state.get_db().await?;
    match db
        .get_reality_level_staleness(
            params.workspace_id.as_deref(),
            params.org_id.as_deref(),
            params.max_staleness_days,
        )
        .await
    {
        Ok(staleness) => Ok(Json(ApiResponse::success(staleness))),
        Err(e) => {
            error!("Failed to get reality level staleness: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get drift percentage metrics
///
/// GET /api/v2/analytics/drift/percentage
pub async fn get_drift_percentage(
    axum::extract::Extension(state): axum::extract::Extension<CoverageMetricsState>,
    Query(params): Query<CoverageQuery>,
) -> Result<Json<ApiResponse<Vec<DriftPercentageMetrics>>>, StatusCode> {
    debug!("Getting drift percentage metrics");

    let db = state.get_db().await?;
    match db
        .get_drift_percentage(
            params.workspace_id.as_deref(),
            params.org_id.as_deref(),
            params.limit,
        )
        .await
    {
        Ok(metrics) => Ok(Json(ApiResponse::success(metrics))),
        Err(e) => {
            error!("Failed to get drift percentage metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create coverage metrics router
/// Note: Returns a router without state - handlers will get state from extensions
pub fn coverage_metrics_router() -> axum::Router {
    use axum::routing::get;

    axum::Router::new()
        .route("/scenarios/usage", get(get_scenario_usage))
        .route("/personas/ci-hits", get(get_persona_ci_hits))
        .route("/endpoints/coverage", get(get_endpoint_coverage))
        .route("/reality-levels/staleness", get(get_reality_level_staleness))
        .route("/drift/percentage", get(get_drift_percentage))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== CoverageQuery Tests ====================

    #[test]
    fn test_coverage_query_empty() {
        let query = CoverageQuery {
            workspace_id: None,
            org_id: None,
            limit: None,
            min_coverage: None,
            max_staleness_days: None,
        };

        assert!(query.workspace_id.is_none());
        assert!(query.org_id.is_none());
        assert!(query.limit.is_none());
    }

    #[test]
    fn test_coverage_query_with_workspace() {
        let query = CoverageQuery {
            workspace_id: Some("ws-123".to_string()),
            org_id: None,
            limit: None,
            min_coverage: None,
            max_staleness_days: None,
        };

        assert_eq!(query.workspace_id, Some("ws-123".to_string()));
    }

    #[test]
    fn test_coverage_query_with_org() {
        let query = CoverageQuery {
            workspace_id: None,
            org_id: Some("org-456".to_string()),
            limit: None,
            min_coverage: None,
            max_staleness_days: None,
        };

        assert_eq!(query.org_id, Some("org-456".to_string()));
    }

    #[test]
    fn test_coverage_query_with_limit() {
        let query = CoverageQuery {
            workspace_id: None,
            org_id: None,
            limit: Some(100),
            min_coverage: None,
            max_staleness_days: None,
        };

        assert_eq!(query.limit, Some(100));
    }

    #[test]
    fn test_coverage_query_with_min_coverage() {
        let query = CoverageQuery {
            workspace_id: None,
            org_id: None,
            limit: None,
            min_coverage: Some(0.75),
            max_staleness_days: None,
        };

        assert_eq!(query.min_coverage, Some(0.75));
    }

    #[test]
    fn test_coverage_query_with_max_staleness() {
        let query = CoverageQuery {
            workspace_id: None,
            org_id: None,
            limit: None,
            min_coverage: None,
            max_staleness_days: Some(30),
        };

        assert_eq!(query.max_staleness_days, Some(30));
    }

    #[test]
    fn test_coverage_query_full() {
        let query = CoverageQuery {
            workspace_id: Some("ws-full".to_string()),
            org_id: Some("org-full".to_string()),
            limit: Some(50),
            min_coverage: Some(0.80),
            max_staleness_days: Some(14),
        };

        assert_eq!(query.workspace_id, Some("ws-full".to_string()));
        assert_eq!(query.org_id, Some("org-full".to_string()));
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.min_coverage, Some(0.80));
        assert_eq!(query.max_staleness_days, Some(14));
    }

    #[test]
    fn test_coverage_query_deserialization() {
        let json = r#"{
            "workspace_id": "ws-deser",
            "org_id": "org-deser",
            "limit": 25,
            "min_coverage": 0.5,
            "max_staleness_days": 7
        }"#;

        let query: CoverageQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.workspace_id, Some("ws-deser".to_string()));
        assert_eq!(query.org_id, Some("org-deser".to_string()));
        assert_eq!(query.limit, Some(25));
        assert_eq!(query.min_coverage, Some(0.5));
        assert_eq!(query.max_staleness_days, Some(7));
    }

    #[test]
    fn test_coverage_query_partial_deserialization() {
        let json = r#"{
            "workspace_id": "ws-partial"
        }"#;

        let query: CoverageQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.workspace_id, Some("ws-partial".to_string()));
        assert!(query.org_id.is_none());
        assert!(query.limit.is_none());
    }

    #[test]
    fn test_coverage_query_empty_json() {
        let json = r#"{}"#;

        let query: CoverageQuery = serde_json::from_str(json).unwrap();
        assert!(query.workspace_id.is_none());
        assert!(query.org_id.is_none());
        assert!(query.limit.is_none());
        assert!(query.min_coverage.is_none());
        assert!(query.max_staleness_days.is_none());
    }

    #[test]
    fn test_coverage_query_debug() {
        let query = CoverageQuery {
            workspace_id: Some("debug-ws".to_string()),
            org_id: None,
            limit: Some(10),
            min_coverage: None,
            max_staleness_days: None,
        };

        let debug = format!("{:?}", query);
        assert!(debug.contains("debug-ws"));
        assert!(debug.contains("10"));
    }

    // ==================== CoverageMetricsState Tests ====================

    #[test]
    fn test_coverage_metrics_state_clone() {
        // CoverageMetricsState is Clone, verify it compiles
        // We can't easily test the actual clone without a real database
        // but we verify the trait is implemented
        fn assert_clone<T: Clone>() {}
        assert_clone::<CoverageMetricsState>();
    }

    // ==================== Router Tests ====================

    #[test]
    fn test_coverage_metrics_router_creation() {
        // Verify router can be created
        let router = coverage_metrics_router();
        // Router is created successfully - this is a compile-time check
        let _ = router;
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_coverage_query_zero_limit() {
        let query = CoverageQuery {
            workspace_id: None,
            org_id: None,
            limit: Some(0),
            min_coverage: None,
            max_staleness_days: None,
        };

        assert_eq!(query.limit, Some(0));
    }

    #[test]
    fn test_coverage_query_negative_limit() {
        let query = CoverageQuery {
            workspace_id: None,
            org_id: None,
            limit: Some(-1),
            min_coverage: None,
            max_staleness_days: None,
        };

        assert_eq!(query.limit, Some(-1));
    }

    #[test]
    fn test_coverage_query_zero_coverage() {
        let query = CoverageQuery {
            workspace_id: None,
            org_id: None,
            limit: None,
            min_coverage: Some(0.0),
            max_staleness_days: None,
        };

        assert_eq!(query.min_coverage, Some(0.0));
    }

    #[test]
    fn test_coverage_query_full_coverage() {
        let query = CoverageQuery {
            workspace_id: None,
            org_id: None,
            limit: None,
            min_coverage: Some(1.0),
            max_staleness_days: None,
        };

        assert_eq!(query.min_coverage, Some(1.0));
    }

    #[test]
    fn test_coverage_query_over_100_coverage() {
        // Edge case: coverage > 100% (shouldn't happen but test handling)
        let query = CoverageQuery {
            workspace_id: None,
            org_id: None,
            limit: None,
            min_coverage: Some(1.5),
            max_staleness_days: None,
        };

        assert_eq!(query.min_coverage, Some(1.5));
    }

    #[test]
    fn test_coverage_query_negative_staleness() {
        let query = CoverageQuery {
            workspace_id: None,
            org_id: None,
            limit: None,
            min_coverage: None,
            max_staleness_days: Some(-7),
        };

        assert_eq!(query.max_staleness_days, Some(-7));
    }

    #[test]
    fn test_coverage_query_large_values() {
        let query = CoverageQuery {
            workspace_id: Some("very-long-workspace-id-123456789".to_string()),
            org_id: Some("very-long-org-id-987654321".to_string()),
            limit: Some(i64::MAX),
            min_coverage: Some(f64::MAX),
            max_staleness_days: Some(i32::MAX),
        };

        assert!(query.workspace_id.is_some());
        assert_eq!(query.limit, Some(i64::MAX));
    }

    #[test]
    fn test_coverage_query_special_characters() {
        let query = CoverageQuery {
            workspace_id: Some("ws-special-!@#$%".to_string()),
            org_id: Some("org/with/slashes".to_string()),
            limit: None,
            min_coverage: None,
            max_staleness_days: None,
        };

        assert_eq!(query.workspace_id, Some("ws-special-!@#$%".to_string()));
        assert_eq!(query.org_id, Some("org/with/slashes".to_string()));
    }

    #[test]
    fn test_coverage_query_unicode() {
        let query = CoverageQuery {
            workspace_id: Some("workspace-日本語".to_string()),
            org_id: Some("org-中文".to_string()),
            limit: None,
            min_coverage: None,
            max_staleness_days: None,
        };

        assert_eq!(query.workspace_id, Some("workspace-日本語".to_string()));
        assert_eq!(query.org_id, Some("org-中文".to_string()));
    }
}
