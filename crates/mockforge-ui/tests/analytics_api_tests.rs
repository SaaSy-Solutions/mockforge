/**
 * Integration tests for Analytics API V2
 */

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use mockforge_analytics::AnalyticsDatabase;
    use mockforge_ui::handlers::analytics_v2::*;
    use std::path::PathBuf;
    use tower::ServiceExt; // for `oneshot`

    async fn setup_test_db() -> AnalyticsDatabase {
        let db = AnalyticsDatabase::new(&PathBuf::from(":memory:"))
            .await
            .expect("Failed to create test database");
        db.run_migrations().await.expect("Failed to run migrations");
        db
    }

    fn create_test_router(db: AnalyticsDatabase) -> Router {
        let state = AnalyticsV2State::new(db);

        Router::new()
            .route("/overview", axum::routing::get(get_overview))
            .route("/requests", axum::routing::get(get_requests_timeseries))
            .route("/latency", axum::routing::get(get_latency_trends))
            .route("/errors", axum::routing::get(get_error_summary))
            .route("/endpoints", axum::routing::get(get_top_endpoints))
            .route("/protocols", axum::routing::get(get_protocol_breakdown))
            .route("/export/csv", axum::routing::get(export_csv))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_overview_endpoint() {
        let db = setup_test_db().await;
        let app = create_test_router(db);

        let response = app
            .oneshot(Request::builder().uri("/overview?duration=3600").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // Parse JSON response
        let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(json["success"], true);
        assert!(json["data"].is_object());
    }

    #[tokio::test]
    async fn test_requests_timeseries_endpoint() {
        let db = setup_test_db().await;
        let app = create_test_router(db);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/requests?duration=3600&granularity=minute")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(json["success"], true);
        assert!(json["data"]["series"].is_array());
    }

    #[tokio::test]
    async fn test_latency_trends_endpoint() {
        let db = setup_test_db().await;
        let app = create_test_router(db);

        let response = app
            .oneshot(Request::builder().uri("/latency?duration=3600").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_errors_endpoint() {
        let db = setup_test_db().await;
        let app = create_test_router(db);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/errors?duration=3600&limit=20")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(json["success"], true);
        assert!(json["data"]["errors"].is_array());
    }

    #[tokio::test]
    async fn test_endpoints_endpoint() {
        let db = setup_test_db().await;
        let app = create_test_router(db);

        let response = app
            .oneshot(Request::builder().uri("/endpoints?limit=10").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(json["success"], true);
        assert!(json["data"]["endpoints"].is_array());
    }

    #[tokio::test]
    async fn test_protocols_endpoint() {
        let db = setup_test_db().await;
        let app = create_test_router(db);

        let response = app
            .oneshot(Request::builder().uri("/protocols").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(json["success"], true);
        assert!(json["data"]["protocols"].is_array());
    }

    #[tokio::test]
    async fn test_csv_export() {
        let db = setup_test_db().await;
        let app = create_test_router(db);

        let response = app
            .oneshot(
                Request::builder().uri("/export/csv?duration=3600").body(Body::empty()).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let csv_data = String::from_utf8(body.to_vec()).unwrap();

        // Check CSV header
        assert!(csv_data.contains("timestamp,protocol"));
    }

    #[tokio::test]
    async fn test_filter_parameters() {
        let db = setup_test_db().await;
        let app = create_test_router(db);

        // Test with multiple filter parameters
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/overview?duration=7200&protocol=HTTP&environment=prod")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
