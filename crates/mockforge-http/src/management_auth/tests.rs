use super::*;
use axum::http::HeaderMap;
use axum::routing::get;
use std::net::IpAddr;
use std::net::{Ipv4Addr, Ipv6Addr};
use tower::ServiceExt;

const TOKEN: &str = "0123456789abcdef0123456789abcdef-test";

fn app(policy: ControlPlanePolicy) -> Router {
    let ok = || async { "ok" };
    let control = protect(
        Router::new()
            .route("/api/v1/auth/tokens/revoke", get(ok).post(ok))
            .route("/api/chaos/config", get(ok).post(ok)),
    );
    let reserved_unprotected = Router::new().route("/__mockforge/late", get(ok));
    let mocks = Router::new()
        .route("/api/v1/auth/login", get(ok).post(ok))
        .route("/__mockforgeX/ping", get(ok))
        .route("/__mockforge/docs", get(ok));
    install(control.merge(reserved_unprotected).merge(mocks), policy)
}

async fn status(app: Router, method: Method, path: &str, headers: &[(&str, &str)]) -> u16 {
    let mut req = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap().status().as_u16()
}

#[test]
fn reserved_namespace_is_normalised() {
    for path in [
        "/__mockforge",
        "/__mockforge/api/mocks",
        "//__mockforge/api/mocks",
        "/__mockforge//api",
        "/%5F_mockforge/api/mocks",
        "/%5f%5fmockforge/api",
        "/%255F_mockforge/api",
        "/__MOCKFORGE/api",
        "/x/../__mockforge/api",
        "/./__mockforge/api",
        "/__mockforge;jsessionid=1/api",
        "\\__mockforge\\api",
    ] {
        assert!(is_reserved_path(path), "{path}");
    }
    for path in [
        "/__mockforgeX/api",
        "/api/__mockforge",
        "/",
        "/health",
        "/__mock",
    ] {
        assert!(!is_reserved_path(path), "{path}");
    }
}

#[test]
fn loopback_is_strict() {
    assert!(is_strict_loopback(Ipv4Addr::LOCALHOST.into()));
    assert!(is_strict_loopback(Ipv4Addr::new(127, 1, 2, 3).into()));
    assert!(is_strict_loopback(Ipv6Addr::LOCALHOST.into()));
    for ip in [
        IpAddr::from(Ipv4Addr::UNSPECIFIED),
        IpAddr::from(Ipv6Addr::UNSPECIFIED),
        IpAddr::from(Ipv4Addr::LOCALHOST.to_ipv6_mapped()),
        IpAddr::from(Ipv4Addr::new(10, 0, 0, 1)),
        "fdaa::1".parse().unwrap(),
    ] {
        assert!(!is_strict_loopback(ip), "{ip}");
    }
}

#[test]
fn insecure_opt_out_is_refused_off_loopback() {
    for bind in ["0.0.0.0", "::", "::ffff:127.0.0.1", "10.1.2.3", "fdaa::3"] {
        let bind: IpAddr = bind.parse().unwrap();
        assert_eq!(
            ControlPlanePolicy::from_values(bind, None, Some("1"), None).unwrap_err(),
            ControlPlaneConfigError::InsecureOnNonLoopback(bind),
        );
        // Even with a token set, the opt-out off loopback fails loudly.
        assert!(ControlPlanePolicy::from_values(bind, Some(TOKEN), Some("true"), None).is_err());
        assert!(ControlPlanePolicy::unauthenticated_loopback(bind).is_err());
    }
    for bind in ["127.0.0.1", "::1"] {
        let policy =
            ControlPlanePolicy::from_values(bind.parse().unwrap(), None, Some("1"), None).unwrap();
        assert!(policy.is_unauthenticated());
    }
}

#[test]
fn policy_from_values() {
    let any: IpAddr = Ipv4Addr::UNSPECIFIED.into();
    assert!(ControlPlanePolicy::from_values(any, None, None, None).unwrap().is_locked());
    assert!(ControlPlanePolicy::from_values(any, Some(""), Some("0"), None)
        .unwrap()
        .is_locked());
    assert_eq!(
        ControlPlanePolicy::from_values(any, Some("short"), None, None).unwrap_err(),
        ControlPlaneConfigError::TokenTooShort
    );
    let admin = AdminCredential::new("admin", "a-long-admin-password");
    let policy = ControlPlanePolicy::from_values(any, None, None, admin).unwrap();
    assert!(!policy.is_locked() && !policy.is_unauthenticated());
    // A credential wins over the loopback opt-out.
    let policy =
        ControlPlanePolicy::from_values(Ipv4Addr::LOCALHOST.into(), Some(TOKEN), Some("1"), None)
            .unwrap();
    assert!(!policy.is_unauthenticated());
    assert!(AdminCredential::new("", "x").is_none());
    assert!(AdminCredential::new("x", "").is_none());
}

