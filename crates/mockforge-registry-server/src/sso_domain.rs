//! SSO domain-ownership verification and issuer SSRF guard (Issue #833, #746, #778).
//!
//! Before an organization's SSO can JIT-provision a new user, or absorb an
//! already-existing account, for a given email, the org must prove it controls
//! that email's DNS domain. Without this gate, a malicious org owner who
//! configures SSO against an IdP they control can mint a (validly signed)
//! assertion for `victim@othercompany.com` and have MockForge either create a
//! pre-verified account squatting that email under their org, or silently add
//! the victim's real, existing account to their org. That is the cross-tenant
//! account-takeover the trust model (#746/#778) exists to prevent.
//!
//! Ownership is proven by a DNS TXT record:
//!
//! ```text
//!   _mockforge-verify.<domain>   TXT   "mockforge-verify=<org_id>"
//! ```
//!
//! The check runs live at provisioning time and **fails closed**: any DNS
//! error, missing record, or org-id mismatch rejects provisioning. Verifying
//! live (rather than persisting a "verified once" flag) means revoking the TXT
//! record immediately revokes the org's ability to provision that domain, and
//! there is no stored state to drift out of sync.
//!
//! This module also exposes an SSRF guard for OIDC issuer URLs so a malicious
//! issuer (loopback / link-local / private / metadata host) can never even be
//! stored on an SSO configuration, ahead of OIDC discovery landing (#746).

use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

use crate::error::ApiError;

/// DNS label prefixed to a domain to locate its MockForge ownership TXT record.
pub const VERIFICATION_TXT_PREFIX: &str = "_mockforge-verify";
/// Key half of the `key=value` TXT record an org publishes to claim a domain.
const VERIFICATION_TXT_KEY: &str = "mockforge-verify";

/// Lower-case, trim surrounding whitespace, and drop a trailing dot so domains
/// compare stably regardless of how the IdP or admin formatted them.
pub fn normalize_domain(domain: &str) -> String {
    domain.trim().trim_end_matches('.').to_ascii_lowercase()
}

/// The fully-qualified name whose TXT records prove ownership of `domain`.
pub fn verification_record_name(domain: &str) -> String {
    format!("{VERIFICATION_TXT_PREFIX}.{}", normalize_domain(domain))
}

/// The exact TXT value an org must publish to prove `org_id` owns the domain.
pub fn expected_txt_value(org_id: Uuid) -> String {
    format!("{VERIFICATION_TXT_KEY}={org_id}")
}

/// Extract the normalized domain portion of an email address.
///
/// Returns `None` for anything that is not a single-`@` address with a
/// dotted, non-empty domain (which the caller must treat as a rejection).
pub fn domain_of_email(email: &str) -> Option<String> {
    let (local, domain) = email.rsplit_once('@')?;
    if local.is_empty() {
        return None;
    }
    let domain = normalize_domain(domain);
    // A real mail domain has a dot and no stray '@'; reject otherwise so a
    // malformed assertion can never slip past the gate.
    if domain.is_empty() || domain.contains('@') || !domain.contains('.') {
        return None;
    }
    Some(domain)
}

/// Pure decision: do any of the TXT records found at the verification name
/// authorize `org_id`? Kept synchronous and side-effect-free so the gate logic
/// is unit-testable without a live resolver.
pub fn txt_records_authorize(records: &[String], org_id: Uuid) -> bool {
    let expected = expected_txt_value(org_id);
    records.iter().any(|r| r.trim() == expected)
}

/// Verifies that an org controls an email's DNS domain. Implemented over DNS in
/// production; the trait makes the gate testable with an injected fake.
#[async_trait::async_trait]
pub trait DomainOwnershipVerifier: Send + Sync {
    /// Returns `true` iff `email`'s domain is DNS-verified as owned by `org_id`.
    /// Implementations MUST fail closed (return `false`) on any error.
    async fn email_domain_verified(&self, org_id: Uuid, email: &str) -> bool;
}

