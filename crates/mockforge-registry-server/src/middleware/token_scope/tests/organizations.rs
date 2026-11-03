//! Organization binding is independent of user membership in both organizations.
use super::super::organization::org_id_path_params;
use super::fixtures::*;
use axum::http::{Method, StatusCode};

#[tokio::test]
async fn token_cannot_act_on_another_org() {
    let t = token(org_a(), &["deploy:mocks", "admin:org"]);
    let other = org_b().to_string();

    // Another org in the path.
    for uri in [
        format!("/api/v1/organizations/{other}/members"),
        format!("/api/v1/organizations/{other}/tunnels"),
        "/api/v1/organizations/not-a-uuid/tunnels".to_string(),
    ] {
        let method = if uri.ends_with("members") {
            Method::POST
        } else {
            Method::GET
        };
        let (status, body) = call(Caller::Token(t.clone()), method, &uri, &[]).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}: {body}");
        assert!(body.contains("TOKEN_ORG_MISMATCH"), "{body}");
    }

    // Another org via header, including case/whitespace/garbage variants.
    let upper = other.to_uppercase();
    for (name, value) in [
        ("X-Organization-Id", other.as_str()),
        ("x-organization-id", other.as_str()),
        ("X-Organization-Id", upper.as_str()),
        ("X-Organization-Id", "not-a-uuid"),
        ("X-Organization-Id", ""),
        ("X-Organization-Slug", "someone-else"),
        ("x-organization-slug", "my-own-org"),
    ] {
        let (status, body) = call(
            Caller::Token(t.clone()),
            Method::DELETE,
            &format!("/api/v1/hosted-mocks/{}", dep()),
            &[(name, value)],
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{name}: {value:?} -> {body}");
        assert!(!body.contains(OK_HANDLER_TAG));
    }

    // Matching header is accepted (and stays pinned).
    let own = org_a().to_string();
    let (status, body) = call(
        Caller::Token(t.clone()),
        Method::POST,
        "/api/v1/hosted-mocks",
        &[("X-Organization-Id", &own)],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, format!("{OK_HANDLER_TAG} org={own}"));

    // A token of org B gets org B pinned, never org A.
    let tb = token(org_b(), &["deploy:mocks"]);
    let (_, body) = call(Caller::Token(tb), Method::GET, "/api/v1/hosted-mocks", &[]).await;
    assert_eq!(body, format!("{OK_HANDLER_TAG} org={other}"));
}

#[test]
fn org_id_params_align_with_template() {
    assert_eq!(
        org_id_path_params(
            "/api/v1/organizations/{org_id}/members/{user_id}",
            "/api/v1/organizations/x/members/y"
        ),
        vec!["x"]
    );
    assert!(
        org_id_path_params("/api/v1/hosted-mocks/{deployment_id}", "/api/v1/hosted-mocks/x")
            .is_empty()
    );
}
