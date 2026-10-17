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

#[test]
fn route_table_is_parsed() {
    let routes = declared_routes();
    // A parser regression must not turn the table test into a no-op.
    assert!(routes.len() > 200, "only {} routes parsed", routes.len());
    for expected in [
        ("GET", "/__mockforge/logs"),
        ("GET", "/__mockforge/config"),
        ("GET", "/__mockforge/env"),
        ("GET", "/__mockforge/recorder/status"),
        ("GET", "/__mockforge/overrides"),
        ("PUT", "/__mockforge/overrides"),
        ("GET", "/__mockforge/logs/sse"),
        ("GET", "/api/resilience/x"),
    ] {
        assert!(
            routes.contains(&(expected.0.to_string(), expected.1.to_string())),
            "{expected:?} missing from the parsed route table"
        );
    }
}

#[tokio::test]
async fn every_admin_route_rejects_anonymous_requests() {
    let router = with_policy(AdminAccessPolicy::default());
    let mut failures = Vec::new();
    let mut checked = 0usize;
    for (method, path) in declared_routes() {
        if PUBLIC_ROUTES.contains(&path.as_str()) {
            continue;
        }
        checked += 1;
        let got = status(&router, anonymous(&method, &path)).await;
        if got != StatusCode::UNAUTHORIZED {
            failures.push(format!("{method} {path} -> {got}"));
        }
    }
    assert!(checked > 200, "only {checked} routes checked");
    assert!(
        failures.is_empty(),
        "admin routes served without auth:\n{}",
        failures.join("\n")
    );
}

#[tokio::test]
async fn router_without_a_policy_fails_closed() {
    // Embedders and tests that skip `start_admin_server` get the strict default.
    let router = admin_router();
    for path in [
        "/__mockforge/logs",
        "/__mockforge/config",
        "/__mockforge/overrides",
    ] {
        assert_eq!(status(&router, anonymous("GET", path)).await, StatusCode::UNAUTHORIZED);
    }
}

#[test]
fn public_list_matches_the_middleware() {
    for path in PUBLIC_ROUTES {
        assert!(is_public_route(path), "{path} expected public");
    }
    for (_, path) in declared_routes() {
        if is_public_route(&path) {
            assert!(
                PUBLIC_ROUTES.contains(&path.as_str()),
                "{path} is public in rbac.rs but not reviewed in PUBLIC_ROUTES"
            );
        }
    }
}

#[tokio::test]
async fn public_routes_stay_reachable() {
    let router = with_policy(AdminAccessPolicy::default());
    for path in ["/", "/__mockforge/health", "/manifest.json"] {
        let got = status(&router, anonymous("GET", path)).await;
        assert_ne!(got, StatusCode::UNAUTHORIZED, "{path}");
    }
}

#[tokio::test]
async fn health_does_not_leak_config() {
    let router = with_policy(AdminAccessPolicy::default());
    for path in ["/__mockforge/health", "/health/live", "/livez"] {
        let response = router
            .clone()
            .oneshot(anonymous("GET", path))
            .await
            .expect("router is infallible");
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("body");
        let body = String::from_utf8_lossy(&bytes).to_lowercase();
        for leak in [
            "upstream", "secret", "token", "password", "jwt", "latency", "proxy", "addr",
        ] {
            assert!(!body.contains(leak), "{path} body mentions {leak}: {body}");
        }
    }
}

