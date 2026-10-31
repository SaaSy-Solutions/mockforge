//! Exact route classification, future-route refusal and automatic HEAD behavior.
use super::super::policy::{ADMIN_ORG, BILLING, DEPLOY, PUBLISH_PKG, READ_USAGE};
use super::super::TokenAccess::{AnyToken, SessionOnly};
use super::super::{classify, TokenAccess};
use axum::http::Method;

/// The sensitive families named in the security report map to the scope
/// they need, never to something a publish/read token holds.
#[test]
fn sensitive_routes_map_to_expected_access() {
    let cases: &[(&str, &str, TokenAccess)] = &[
        ("POST", "/api/v1/hosted-mocks", DEPLOY),
        ("PATCH", "/api/v1/hosted-mocks/{deployment_id}/status", DEPLOY),
        ("DELETE", "/api/v1/hosted-mocks/{deployment_id}", DEPLOY),
        ("POST", "/api/v1/hosted-mocks/{deployment_id}/redeploy", DEPLOY),
        ("GET", "/api/v1/hosted-mocks/{deployment_id}/captures/export/har", DEPLOY),
        ("POST", "/api/v1/test-suites/{id}/runs", DEPLOY),
        ("POST", "/api/v1/capture-sessions/{id}/replay", DEPLOY),
        ("POST", "/api/v1/tokens", SessionOnly),
        ("DELETE", "/api/v1/tokens/{token_id}", SessionOnly),
        ("POST", "/api/v1/tokens/{token_id}/rotate", SessionOnly),
        ("POST", "/api/v1/auth/change-password", SessionOnly),
        ("POST", "/api/v1/auth/2fa/disable", SessionOnly),
        ("DELETE", "/api/v1/gdpr/erase", SessionOnly),
        ("POST", "/api/v1/sso/config", SessionOnly),
        ("DELETE", "/api/v1/organizations/{org_id}", SessionOnly),
        ("POST", "/api/v1/organizations/{org_id}/members", ADMIN_ORG),
        ("POST", "/api/v1/billing/checkout", BILLING),
        ("GET", "/api/v1/usage", READ_USAGE),
        ("POST", "/api/v1/plugins/publish", PUBLISH_PKG),
        ("POST", "/api/v1/admin/plugins/{name}/takedown", SessionOnly),
        ("GET", "/api/v1/auth/me", AnyToken),
    ];
    for (method, path, expected) in cases {
        let m = Method::from_bytes(method.as_bytes()).expect("valid method");
        assert_eq!(classify(&m, path).as_ref(), Some(expected), "{method} {path}");
    }
}

#[test]
fn unknown_routes_are_unclassified() {
    assert_eq!(classify(&Method::POST, "/api/v1/brand-new-feature"), None);
    assert_eq!(classify(&Method::GET, "/api/v1/hosted-mocksX"), None);
    assert_eq!(classify(&Method::GET, "/api/v1/tokens-extra"), None);
    assert_eq!(classify(&Method::POST, "/api/v1/hosted-mocks/new-operation"), None);
    assert_eq!(classify(&Method::GET, "/api/v1/usage/new-operation"), None);
}

#[test]
fn automatic_head_requires_a_declared_get_route() {
    assert_eq!(classify(&Method::HEAD, "/api/v1/usage"), Some(READ_USAGE));
    assert_eq!(classify(&Method::HEAD, "/api/v1/usage/new-operation"), None);
}
