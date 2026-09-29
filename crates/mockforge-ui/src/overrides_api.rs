//! Admin API for the running server's response override rules.
//!
//! `GET /__mockforge/overrides` returns the live rule set and
//! `PUT /__mockforge/overrides` replaces it. A PUT is validated as a whole;
//! a rejected set leaves the previous rules in place.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use mockforge_core::{OverrideRule, Overrides, SharedOverrides};
use serde::{Deserialize, Serialize};

/// Body of both the GET response and the PUT request
#[derive(Debug, Serialize, Deserialize)]
pub struct OverrideRules {
    /// Rules in application order
    pub rules: Vec<OverrideRule>,
}

async fn list_rules(State(overrides): State<SharedOverrides>) -> Json<OverrideRules> {
    Json(OverrideRules {
        rules: overrides.rules(),
    })
}

async fn replace_rules(
    State(overrides): State<SharedOverrides>,
    Json(body): Json<OverrideRules>,
) -> Result<Json<OverrideRules>, (StatusCode, Json<serde_json::Value>)> {
    let validated = Overrides::new(body.rules).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e.to_string() })))
    })?;
    overrides.replace(validated);
    Ok(Json(OverrideRules {
        rules: overrides.rules(),
    }))
}

/// Routes for reading and replacing the live override rules
pub fn overrides_router(overrides: SharedOverrides) -> Router {
    Router::new()
        .route("/__mockforge/overrides", get(list_rules).put(replace_rules))
        .with_state(overrides)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    async fn send(router: &Router, method: &str, body: Option<Value>) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(method)
            .uri("/__mockforge/overrides")
            .header("content-type", "application/json")
            .body(body.map_or(Body::empty(), |b| Body::from(b.to_string())))
            .unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    #[tokio::test]
    async fn put_replaces_rules_and_rejects_invalid_sets_atomically() {
        let shared = SharedOverrides::default();
        let router = overrides_router(shared.clone());

        let rule = json!({
            "name": "vip status",
            "targets": ["operation:getUser"],
            "patch": [{"op": "replace", "path": "/status", "value": "vip"}]
        });
        let (status, body) = send(&router, "PUT", Some(json!({"rules": [rule]}))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["rules"][0]["name"], "vip status");
        assert_eq!(body["rules"][0]["enabled"], true);

        let bad = json!({"rules": [{"targets": ["nonsense"], "patch": []}]});
        let (status, body) = send(&router, "PUT", Some(bad)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("invalid target"));

        let (status, body) = send(&router, "GET", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["rules"].as_array().unwrap().len(), 1);
        assert_eq!(shared.rules()[0].name.as_deref(), Some("vip status"));
    }
}
