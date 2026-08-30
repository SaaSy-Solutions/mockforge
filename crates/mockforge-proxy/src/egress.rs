//! Upstream egress guard (SSRF mitigation, #1012 / MF-002).
//!
//! The browser/mobile proxy strips a configurable prefix (default
//! `/proxy/`) from request paths; anything that remains and parses as an
//! absolute `http(s)://` URL used to be forwarded verbatim to an
//! attacker-controlled host (CWE-918). This module provides the second
//! layer of defense for URLs that *are* allowed through (explicit opt-in):
//!
//! - denylisted IP ranges (loopback, RFC1918, link-local 169.254.0.0/16,
//!   cloud-metadata endpoints, IPv4-mapped IPv6 equivalents, `::1`,
//!   unique-local and link-local IPv6),
//! - denylisted cloud-metadata hostnames,
//! - DNS resolution before connecting, so a hostname that rebinding
//!   attacks point at private space is caught before any socket is
//!   opened.
//!
//! The guard applies only to URLs derived from the *request path*
//! (attacker-controlled). Operator-configured upstreams (`target_url`,
//! rule targets) are trusted configuration and are not re-checked.

use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Hostname denylist for cloud metadata services.
const BLOCKED_METADATA_HOSTS: &[&str] = &[
    "metadata.google.internal",
    "metadata.goog",
    "metadata",
    "instance-data",
    "instance-data.ec2.internal",
];

/// Link-local metadata service endpoints (AWS/GCP/Azure IMDS, Alibaba IMDS,
/// and AWS IMDSv2 over IPv6).
const BLOCKED_METADATA_IPS: &[IpAddr] = &[
    IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
    IpAddr::V4(Ipv4Addr::new(100, 100, 100, 200)), // Alibaba Cloud IMDS
    IpAddr::V6(Ipv6Addr::new(0xfd00, 0xec2, 0, 0, 0, 0, 0, 0x254)), // AWS IMDSv6
];

/// Why an upstream URL was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EgressError {
    /// URL could not be parsed at all.
    InvalidUrl(String),
    /// Host resolved to (or was literally) a blocked address.
    BlockedAddress { host: String, ip: IpAddr },
    /// Host is on the cloud-metadata hostname denylist.
    BlockedHost(String),
}

impl std::fmt::Display for EgressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EgressError::InvalidUrl(u) => write!(f, "invalid upstream URL: {}", u),
            EgressError::BlockedAddress { host, ip } => {
                write!(f, "upstream host {} resolves to blocked address {}", host, ip)
            }
            EgressError::BlockedHost(h) => write!(f, "upstream host {} is blocked", h),
        }
    }
}

impl std::error::Error for EgressError {}

/// Optional explicit allowlist for upstreams. When present it overrides the
/// egress denylist: a URL matching one of these prefixes/hosts is proxied
/// even if it would otherwise be blocked.
///
/// # Security
/// Only set entries here for hosts you genuinely intend the proxy to reach;
/// an allowlist entry pointing at loopback/private space re-opens SSRF by
/// design.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UpstreamAllowlist {
    /// URL prefixes to allow verbatim, e.g. `http://api.example.com/`.
    #[serde(default)]
    pub url_prefixes: Vec<String>,
    /// Bare hostnames (no scheme/port) to allow regardless of address,
    /// e.g. `api.example.com`.
    #[serde(default)]
    pub hosts: Vec<String>,
}

impl UpstreamAllowlist {
    fn allows(&self, url: &url::Url) -> bool {
        let Some(host) = url.host_str() else {
            return false;
        };
        if self.hosts.iter().any(|h| h.eq_ignore_ascii_case(host)) {
            return true;
        }
        self.url_prefixes.iter().any(|p| url.as_str().starts_with(p))
    }
}

/// Result of a pre-flight check on an upstream URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EgressDecision {
    /// Forward as requested.
    Allowed,
    /// Refuse with this reason.
    Blocked(EgressError),
}

/// Egress guard over request-derived upstream URLs.
#[derive(Debug, Clone, Default)]
pub struct EgressGuard {
    allowlist: Option<UpstreamAllowlist>,
}

impl EgressGuard {
    pub fn new(allowlist: Option<UpstreamAllowlist>) -> Self {
        Self { allowlist }
    }

