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
//!   default `admin`): the admin UI login, replacing the built-in demo users.
//! - `JWT_SECRET`: the operator signs admin JWTs themselves.

use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;

use axum::http::{header, HeaderMap, Method, Uri};
use subtle::ConstantTimeEq;

mod startup;
pub use startup::{policy_for_bind, AdminAuthEnv};

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

#[cfg(test)]
mod tests;
