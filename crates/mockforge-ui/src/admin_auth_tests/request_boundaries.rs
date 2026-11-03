use super::*;

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
