//! SSRF (Server-Side Request Forgery) guard for cloud-driven runs.
//!
//! When `mockforge-test-runner` (or any other cloud-side caller) accepts a
//! user-supplied target URL and dispatches HTTP traffic at it, an attacker
//! can point the runner at internal infrastructure: metadata endpoints
//! (`169.254.169.254`), localhost services, RFC1918 / ULA private ranges,
//! and so on. Without a guard, we hand them a free internal-network
//! scanner running inside our Fly.io org.
//!
//! [`validate_target_url`] is the single chokepoint. It:
//!
//! 1. Parses the URL and rejects anything that isn't `http://` or
//!    `https://` (no `file://`, `gopher://`, etc.).
//! 2. Resolves the hostname via DNS (asynchronously) and rejects if any
//!    resolved IP is in a blocked range.
//! 3. Treats literal-IP hostnames (`http://10.0.0.1/`) the same way —
//!    parsing them as `IpAddr` directly so DNS resolution isn't needed.
//!
//! Blocked ranges:
//!
//! * IPv4: loopback (`127.0.0.0/8`), link-local (`169.254.0.0/16` —
//!   includes the AWS/GCP/Fly metadata IP `169.254.169.254`), unspecified
//!   (`0.0.0.0/8`), broadcast, RFC1918 private (`10/8`, `172.16/12`,
//!   `192.168/16`), CGNAT (`100.64.0.0/10`), benchmark (`198.18/15`).
//! * IPv6: loopback (`::1`), unspecified (`::`), link-local (`fe80::/10`),
//!   ULA (`fc00::/7`), IPv4-mapped (`::ffff:0:0/96` — caller could smuggle
//!   in a private v4 address).
//!
//! There is no escape hatch — production callers MUST ensure their target
//! is publicly reachable. Tests can override with the `loopback-ok` env
//! var (see [`Policy::for_test`]) for integration-test endpoints on
//! `127.0.0.1`.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use thiserror::Error;

/// Reasons a target URL can be rejected by [`validate_target_url`].
#[derive(Debug, Error)]
pub enum SsrfError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("URL scheme '{0}' not allowed — only http/https")]
    DisallowedScheme(String),

    #[error("URL has no host component")]
    MissingHost,

    #[error("DNS resolution failed for '{host}': {source}")]
    DnsResolutionFailed {
        host: String,
        #[source]
        source: std::io::Error,
    },

    #[error("DNS resolution returned no addresses for '{0}'")]
    NoAddressesResolved(String),

    #[error(
        "target '{host}' resolves to {ip} which is in a blocked range ({reason}); \
         pointing the cloud runner at internal addresses is not allowed"
    )]
    BlockedAddress {
        host: String,
        ip: IpAddr,
        reason: &'static str,
    },
}

/// Knobs for [`validate_target_url`]. Default policy is the strict
/// production one; tests can relax it via [`Policy::allow_loopback`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Policy {
    /// When true, addresses in the IPv4/IPv6 loopback range are allowed.
    /// Production callers should leave this `false`. Tests against
    /// `127.0.0.1` set it via [`Policy::for_test`].
    pub allow_loopback: bool,
}

impl Policy {
    /// Strict production policy: nothing private, nothing local.
    pub const fn strict() -> Self {
        Self {
            allow_loopback: false,
        }
    }

    /// Test policy: allows loopback so integration tests against
    /// `127.0.0.1:<port>` mock servers can run.
    pub const fn for_test() -> Self {
        Self {
            allow_loopback: true,
        }
    }
}

/// Validate that a URL is safe for a cloud runner to hit. Returns `Ok(())`
/// when the target is a publicly-routable HTTP/S endpoint.
///
/// Performs DNS resolution, so this is async. Cache results at the call
/// site if the same URL is validated repeatedly within one request.
pub async fn validate_target_url(url: &str, policy: Policy) -> Result<(), SsrfError> {
    let parsed = url::Url::parse(url).map_err(|e| SsrfError::InvalidUrl(e.to_string()))?;

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(SsrfError::DisallowedScheme(scheme.to_string()));
    }

    let host = parsed.host_str().ok_or(SsrfError::MissingHost)?.to_string();
    let port = parsed.port_or_known_default().unwrap_or(80);

    // Literal IP — no DNS needed.
    if let Ok(ip) = host.parse::<IpAddr>() {
        check_ip(&host, ip, policy)?;
        return Ok(());
    }

    // Hostname — resolve and check every resolved address. An attacker
    // can register a public DNS name pointing at 169.254.169.254 (a.k.a.
    // "DNS rebinding" / "0.0.0.0 day"), so we MUST inspect resolved IPs,
    // not just the literal-IP form.
    let lookup_target = format!("{}:{}", host, port);
    let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host(&lookup_target)
        .await
        .map_err(|source| SsrfError::DnsResolutionFailed {
            host: host.clone(),
            source,
        })?
        .collect();

    if addrs.is_empty() {
        return Err(SsrfError::NoAddressesResolved(host));
    }

    for addr in addrs {
        check_ip(&host, addr.ip(), policy)?;
    }

    Ok(())
}

