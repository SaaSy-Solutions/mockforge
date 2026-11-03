use super::*;

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