#[tokio::test]
async fn bad_credentials_are_rejected() {
    let router = with_policy(AdminAccessPolicy::authenticated(true, Some(MANAGEMENT_TOKEN.into())));
    let forged_with_old_constant =
        jwt("admin", b"mockforge-auto-jwt-secret-for-admin-ui-1234", 3600);
    let expired = jwt("admin", TEST_JWT_SECRET, -3600);
    let wrong_token = format!("{MANAGEMENT_TOKEN}x");
    let unsigned = {
        // alg=none, role admin
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        format!(
            "{}.{}.",
            b64.encode(r#"{"alg":"none","typ":"JWT"}"#),
            b64.encode(r#"{"sub":"x","username":"x","role":"admin","iat":0,"exp":9999999999}"#)
        )
    };
    let cases: Vec<(&str, String)> = vec![
        ("authorization", String::new()),
        ("authorization", "Bearer".to_string()),
        ("authorization", "Bearer ".to_string()),
        ("authorization", "Bearer not-a-jwt".to_string()),
        ("authorization", "Bearer a.b.c".to_string()),
        ("authorization", format!("Bearer {forged_with_old_constant}")),
        ("authorization", format!("Bearer {expired}")),
        ("authorization", format!("Bearer {unsigned}")),
        ("authorization", format!("Basic {MANAGEMENT_TOKEN}")),
        ("authorization", format!("Bearer {wrong_token}")),
        ("authorization", format!("Bearer mock.x.{}", "e30")),
        (MANAGEMENT_TOKEN_HEADER, String::new()),
        (MANAGEMENT_TOKEN_HEADER, wrong_token.clone()),
        (MANAGEMENT_TOKEN_HEADER, MANAGEMENT_TOKEN[..20].to_string()),
        (MANAGEMENT_TOKEN_HEADER, MANAGEMENT_TOKEN.to_uppercase()),
    ];
    for (header, value) in cases {
        for path in ["/__mockforge/logs", "/__mockforge/config"] {
            let req =
                request("GET", path).header(header, &value).body(Body::empty()).expect("valid");
            assert_eq!(
                status(&router, req).await,
                StatusCode::UNAUTHORIZED,
                "{header}: {value:?} on {path}"
            );
        }
    }
}

#[tokio::test]
async fn valid_credentials_are_accepted() {
    let router = with_policy(AdminAccessPolicy::authenticated(true, Some(MANAGEMENT_TOKEN.into())));
    let by_header = request("GET", "/__mockforge/chaos")
        .header(MANAGEMENT_TOKEN_HEADER, MANAGEMENT_TOKEN)
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, by_header).await, StatusCode::OK);
    let by_bearer = request("GET", "/__mockforge/chaos")
        .header("authorization", format!("Bearer {MANAGEMENT_TOKEN}"))
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, by_bearer).await, StatusCode::OK);
    let by_jwt = request("GET", "/__mockforge/chaos")
        .header("authorization", format!("Bearer {}", jwt("viewer", TEST_JWT_SECRET, 3600)))
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, by_jwt).await, StatusCode::OK, "viewers keep read access");
    // A viewer may not read environment variables or write.
    let viewer = jwt("viewer", TEST_JWT_SECRET, 3600);
    let env = request("GET", "/__mockforge/env")
        .header("authorization", format!("Bearer {viewer}"))
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, env).await, StatusCode::FORBIDDEN);
    let overrides = request("GET", "/__mockforge/overrides")
        .header(MANAGEMENT_TOKEN_HEADER, MANAGEMENT_TOKEN)
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, overrides).await, StatusCode::OK);
}

#[tokio::test]
async fn token_only_policy_refuses_jwts_and_dev_headers() {
    // Non-loopback with only a management token: no admin login exists, so
    // no JWT (and no dev header fallback) may authenticate.
    let router =
        with_policy(AdminAccessPolicy::authenticated(false, Some(MANAGEMENT_TOKEN.into())));
    let by_jwt = request("GET", "/__mockforge/logs")
        .header("authorization", format!("Bearer {}", jwt("admin", TEST_JWT_SECRET, 3600)))
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, by_jwt).await, StatusCode::UNAUTHORIZED);
    let by_dev_headers = request("GET", "/__mockforge/logs")
        .header("x-user-id", "admin-001")
        .header("x-username", "admin")
        .header("x-user-role", "admin")
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, by_dev_headers).await, StatusCode::UNAUTHORIZED);
    let by_token = request("GET", "/__mockforge/logs")
        .header(MANAGEMENT_TOKEN_HEADER, MANAGEMENT_TOKEN)
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, by_token).await, StatusCode::OK);
}

const PATH_VARIANTS: &[&str] = &[
    "/__mockforge/logs",
    "/__mockforge/logs/",
    "//__mockforge/logs",
    "/__mockforge//logs",
    "/__MOCKFORGE/logs",
    "/__mockforge/LOGS",
    "/__mockforge/%6Cogs",
    "/%5F%5Fmockforge/logs",
    "/__mockforge/./logs",
    "/__mockforge/logs;x",
    "/__mockforge/logs%2F",
    "/__mockforge/logs?x=1",
    "/__mockforge/logs/sse?access_token=",
    "/__mockforge/logs?access_token=x",
    "/assets/../__mockforge/logs",
    "/assets/..%2F__mockforge/logs",
    "/__mockforge/health/../logs",
    "/__mockforge/auth/login/../../__mockforge/logs",
    "/mockforge-icon.png/../__mockforge/logs",
    "/__mockforge/config",
    "/__mockforge/config/",
    "/__mockforge/env",
    "/__mockforge/overrides",
    "/__mockforge/overrides/",
    "/__mockforge/recorder/status",
    "/api/resilience/bulkheads",
];