/// Check a URL without DNS: the scheme must be http/https, and a literal-IP
/// host must not be in a blocked range. Hostnames pass here; a client from
/// [`guarded_client_builder`] checks what they resolve to at connect time.
///
/// reqwest never calls the DNS resolver for a literal IP, so this is the half
/// of the guard the resolver cannot do. Run it on every URL handed to a
/// guarded client; the redirect policy runs it on every hop.
pub fn check_url(url: &url::Url, policy: Policy) -> Result<(), SsrfError> {
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(SsrfError::DisallowedScheme(scheme.to_string()));
    }
    match url.host() {
        None => Err(SsrfError::MissingHost),
        Some(url::Host::Ipv4(ip)) => check_ip(&ip.to_string(), IpAddr::V4(ip), policy),
        Some(url::Host::Ipv6(ip)) => check_ip(&ip.to_string(), IpAddr::V6(ip), policy),
        Some(url::Host::Domain(_)) => Ok(()),
    }
}

/// reqwest DNS resolver that fails the lookup when any resolved address is in
/// a blocked range. Checking at connect time, on the addresses the connection
/// will actually use, closes the DNS-rebinding gap that a separate
/// [`validate_target_url`] pre-check leaves open.
#[derive(Debug, Clone, Copy)]
pub struct GuardedResolver {
    policy: Policy,
}

impl GuardedResolver {
    /// Resolver enforcing `policy`.
    pub const fn new(policy: Policy) -> Self {
        Self { policy }
    }
}

impl reqwest::dns::Resolve for GuardedResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let policy = self.policy;
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await
                .map_err(|source| SsrfError::DnsResolutionFailed {
                    host: host.clone(),
                    source,
                })?
                .collect();
            if addrs.is_empty() {
                return Err(SsrfError::NoAddressesResolved(host).into());
            }
            for addr in &addrs {
                check_ip(&host, addr.ip(), policy)?;
            }
            let addrs: reqwest::dns::Addrs = Box::new(addrs.into_iter());
            Ok(addrs)
        })
    }
}

/// A reqwest redirect policy that follows at most `max` redirects and runs
/// [`check_url`] on every hop, so a public target cannot bounce the client to
/// `http://10.0.0.1/`.
pub fn guarded_redirect_policy(policy: Policy, max: usize) -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() >= max {
            return attempt.error(format!("more than {max} redirects"));
        }
        match check_url(attempt.url(), policy) {
            Ok(()) => attempt.follow(),
            Err(e) => attempt.error(e),
        }
    })
}

/// A reqwest client builder whose connections can only reach addresses the
/// policy allows: the [`GuardedResolver`], a [`guarded_redirect_policy`] of 10
/// hops, and no proxy (a proxy would resolve the target itself, out of the
/// resolver's sight). Callers must still run [`check_url`] on each URL for the
/// literal-IP case, and must not replace the resolver, redirect policy, or
/// proxy setting on the returned builder.
pub fn guarded_client_builder(policy: Policy) -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .no_proxy()
        .dns_resolver(Arc::new(GuardedResolver::new(policy)))
        .redirect(guarded_redirect_policy(policy, 10))
}

fn check_ip(host: &str, ip: IpAddr, policy: Policy) -> Result<(), SsrfError> {
    if let Some(reason) = blocked_reason(ip, policy) {
        return Err(SsrfError::BlockedAddress {
            host: host.to_string(),
            ip,
            reason,
        });
    }
    Ok(())
}

fn blocked_reason(ip: IpAddr, policy: Policy) -> Option<&'static str> {
    match ip {
        IpAddr::V4(v4) => blocked_reason_v4(v4, policy),
        IpAddr::V6(v6) => blocked_reason_v6(v6, policy),
    }
}

