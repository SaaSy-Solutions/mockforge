use super::*;

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