/// Production verifier: looks up the ownership TXT record via the system DNS
/// resolver and fails closed on any error.
pub struct DnsDomainVerifier;

#[async_trait::async_trait]
impl DomainOwnershipVerifier for DnsDomainVerifier {
    async fn email_domain_verified(&self, org_id: Uuid, email: &str) -> bool {
        let Some(domain) = domain_of_email(email) else {
            tracing::warn!("SSO domain gate: assertion email has no usable domain; failing closed");
            return false;
        };
        let name = verification_record_name(&domain);
        match lookup_txt_records(&name).await {
            Ok(records) => {
                let authorized = txt_records_authorize(&records, org_id);
                if !authorized {
                    tracing::warn!(
                        org_id = %org_id,
                        domain = %domain,
                        "SSO domain gate: no authorizing TXT record at {name}; rejecting"
                    );
                }
                authorized
            }
            Err(e) => {
                tracing::warn!(
                    org_id = %org_id,
                    domain = %domain,
                    error = %e,
                    "SSO domain gate: TXT lookup failed; failing closed"
                );
                false
            }
        }
    }
}

/// Gate an SSO-provisioned `email` against `org_id`'s verified domains.
///
/// On rejection, returns an `InvalidRequest` whose message tells the admin
/// exactly which TXT record to publish, so a legitimate org can self-serve the
/// fix without a support ticket.
pub async fn assert_email_in_verified_domain(
    verifier: &dyn DomainOwnershipVerifier,
    org_id: Uuid,
    email: &str,
) -> Result<(), ApiError> {
    if verifier.email_domain_verified(org_id, email).await {
        return Ok(());
    }
    let record_name = domain_of_email(email)
        .map(|d| verification_record_name(&d))
        .unwrap_or_else(|| format!("{VERIFICATION_TXT_PREFIX}.<your-domain>"));
    Err(ApiError::InvalidRequest(format!(
        "SSO provisioning blocked: this organization has not verified ownership of the \
         email domain. Publish a DNS TXT record at \"{record_name}\" with value \
         \"{}\" and try again.",
        expected_txt_value(org_id),
    )))
}

/// Resolve TXT records for `name` using the system DNS resolver, joining the
/// chunked-string segments DNS uses so each entry is the single value an admin
/// put in their zone file. Mirrors the resolver setup used for tunnel domain
/// validation so behavior is consistent across the server.
async fn lookup_txt_records(name: &str) -> Result<Vec<String>, String> {
    use hickory_resolver::config::{ResolverConfig, CLOUDFLARE};
    use hickory_resolver::net::runtime::TokioRuntimeProvider;
    use hickory_resolver::proto::rr::{RData, RecordType};
    use hickory_resolver::TokioResolver;

    let builder = match TokioResolver::builder_tokio() {
        Ok(b) => b,
        Err(e) => {
            tracing::debug!(error = %e, "system resolv.conf unreadable; falling back to Cloudflare");
            TokioResolver::builder_with_config(
                ResolverConfig::udp_and_tcp(&CLOUDFLARE),
                TokioRuntimeProvider::default(),
            )
        }
    };
    let resolver = builder.build().map_err(|e| format!("resolver build failed: {e}"))?;

    let response = resolver.lookup(name, RecordType::TXT).await.map_err(|e| format!("{e}"))?;
    let mut out = Vec::new();
    for record in response.answers() {
        let RData::TXT(ref txt) = record.data else {
            continue;
        };
        let mut joined = String::new();
        for chunk in txt.txt_data.iter() {
            if let Ok(s) = std::str::from_utf8(chunk) {
                joined.push_str(s);
            }
        }
        if !joined.is_empty() {
            out.push(joined);
        }
    }
    Ok(out)
}

