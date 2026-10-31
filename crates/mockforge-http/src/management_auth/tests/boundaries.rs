use super::*;

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
