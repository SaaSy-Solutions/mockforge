//! The control plane on the HTTP data port needs a management credential (#1011).
//!
//! Two checks keep new routes honest:
//!
//! - `every_control_mount_is_protected` reads the router assembly in
//!   `lib.rs` and the CLI's `serve.rs` and fails when a router is nested or
//!   merged onto the data port without `management_auth::protect`, unless it
//!   is on the reviewed list of mock-traffic routers.
//! - `every_control_route_refuses_bad_credentials` builds the real data-port
//!   router, derives every route of every protected router from its source,
//!   and sends each one with no, empty, wrong and malformed credentials.

use axum::body::Body;
use axum::http::{Method, Request};
use axum::Router;
use mockforge_core::SharedOverrides;
use mockforge_http::management_auth::{
    install, ControlPlanePolicy, MANAGEMENT_TOKEN_HEADER, WS_PROTOCOL, WS_TOKEN_PROTOCOL_PREFIX,
};
use mockforge_http::{build_router_with_chains_and_multi_tenant, HealthManager};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tower::ServiceExt;

const TOKEN: &str = "mfm_0123456789abcdef0123456789abcdef0123456789abcdef";

/// Routers merged or nested onto the data port that serve mock traffic or
/// static assets, so they stay public. Each entry needs a reason.
const PUBLIC_MOUNTS: &[(&str, &str)] = &[
    ("openapi_router", "the user's OpenAPI mocks"),
    ("spec_router", "the user's OpenAPI mocks"),
    ("sse::sse_router", "mock server-sent events"),
    ("file_server::file_serving_router", "mock file downloads"),
    ("health::health_router_with_exclusions", "liveness and readiness; no config"),
    ("oidc_router", "mock OIDC provider discovery and JWKS"),
    ("oauth2_server_router", "mock OAuth2 authorization server"),
    ("consent_router", "mock OAuth2 consent screen"),
    ("mockforge_ws::router", "mock WebSocket endpoints"),
    ("gql_router", "mock GraphQL endpoint"),
    ("mockforge_http::llm_mock::router", "mock LLM API"),
    ("mockforge_http::mcp_mock::router", "mock MCP server"),
    (
        "\"/__mockforge/coverage.html\"",
        "static file in the reserved namespace; the outer gate requires the credential",
    ),
    ("\"/__mockforge/docs\"", "static docs page; it fetches the spec with the token"),
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Strip `#[cfg(test)]` modules and comment lines so only production
/// assembly code is scanned.
fn production_source(source: &str) -> String {
    let cut = source.find("#[cfg(test)]\nmod tests").unwrap_or(source.len());
    source[..cut]
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The argument text of a call whose `(` is at `open`.
fn call_args(source: &str, open: usize) -> &str {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    for (i, &b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return &source[open + 1..i];
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced call at byte {open}");
}

/// Every `.nest(`, `.nest_service(`, `.merge(` and `.route("/__mockforge…`
/// call on the data-port app, with its arguments.
fn mount_calls(source: &str) -> Vec<(String, String)> {
    let mut calls = Vec::new();
    for needle in [".nest(", ".nest_service(", ".merge("] {
        let mut from = 0;
        while let Some(pos) = source[from..].find(needle) {
            let open = from + pos + needle.len() - 1;
            calls.push((needle.to_string(), call_args(source, open).trim().to_string()));
            from = open + 1;
        }
    }
    let mut from = 0;
    while let Some(pos) = source[from..].find(".route(\"/__mockforge") {
        let open = from + pos + ".route".len();
        calls.push((".route(".to_string(), call_args(source, open).trim().to_string()));
        from = open + 1;
    }
    calls
}

fn is_public_mount(args: &str) -> bool {
    PUBLIC_MOUNTS.iter().any(|(name, _)| {
        args.starts_with(name)
            || args
                .split_once(',')
                .is_some_and(|(first, rest)| first.trim() == *name || rest.trim().starts_with(name))
    })
}

#[test]
fn every_control_mount_is_protected() {
    let lib = production_source(&read(&manifest_dir().join("src/lib.rs")));
    let serve = production_source(&read(&manifest_dir().join("../mockforge-cli/src/serve.rs")));
    let mut unprotected = Vec::new();
    let mut protected = 0;
    for (file, source) in [("lib.rs", &lib), ("serve.rs", &serve)] {
        for (call, args) in mount_calls(source) {
            // Routers built inside a control router (e.g. `Router::new()
            // .route(...)` chains passed to protect) are covered by it.
            if args.contains("management_auth::protect(") {
                protected += 1;
                continue;
            }
            if is_public_mount(&args) {
                continue;
            }
            // `.route("/__mockforge/...")` inside a router that is itself
            // passed to protect: the router variable is checked at its mount.
            if call == ".route("
                && (args.starts_with("\"/__mockforge/routes\"")
                    || args.starts_with("\"/__mockforge/coverage\""))
            {
                continue;
            }
            unprotected.push(format!("{file}: {call}{}", args.replace('\n', " ")));
        }
    }
    assert!(
        unprotected.is_empty(),
        "control routers mounted on the data port without management_auth::protect \
         (wrap them, or add a reviewed entry to PUBLIC_MOUNTS):\n{}",
        unprotected.join("\n")
    );
    assert!(protected >= 40, "only {protected} protected mounts found; is the scan broken?");
}

/// `(mount prefix or "" for merge, router fn name)`
/// for every mount that is not on the public list, protected or not, so an
/// unprotected control router is probed (and fails) here too.
fn control_mounts(lib: &str) -> Vec<(String, String)> {
    let mut mounts = Vec::new();
    for (call, args) in mount_calls(lib) {
        if call == ".route(" || call == ".nest_service(" || is_public_mount(&args) {
            continue;
        }
        let inner = match args.find("management_auth::protect(") {
            Some(at) => &args[at + "management_auth::protect(".len()..],
            None if call == ".nest(" => args.split_once(',').map_or("", |(_, rest)| rest.trim()),
            None => args.as_str(),
        };
        let name: String = inner
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
            .collect();
        let name = name.rsplit("::").next().unwrap_or(&name).to_string();
        let prefix = if call == ".nest(" {
            args.split('"').nth(1).unwrap_or_default().to_string()
        } else {
            String::new()
        };
        mounts.push((prefix, name));
    }
    mounts
}

/// Production source of every Rust file in the workspace's crates, read once
fn workspace_sources() -> &'static Vec<(PathBuf, String)> {
    static SOURCES: std::sync::OnceLock<Vec<(PathBuf, String)>> = std::sync::OnceLock::new();
    SOURCES.get_or_init(|| {
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                let name = entry.file_name();
                if path.is_dir() {
                    if name != "target" && name != "node_modules" && name != "tests" {
                        walk(&path, out);
                    }
                } else if path.extension().is_some_and(|e| e == "rs") {
                    out.push(path);
                }
            }
        }
        let mut files = Vec::new();
        // This crate first, so its definitions win over same-named ones elsewhere.
        walk(&manifest_dir().join("src"), &mut files);
        walk(&manifest_dir().join(".."), &mut files);
        files
            .into_iter()
            .map(|f| {
                let source = production_source(&read(&f));
                (f, source)
            })
            .collect()
    })
}