#[tokio::test]
async fn bad_credentials_are_refused_and_the_token_is_accepted() {
    let policy = || ControlPlanePolicy::with_token(TOKEN).unwrap();
    let basic_wrong = format!(
        "Basic {}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, "admin:wrong")
    );
    let bearer_prefix = format!("Bearer {}", &TOKEN[..TOKEN.len() - 1]);
    let protocol_no_upgrade = format!("{WS_TOKEN_PROTOCOL_PREFIX}{TOKEN}");
    let refused: Vec<Vec<(&str, &str)>> = vec![
        vec![],
        vec![(MANAGEMENT_TOKEN_HEADER, "")],
        vec![(MANAGEMENT_TOKEN_HEADER, "wrong")],
        vec![(MANAGEMENT_TOKEN_HEADER, &TOKEN[..10])],
        vec![("authorization", "Bearer ")],
        vec![("authorization", "Bearer wrong")],
        vec![("authorization", &bearer_prefix)],
        vec![("authorization", TOKEN)],
        vec![("authorization", "Basic !!!not-base64")],
        vec![("authorization", &basic_wrong)],
        // The subprotocol form only counts on a WebSocket upgrade.
        vec![("sec-websocket-protocol", &protocol_no_upgrade)],
    ];
    for path in [
        "/__mockforge/late",
        "/api/chaos/config",
        "/api/v1/auth/tokens/revoke",
    ] {
        for method in [Method::GET, Method::POST] {
            for headers in &refused {
                assert_eq!(
                    status(app(policy()), method.clone(), path, headers).await,
                    401,
                    "{method} {path} {headers:?}"
                );
            }
        }
        let bearer = format!("Bearer {TOKEN}");
        for headers in [
            vec![(MANAGEMENT_TOKEN_HEADER, TOKEN)],
            vec![("authorization", bearer.as_str())],
            vec![
                ("upgrade", "websocket"),
                ("sec-websocket-protocol", &format!("{WS_PROTOCOL}, {protocol_no_upgrade}")),
            ],
        ] {
            assert_eq!(status(app(policy()), Method::GET, path, &headers).await, 200, "{path}");
        }
    }
}

#[tokio::test]
async fn the_token_never_reaches_inner_layers_or_handlers() {
    let echo = |headers: HeaderMap| async move {
        let seen: Vec<String> = headers
            .iter()
            .map(|(k, v)| format!("{k}={}", v.to_str().unwrap_or("")))
            .collect();
        seen.join("\n")
    };
    let app = install(
        protect(Router::new().route("/api/chaos/config", get(echo)))
            .merge(Router::new().route("/__mockforge/ws", get(echo)).route("/mock", get(echo))),
        ControlPlanePolicy::with_token(TOKEN).unwrap(),
    );
    let bearer = format!("Bearer {TOKEN}");
    let protocol = format!("{WS_PROTOCOL}, {WS_TOKEN_PROTOCOL_PREFIX}{TOKEN}");
    for (path, headers) in [
        ("/api/chaos/config", vec![(MANAGEMENT_TOKEN_HEADER, TOKEN)]),
        ("/api/chaos/config", vec![("authorization", bearer.as_str())]),
        ("/mock", vec![(MANAGEMENT_TOKEN_HEADER, TOKEN)]),
        (
            "/__mockforge/ws",
            vec![
                ("upgrade", "websocket"),
                ("sec-websocket-protocol", protocol.as_str()),
            ],
        ),
    ] {
        let mut req = Request::builder().uri(path);
        for (k, v) in &headers {
            req = req.header(*k, *v);
        }
        let response = app.clone().oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let body = axum::body::to_bytes(response.into_body(), 1 << 16).await.unwrap();
        let body = String::from_utf8_lossy(&body);
        assert!(!body.contains(TOKEN), "{path} saw the token: {body}");
    }
    // A Bearer that is not the management token is left for mock auth.
    let req = Request::builder()
        .uri("/mock")
        .header("authorization", "Bearer user-jwt")
        .body(Body::empty())
        .unwrap();
    let body = axum::body::to_bytes(app.oneshot(req).await.unwrap().into_body(), 1 << 16)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&body).contains("Bearer user-jwt"));
}