/// SSRF guard for an OIDC issuer URL. Returns `true` only for an `https` URL
/// whose host is not a loopback / private / link-local / unique-local /
/// unspecified address or a localhost-class name. Used to reject a malicious
/// issuer at config-create time, before any discovery fetch exists (#746).
pub fn issuer_url_is_safe(raw: &str) -> bool {
    let raw = raw.trim();
    // Only https, and reject embedded credentials outright.
    let Some(rest) = raw.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    // Split host from optional port, handling [IPv6] literals.
    let host = if let Some(after_bracket) = authority.strip_prefix('[') {
        match after_bracket.split(']').next() {
            Some(h) if !h.is_empty() => h,
            _ => return false,
        }
    } else {
        authority.split(':').next().unwrap_or("")
    };
    if host.is_empty() {
        return false;
    }
    !host_is_blocked(host)
}

/// Validate an OIDC issuer URL, returning a descriptive `InvalidRequest` when
/// it fails the SSRF guard.
pub fn validate_issuer_url(raw: &str) -> Result<(), ApiError> {
    if issuer_url_is_safe(raw) {
        Ok(())
    } else {
        Err(ApiError::InvalidRequest(
            "OIDC issuer URL must be an https URL pointing at a public host (loopback, \
             private, link-local, and metadata addresses are not allowed)."
                .to_string(),
        ))
    }
}

/// True if `host` is a name or literal IP we must never let SSO reach.
fn host_is_blocked(host: &str) -> bool {
    let host = host.trim().to_ascii_lowercase();
    // Name-based localhost / internal aliases.
    if host == "localhost"
        || host == "ip6-localhost"
        || host == "metadata.google.internal"
        || host.ends_with(".localhost")
        || host.ends_with(".internal")
        || host.ends_with(".local")
    {
        return true;
    }
    // Literal IPs: block loopback, private, link-local, unique-local, unspecified.
    if let Ok(ip) = host.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(v4) => ipv4_is_blocked(v4),
            IpAddr::V6(v6) => {
                // Unmap IPv4-mapped/compatible v6 (e.g. ::ffff:169.254.169.254)
                // and NAT64-embedded v4 so they're judged by the v4 rules
                // rather than slipping past the v6 checks.
                if let Some(v4) = v6.to_ipv4_mapped() {
                    return ipv4_is_blocked(v4);
                }
                let o = v6.octets();
                let is_nat64 = o[..4] == [0x00, 0x64, 0xff, 0x9b] && o[4..12] == [0; 8];
                if is_nat64 {
                    return ipv4_is_blocked(Ipv4Addr::new(o[12], o[13], o[14], o[15]));
                }
                v6.is_loopback()
                    || v6.is_unspecified()
                    // Unique-local fc00::/7.
                    || (o[0] & 0xfe) == 0xfc
                    // Link-local fe80::/10.
                    || (o[0] == 0xfe && (o[1] & 0xc0) == 0x80)
            }
        };
    }
    false
}

