//! Override rules applied by the OpenAPI router, swapped at runtime.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use mockforge_core::{OverrideRule, Overrides, SharedOverrides};
use mockforge_openapi::openapi_routes::{OpenApiRouteRegistry, ValidationOptions};
use mockforge_openapi::OpenApiSpec;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn registry(overrides: &SharedOverrides) -> OpenApiRouteRegistry {
    let spec = OpenApiSpec::from_json(json!({
        "openapi": "3.0.0",
        "info": {"title": "Overrides", "version": "1"},
        "paths": {
            "/users/{id}": {
                "get": {
                    "operationId": "getUser",
                    "tags": ["Users"],
                    "parameters": [{"name": "id", "in": "path", "required": true, "schema": {"type": "string"}}],
                    "responses": {"200": {
                        "description": "ok",
                        "content": {"application/json": {"example": {"id": "1", "tier": "standard"}}}
                    }}
                }
            }
        }
    }))
    .unwrap();
    OpenApiRouteRegistry::new_with_options(spec, ValidationOptions::default())
        .with_overrides(Arc::new(overrides.clone()))
}

fn router(overrides: &SharedOverrides) -> Router {
    registry(overrides).build_router()
}

fn rules(value: Value) -> Overrides {
    let rules: Vec<OverrideRule> = serde_json::from_value(value).unwrap();
    Overrides::new(rules).unwrap()
}

async fn get_user(app: &Router, tier_header: Option<&str>) -> Value {
    let mut request = Request::builder().uri("/users/1");
    if let Some(tier) = tier_header {
        request = request.header("x-tier", tier);
    }
    let response = app.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn replacing_rules_changes_the_next_response_without_rebuilding_the_router() {
    let overrides = SharedOverrides::default();
    let app = router(&overrides);
    assert_eq!(get_user(&app, None).await["tier"], "standard");

    overrides.replace(rules(json!([{
        "targets": ["tag:Users"],
        "patch": [{"op": "replace", "path": "/tier", "value": "gold"}]
    }])));
    assert_eq!(get_user(&app, None).await["tier"], "gold");

    overrides.replace(Overrides::default());
    assert_eq!(get_user(&app, None).await["tier"], "standard");
}

#[tokio::test]
async fn when_conditions_see_the_request_and_disabled_rules_do_nothing() {
    let overrides = SharedOverrides::default();
    let app = router(&overrides);
    overrides.replace(rules(json!([
        {
            "targets": ["operation:getUser"],
            "when": "header[x-tier]=vip",
            "patch": [{"op": "replace", "path": "/tier", "value": "vip"}]
        },
        {
            "enabled": false,
            "targets": ["*"],
            "patch": [{"op": "add", "path": "/disabled", "value": true}]
        }
    ])));

    let standard = get_user(&app, None).await;
    assert_eq!(standard["tier"], "standard");
    assert!(standard.get("disabled").is_none());
    assert_eq!(get_user(&app, Some("vip")).await["tier"], "vip");
}

#[tokio::test]
async fn the_mockai_router_used_by_serve_applies_rules_too() {
    let overrides = SharedOverrides::default();
    let app = registry(&overrides).build_router_with_mockai(None);
    overrides.replace(rules(json!([{
        "targets": ["path:^/users/"],
        "when": "header[x-tier]=vip",
        "patch": [{"op": "replace", "path": "/tier", "value": "vip"}]
    }])));
    assert_eq!(get_user(&app, None).await["tier"], "standard");
    assert_eq!(get_user(&app, Some("vip")).await["tier"], "vip");
}