#[tokio::test]
async fn path_variants_do_not_bypass_auth() {
    let strict = with_policy(AdminAccessPolicy::default());
    for path in PATH_VARIANTS {
        for method in ["GET", "HEAD", "POST", "PUT", "DELETE"] {
            let got = status(&strict, anonymous(method, path)).await;
            assert_eq!(got, StatusCode::UNAUTHORIZED, "{method} {path}");
        }
    }
}

#[tokio::test]
async fn loopback_dev_mode_allows_local_reads_only() {
    let router = with_policy(AdminAccessPolicy::loopback_dev());
    // A developer's TUI or curl on the same machine still reads anonymously.
    for path in [
        "/__mockforge/chaos",
        "/__mockforge/recorder/status",
        "/__mockforge/overrides",
    ] {
        assert_eq!(status(&router, anonymous("GET", path)).await, StatusCode::OK, "{path}");
    }
    // Writes still need credentials.
    for (method, path) in [
        ("POST", "/__mockforge/chaos/toggle"),
        ("PUT", "/__mockforge/overrides"),
        ("DELETE", "/__mockforge/logs"),
    ] {
        assert_eq!(
            status(&router, anonymous(method, path)).await,
            StatusCode::UNAUTHORIZED,
            "{method} {path}"
        );
    }
    // Reads outside /__mockforge/ need credentials even locally.
    assert_eq!(
        status(&router, anonymous("GET", "/api/resilience/bulkheads")).await,
        StatusCode::UNAUTHORIZED
    );
    // A rebound DNS name or a non-loopback Host gets nothing.
    for host in [
        "evil.example",
        "evil.example:9080",
        "localhost.evil.example",
        "0.0.0.0:9080",
        "[::ffff:127.0.0.1]:9080",
        "[::]:9080",
        "192.168.1.20:9080",
    ] {
        for path in PATH_VARIANTS {
            let req = Request::builder()
                .method("GET")
                .uri(*path)
                .header("host", host)
                .body(Body::empty())
                .expect("valid");
            assert_eq!(status(&router, req).await, StatusCode::UNAUTHORIZED, "{host} {path}");
        }
    }
    let no_host = Request::builder()
        .method("GET")
        .uri("/__mockforge/logs")
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, no_host).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sse_accepts_a_query_token_only_on_sse_streams() {
    let router = with_policy(AdminAccessPolicy::default());
    let token = jwt("viewer", TEST_JWT_SECRET, 3600);
    let sse = request("GET", &format!("/__mockforge/logs/sse?access_token={token}"))
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, sse).await, StatusCode::OK);
    let bad = request("GET", "/__mockforge/logs/sse?access_token=a.b.c")
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, bad).await, StatusCode::UNAUTHORIZED);
    let not_sse = request("GET", &format!("/__mockforge/logs?access_token={token}"))
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, not_sse).await, StatusCode::UNAUTHORIZED);
    let post = request("POST", &format!("/__mockforge/logs/sse?access_token={token}"))
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&router, post).await, StatusCode::UNAUTHORIZED);
    // Token-only policy refuses JWTs in the query too.
    let strict =
        with_policy(AdminAccessPolicy::authenticated(false, Some(MANAGEMENT_TOKEN.into())));
    let sse = request("GET", &format!("/__mockforge/logs/sse?access_token={token}"))
        .body(Body::empty())
        .expect("valid");
    assert_eq!(status(&strict, sse).await, StatusCode::UNAUTHORIZED);
}

#[test]
fn log_headers_and_proxy_urls_are_redacted() {
    use crate::handlers::{redact_sensitive_headers, redact_url_userinfo};
    let headers = [
        ("Authorization", "Bearer abc"),
        ("cookie", "s=1"),
        ("X-Api-Key", "k"),
        ("x-refresh-token", "t"),
        ("X-MockForge-Management-Token", "m"),
        ("content-type", "application/json"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let redacted = redact_sensitive_headers(headers);
    for name in [
        "Authorization",
        "cookie",
        "X-Api-Key",
        "x-refresh-token",
        "X-MockForge-Management-Token",
    ] {
        assert_eq!(redacted[name], "[REDACTED]", "{name}");
    }
    assert_eq!(redacted["content-type"], "application/json");

    assert_eq!(redact_url_userinfo("https://u:p@up.example/a@b"), "https://***@up.example/a@b");
    assert_eq!(redact_url_userinfo("https://up.example/a@b"), "https://up.example/a@b");
    assert_eq!(redact_url_userinfo("http://tok@h:1?q=@"), "http://***@h:1?q=@");
    assert_eq!(redact_url_userinfo("not a url"), "not a url");
}
