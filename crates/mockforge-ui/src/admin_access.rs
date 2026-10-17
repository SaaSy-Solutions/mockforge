//! Who may reach the admin server (#1019).
//!
//! The admin server exposes request logs, recorder captures, chaos state and
//! configuration. Reads need the same credentials as writes. The one
//! exception is a single developer running `mockforge serve --admin` on
//! their own machine: when the server is bound to a loopback address and no
//! credentials are configured, anonymous reads from a loopback `Host` are
//! allowed, as before. Bound anywhere else, the server refuses to start
//! until credentials are configured.
//!
//! Credentials, any of which counts as "configured":
//! - `MOCKFORGE_MANAGEMENT_TOKEN`: a shared token sent as
//!   `X-MockForge-Management-Token` or `Authorization: Bearer`. The registry
//!   sets it on every hosted mock and sends it on its private-network calls.
//! - `MOCKFORGE_ADMIN_PASSWORD` (with optional `MOCKFORGE_ADMIN_USERNAME`,
//!   default `admin`): the admin UI login. It is the only login the admin UI
//!   ever creates; there are no built-in users (#1016).
//! - `JWT_SECRET`: the operator signs admin JWTs themselves.

use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::http::{header, HeaderMap, Method, Uri};
use subtle::ConstantTimeEq;

/// Environment variable holding the shared management token
pub const MANAGEMENT_TOKEN_ENV: &str = "MOCKFORGE_MANAGEMENT_TOKEN";
/// Header that carries the management token
pub const MANAGEMENT_TOKEN_HEADER: &str = "x-mockforge-management-token";
/// Environment variable holding the admin UI password
pub const ADMIN_PASSWORD_ENV: &str = "MOCKFORGE_ADMIN_PASSWORD";
/// Environment variable holding the admin UI username
pub const ADMIN_USERNAME_ENV: &str = "MOCKFORGE_ADMIN_USERNAME";

/// Shortest management token or JWT secret accepted
pub const MIN_SECRET_LEN: usize = 32;
/// Shortest admin password accepted
pub const MIN_ADMIN_PASSWORD_LEN: usize = 12;

/// How the RBAC middleware treats requests without credentials.
///
/// The admin server inserts this into every request's extensions. A router
/// built without it (tests, embedders) gets [`AdminAccessPolicy::default`],
/// which requires credentials on every non-public route.
#[derive(Clone)]
pub struct AdminAccessPolicy {
    anonymous_loopback_reads: bool,
    accept_jwt: bool,
    management_token: Option<Arc<str>>,
}

impl Default for AdminAccessPolicy {
    fn default() -> Self {
        Self::authenticated(true, None)
    }
}

