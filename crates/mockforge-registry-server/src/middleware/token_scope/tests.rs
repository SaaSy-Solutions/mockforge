use super::organization::org_id_path_params;
use super::policy::{ADMIN_ORG, BILLING, DEPLOY, PUBLISH_PKG, READ_USAGE};
use super::TokenAccess::{AnyToken, SessionOnly};
use super::*;
use axum::{
    body::Body,
    http::HeaderMap,
    middleware,
    routing::{delete, get, patch, post},
    Router,
};
use std::collections::BTreeMap;
use tower::ServiceExt;
use uuid::Uuid;

const ROUTES_RS: &str = include_str!("../../routes.rs");
const MANIFEST: &str = include_str!("../token_scope_routes.txt");

/// Extract `(METHOD, path)` for every `.route(...)` between `start` and
/// `end` markers of `routes.rs`.
fn parse_routes(start: &str, end: &str) -> Vec<(String, String)> {
    let from = ROUTES_RS.find(start).expect("start marker present in routes.rs");
    let to = from + ROUTES_RS[from..].find(end).expect("end marker present in routes.rs");
    let body: String = ROUTES_RS[from..to]
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let bytes = body.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(rel) = body[i..].find(".route(") {
        let open = i + rel + ".route(".len();
        let mut depth = 1;
        let mut p = open;
        while depth > 0 {
            match bytes[p] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            p += 1;
        }
        let inner = &body[open..p - 1];
        let q1 = inner.find('"').expect("route path literal");
        let q2 = q1 + 1 + inner[q1 + 1..].find('"').expect("route path literal end");
        let path = &inner[q1 + 1..q2];
        let rest = &inner[q2 + 1..];
        for m in ["get", "post", "put", "patch", "delete"] {
            let needle = format!("{m}(");
            let mut j = 0;
            while let Some(k) = rest[j..].find(&needle) {
                let at = j + k;
                let prev = rest[..at].chars().last().unwrap_or(' ');
                if !(prev.is_alphanumeric() || prev == '_') {
                    out.push((m.to_uppercase(), path.to_string()));
                }
                j = at + needle.len();
            }
        }
        i = p;
    }
    out
}

fn authenticated_routes() -> Vec<(String, String)> {
    let mut routes = parse_routes("let auth_routes = Router::new()", "let sso_public_routes");
    routes.extend(parse_routes("let admin_routes = Router::new()", "// Combine all routes"));
    routes
}

fn manifest() -> BTreeMap<(String, String), String> {
    MANIFEST
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(parts.len(), 3, "manifest line must be 'METHOD PATH ACCESS': {l}");
            ((parts[0].to_string(), parts[1].to_string()), parts[2].to_string())
        })
        .collect()
}

/// Every authenticated route must be in the manifest with the access the
/// policy table actually grants, and the manifest must list nothing that
/// is not a route. Adding a route without declaring its token access
/// fails here (and the route rejects tokens at runtime until declared).
#[test]
fn every_authenticated_route_is_classified() {
    let routes = authenticated_routes();
    assert!(routes.len() >= 360, "route parser found only {} routes", routes.len());
    let manifest = manifest();
    let mut problems = Vec::new();
    for (method, path) in &routes {
        let m = Method::from_bytes(method.as_bytes()).expect("valid method");
        let actual = classify(&m, path).map(|a| a.manifest_name());
        match manifest.get(&(method.clone(), path.clone())) {
            None => problems.push(format!(
                "{method} {path}: not in token_scope_routes.txt (policy says {actual:?})"
            )),
            Some(declared) if actual.as_deref() != Some(declared.as_str()) => problems
                .push(format!("{method} {path}: manifest says {declared}, policy says {actual:?}")),
            Some(_) => {}
        }
    }
    let route_set: std::collections::BTreeSet<_> = routes.iter().cloned().collect();
    for key in manifest.keys() {
        if !route_set.contains(key) {
            problems.push(format!("{} {}: in manifest but not a route", key.0, key.1));
        }
    }
    assert!(problems.is_empty(), "token scope policy drift:\n{}", problems.join("\n"));
}