#[tokio::test]
async fn locked_policy_refuses_everything_on_the_control_plane() {
    for path in [
        "/__mockforge/late",
        "/api/chaos/config",
        "/api/v1/auth/tokens/revoke",
    ] {
        let headers = [(MANAGEMENT_TOKEN_HEADER, TOKEN)];
        assert_eq!(
            status(app(ControlPlanePolicy::locked()), Method::GET, path, &headers).await,
            401
        );
    }
}

#[tokio::test]
async fn admin_basic_credentials_are_accepted() {
    let admin = AdminCredential::new("admin", "correct horse battery staple");
    let policy = ControlPlanePolicy::locked().with_admin(admin);
    let good = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "admin:correct horse battery staple"
        )
    );
    let headers = [("authorization", good.as_str())];
    assert_eq!(
        status(app(policy.clone()), Method::POST, "/api/chaos/config", &headers).await,
        200
    );
    assert_eq!(status(app(policy), Method::GET, "/__mockforge/late", &headers).await, 200);
}

#[tokio::test]
async fn mock_traffic_and_public_paths_stay_open() {
    let policy = || ControlPlanePolicy::with_token(TOKEN).unwrap();
    // A user mock that shares a control router's prefix is untouched.
    assert_eq!(status(app(policy()), Method::POST, "/api/v1/auth/login", &[]).await, 200);
    assert_eq!(status(app(policy()), Method::GET, "/__mockforgeX/ping", &[]).await, 200);
    assert_eq!(status(app(policy()), Method::GET, "/__mockforge/docs", &[]).await, 200);
    // Only the exact docs path is public.
    assert_eq!(status(app(policy()), Method::GET, "/__mockforge/docs/", &[]).await, 401);
    assert_eq!(status(app(policy()), Method::POST, "/__mockforge/docs", &[]).await, 401);
}

#[tokio::test]
async fn path_tricks_do_not_reach_control_handlers() {
    let policy = || ControlPlanePolicy::with_token(TOKEN).unwrap();
    for path in [
        "/__mockforge/late/",
        "//__mockforge/late",
        "/__mockforge//late",
        "/%5F_mockforge/late",
        "/__MOCKFORGE/late",
        "/__mockforge/x/../late",
        "/api/../__mockforge/late",
        "/api//chaos/config",
        "/api/chaos/config/",
        "/API/chaos/config",
        "/api/%63haos/config",
        "/api/chaos/./config",
    ] {
        let code = status(app(policy()), Method::GET, path, &[]).await;
        assert!(code == 401 || code == 404, "{path} answered {code}");
    }
}

#[tokio::test]
async fn cors_preflight_passes_the_outer_gate_only() {
    let headers = [
        ("origin", "https://app.mockforge.dev"),
        ("access-control-request-method", "GET"),
    ];
    let policy = ControlPlanePolicy::with_token(TOKEN).unwrap();
    // The reserved route has no OPTIONS handler: 405 from the router,
    // not a handler run.
    assert_eq!(
        status(app(policy.clone()), Method::OPTIONS, "/__mockforge/late", &headers).await,
        405
    );
    // A protected route refuses even a preflight-shaped request.
    let any_router = protect(Router::new().route("/api/x", axum::routing::any(|| async { "ran" })));
    assert_eq!(
        status(install(any_router, policy), Method::OPTIONS, "/api/x", &headers).await,
        401
    );
}

#[tokio::test]
async fn unauthenticated_loopback_checks_the_host_header() {
    let policy =
        || ControlPlanePolicy::unauthenticated_loopback(Ipv4Addr::LOCALHOST.into()).unwrap();
    for host in [
        "localhost:3000",
        "127.0.0.1:3000",
        "[::1]:3000",
        "LOCALHOST",
    ] {
        assert_eq!(
            status(app(policy()), Method::POST, "/api/chaos/config", &[("host", host)]).await,
            200,
            "{host}"
        );
    }
    for host in [
        "evil.example",
        "evil.example:3000",
        "0.0.0.0:3000",
        "[::]:3000",
        "[::ffff:127.0.0.1]:3000",
        "127.0.0.1.evil.example",
    ] {
        assert_eq!(
            status(app(policy()), Method::POST, "/api/chaos/config", &[("host", host)]).await,
            403,
            "{host}"
        );
    }
}

#[tokio::test]
async fn protect_without_install_falls_back_closed() {
    // No install(): the env token decides, and with none set the route is locked.
    let router = protect(Router::new().route("/api/chaos/config", get(|| async { "ok" })));
    let code = status(router, Method::GET, "/api/chaos/config", &[]).await;
    assert_eq!(code, 401);
}
