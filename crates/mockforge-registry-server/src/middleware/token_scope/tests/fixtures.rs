//! Shared owned principals and an Axum router that exercises real scope middleware.
use super::super::token_scope_middleware;
use crate::{middleware::AuthType, models::ApiToken};
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, Method, StatusCode},
    middleware::{self, Next},
    routing::{delete, get, patch, post},
    Router,
};
use tower::ServiceExt;
use uuid::Uuid;

pub(super) const OK_HANDLER_TAG: &str = "handler-ran";

pub(super) fn org_a() -> Uuid {
    Uuid::from_u128(0xA)
}
pub(super) fn org_b() -> Uuid {
    Uuid::from_u128(0xB)
}

pub(super) fn token(org_id: Uuid, scopes: &[&str]) -> ApiToken {
    let now = chrono::Utc::now();
    ApiToken {
        id: Uuid::new_v4(),
        org_id,
        user_id: Some(Uuid::from_u128(0x1)),
        name: "t".into(),
        token_prefix: "mfx_test1234".into(),
        hashed_token: String::new(),
        scopes: scopes.iter().map(|s| s.to_string()).collect(),
        last_used_at: None,
        expires_at: None,
        created_at: now,
        updated_at: now,
    }
}

#[derive(Clone)]
pub(super) enum Caller {
    Token(ApiToken),
    Session,
}

async fn echo_org(headers: HeaderMap) -> String {
    format!(
        "{OK_HANDLER_TAG} org={}",
        headers.get("X-Organization-Id").and_then(|v| v.to_str().ok()).unwrap_or("-")
    )
}

/// A router with real route templates from `routes.rs`, the scope layer,
/// and a stand-in for `auth_middleware` that attaches `caller`.
fn app(caller: Caller) -> Router {
    let fake_auth = move |mut req: Request, next: Next| {
        let caller = caller.clone();
        async move {
            match caller {
                Caller::Token(t) => {
                    req.extensions_mut().insert(AuthType::ApiToken);
                    req.extensions_mut().insert(t);
                }
                Caller::Session => {
                    req.extensions_mut().insert(AuthType::Jwt);
                }
            }
            next.run(req).await
        }
    };
    Router::new()
        .route("/api/v1/hosted-mocks", post(echo_org).get(echo_org))
        .route("/api/v1/hosted-mocks/{deployment_id}", delete(echo_org))
        .route("/api/v1/hosted-mocks/{deployment_id}/status", patch(echo_org))
        .route("/api/v1/hosted-mocks/{deployment_id}/redeploy", post(echo_org))
        .route("/api/v1/tokens", post(echo_org).get(echo_org))
        .route("/api/v1/tokens/{token_id}", delete(echo_org))
        .route("/api/v1/test-suites/{id}/runs", post(echo_org))
        .route("/api/v1/organizations/{org_id}/members", post(echo_org))
        .route("/api/v1/organizations/{org_id}/tunnels", get(echo_org))
        .route("/api/v1/plugins/publish", post(echo_org))
        .route("/api/v1/auth/me", get(echo_org))
        .route("/api/v1/brand-new-feature", post(echo_org))
        .route("/api/v1/hosted-mocks/new-operation", post(echo_org))
        .route_layer(middleware::from_fn(token_scope_middleware))
        .route_layer(middleware::from_fn(fake_auth))
}

pub(super) async fn call(
    caller: Caller,
    method: Method,
    uri: &str,
    headers: &[(&str, &str)],
) -> (StatusCode, String) {
    let mut builder = axum::http::Request::builder().method(method).uri(uri);
    for (k, v) in headers {
        builder = builder.header(*k, *v);
    }
    let req = builder.body(Body::empty()).expect("valid request");
    let res = app(caller).oneshot(req).await.expect("infallible router");
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), 1 << 20).await.expect("body");
    (status, String::from_utf8_lossy(&body).into_owned())
}

pub(super) fn dep() -> String {
    Uuid::from_u128(0xD).to_string()
}

/// (method, uri) of the mutating cloud routes the report called out.
pub(super) fn mock_and_admin_ops() -> Vec<(Method, String)> {
    vec![
        (Method::POST, "/api/v1/hosted-mocks".into()),
        (Method::PATCH, format!("/api/v1/hosted-mocks/{}/status", dep())),
        (Method::DELETE, format!("/api/v1/hosted-mocks/{}", dep())),
        (Method::POST, format!("/api/v1/hosted-mocks/{}/redeploy", dep())),
        (Method::POST, format!("/api/v1/test-suites/{}/runs", dep())),
        (Method::POST, "/api/v1/tokens".into()),
        (Method::GET, "/api/v1/tokens".into()),
        (Method::DELETE, format!("/api/v1/tokens/{}", dep())),
        (Method::POST, format!("/api/v1/organizations/{}/members", org_a())),
    ]
}