/// The sensitive families named in the security report map to the scope
/// they need, never to something a publish/read token holds.
#[test]
fn sensitive_routes_map_to_expected_access() {
    let cases: &[(&str, &str, TokenAccess)] = &[
        ("POST", "/api/v1/hosted-mocks", DEPLOY),
        ("PATCH", "/api/v1/hosted-mocks/{deployment_id}/status", DEPLOY),
        ("DELETE", "/api/v1/hosted-mocks/{deployment_id}", DEPLOY),
        ("POST", "/api/v1/hosted-mocks/{deployment_id}/redeploy", DEPLOY),
        ("GET", "/api/v1/hosted-mocks/{deployment_id}/captures/export/har", DEPLOY),
        ("POST", "/api/v1/test-suites/{id}/runs", DEPLOY),
        ("POST", "/api/v1/capture-sessions/{id}/replay", DEPLOY),
        ("POST", "/api/v1/tokens", SessionOnly),
        ("DELETE", "/api/v1/tokens/{token_id}", SessionOnly),
        ("POST", "/api/v1/tokens/{token_id}/rotate", SessionOnly),
        ("POST", "/api/v1/auth/change-password", SessionOnly),
        ("POST", "/api/v1/auth/2fa/disable", SessionOnly),
        ("DELETE", "/api/v1/gdpr/erase", SessionOnly),
        ("POST", "/api/v1/sso/config", SessionOnly),
        ("DELETE", "/api/v1/organizations/{org_id}", SessionOnly),
        ("POST", "/api/v1/organizations/{org_id}/members", ADMIN_ORG),
        ("POST", "/api/v1/billing/checkout", BILLING),
        ("GET", "/api/v1/usage", READ_USAGE),
        ("POST", "/api/v1/plugins/publish", PUBLISH_PKG),
        ("POST", "/api/v1/admin/plugins/{name}/takedown", SessionOnly),
        ("GET", "/api/v1/auth/me", AnyToken),
    ];
    for (method, path, expected) in cases {
        let m = Method::from_bytes(method.as_bytes()).expect("valid method");
        assert_eq!(classify(&m, path).as_ref(), Some(expected), "{method} {path}");
    }
}

#[test]
fn unknown_routes_are_unclassified() {
    assert_eq!(classify(&Method::POST, "/api/v1/brand-new-feature"), None);
    assert_eq!(classify(&Method::GET, "/api/v1/hosted-mocksX"), None);
    assert_eq!(classify(&Method::GET, "/api/v1/tokens-extra"), None);
    assert_eq!(classify(&Method::POST, "/api/v1/hosted-mocks/new-operation"), None);
    assert_eq!(classify(&Method::GET, "/api/v1/usage/new-operation"), None);
}

#[test]
fn automatic_head_requires_a_declared_get_route() {
    assert_eq!(classify(&Method::HEAD, "/api/v1/usage"), Some(READ_USAGE));
    assert_eq!(classify(&Method::HEAD, "/api/v1/usage/new-operation"), None);
}

// ---- Middleware behaviour -------------------------------------------

const OK_HANDLER_TAG: &str = "handler-ran";

fn org_a() -> Uuid {
    Uuid::from_u128(0xA)
}
fn org_b() -> Uuid {
    Uuid::from_u128(0xB)
}

