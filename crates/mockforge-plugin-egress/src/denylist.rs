//! Hard denylist that takes precedence over the per-plugin allowlist.
//!
//! Even if an org admin grants `*` egress to a plugin, traffic to
//! these targets is still blocked. Mirrors the hard-deny set from
//! `cloud-trust-permissions-rfc.md` §5.3.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use ipnet::{Ipv4Net, Ipv6Net};

/// Hostnames that are never allowed regardless of the plugin's
/// allowlist. These resolve to cloud metadata services and are a
/// classic exfiltration target.
const DENY_HOSTS: &[&str] = &[
    "metadata.google.internal",
    "metadata.googleapis.com",
    "instance-data.ec2.internal",
];

/// Suffixes that are denied — catches subdomains of MockForge's own
/// control plane so a compromised plugin can't loop back.
const DENY_SUFFIXES: &[&str] = &[
    ".mockforge.dev", // app.mockforge.dev, registry.mockforge.dev, etc.
    ".fly.dev",       // any Fly app, including other tenants'
    ".internal",      // Fly's internal DNS domain
    ".flycast",       // Fly's flycast addresses
];

/// Whether `host` (a hostname or IP-literal) should be hard-denied
/// before any allowlist check runs. Returns the matching reason as
/// a stable string for audit logging.
pub fn is_denied_target(host: &str) -> Option<&'static str> {
    let host_lc = host.to_ascii_lowercase();

    // Exact-match hostname denylist.
    if DENY_HOSTS.iter().any(|h| host_lc == *h) {
        return Some("denied: cloud metadata hostname");
    }

    // Suffix-match denylist — catches subdomains.
    for suffix in DENY_SUFFIXES {
        if host_lc.ends_with(suffix) {
            return Some("denied: protected platform suffix");
        }
    }

    // IP-literal checks. If the "host" parses as an IP address,
    // walk the deny CIDRs.
    if let Ok(ip) = IpAddr::from_str(&host_lc) {
        if denied_ip(ip) {
            return Some("denied: reserved or internal IP range");
        }
    }

    None
}

/// Whether a resolved IP address falls in a denied CIDR block.
/// Called by the proxy *after* DNS resolution — even an
/// allowlisted hostname is rejected if its A/AAAA record points
/// into a private range. This catches `evil.example.com → 10.0.0.5`
/// rebinding tricks.
pub fn denied_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => denied_ipv4(v4),
        IpAddr::V6(v6) => denied_ipv6(v6),
    }
}

fn denied_ipv4(addr: Ipv4Addr) -> bool {
    let nets: &[Ipv4Net] = &[
        // RFC1918 private use
        "10.0.0.0/8".parse().unwrap(),
        "172.16.0.0/12".parse().unwrap(),
        "192.168.0.0/16".parse().unwrap(),
        // Loopback
        "127.0.0.0/8".parse().unwrap(),
        // Link-local — covers 169.254.169.254 (cloud metadata)
        "169.254.0.0/16".parse().unwrap(),
        // Multicast + reserved
        "224.0.0.0/4".parse().unwrap(),
        "240.0.0.0/4".parse().unwrap(),
        // Carrier-grade NAT
        "100.64.0.0/10".parse().unwrap(),
        // RFC1122 "this network"
        "0.0.0.0/8".parse().unwrap(),
    ];
    nets.iter().any(|n| n.contains(&addr))
}

fn denied_ipv6(addr: Ipv6Addr) -> bool {
    let nets: &[Ipv6Net] = &[
        // Loopback
        "::1/128".parse().unwrap(),
        // Link-local
        "fe80::/10".parse().unwrap(),
        // Unique-local addresses
        "fc00::/7".parse().unwrap(),
        // Multicast
        "ff00::/8".parse().unwrap(),
        // IPv4-mapped — block these too because the underlying v4
        // address might still be private. Stricter parsing of the
        // mapped form happens in the proxy connect path.
        "::ffff:0:0/96".parse().unwrap(),
        // Discard prefix
        "100::/64".parse().unwrap(),
    ];
    nets.iter().any(|n| n.contains(&addr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_hostname_is_denied() {
        assert!(is_denied_target("metadata.google.internal").is_some());
        assert!(is_denied_target("METADATA.GOOGLE.INTERNAL").is_some()); // case-insensitive
    }

    #[test]
    fn fly_dev_subdomain_is_denied() {
        assert!(is_denied_target("anyone.fly.dev").is_some());
        assert!(is_denied_target("evil.subdomain.fly.dev").is_some());
    }

    #[test]
    fn mockforge_dev_subdomain_is_denied() {
        assert!(is_denied_target("registry.mockforge.dev").is_some());
        assert!(is_denied_target("app.mockforge.dev").is_some());
    }

    #[test]
    fn rfc1918_ipv4_is_denied() {
        for ip in &["10.0.0.5", "172.16.5.5", "192.168.1.1", "172.31.255.255"] {
            assert!(is_denied_target(ip).is_some(), "expected {} denied", ip);
        }
    }

    #[test]
    fn cloud_metadata_ipv4_is_denied() {
        assert!(is_denied_target("169.254.169.254").is_some());
    }

    #[test]
    fn loopback_is_denied() {
        assert!(is_denied_target("127.0.0.1").is_some());
        assert!(is_denied_target("::1").is_some());
    }

    #[test]
    fn ipv6_link_local_is_denied() {
        assert!(is_denied_target("fe80::1").is_some());
    }

    #[test]
    fn ipv6_unique_local_is_denied() {
        assert!(is_denied_target("fc00::1").is_some());
        assert!(is_denied_target("fd00:dead:beef::1").is_some());
    }

    #[test]
    fn public_hostname_passes_denylist() {
        assert!(is_denied_target("api.stripe.com").is_none());
        assert!(is_denied_target("github.com").is_none());
    }

    #[test]
    fn public_ip_passes_denylist() {
        // 8.8.8.8 (Google DNS) — public, not denied
        assert!(is_denied_target("8.8.8.8").is_none());
        // 1.1.1.1 (Cloudflare)
        assert!(is_denied_target("1.1.1.1").is_none());
    }

    #[test]
    fn just_under_172_16_is_public() {
        // 172.15.x.x is *not* RFC1918 — only 172.16.0.0/12 is.
        // This guards against a /16 confusion bug in the CIDR
        // table.
        assert!(!denied_ipv4("172.15.0.1".parse().unwrap()));
        assert!(denied_ipv4("172.16.0.1".parse().unwrap()));
        assert!(denied_ipv4("172.31.255.255".parse().unwrap()));
        assert!(!denied_ipv4("172.32.0.1".parse().unwrap()));
    }
}
