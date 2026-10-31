//! Credential parsing, constant-time token matching, and request redaction.
use super::policy::{is_strict_loopback, ControlPlanePolicy, Mode};
use super::{Denial, MANAGEMENT_TOKEN_HEADER, WS_TOKEN_PROTOCOL_PREFIX};
use axum::http::{header, HeaderMap, HeaderValue};
use std::net::IpAddr;
use subtle::ConstantTimeEq;

impl ControlPlanePolicy {
    /// Whether the request presents the configured management token
    pub(super) fn token_matches(&self, headers: &HeaderMap) -> bool {
        let Mode::Credentials {
            token: Some(expected),
            ..
        } = &self.mode
        else {
            return false;
        };
        presented_tokens(headers)
            .any(|presented| bool::from(presented.as_bytes().ct_eq(expected.as_bytes())))
    }

    pub(super) fn admin_matches(&self, headers: &HeaderMap) -> bool {
        let Mode::Credentials {
            admin: Some(admin), ..
        } = &self.mode
        else {
            return false;
        };
        headers
            .get_all(header::AUTHORIZATION)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .any(|value| {
                crate::auth::admin_auth::basic_credentials_match(
                    value,
                    &admin.username,
                    &admin.password,
                )
            })
    }

    pub(super) fn authorize(&self, headers: &HeaderMap) -> Result<(), Denial> {
        match &self.mode {
            Mode::UnauthenticatedLoopback => {
                // A loopback bind is still reachable from a browser through
                // DNS rebinding; insist on a loopback Host header.
                if host_is_loopback(headers) {
                    Ok(())
                } else {
                    Err(Denial::NonLoopbackHost)
                }
            }
            Mode::Credentials {
                token: None,
                admin: None,
            } => Err(Denial::Locked),
            Mode::Credentials { .. } => {
                if self.token_matches(headers) || self.admin_matches(headers) {
                    Ok(())
                } else {
                    Err(Denial::MissingCredential)
                }
            }
        }
    }
}

fn host_is_loopback(headers: &HeaderMap) -> bool {
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        // HTTP/1.0 or a test client without Host: nothing to rebind.
        return true;
    };
    let host = host.trim();
    let name = if let Some(rest) = host.strip_prefix('[') {
        match rest.split_once(']') {
            Some((inner, _)) => inner,
            None => return false,
        }
    } else {
        host.rsplit_once(':').map_or(host, |(name, _)| name)
    };
    if name.eq_ignore_ascii_case("localhost") {
        return true;
    }
    name.parse::<IpAddr>().is_ok_and(is_strict_loopback)
}

/// Tokens presented on a request: the management header, a Bearer token,
/// and on a WebSocket upgrade the `mockforge-token.` subprotocol.
fn presented_tokens(headers: &HeaderMap) -> impl Iterator<Item = &str> {
    let from_header = headers
        .get_all(MANAGEMENT_TOKEN_HEADER)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(str::trim);
    let from_bearer = headers
        .get_all(header::AUTHORIZATION)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|v| v.strip_prefix("Bearer "))
        .map(str::trim);
    let upgrade = headers
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"));
    let from_protocol = headers
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter(move |_| upgrade)
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .filter_map(|p| p.trim().strip_prefix(WS_TOKEN_PROTOCOL_PREFIX));
    from_header.chain(from_bearer).chain(from_protocol).filter(|t| !t.is_empty())
}

/// Remove every place the management token can travel: its header, a
/// Bearer header carrying it, and `mockforge-token.` WebSocket subprotocols.
pub(super) fn strip_token(headers: &mut HeaderMap, policy: &ControlPlanePolicy) {
    headers.remove(MANAGEMENT_TOKEN_HEADER);
    if let Mode::Credentials {
        token: Some(expected),
        ..
    } = &policy.mode
    {
        let bearer_is_token = headers
            .get_all(header::AUTHORIZATION)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .filter_map(|v| v.strip_prefix("Bearer "))
            .any(|t| bool::from(t.trim().as_bytes().ct_eq(expected.as_bytes())));
        if bearer_is_token {
            headers.remove(header::AUTHORIZATION);
        }
    }
    let protocols: Vec<String> = headers
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    if protocols.iter().any(|p| p.starts_with(WS_TOKEN_PROTOCOL_PREFIX)) {
        headers.remove(header::SEC_WEBSOCKET_PROTOCOL);
        let kept: Vec<&str> = protocols
            .iter()
            .map(String::as_str)
            .filter(|p| !p.starts_with(WS_TOKEN_PROTOCOL_PREFIX))
            .collect();
        if let Ok(value) = HeaderValue::from_str(&kept.join(", ")) {
            if !kept.is_empty() {
                headers.insert(header::SEC_WEBSOCKET_PROTOCOL, value);
            }
        }
    }
}