fn token(org_id: Uuid, scopes: &[&str]) -> ApiToken {
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
enum Caller {
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

async fn call(
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

fn dep() -> String {
    Uuid::from_u128(0xD).to_string()
}

/// (method, uri) of the mutating cloud routes the report called out.
fn mock_and_admin_ops() -> Vec<(Method, String)> {
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

#[tokio::test]
async fn publish_or_read_token_cannot_touch_mocks_tokens_tests_or_members() {
    for scopes in [
        vec!["publish:packages"],
        vec!["read:packages"],
        vec![
            "read:packages",
            "publish:packages",
            "read:usage",
            "manage:billing",
        ],
        vec![],
    ] {
        let t = token(org_a(), &scopes);
        for (method, uri) in mock_and_admin_ops() {
            let (status, body) = call(Caller::Token(t.clone()), method.clone(), &uri, &[]).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{scopes:?} {method} {uri}: {body}");
            assert!(!body.contains(OK_HANDLER_TAG), "handler ran for {scopes:?} {method} {uri}");
        }
    }
}

#[tokio::test]
async fn missing_scope_error_names_the_required_scope() {
    let t = token(org_a(), &["publish:packages"]);
    let (status, body) = call(Caller::Token(t), Method::POST, "/api/v1/hosted-mocks", &[]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body.contains("INSUFFICIENT_SCOPE"), "{body}");
    assert!(body.contains("deploy:mocks"), "{body}");
}

#[tokio::test]
async fn scope_string_variants_do_not_grant_access() {
    for variant in [
        "DEPLOY:MOCKS",
        "Deploy:Mocks",
        " deploy:mocks",
        "deploy:mocks ",
        "deploy: mocks",
        "deploy:mocks\n",
        "deploy:*",
        "*",
        "deploy_mocks",
        "deploymocks",
        "admin:org",
    ] {
        let t = token(org_a(), &[variant]);
        let (status, body) =
            call(Caller::Token(t), Method::POST, "/api/v1/hosted-mocks", &[]).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "variant {variant:?} granted access: {body}");
    }
}

#[tokio::test]
async fn correct_scope_still_works_and_pins_org() {
    let t = token(org_a(), &["deploy:mocks"]);
    for (method, uri) in [
        (Method::POST, "/api/v1/hosted-mocks".to_string()),
        (Method::GET, "/api/v1/hosted-mocks".to_string()),
        (Method::PATCH, format!("/api/v1/hosted-mocks/{}/status", dep())),
        (Method::DELETE, format!("/api/v1/hosted-mocks/{}", dep())),
        (Method::POST, format!("/api/v1/hosted-mocks/{}/redeploy", dep())),
        (Method::POST, format!("/api/v1/test-suites/{}/runs", dep())),
        (Method::GET, format!("/api/v1/organizations/{}/tunnels", org_a())),
    ] {
        let (status, body) = call(Caller::Token(t.clone()), method.clone(), &uri, &[]).await;
        assert_eq!(status, StatusCode::OK, "{method} {uri}: {body}");
        assert_eq!(body, format!("{OK_HANDLER_TAG} org={}", org_a()));
    }

    let publisher = token(org_a(), &["publish:packages"]);
    let (status, _) =
        call(Caller::Token(publisher), Method::POST, "/api/v1/plugins/publish", &[]).await;
    assert_eq!(status, StatusCode::OK);

    let admin = token(org_a(), &["admin:org"]);
    let uri = format!("/api/v1/organizations/{}/members", org_a());
    let (status, _) = call(Caller::Token(admin), Method::POST, &uri, &[]).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn token_management_is_session_only_even_with_every_scope() {
    let all = token(
        org_a(),
        &[
            "read:packages",
            "publish:packages",
            "deploy:mocks",
            "admin:org",
            "read:usage",
            "manage:billing",
        ],
    );
    for (method, uri) in [
        (Method::POST, "/api/v1/tokens".to_string()),
        (Method::GET, "/api/v1/tokens".to_string()),
        (Method::DELETE, format!("/api/v1/tokens/{}", dep())),
    ] {
        let (status, body) = call(Caller::Token(all.clone()), method.clone(), &uri, &[]).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}");
        assert!(body.contains("TOKEN_NOT_ALLOWED"), "{body}");
    }
}

#[tokio::test]
async fn undeclared_route_rejects_tokens_but_not_sessions() {
    let all = token(org_a(), &["deploy:mocks", "admin:org"]);
    for uri in [
        "/api/v1/brand-new-feature",
        "/api/v1/hosted-mocks/new-operation",
    ] {
        let (status, body) = call(Caller::Token(all.clone()), Method::POST, uri, &[]).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert!(body.contains("TOKEN_NOT_ALLOWED"));

        let (status, _) = call(Caller::Session, Method::POST, uri, &[]).await;
        assert_eq!(status, StatusCode::OK);
    }
}

#[tokio::test]
async fn session_auth_is_unchanged() {
    for (method, uri) in mock_and_admin_ops() {
        let (status, body) = call(Caller::Session, method.clone(), &uri, &[]).await;
        assert_eq!(status, StatusCode::OK, "{method} {uri}: {body}");
    }
    // Session requests keep their own org selection.
    let other = org_b().to_string();
    let (status, body) = call(
        Caller::Session,
        Method::POST,
        "/api/v1/hosted-mocks",
        &[("X-Organization-Id", &other)],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, format!("{OK_HANDLER_TAG} org={other}"));
}

#[tokio::test]
async fn token_cannot_act_on_another_org() {
    let t = token(org_a(), &["deploy:mocks", "admin:org"]);
    let other = org_b().to_string();

    // Another org in the path.
    for uri in [
        format!("/api/v1/organizations/{other}/members"),
        format!("/api/v1/organizations/{other}/tunnels"),
        "/api/v1/organizations/not-a-uuid/tunnels".to_string(),
    ] {
        let method = if uri.ends_with("members") {
            Method::POST
        } else {
            Method::GET
        };
        let (status, body) = call(Caller::Token(t.clone()), method, &uri, &[]).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}: {body}");
        assert!(body.contains("TOKEN_ORG_MISMATCH"), "{body}");
    }

    // Another org via header, including case/whitespace/garbage variants.
    let upper = other.to_uppercase();
    for (name, value) in [
        ("X-Organization-Id", other.as_str()),
        ("x-organization-id", other.as_str()),
        ("X-Organization-Id", upper.as_str()),
        ("X-Organization-Id", "not-a-uuid"),
        ("X-Organization-Id", ""),
        ("X-Organization-Slug", "someone-else"),
        ("x-organization-slug", "my-own-org"),
    ] {
        let (status, body) = call(
            Caller::Token(t.clone()),
            Method::DELETE,
            &format!("/api/v1/hosted-mocks/{}", dep()),
            &[(name, value)],
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{name}: {value:?} -> {body}");
        assert!(!body.contains(OK_HANDLER_TAG));
    }

    // Matching header is accepted (and stays pinned).
    let own = org_a().to_string();
    let (status, body) = call(
        Caller::Token(t.clone()),
        Method::POST,
        "/api/v1/hosted-mocks",
        &[("X-Organization-Id", &own)],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, format!("{OK_HANDLER_TAG} org={own}"));

    // A token of org B gets org B pinned, never org A.
    let tb = token(org_b(), &["deploy:mocks"]);
    let (_, body) = call(Caller::Token(tb), Method::GET, "/api/v1/hosted-mocks", &[]).await;
    assert_eq!(body, format!("{OK_HANDLER_TAG} org={other}"));
}

#[tokio::test]
async fn empty_scope_token_gets_identity_only() {
    let t = token(org_a(), &[]);
    let (status, _) = call(Caller::Token(t.clone()), Method::GET, "/api/v1/auth/me", &[]).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(Caller::Token(t), Method::GET, "/api/v1/hosted-mocks", &[]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[test]
fn org_id_params_align_with_template() {
    assert_eq!(
        org_id_path_params(
            "/api/v1/organizations/{org_id}/members/{user_id}",
            "/api/v1/organizations/x/members/y"
        ),
        vec!["x"]
    );
    assert!(
        org_id_path_params("/api/v1/hosted-mocks/{deployment_id}", "/api/v1/hosted-mocks/x")
            .is_empty()
    );
}