impl fmt::Debug for AdminAccessPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdminAccessPolicy")
            .field("anonymous_loopback_reads", &self.anonymous_loopback_reads)
            .field("accept_jwt", &self.accept_jwt)
            .field("management_token", &self.management_token.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl AdminAccessPolicy {
    /// Every non-public route needs credentials
    pub fn authenticated(accept_jwt: bool, management_token: Option<String>) -> Self {
        Self {
            anonymous_loopback_reads: false,
            accept_jwt,
            management_token: management_token.map(Arc::from),
        }
    }

    /// Loopback bind with no credentials configured
    pub fn loopback_dev() -> Self {
        Self {
            anonymous_loopback_reads: true,
            accept_jwt: true,
            management_token: None,
        }
    }

    /// Whether anonymous reads from a loopback `Host` are allowed
    pub fn allows_anonymous_loopback_reads(&self) -> bool {
        self.anonymous_loopback_reads
    }

    /// Whether admin UI JWTs (and the dev header fallback) are honoured
    pub fn accepts_jwt(&self) -> bool {
        self.accept_jwt
    }

    /// Whether `headers` carry the configured management token
    pub fn management_token_matches(&self, headers: &HeaderMap) -> bool {
        let Some(expected) = self.management_token.as_deref() else {
            return false;
        };
        let presented = headers
            .get(MANAGEMENT_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .or_else(|| bearer_token(headers));
        presented.is_some_and(|token| {
            !token.is_empty() && bool::from(token.as_bytes().ct_eq(expected.as_bytes()))
        })
    }

    /// Whether an anonymous request may read `path` under this policy
    pub fn allows_anonymous_read(&self, method: &Method, uri: &Uri, headers: &HeaderMap) -> bool {
        self.anonymous_loopback_reads
            && (method == Method::GET || method == Method::HEAD)
            && uri.path().starts_with("/__mockforge/")
            && host_is_loopback(uri, headers)
    }
}

/// The token in `Authorization: Bearer <token>`, if any
pub fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

/// Whether `ip` is a loopback address.
///
/// Only `127.0.0.0/8` and `::1`. The unspecified addresses (`0.0.0.0`, `::`)
/// listen on every interface, and an IPv4-mapped IPv6 address
/// (`::ffff:127.0.0.1`) is treated as non-loopback rather than guessed at.
pub fn is_loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

/// Whether the request names a loopback host.
///
/// A DNS-rebinding page reaches a loopback listener under its own hostname,
/// so anonymous reads also need the `Host` (or HTTP/2 authority) to be
/// `localhost`, a `127.0.0.0/8` address or `[::1]`.
fn host_is_loopback(uri: &Uri, headers: &HeaderMap) -> bool {
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .or_else(|| uri.authority().map(|a| a.as_str().to_string()));
    let Some(host) = host else {
        return false;
    };
    let host = host.trim();
    let name = if let Some(rest) = host.strip_prefix('[') {
        match rest.split_once(']') {
            Some((inner, port)) if port.is_empty() || port.starts_with(':') => inner,
            _ => return false,
        }
    } else {
        match host.rsplit_once(':') {
            Some((name, _port)) => name,
            None => host,
        }
    };
    if name.eq_ignore_ascii_case("localhost") {
        return true;
    }
    name.parse::<IpAddr>().is_ok_and(is_loopback)
}

/// The credentials the process environment provides
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdminAuthEnv {
    /// A usable `MOCKFORGE_MANAGEMENT_TOKEN`
    pub management_token: Option<String>,
    /// A usable `MOCKFORGE_ADMIN_PASSWORD`
    pub admin_password: bool,
    /// A usable `JWT_SECRET`
    pub jwt_secret: bool,
    /// Header-based dev auth or anonymous dev access is switched on
    pub dev_bypass: bool,
    /// Credentials that were set but rejected, with the reason
    pub rejected: Vec<String>,
}

fn env_truthy(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref().map(str::to_ascii_lowercase).as_deref(),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}

impl AdminAuthEnv {
    /// Read the credentials from the process environment
    pub fn from_process_env() -> Self {
        let mut env = Self::default();
        match std::env::var(MANAGEMENT_TOKEN_ENV) {
            Ok(t) if t.len() >= MIN_SECRET_LEN => env.management_token = Some(t),
            Ok(t) if !t.is_empty() => env.rejected.push(format!(
                "{MANAGEMENT_TOKEN_ENV} is shorter than {MIN_SECRET_LEN} characters"
            )),
            _ => {}
        }
        match std::env::var(ADMIN_PASSWORD_ENV) {
            Ok(p) if p.len() >= MIN_ADMIN_PASSWORD_LEN => env.admin_password = true,
            Ok(p) if !p.is_empty() => env.rejected.push(format!(
                "{ADMIN_PASSWORD_ENV} is shorter than {MIN_ADMIN_PASSWORD_LEN} characters"
            )),
            _ => {}
        }
        match std::env::var("JWT_SECRET") {
            Ok(s) if s.len() >= MIN_SECRET_LEN => env.jwt_secret = true,
            Ok(s) if !s.is_empty() => env
                .rejected
                .push(format!("JWT_SECRET is shorter than {MIN_SECRET_LEN} characters")),
            _ => {}
        }
        let dev_environment = matches!(
            std::env::var("ENVIRONMENT").unwrap_or_default().to_ascii_lowercase().as_str(),
            "development" | "dev" | "local"
        );
        env.dev_bypass = dev_environment
            && (env_truthy("MOCKFORGE_ENABLE_DEV_AUTH")
                || env_truthy("MOCKFORGE_ALLOW_UNAUTHENTICATED")
                || env_truthy("MOCKFORGE_ALLOW_UNAUTHENTICATED_DEV"));
        env
    }

    fn any_credentials(&self) -> bool {
        self.management_token.is_some() || self.admin_password || self.jwt_secret
    }
}

/// Decide the access policy for an admin server bound to `addr`.
///
/// Returns an error, and the server must not start, when `addr` is not
/// loopback and no credentials are configured, or when a dev bypass is on.
pub fn policy_for_bind(addr: SocketAddr, env: &AdminAuthEnv) -> Result<AdminAccessPolicy, String> {
    let loopback = is_loopback(addr.ip());
    if loopback {
        if env.any_credentials() {
            return Ok(AdminAccessPolicy::authenticated(true, env.management_token.clone()));
        }
        return Ok(AdminAccessPolicy::loopback_dev());
    }

    if env.dev_bypass {
        return Err(format!(
            "Refusing to start the admin UI on non-loopback address {addr}: development \
             auth bypass (MOCKFORGE_ENABLE_DEV_AUTH / MOCKFORGE_ALLOW_UNAUTHENTICATED) is \
             enabled. Unset it, or bind the admin UI to 127.0.0.1 with --admin-host."
        ));
    }
    if !env.any_credentials() {
        let rejected = if env.rejected.is_empty() {
            String::new()
        } else {
            format!(" ({})", env.rejected.join("; "))
        };
        return Err(format!(
            "Refusing to start the admin UI on non-loopback address {addr} without \
             credentials{rejected}. Set {ADMIN_PASSWORD_ENV} (at least \
             {MIN_ADMIN_PASSWORD_LEN} characters) to log in to the admin UI, or \
             {MANAGEMENT_TOKEN_ENV} (at least {MIN_SECRET_LEN} characters) for API \
             access, or bind the admin UI to 127.0.0.1 with --admin-host / \
             MOCKFORGE_ADMIN_HOST."
        ));
    }
    // With only a management token there is no admin login and no operator
    // JWT secret, so no admin UI JWT can be legitimate: refuse them.
    let accept_jwt = env.admin_password || env.jwt_secret;
    Ok(AdminAccessPolicy::authenticated(accept_jwt, env.management_token.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

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
}