    /// Synchronous checks that need no network: literal IPs, metadata
    /// hostnames, and allowlist matching.
    ///
    /// Exposed separately so callers (and tests) can exercise the pure
    /// logic without DNS.
    pub fn check_without_dns(&self, target: &str) -> EgressDecision {
        let url = match target.parse::<url::Url>() {
            Ok(u) => u,
            Err(_) => return EgressDecision::Blocked(EgressError::InvalidUrl(target.to_string())),
        };
        // Explicit operator opt-in wins over every denylist entry.
        if self.allowlist.as_ref().is_some_and(|a| a.allows(&url)) {
            return EgressDecision::Allowed;
        }

        match url.host() {
            Some(url::Host::Domain(domain)) => {
                let normalized = domain.trim_end_matches('.').to_ascii_lowercase();
                if BLOCKED_METADATA_HOSTS.contains(&normalized.as_str()) {
                    return EgressDecision::Blocked(EgressError::BlockedHost(normalized));
                }
                EgressDecision::Allowed
            }
            Some(url::Host::Ipv4(ip)) => {
                if is_blocked_ip(IpAddr::V4(ip)) {
                    EgressDecision::Blocked(EgressError::BlockedAddress {
                        host: url.host_str().unwrap_or_default().to_string(),
                        ip: IpAddr::V4(ip),
                    })
                } else {
                    EgressDecision::Allowed
                }
            }
            Some(url::Host::Ipv6(ip)) => {
                if is_blocked_ip(IpAddr::V6(ip)) {
                    EgressDecision::Blocked(EgressError::BlockedAddress {
                        host: url.host_str().unwrap_or_default().to_string(),
                        ip: IpAddr::V6(ip),
                    })
                } else {
                    EgressDecision::Allowed
                }
            }
            None => EgressDecision::Blocked(EgressError::InvalidUrl(target.to_string())),
        }
    }

    /// Full check with an injected resolver: synchronous rules first, then
    /// resolve non-literal hostnames and re-check every returned address.
    ///
    /// Resolution closes the DNS-rebinding window between check and
    /// connect: whatever addresses the name currently points at must all
    /// be public. Residual risk: the OS resolver could answer differently
    /// when reqwest dials; pinning the checked address onto the connection
    /// is out of scope here.
    pub async fn check_with_resolver<R, F>(&self, target: &str, resolver: R) -> EgressDecision
    where
        R: FnOnce(String) -> F,
        F: Future<Output = std::io::Result<Vec<IpAddr>>>,
    {
        match self.check_without_dns(target) {
            blocked @ EgressDecision::Blocked(_) => return blocked,
            EgressDecision::Allowed => {}
        }

        let Ok(url) = target.parse::<url::Url>() else {
            return EgressDecision::Blocked(EgressError::InvalidUrl(target.to_string()));
        };
        // Allowlisted entries skip resolution entirely.
        if self.allowlist.as_ref().is_some_and(|a| a.allows(&url)) {
            return EgressDecision::Allowed;
        }
        let Some(url::Host::Domain(domain)) = url.host() else {
            // Literal IP already validated above.
            return EgressDecision::Allowed;
        };

        let domain = domain.trim_end_matches('.').to_ascii_lowercase();
        match resolver(domain).await {
            Ok(ips) => {
                for ip in ips {
                    if is_blocked_ip(ip) {
                        return EgressDecision::Blocked(EgressError::BlockedAddress {
                            host: url.host_str().unwrap_or_default().to_string(),
                            ip,
                        });
                    }
                }
                EgressDecision::Allowed
            }
            // Cannot verify the name points at public space: fail closed.
            Err(e) => EgressDecision::Blocked(EgressError::InvalidUrl(format!(
                "{} (DNS resolution failed: {})",
                target, e
            ))),
        }
    }

    /// Full check using blocking system DNS off the async runtime's core
    /// threads.
    pub async fn check(&self, target: &str) -> EgressDecision {
        self.check_with_resolver(target, |host| async move {
            tokio::task::spawn_blocking(move || {
                use std::net::ToSocketAddrs;
                Ok((host.as_str(), 0u16).to_socket_addrs()?.map(|sa| sa.ip()).collect::<Vec<_>>())
            })
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?
        })
        .await
    }
}

