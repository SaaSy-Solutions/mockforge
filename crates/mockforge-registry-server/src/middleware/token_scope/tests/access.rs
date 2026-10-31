//! Scope boundaries, session behavior and credential-variant refusal.
use super::fixtures::*;
use axum::http::{Method, StatusCode};

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
async fn empty_scope_token_gets_identity_only() {
    let t = token(org_a(), &[]);
    let (status, _) = call(Caller::Token(t.clone()), Method::GET, "/api/v1/auth/me", &[]).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(Caller::Token(t), Method::GET, "/api/v1/hosted-mocks", &[]).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
