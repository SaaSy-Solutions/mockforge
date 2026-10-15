//! Regression checks for explicit mock routes that overlap built-in health probes.

use axum::{body::Body, extract::ConnectInfo, http::Request, Router};
use mockforge_core::{config::RouteConfig, SharedOverrides};
use mockforge_http::{build_router_with_chains_and_multi_tenant, HealthManager};
use mockforge_openapi::openapi_routes::ValidationOptions;
use serde_json::{json, Value};
use std::{error::Error, net::SocketAddr, sync::Arc};
use tempfile::NamedTempFile;
use tower::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn specification(paths: &[(&str, &str)]) -> TestResult<NamedTempFile> {
    let mut operations = serde_json::Map::new();
    for (path, method) in paths {
        operations.insert(
            (*path).to_string(),
            json!({
                (*method): {
                    "responses": {"200": {
                        "description": "Mock health response",
                        "content": {"application/json": {
                            "schema": {"type": "object", "properties": {"marker": {"type": "string"}}},
                            "example": {"marker": path}
                        }}
                    }}
                }
            }),
        );
    }
    let mut file = NamedTempFile::new()?;
    serde_json::to_writer(
        file.as_file_mut(),
        &json!({
            "openapi": "3.0.3",
            "info": {"title": "Health route precedence", "version": "1.0.0"},
            "paths": operations
        }),
    )?;
    Ok(file)
}

async fn router(
    spec: Option<String>,
    routes: Option<Vec<RouteConfig>>,
    health: Option<Arc<HealthManager>>,
) -> Router {
    build_router_with_chains_and_multi_tenant(
        spec,
        Some(ValidationOptions::default()),
        None,
        None,
        routes,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        health,
        None,
        None,
        None,
        SharedOverrides::load(Vec::new()).await,
    )
    .await
}

async fn response(app: Router, method: &str, path: &str) -> TestResult<Value> {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345))))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), 65536).await?;
    Ok(serde_json::from_slice(&body)?)
}

#[tokio::test]
async fn openapi_health_route_keeps_its_response_and_other_probes_still_work() -> TestResult {
    let spec = specification(&[("/health", "get")])?;
    let manager = Arc::new(HealthManager::new());
    manager.set_ready().await;
    let app = router(Some(spec.path().to_string_lossy().into_owned()), None, Some(manager)).await;
    assert_eq!(response(app.clone(), "GET", "/health").await?["marker"], "/health");
    for path in ["/health/live", "/health/ready", "/health/startup"] {
        assert!(response(app.clone(), "GET", path).await?.get("marker").is_none());
    }
    Ok(())
}

#[tokio::test]
async fn all_explicit_openapi_health_probes_keep_their_mock_responses() -> TestResult {
    let paths = [
        "/health",
        "/health/live",
        "/health/ready",
        "/health/startup",
    ];
    let operations = paths.map(|path| (path, "get"));
    let spec = specification(&operations)?;
    let manager = Arc::new(HealthManager::new());
    manager.set_failed("Fixture health manager is deliberately unavailable").await;
    let app = router(Some(spec.path().to_string_lossy().into_owned()), None, Some(manager)).await;
    for path in paths {
        assert_eq!(response(app.clone(), "GET", path).await?["marker"], path);
    }
    Ok(())
}

#[tokio::test]
async fn configured_health_route_overrides_both_basic_and_managed_defaults() -> TestResult {
    for managed in [false, true] {
        let route: RouteConfig = serde_json::from_value(json!({
            "path": "/health", "method": "get",
            "response": {"status": 200, "body": {"marker": "configuration"}}
        }))?;
        let health = managed.then(|| Arc::new(HealthManager::new()));
        let app = router(None, Some(vec![route]), health).await;
        assert_eq!(response(app, "GET", "/health").await?["marker"], "configuration");
    }
    Ok(())
}

#[tokio::test]
async fn non_get_health_mock_does_not_remove_the_default_get_probe() -> TestResult {
    let spec = specification(&[("/health", "post")])?;
    let manager = Arc::new(HealthManager::new());
    manager.set_ready().await;
    let app = router(Some(spec.path().to_string_lossy().into_owned()), None, Some(manager)).await;
    assert_eq!(response(app.clone(), "POST", "/health").await?["marker"], "/health");
    assert!(response(app, "GET", "/health").await?.get("marker").is_none());
    Ok(())
}

#[tokio::test]
async fn configured_non_get_health_dispatcher_keeps_its_method_guard() -> TestResult {
    for managed in [false, true] {
        let route: RouteConfig = serde_json::from_value(json!({
            "path": "/health", "method": "POST",
            "response": {"status": 200, "body": {"marker": "configuration"}}
        }))?;
        let health = managed.then(|| Arc::new(HealthManager::new()));
        let app = router(None, Some(vec![route]), health).await;
        assert_eq!(response(app.clone(), "POST", "/health").await?["marker"], "configuration");
        let request = Request::builder()
            .uri("/health")
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345))))
            .body(Body::empty())?;
        assert_eq!(app.oneshot(request).await?.status(), 405);
    }
    Ok(())
}