fn blocked_reason_v4(ip: Ipv4Addr, policy: Policy) -> Option<&'static str> {
    if ip.is_loopback() {
        if policy.allow_loopback {
            return None;
        }
        return Some("IPv4 loopback (127.0.0.0/8)");
    }
    if ip.is_unspecified() {
        return Some("IPv4 unspecified (0.0.0.0)");
    }
    if ip.is_broadcast() {
        return Some("IPv4 broadcast");
    }
    if ip.is_link_local() {
        return Some("IPv4 link-local (169.254.0.0/16, includes cloud metadata IP)");
    }
    if ip.is_private() {
        return Some("IPv4 RFC1918 private (10/8, 172.16/12, 192.168/16)");
    }
    if ip.is_documentation() {
        return Some("IPv4 documentation range (RFC5737)");
    }
    // CGNAT range (100.64.0.0/10) — not is_private but still
    // not-publicly-routable. Cloud providers sometimes use it for inter-VM
    // links, which is exactly the kind of thing we don't want to expose.
    let octets = ip.octets();
    if octets[0] == 100 && (64..=127).contains(&octets[1]) {
        return Some("IPv4 CGNAT (100.64.0.0/10)");
    }
    // Benchmark range 198.18.0.0/15 (RFC2544).
    if octets[0] == 198 && (octets[1] == 18 || octets[1] == 19) {
        return Some("IPv4 benchmark (198.18.0.0/15)");
    }
    None
}