/// True when the address is in a range the proxy must never dial.
pub fn is_blocked_ip(ip: IpAddr) -> bool {
    if BLOCKED_METADATA_IPS.contains(&ip) {
        return true;
    }
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        IpAddr::V6(v6) => {
            // ::1 and the unspecified address.
            if v6.is_loopback() || v6.is_unspecified() {
                return true;
            }
            // IPv4-mapped / IPv4-compatible: evaluate the embedded v4.
            if let Some(embedded) = v6.to_ipv4_mapped() {
                return is_blocked_ipv4(embedded);
            }
            if let Some(embedded) = embedded_v4_compat(v6) {
                return is_blocked_ipv4(embedded);
            }
            // fc00::/7 unique-local, fe80::/10 link-local.
            (v6.segments()[0] & 0xfe00) == 0xfc00 || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

fn is_blocked_ipv4(v4: Ipv4Addr) -> bool {
    let o = v4.octets();
    v4.is_loopback()
        || v4.is_private() // RFC1918: 10/8, 172.16/12, 192.168/16
        || v4.is_link_local() // 169.254/16 (incl. cloud metadata)
        || o[0] == 0 // 0.0.0.0/8 "this network"
}

/// `to_ipv4_mapped` covers `::ffff:a.b.c.d`; some stacks also accept the
/// deprecated IPv4-compatible form `::a.b.c.d`. Treat both as v4.
fn embedded_v4_compat(v6: Ipv6Addr) -> Option<Ipv4Addr> {
    let segs = v6.segments();
    if segs[0..5] == [0, 0, 0, 0, 0] && segs[5] == 0 && !(segs[6] == 0 && segs[7] <= 1) {
        Some(Ipv4Addr::new(
            (segs[6] >> 8) as u8,
            segs[6] as u8,
            (segs[7] >> 8) as u8,
            segs[7] as u8,
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_ipv4_is_blocked() {
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 10, 1))));
    }

    #[test]
    fn link_local_rfc1918_loopback_blocked() {
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 5))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
    }

    #[test]
    fn ipv6_equivalents_blocked() {
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        // ::ffff:169.254.169.254
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xa9fe, 0xa9fe))));
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::new(0xfd00, 0xec2, 0, 0, 0, 0, 0, 0x254))));
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1))));
    }

    #[test]
    fn public_ips_allowed() {
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))));
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_blocked_ip(IpAddr::V6(Ipv6Addr::new(
            0x2606, 0x2800, 0x220, 0x1, 0x248, 0x1893, 0x25c8, 0x1946
        ))));
    }

    #[test]
    fn guard_blocks_metadata_url() {
        let guard = EgressGuard::new(None);
        assert_eq!(
            guard.check_without_dns("http://169.254.169.254/latest/meta-data/"),
            EgressDecision::Blocked(EgressError::BlockedAddress {
                host: "169.254.169.254".to_string(),
                ip: IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
            })
        );
    }

    #[test]
    fn guard_blocks_metadata_hostnames() {
        let guard = EgressGuard::new(None);
        assert!(matches!(
            guard.check_without_dns("http://metadata.google.internal/computeMetadata/v1/"),
            EgressDecision::Blocked(EgressError::BlockedHost(_))
        ));
        assert!(matches!(
            guard.check_without_dns("http://METADATA.goog/foo"),
            EgressDecision::Blocked(EgressError::BlockedHost(_))
        ));
    }

    #[test]
    fn guard_allows_public_literal() {
        let guard = EgressGuard::new(None);
        assert_eq!(guard.check_without_dns("https://93.184.216.34/x"), EgressDecision::Allowed);
    }

    #[test]
    fn allowlist_overrides_blocklist() {
        let guard = EgressGuard::new(Some(UpstreamAllowlist {
            url_prefixes: vec!["http://169.254.169.254/".to_string()],
            hosts: Vec::new(),
        }));
        assert_eq!(
            guard.check_without_dns("http://169.254.169.254/latest/meta-data/"),
            EgressDecision::Allowed
        );
    }

    #[tokio::test]
    async fn dns_rebinding_to_private_is_blocked() {
        let guard = EgressGuard::new(None);
        let decision = guard
            .check_with_resolver("http://evil.example.com/", |host| async move {
                assert_eq!(host, "evil.example.com");
                Ok(vec![
                    IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
                    IpAddr::V4(Ipv4Addr::LOCALHOST),
                ])
            })
            .await;
        assert_eq!(
            decision,
            EgressDecision::Blocked(EgressError::BlockedAddress {
                host: "evil.example.com".to_string(),
                ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            })
        );
    }

    #[tokio::test]
    async fn dns_failure_fails_closed() {
        let guard = EgressGuard::new(None);
        let decision = guard
            .check_with_resolver("http://nx.example.com/", |_host| async move {
                Err(std::io::Error::other("nx"))
            })
            .await;
        assert!(matches!(decision, EgressDecision::Blocked(EgressError::InvalidUrl(_))));
    }
}
