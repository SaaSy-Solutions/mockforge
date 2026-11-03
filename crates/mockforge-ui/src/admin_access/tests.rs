use super::*;
use axum::http::HeaderValue;
use std::net::SocketAddr;

const TOKEN: &str = "mfm_0123456789abcdef0123456789abcdef";

fn addr(s: &str) -> SocketAddr {
    s.parse().expect("test address literal is valid")
}

fn token_env() -> AdminAuthEnv {
    AdminAuthEnv {
        management_token: Some(TOKEN.to_string()),
        ..Default::default()
    }
}

#[test]
fn loopback_detection() {
    for a in ["127.0.0.1:9080", "127.1.2.3:9080", "[::1]:9080"] {
        assert!(is_loopback(addr(a).ip()), "{a} is loopback");
    }
    for a in [
        "0.0.0.0:9080",
        "[::]:9080",
        "[::ffff:127.0.0.1]:9080",
        "[::ffff:0.0.0.0]:9080",
        "10.0.0.5:9080",
        "192.168.1.10:9080",
        "[fdaa::1]:9080",
    ] {
        assert!(!is_loopback(addr(a).ip()), "{a} must be treated as non-loopback");
    }
}

#[test]
fn non_loopback_without_credentials_refuses_to_start() {
    for a in [
        "0.0.0.0:9080",
        "[::]:9080",
        "[::ffff:127.0.0.1]:9080",
        "10.0.0.5:9080",
    ] {
        let err = policy_for_bind(addr(a), &AdminAuthEnv::default())
            .expect_err("non-loopback bind without credentials must refuse");
        assert!(err.contains("Refusing"), "{a}: {err}");
    }
}

#[test]
fn rejected_short_credentials_do_not_count() {
    let env = AdminAuthEnv {
        rejected: vec!["MOCKFORGE_MANAGEMENT_TOKEN is shorter than 32 characters".into()],
        ..Default::default()
    };
    let err = policy_for_bind(addr("0.0.0.0:9080"), &env).expect_err("must refuse");
    assert!(err.contains("shorter than 32"), "{err}");
}

#[test]
fn non_loopback_with_dev_bypass_refuses_even_with_credentials() {
    let env = AdminAuthEnv {
        dev_bypass: true,
        admin_password: true,
        ..Default::default()
    };
    assert!(policy_for_bind(addr("0.0.0.0:9080"), &env).is_err());
    // Loopback dev keeps working with the bypass on.
    assert!(policy_for_bind(addr("127.0.0.1:9080"), &env).is_ok());
}

#[test]
fn non_loopback_token_only_refuses_jwts() {
    let policy = policy_for_bind(addr("[::]:9080"), &token_env()).expect("token configured");
    assert!(!policy.allows_anonymous_loopback_reads());
    assert!(!policy.accepts_jwt(), "no JWT can be legitimate without a login or secret");
}

#[test]
fn non_loopback_with_password_or_secret_accepts_jwt() {
    for env in [
        AdminAuthEnv {
            admin_password: true,
            ..Default::default()
        },
        AdminAuthEnv {
            jwt_secret: true,
            ..Default::default()
        },
    ] {
        let policy = policy_for_bind(addr("0.0.0.0:9080"), &env).expect("credentials set");
        assert!(policy.accepts_jwt());
        assert!(!policy.allows_anonymous_loopback_reads());
    }
}

#[test]
fn loopback_without_credentials_is_dev_mode() {
    for a in ["127.0.0.1:9080", "[::1]:9080"] {
        let policy = policy_for_bind(addr(a), &AdminAuthEnv::default()).expect("loopback");
        assert!(policy.allows_anonymous_loopback_reads(), "{a}");
    }
}

#[test]
fn loopback_with_credentials_requires_them() {
    let policy = policy_for_bind(addr("127.0.0.1:9080"), &token_env()).expect("loopback");
    assert!(!policy.allows_anonymous_loopback_reads());
}

#[test]
fn management_token_comparison() {
    let policy = AdminAccessPolicy::authenticated(false, Some(TOKEN.to_string()));
    let check = |name: &str, value: &str| {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::HeaderName::from_bytes(name.as_bytes()).expect("valid header name"),
            HeaderValue::from_str(value).expect("valid header value"),
        );
        policy.management_token_matches(&h)
    };
    assert!(check(MANAGEMENT_TOKEN_HEADER, TOKEN));
    assert!(check("authorization", &format!("Bearer {TOKEN}")));
    assert!(!check(MANAGEMENT_TOKEN_HEADER, ""));
    assert!(!check(MANAGEMENT_TOKEN_HEADER, &TOKEN[..TOKEN.len() - 1]));
    assert!(!check(MANAGEMENT_TOKEN_HEADER, &format!("{TOKEN}x")));
    assert!(!check(MANAGEMENT_TOKEN_HEADER, &TOKEN.to_uppercase()));
    assert!(!check("authorization", &format!("Basic {TOKEN}")));
    assert!(!check("authorization", &format!("bearer{TOKEN}")));
    assert!(!policy.management_token_matches(&HeaderMap::new()));
    // No token configured: nothing matches, not even an empty header.
    let none = AdminAccessPolicy::authenticated(true, None);
    let mut h = HeaderMap::new();
    h.insert(MANAGEMENT_TOKEN_HEADER, HeaderValue::from_static(""));
    assert!(!none.management_token_matches(&h));
}

#[test]
fn debug_output_redacts_the_token() {
    let policy = AdminAccessPolicy::authenticated(false, Some(TOKEN.to_string()));
    assert!(!format!("{policy:?}").contains(TOKEN));
}

#[test]
fn anonymous_reads_need_a_loopback_host() {
    let policy = AdminAccessPolicy::loopback_dev();
    let uri: Uri = "/__mockforge/logs".parse().expect("valid uri");
    let with_host = |host: &str| {
        let mut h = HeaderMap::new();
        h.insert(header::HOST, HeaderValue::from_str(host).expect("valid host"));
        policy.allows_anonymous_read(&Method::GET, &uri, &h)
    };
    for host in [
        "localhost",
        "localhost:9080",
        "127.0.0.1:9080",
        "[::1]:9080",
        "LOCALHOST",
    ] {
        assert!(with_host(host), "{host} should be allowed");
    }
    for host in [
        "evil.example:9080",
        "localhost.evil.example",
        "127.0.0.1.nip.io",
        "0.0.0.0:9080",
        "[::ffff:127.0.0.1]:9080",
        "[::]:9080",
        "10.0.0.1",
        "[::1]evil",
        "",
    ] {
        assert!(!with_host(host), "{host:?} must be refused");
    }
    assert!(!policy.allows_anonymous_read(&Method::GET, &uri, &HeaderMap::new()));
    let mut h = HeaderMap::new();
    h.insert(header::HOST, HeaderValue::from_static("localhost"));
    assert!(!policy.allows_anonymous_read(&Method::POST, &uri, &h));
    let other: Uri = "/api/resilience/bulkheads".parse().expect("valid uri");
    assert!(!policy.allows_anonymous_read(&Method::GET, &other, &h));
    // Authenticated policy never allows anonymous reads.
    let strict = AdminAccessPolicy::default();
    assert!(!strict.allows_anonymous_read(&Method::GET, &uri, &h));
}

#[test]
fn environment_debug_output_redacts_management_credentials() {
    let env = token_env();
    let shown = format!("{env:?}");
    assert!(!shown.contains(env.management_token.as_deref().unwrap()));
    assert!(shown.contains("<redacted>"));
}