/// The body of `fn name`, looked up in this crate first, then the workspace
fn router_fn_body(name: &str) -> Option<String> {
    let needles = [format!("fn {name}("), format!("fn {name}<")];
    for (_, source) in workspace_sources() {
        let Some(start) = needles.iter().find_map(|n| source.find(n.as_str())) else {
            continue;
        };
        let body_open = start + source[start..].find('{')?;
        let mut depth = 0usize;
        for (i, b) in source.bytes().enumerate().skip(body_open) {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(source[body_open..=i].to_string());
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// `(method, path)` for every `.route("path", ...)` in `body`
fn routes_in(body: &str) -> Vec<(Method, String)> {
    let mut routes = Vec::new();
    let mut from = 0;
    while let Some(pos) = body[from..].find(".route(") {
        let open = from + pos + ".route".len();
        let args = call_args(body, open);
        from = open + 1;
        let Some(path) = args.split('"').nth(1) else {
            continue;
        };
        let handler = &args[args.find(',').unwrap_or(0)..];
        let mut methods = Vec::new();
        for (token, method) in [
            ("get(", Method::GET),
            ("post(", Method::POST),
            ("put(", Method::PUT),
            ("patch(", Method::PATCH),
            ("delete(", Method::DELETE),
            ("any(", Method::POST),
        ] {
            let found = handler.match_indices(token).any(|(i, _)| {
                i == 0
                    || !handler.as_bytes()[i - 1].is_ascii_alphanumeric()
                        && handler.as_bytes()[i - 1] != b'_'
            });
            if found {
                methods.push(method);
            }
        }
        for method in methods {
            routes.push((method, path.to_string()));
        }
    }
    routes
}

fn concrete(path: &str) -> String {
    path.split('/')
        .map(|seg| {
            if seg.starts_with('{') || seg.starts_with(':') || seg.starts_with('*') {
                "probe"
            } else {
                seg
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn join(prefix: &str, path: &str) -> String {
    match (prefix.is_empty(), path) {
        (true, p) => p.to_string(),
        (false, "/" | "") => prefix.to_string(),
        (false, p) => format!("{prefix}{p}"),
    }
}

async fn data_port_app() -> Router {
    let health = Arc::new(HealthManager::new());
    health.set_ready().await;
    build_router_with_chains_and_multi_tenant(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        Some(health),
        None,
        None,
        None,
        SharedOverrides::load(Vec::new()).await,
    )
    .await
}

async fn send(
    app: &Router,
    method: &Method,
    path: &str,
    headers: &[(&str, &str)],
) -> (u16, String) {
    let mut req = Request::builder()
        .method(method.clone())
        .uri(path)
        .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from(([127, 0, 0, 1], 40000))));
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    let body = if *method == Method::GET || *method == Method::DELETE {
        Body::empty()
    } else {
        req = req.header("content-type", "application/json");
        Body::from("{}")
    };
    let response = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20).await.unwrap_or_default();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn every_control_route_refuses_bad_credentials() {
    let fixtures = tempfile::tempdir().unwrap();
    std::env::set_var("MOCKFORGE_FIXTURES_DIR", fixtures.path());
    let app = install(data_port_app().await, ControlPlanePolicy::with_token(TOKEN).unwrap());

    let lib = production_source(&read(&manifest_dir().join("src/lib.rs")));
    let mut table: BTreeMap<String, (Method, String)> = BTreeMap::new();
    for (prefix, router) in control_mounts(&lib) {
        // `routes_router` is a local `Router::new().route(...)` chain in lib.rs.
        let body = if router == "routes_router" {
            ".route(\"/__mockforge/routes\", get(h)).route(\"/__mockforge/coverage\", get(h))"
                .to_string()
        } else {
            router_fn_body(&router)
                .unwrap_or_else(|| panic!("router fn {router} not found in the workspace"))
        };
        for (method, path) in routes_in(&body) {
            let full = concrete(&join(&prefix, &path));
            table.insert(format!("{method} {full} ({router})"), (method, full));
        }
    }
    assert!(
        table.len() >= 150,
        "only {} control routes derived; is the parser broken?",
        table.len()
    );

    let wrong_basic = "Basic YWRtaW46d3Jvbmc=";
    let bad: [&[(&str, &str)]; 8] = [
        &[],
        &[(MANAGEMENT_TOKEN_HEADER, "")],
        &[(MANAGEMENT_TOKEN_HEADER, "wrong")],
        &[(MANAGEMENT_TOKEN_HEADER, "mfm_0123456789abcdef0123456789abcdef0123456789abcdeX")],
        &[("authorization", "Bearer")],
        &[("authorization", "Bearer mfm_ 0123 garbage")],
        &[("authorization", wrong_basic)],
        &[("authorization", "Basic %%%")],
    ];
    let (fallback_status, fallback_body) = send(
        &app,
        &Method::GET,
        "/no-such-mock-route-anywhere",
        &[(MANAGEMENT_TOKEN_HEADER, TOKEN)],
    )
    .await;

    let mut failures = Vec::new();
    let mut mounted = 0;
    let mut not_mounted = Vec::new();
    for (label, (method, path)) in &table {
        let (status, body) = send(&app, method, path, &[(MANAGEMENT_TOKEN_HEADER, TOKEN)]).await;
        if status == 401 {
            failures.push(format!("{label}: valid token refused"));
            continue;
        }
        // A route that is not mounted in this configuration falls through
        // to the dynamic-mock fallback; outside the reserved namespace it
        // cannot be told apart from a mock, so it is reported, not probed.
        let unmatched = status == fallback_status && body == fallback_body;
        if unmatched && !path.starts_with("/__mockforge") {
            not_mounted.push(label.clone());
            continue;
        }
        mounted += 1;
        for headers in bad {
            let (status, _) = send(&app, method, path, headers).await;
            if status != 401 {
                failures.push(format!("{label} with {headers:?}: {status}"));
            }
        }
    }
    eprintln!(
        "control routes: {} derived, {mounted} mounted and probed, {} not mounted in this build",
        table.len(),
        not_mounted.len()
    );
    assert!(
        failures.is_empty(),
        "control routes answered without a valid credential:\n{}",
        failures.join("\n")
    );
    assert!(
        mounted >= 100,
        "only {mounted} control routes were probed:\nnot mounted: {not_mounted:#?}"
    );
}

#[tokio::test]
async fn path_tricks_on_the_real_router_never_reach_control_handlers() {
    let app = install(data_port_app().await, ControlPlanePolicy::with_token(TOKEN).unwrap());
    // Sanity: the target answers with the token.
    let (status, _) =
        send(&app, &Method::GET, "/__mockforge/routes", &[(MANAGEMENT_TOKEN_HEADER, TOKEN)]).await;
    assert_eq!(status, 200);
    for path in [
        "/__mockforge/routes",
        "/__mockforge/routes/",
        "//__mockforge/routes",
        "/__mockforge//time-travel/status",
        "/%5F_mockforge/time-travel/status",
        "/%5f%5fmockforge/time-travel/status",
        "/__MOCKFORGE/time-travel/status",
        "/__Mockforge/time-travel/status",
        "/x/../__mockforge/routes",
        "/__mockforge/x/../routes",
        "/__mockforge/%2e%2e/__mockforge/routes",
        "/__mockforge;x=1/time-travel/status",
        "/__mockforge/routes;x=1",
        "/__mockforge/routes?x=../../",
    ] {
        let (status, body) = send(&app, &Method::GET, path, &[]).await;
        assert_eq!(status, 401, "{path} answered {status}: {body}");
    }
    for method in [Method::POST, Method::PUT, Method::DELETE] {
        let (status, _) = send(&app, &method, "/__mockforge/time-travel/enable", &[]).await;
        assert_eq!(status, 401, "{method}");
    }
    // Prefix confusion: not the reserved namespace, so it is treated as a
    // (missing) mock, never as a control route.
    let (status, body) = send(&app, &Method::GET, "/__mockforgeX/routes", &[]).await;
    assert_ne!(status, 200, "{body}");
    assert!(!body.contains("\"routes\""), "{body}");
}

#[tokio::test]
async fn mock_traffic_and_health_stay_public() {
    let route: mockforge_core::config::RouteConfig = serde_json::from_value(serde_json::json!({
        "path": "/api/v1/auth/login", "method": "post",
        "response": {"status": 200, "body": {"marker": "user-mock"}}
    }))
    .unwrap();
    let health = Arc::new(HealthManager::new());
    health.set_ready().await;
    let app = build_router_with_chains_and_multi_tenant(
        None,
        None,
        None,
        None,
        Some(vec![route]),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        Some(health),
        None,
        None,
        None,
        SharedOverrides::load(Vec::new()).await,
    )
    .await;
    let app = install(app, ControlPlanePolicy::with_token(TOKEN).unwrap());
    // A user mock under a control router's prefix stays public.
    let (status, body) = send(&app, &Method::POST, "/api/v1/auth/login", &[]).await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("user-mock"), "{body}");
    let (status, body) = send(&app, &Method::GET, "/health", &[]).await;
    assert_eq!(status, 200, "{body}");
    for secret_ish in ["token", "password", "secret", "fixtures", "spec"] {
        assert!(!body.to_lowercase().contains(secret_ish), "health leaks {secret_ish}: {body}");
    }
    let (status, _) = send(&app, &Method::GET, "/__mockforge/docs", &[]).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn management_websocket_accepts_the_subprotocol_token() {
    let app = install(data_port_app().await, ControlPlanePolicy::with_token(TOKEN).unwrap());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
            .await
    });

    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let url = format!("ws://{addr}/__mockforge/ws");

    // No token: refused before the upgrade.
    let err = tokio_tungstenite::connect_async(url.clone()).await.unwrap_err();
    assert!(err.to_string().contains("401"), "{err}");

    // The browser form: token as a subprotocol, server selects WS_PROTOCOL.
    let mut req = url.clone().into_client_request().unwrap();
    req.headers_mut().insert(
        "sec-websocket-protocol",
        format!("{WS_PROTOCOL}, {WS_TOKEN_PROTOCOL_PREFIX}{TOKEN}").parse().unwrap(),
    );
    let (_socket, response) = tokio_tungstenite::connect_async(req).await.unwrap();
    assert_eq!(
        response.headers().get("sec-websocket-protocol").and_then(|v| v.to_str().ok()),
        Some(WS_PROTOCOL)
    );

    // A wrong subprotocol token is refused.
    let mut req = url.into_client_request().unwrap();
    req.headers_mut().insert(
        "sec-websocket-protocol",
        format!("{WS_PROTOCOL}, {WS_TOKEN_PROTOCOL_PREFIX}wrong").parse().unwrap(),
    );
    assert!(tokio_tungstenite::connect_async(req).await.is_err());
    server.abort();
}
