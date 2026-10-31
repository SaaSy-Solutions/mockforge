use super::*;

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