fn blocked_reason_v6(ip: Ipv6Addr, policy: Policy) -> Option<&'static str> {
    if ip.is_loopback() {
        if policy.allow_loopback {
            return None;
        }
        return Some("IPv6 loopback (::1)");
    }
    if ip.is_unspecified() {
        return Some("IPv6 unspecified (::)");
    }
    let segments = ip.segments();
    // Link-local fe80::/10
    if (segments[0] & 0xffc0) == 0xfe80 {
        return Some("IPv6 link-local (fe80::/10)");
    }
    // ULA fc00::/7
    if (segments[0] & 0xfe00) == 0xfc00 {
        return Some("IPv6 unique-local (fc00::/7)");
    }
    // IPv4-mapped ::ffff:0:0/96 — recurse so an attacker can't smuggle a
    // private v4 through the v6 form (`http://[::ffff:10.0.0.1]/`).
    if let Some(v4) = ip.to_ipv4_mapped() {
        return blocked_reason_v4(v4, policy);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_blocked(addr: &str, policy: Policy, fragment: &str) {
        let ip: IpAddr = addr.parse().unwrap();
        let reason =
            blocked_reason(ip, policy).unwrap_or_else(|| panic!("expected {addr} to be blocked"));
        assert!(
            reason.contains(fragment),
            "{addr} blocked but reason '{reason}' missing fragment '{fragment}'"
        );
    }

    fn assert_allowed(addr: &str, policy: Policy) {
        let ip: IpAddr = addr.parse().unwrap();
        assert!(blocked_reason(ip, policy).is_none(), "{addr} unexpectedly blocked");
    }

    #[test]
    fn blocks_loopback_v4_strict() {
        assert_blocked("127.0.0.1", Policy::strict(), "loopback");
        assert_blocked("127.255.255.254", Policy::strict(), "loopback");
    }

    #[test]
    fn allows_loopback_v4_in_test_policy() {
        assert_allowed("127.0.0.1", Policy::for_test());
    }

    #[test]
    fn blocks_link_local_aws_metadata() {
        assert_blocked("169.254.169.254", Policy::strict(), "link-local");
    }

    #[test]
    fn blocks_rfc1918_ranges() {
        assert_blocked("10.0.0.1", Policy::strict(), "RFC1918");
        assert_blocked("172.16.0.1", Policy::strict(), "RFC1918");
        assert_blocked("172.31.255.255", Policy::strict(), "RFC1918");
        assert_blocked("192.168.0.1", Policy::strict(), "RFC1918");
    }

    #[test]
    fn blocks_cgnat() {
        assert_blocked("100.64.0.1", Policy::strict(), "CGNAT");
        assert_blocked("100.127.255.255", Policy::strict(), "CGNAT");
    }

    #[test]
    fn allows_ranges_outside_cgnat() {
        // 100.0.0.0/8 outside the CGNAT slice is publicly routable.
        assert_allowed("100.63.255.255", Policy::strict());
        assert_allowed("100.128.0.1", Policy::strict());
    }

    #[test]
    fn blocks_benchmark_range() {
        assert_blocked("198.18.0.1", Policy::strict(), "benchmark");
        assert_blocked("198.19.255.255", Policy::strict(), "benchmark");
    }

    #[test]
    fn allows_public_v4() {
        assert_allowed("8.8.8.8", Policy::strict());
        assert_allowed("1.1.1.1", Policy::strict());
        assert_allowed("142.250.190.78", Policy::strict()); // google.com
    }

    #[test]
    fn blocks_loopback_v6_strict() {
        assert_blocked("::1", Policy::strict(), "loopback");
    }

    #[test]
    fn blocks_link_local_v6() {
        assert_blocked("fe80::1", Policy::strict(), "link-local");
        assert_blocked("febf::1", Policy::strict(), "link-local");
    }

    #[test]
    fn blocks_ula() {
        assert_blocked("fc00::1", Policy::strict(), "unique-local");
        assert_blocked("fd12:3456::1", Policy::strict(), "unique-local");
    }

    #[test]
    fn blocks_ipv4_mapped_private() {
        assert_blocked("::ffff:10.0.0.1", Policy::strict(), "RFC1918");
        assert_blocked("::ffff:127.0.0.1", Policy::strict(), "loopback");
    }

    #[test]
    fn allows_public_v6() {
        assert_allowed("2606:4700:4700::1111", Policy::strict()); // cloudflare
        assert_allowed("2001:4860:4860::8888", Policy::strict()); // google
    }

    #[tokio::test]
    async fn validate_rejects_non_http_scheme() {
        let err = validate_target_url("file:///etc/passwd", Policy::strict()).await.unwrap_err();
        assert!(matches!(err, SsrfError::DisallowedScheme(s) if s == "file"));
    }

    #[tokio::test]
    async fn validate_rejects_garbage_url() {
        let err = validate_target_url("not a url", Policy::strict()).await.unwrap_err();
        assert!(matches!(err, SsrfError::InvalidUrl(_)));
    }

    #[tokio::test]
    async fn validate_rejects_literal_loopback() {
        let err = validate_target_url("http://127.0.0.1/", Policy::strict()).await.unwrap_err();
        assert!(matches!(err, SsrfError::BlockedAddress { .. }));
    }

    #[tokio::test]
    async fn validate_rejects_literal_metadata_ip() {
        let err = validate_target_url("http://169.254.169.254/latest/meta-data/", Policy::strict())
            .await
            .unwrap_err();
        match err {
            SsrfError::BlockedAddress { reason, .. } => assert!(reason.contains("link-local")),
            other => panic!("expected BlockedAddress, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn validate_rejects_literal_rfc1918() {
        let err = validate_target_url("http://10.0.0.1/", Policy::strict()).await.unwrap_err();
        assert!(matches!(err, SsrfError::BlockedAddress { .. }));
    }

    #[tokio::test]
    async fn validate_allows_loopback_in_test_policy() {
        validate_target_url("http://127.0.0.1:8080/", Policy::for_test()).await.unwrap();
    }

    fn url(s: &str) -> url::Url {
        url::Url::parse(s).unwrap()
    }

    #[test]
    fn check_url_blocks_literal_ips_in_every_spelling() {
        for u in [
            "http://127.0.0.1/",
            "http://2130706433/",
            "http://0x7f000001/",
            "http://10.0.0.1:6379/",
            "http://[::1]/",
            "http://[::ffff:172.18.0.2]/",
            "http://169.254.169.254/latest/meta-data/",
            "http://example.com@10.0.0.1/",
        ] {
            let err = check_url(&url(u), Policy::strict()).unwrap_err();
            assert!(matches!(err, SsrfError::BlockedAddress { .. }), "{u}: {err:?}");
        }
    }

    #[test]
    fn check_url_rejects_other_schemes_and_defers_hostnames_to_the_resolver() {
        let err = check_url(&url("file:///etc/passwd"), Policy::strict()).unwrap_err();
        assert!(matches!(err, SsrfError::DisallowedScheme(_)));
        check_url(&url("https://example.com/"), Policy::strict()).unwrap();
        check_url(&url("http://127.0.0.1:3000/"), Policy::for_test()).unwrap();
    }

    /// One-shot HTTP server on 127.0.0.1 that answers every connection with
    /// `response`. Returns its port.
    async fn serve(response: &'static str) -> u16 {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = sock.read(&mut buf).await;
                let _ = sock.write_all(response.as_bytes()).await;
            }
        });
        port
    }

    const OK: &str = "HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok";

    #[tokio::test]
    async fn guarded_client_refuses_a_name_resolving_to_loopback() {
        let port = serve(OK).await;
        let client = guarded_client_builder(Policy::strict()).build().unwrap();
        let err = client.get(format!("http://localhost:{port}/")).send().await.unwrap_err();
        assert!(err.is_connect(), "{err:?}");

        let relaxed = guarded_client_builder(Policy::for_test()).build().unwrap();
        let resp = relaxed.get(format!("http://localhost:{port}/")).send().await.unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn guarded_client_refuses_a_redirect_to_a_private_address() {
        let port = serve(
            "HTTP/1.1 302 Found\r\nlocation: http://10.0.0.1/\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        )
        .await;
        // Loopback allowed so the first hop connects; RFC1918 never is.
        let client = guarded_client_builder(Policy::for_test()).build().unwrap();
        let err = client.get(format!("http://127.0.0.1:{port}/")).send().await.unwrap_err();
        assert!(err.is_redirect(), "{err:?}");
    }
}