/// True if an IPv4 literal falls in a range SSO must never reach (loopback,
/// private, link-local incl. cloud metadata 169.254.169.254, CGNAT, etc.).
fn ipv4_is_blocked(v4: Ipv4Addr) -> bool {
    v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_unspecified()
        || v4.is_broadcast()
        || v4.is_documentation()
        // Carrier-grade NAT 100.64.0.0/10.
        || (v4.octets()[0] == 100 && (64..=127).contains(&v4.octets()[1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn org() -> Uuid {
        Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()
    }

    #[test]
    fn normalizes_domains() {
        assert_eq!(normalize_domain("  Example.COM. "), "example.com");
    }

    #[test]
    fn verification_record_name_is_prefixed_and_normalized() {
        assert_eq!(verification_record_name("Acme.Com"), "_mockforge-verify.acme.com");
    }

    #[test]
    fn expected_txt_value_embeds_org_id() {
        assert_eq!(
            expected_txt_value(org()),
            "mockforge-verify=11111111-1111-1111-1111-111111111111"
        );
    }

    #[test]
    fn domain_of_email_extracts_and_rejects_malformed() {
        assert_eq!(domain_of_email("alice@Acme.com").as_deref(), Some("acme.com"));
        assert_eq!(domain_of_email("alice@sub.acme.com").as_deref(), Some("sub.acme.com"));
        // No domain, no '@', empty local part, or dotless host => rejected.
        assert_eq!(domain_of_email("alice@"), None);
        assert_eq!(domain_of_email("alice"), None);
        assert_eq!(domain_of_email("@acme.com"), None);
        assert_eq!(domain_of_email("alice@localhost"), None);
    }

    #[test]
    fn txt_records_authorize_requires_exact_org_match() {
        let good = expected_txt_value(org());
        assert!(txt_records_authorize(std::slice::from_ref(&good), org()));
        // Tolerates surrounding whitespace from zone files.
        assert!(txt_records_authorize(&[format!("  {good}  ")], org()));
        // A different org's token must not authorize.
        let other = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
        assert!(!txt_records_authorize(&[expected_txt_value(other)], org()));
        // Unrelated TXT records never authorize.
        assert!(!txt_records_authorize(&["v=spf1 -all".to_string()], org()));
        // No records => not authorized (fail closed).
        assert!(!txt_records_authorize(&[], org()));
    }

    /// Fake verifier so the gate decision is exercised without DNS.
    struct FakeVerifier {
        verified: bool,
    }
    #[async_trait::async_trait]
    impl DomainOwnershipVerifier for FakeVerifier {
        async fn email_domain_verified(&self, _org_id: Uuid, _email: &str) -> bool {
            self.verified
        }
    }

    #[tokio::test]
    async fn gate_allows_verified_domain() {
        let v = FakeVerifier { verified: true };
        assert!(assert_email_in_verified_domain(&v, org(), "alice@acme.com").await.is_ok());
    }

    #[tokio::test]
    async fn gate_blocks_unverified_domain_with_instructions() {
        let v = FakeVerifier { verified: false };
        let err = assert_email_in_verified_domain(&v, org(), "attacker@bigcorp.com")
            .await
            .unwrap_err();
        let msg = err.to_string();
        // Error must tell the admin the exact record name and value to publish.
        assert!(msg.contains("_mockforge-verify.bigcorp.com"), "got: {msg}");
        assert!(msg.contains(&expected_txt_value(org())), "got: {msg}");
    }

    #[test]
    fn ssrf_guard_allows_public_https_hosts() {
        assert!(issuer_url_is_safe("https://login.okta.com/oauth2/default"));
        assert!(issuer_url_is_safe("https://accounts.google.com"));
        assert!(issuer_url_is_safe("https://idp.example.com:8443/realms/x"));
    }

    #[test]
    fn ssrf_guard_blocks_dangerous_hosts() {
        // Non-https.
        assert!(!issuer_url_is_safe("http://login.okta.com"));
        // Loopback / localhost.
        assert!(!issuer_url_is_safe("https://localhost/x"));
        assert!(!issuer_url_is_safe("https://127.0.0.1/x"));
        assert!(!issuer_url_is_safe("https://[::1]/x"));
        // Cloud metadata endpoint (link-local) and its DNS alias.
        assert!(!issuer_url_is_safe("https://169.254.169.254/latest/meta-data"));
        assert!(!issuer_url_is_safe("https://metadata.google.internal/x"));
        // IPv4-mapped IPv6 and NAT64 must not bypass the v4 ranges.
        assert!(!issuer_url_is_safe("https://[::ffff:169.254.169.254]/x"));
        assert!(!issuer_url_is_safe("https://[::ffff:127.0.0.1]/x"));
        assert!(!issuer_url_is_safe("https://[::ffff:10.0.0.1]/x"));
        assert!(!issuer_url_is_safe("https://[64:ff9b::a9fe:a9fe]/x"));
        // Private ranges.
        assert!(!issuer_url_is_safe("https://10.0.0.5/x"));
        assert!(!issuer_url_is_safe("https://192.168.1.1/x"));
        assert!(!issuer_url_is_safe("https://172.16.4.4/x"));
        // Embedded credentials and empty host.
        assert!(!issuer_url_is_safe("https://user@evil.com/x"));
        assert!(!issuer_url_is_safe("https:///x"));
    }
}
