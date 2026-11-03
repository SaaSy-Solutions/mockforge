//! Adversarial tests for admin authentication (#1019).
//!
//! The route table is read from the router source, so a route added to
//! `routes.rs` or `overrides_api.rs` is covered without editing this file:
//! it must answer 401 to an anonymous request unless it is on the explicit
//! public list.

use std::collections::BTreeSet;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::{Extension, Router};
use tower::ServiceExt;

use crate::admin_access::{AdminAccessPolicy, MANAGEMENT_TOKEN_HEADER};
use crate::rbac::is_public_route;

const MANAGEMENT_TOKEN: &str = "mfm_test_management_token_0123456789abcdef";
const TEST_JWT_SECRET: &[u8] = b"test-jwt-secret-which-is-long-enough";

/// The routes the server answers without credentials
const PUBLIC_ROUTES: &[&str] = &[
    "/",
    "/assets/index.css",
    "/assets/index.js",
    "/assets/x",
    "/api-docs",
    "/mockforge-icon.png",
    "/mockforge-icon-32.png",
    "/mockforge-icon-48.png",
    "/mockforge-logo.png",
    "/mockforge-logo-40.png",
    "/mockforge-logo-80.png",
    "/manifest.json",
    "/sw.js",
    "/__mockforge/auth/login",
    "/__mockforge/auth/refresh",
    "/__mockforge/auth/logout",
    "/__mockforge/health",
    "/health/live",
    "/livez",
];

fn admin_router() -> Router {
    let router = crate::routes::create_admin_router(
        None,
        None,
        None,
        None,
        true,
        9080,
        "http://localhost:9090".to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    crate::merge_overrides_router(router, mockforge_core::SharedOverrides::default())
}

fn with_policy(policy: AdminAccessPolicy) -> Router {
    admin_router().layer(Extension(policy))
}

/// Every `(method, path)` declared with `.route(...)` in the router sources,
/// plus one path under each `nest_service` prefix.
fn declared_routes() -> BTreeSet<(String, String)> {
    let sources = [include_str!("routes.rs"), include_str!("overrides_api.rs")];
    let mut routes = BTreeSet::new();
    for source in sources {
        // Stop at the test module: its literals are requests, not routes.
        let source = source.split("#[cfg(test)]\nmod tests").next().unwrap_or(source);
        for (i, _) in source.match_indices(".route(") {
            let rest = &source[i + ".route(".len()..];
            let Some(open) = rest.find('"') else { continue };
            let Some(close) = rest[open + 1..].find('"') else {
                continue;
            };
            let path = &rest[open + 1..open + 1 + close];
            if !path.starts_with('/') {
                continue;
            }
            let concrete = concretize(path);
            let handlers = &rest[open + 1 + close..];
            let end = handlers.find(".route(").unwrap_or(handlers.len());
            let handlers = &handlers[..end];
            for (needle, method) in [
                ("get(", "GET"),
                ("post(", "POST"),
                ("put(", "PUT"),
                ("delete(", "DELETE"),
                ("patch(", "PATCH"),
            ] {
                if handlers.contains(needle) {
                    routes.insert((method.to_string(), concrete.clone()));
                }
            }
        }
        for (i, _) in source.match_indices(".nest_service(") {
            let rest = &source[i..];
            let Some(open) = rest.find('"') else { continue };
            let Some(close) = rest[open + 1..].find('"') else {
                continue;
            };
            let prefix = &rest[open + 1..open + 1 + close];
            routes.insert(("GET".to_string(), format!("{prefix}/x")));
            routes.insert(("POST".to_string(), format!("{prefix}/x")));
        }
    }
    routes
}

/// `/a/{id}/b` -> `/a/x/b`, `/{*path}` -> `/x`
fn concretize(path: &str) -> String {
    path.split('/')
        .map(|seg| if seg.starts_with('{') { "x" } else { seg })
        .collect::<Vec<_>>()
        .join("/")
}

async fn status(router: &Router, request: Request<Body>) -> StatusCode {
    router.clone().oneshot(request).await.expect("router is infallible").status()
}

fn request(method: &str, uri: &str) -> axum::http::request::Builder {
    Request::builder().method(method).uri(uri).header("host", "localhost:9080")
}

fn anonymous(method: &str, uri: &str) -> Request<Body> {
    let body = if method == "GET" {
        Body::empty()
    } else {
        Body::from("{}")
    };
    request(method, uri)
        .header("content-type", "application/json")
        .body(body)
        .expect("valid")
}

fn jwt(role: &str, secret: &[u8], exp_offset: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = crate::auth::Claims {
        sub: "u-1".to_string(),
        username: "u".to_string(),
        role: role.to_string(),
        email: None,
        iat: now,
        exp: now + exp_offset,
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret),
    )
    .expect("test claims encode")
}

mod credentials;
mod redaction;
mod request_boundaries;
mod route_access;
