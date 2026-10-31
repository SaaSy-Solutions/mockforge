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
use std::sync::Arc;
use tower::ServiceExt;

#[path = "control_plane_auth/assembly.rs"]
mod assembly;
#[path = "control_plane_auth/discovery.rs"]
mod discovery;
#[path = "control_plane_auth/mount_invariants.rs"]
mod mount_invariants;
#[path = "control_plane_auth/sources.rs"]
mod sources;
use assembly::{manifest_dir, production_source, read};
use discovery::{concrete, control_mounts, join, routes_in};
use sources::router_fn_body;

/// Upper bound for one probe, so a handler that never answers fails the
/// test with its route instead of stalling the CI job until it times out.
const RESPONSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

const TOKEN: &str = "mfm_0123456789abcdef0123456789abcdef0123456789abcdef";

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
    let response =
        tokio::time::timeout(RESPONSE_TIMEOUT, app.clone().oneshot(req.body(body).unwrap()))
            .await
            .unwrap_or_else(|_| {
                panic!("{method} {path} did not answer within {RESPONSE_TIMEOUT:?}")
            })
            .unwrap();
    let status = response.status().as_u16();
    // Event streams (for example the MQTT message stream) never end; the
    // status is all the probe needs, and reading the body would hang.
    let streaming = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/event-stream"));
    if streaming {
        return (status, "<event stream>".to_string());
    }
    let bytes =
        tokio::time::timeout(RESPONSE_TIMEOUT, axum::body::to_bytes(response.into_body(), 1 << 20))
            .await
            .unwrap_or_else(|_| {
                panic!("{method} {path} body did not finish within {RESPONSE_TIMEOUT:?}")
            })
            .unwrap_or_default();
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
