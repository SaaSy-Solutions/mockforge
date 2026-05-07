//! Per-plugin allowlist policy.
//!
//! A `HostPolicy` carries the set of hostnames + wildcard patterns
//! a plugin is allowed to reach. The proxy consults it for each
//! request after the hard denylist runs (denylist wins on conflict).
//!
//! Patterns supported:
//!   - Exact match: `api.stripe.com`
//!   - Leading wildcard: `*.stripe.com` matches `api.stripe.com`,
//!     `events.stripe.com`, `foo.bar.stripe.com`. Does **not**
//!     match the bare `stripe.com` (require explicit listing).
//!
//! Out of scope for v1:
//!   - Path-level rules (allow GET /v1/charges only)
//!   - Header-based policies
//!   - SNI inspection on TLS pass-through
//!
//! The cloud trust RFC §5.4 explicitly accepts these limitations:
//! once a host is allowed, body content is opaque. That's the
//! price of supporting useful plugins without MITM-ing TLS.

use crate::denylist::is_denied_target;

/// What the policy decided about a given host. The `Denied` variant
/// carries a stable reason string for audit logging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Host matched the allowlist (and isn't on the hard denylist).
    Allowed,
    /// Host failed the policy check. The reason is a stable string
    /// suitable for both logs and the 403 body the proxy returns.
    Denied(&'static str),
}

/// Per-plugin allowlist + the global hard denylist.
///
/// Construct with [`HostPolicy::from_patterns`]. The policy is
/// immutable for the lifetime of the plugin; re-attach with
/// updated permissions creates a fresh policy.
#[derive(Debug, Clone)]
pub struct HostPolicy {
    /// Compiled patterns. Each entry is either an exact hostname
    /// (lowercased) or a leading-wildcard pattern represented as
    /// `(suffix_with_dot, _)` — e.g. `*.stripe.com` becomes
    /// `(".stripe.com", true)` so we just check `host.ends_with`.
    patterns: Vec<HostPattern>,
}

#[derive(Debug, Clone)]
enum HostPattern {
    /// Exact match (lowercased).
    Exact(String),
    /// Leading-wildcard match: stored as the dotted suffix, e.g.
    /// `.stripe.com`. A host matches if it ends with this suffix
    /// AND has at least one extra label before it.
    Wildcard(String),
}

impl HostPolicy {
    /// Build a policy from the grant payload's `egress.allow`
    /// list. Patterns that don't conform to the supported syntax
    /// (no `*` other than as a leading label) are rejected — the
    /// caller should surface this as a permission-grant error.
    pub fn from_patterns(patterns: &[String]) -> Result<Self, PolicyError> {
        let mut compiled = Vec::with_capacity(patterns.len());
        for raw in patterns {
            compiled.push(compile_pattern(raw)?);
        }
        Ok(Self { patterns: compiled })
    }

    /// Empty policy — denies everything. Useful as the deny-all
    /// default when a plugin has no `egress.allow` entries.
    pub fn deny_all() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// Decide whether `host` is allowed. Hard denylist runs first;
    /// allowlist runs only if the denylist passes.
    pub fn check(&self, host: &str) -> PolicyDecision {
        if let Some(reason) = is_denied_target(host) {
            return PolicyDecision::Denied(reason);
        }

        let host_lc = host.to_ascii_lowercase();
        for pattern in &self.patterns {
            match pattern {
                HostPattern::Exact(s) => {
                    if host_lc == *s {
                        return PolicyDecision::Allowed;
                    }
                }
                HostPattern::Wildcard(suffix) => {
                    // Suffix is `.example.com`; require host to be
                    // `<at-least-one-label>.example.com`.
                    if host_lc.len() > suffix.len() && host_lc.ends_with(suffix) {
                        return PolicyDecision::Allowed;
                    }
                }
            }
        }

        PolicyDecision::Denied("denied: host not in allowlist")
    }
}

/// Errors that can occur while compiling a pattern list.
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    /// The pattern uses `*` somewhere other than the leading
    /// label, which we don't support in v1.
    #[error("unsupported wildcard placement in pattern '{0}'; only leading-label `*.` is allowed")]
    UnsupportedWildcard(String),
    /// The pattern is empty or otherwise malformed.
    #[error("invalid pattern '{0}'")]
    InvalidPattern(String),
}

fn compile_pattern(raw: &str) -> Result<HostPattern, PolicyError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PolicyError::InvalidPattern(raw.to_string()));
    }
    let lc = trimmed.to_ascii_lowercase();

    if let Some(suffix) = lc.strip_prefix("*.") {
        // The remainder must be a plain hostname — no further `*`s.
        if suffix.contains('*') {
            return Err(PolicyError::UnsupportedWildcard(raw.to_string()));
        }
        if suffix.is_empty() {
            return Err(PolicyError::InvalidPattern(raw.to_string()));
        }
        // Store with the leading dot so `host.ends_with(".stripe.com")`
        // correctly rejects `evilstripe.com`.
        Ok(HostPattern::Wildcard(format!(".{suffix}")))
    } else {
        if lc.contains('*') {
            return Err(PolicyError::UnsupportedWildcard(raw.to_string()));
        }
        Ok(HostPattern::Exact(lc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow(patterns: &[&str]) -> HostPolicy {
        HostPolicy::from_patterns(&patterns.iter().map(|s| (*s).to_string()).collect::<Vec<_>>())
            .expect("test patterns should compile")
    }

    #[test]
    fn deny_all_denies_everything() {
        let policy = HostPolicy::deny_all();
        assert!(matches!(policy.check("api.stripe.com"), PolicyDecision::Denied(_)));
    }

    #[test]
    fn exact_match_allows_only_that_host() {
        let policy = allow(&["api.stripe.com"]);
        assert_eq!(policy.check("api.stripe.com"), PolicyDecision::Allowed);
        assert!(matches!(policy.check("events.stripe.com"), PolicyDecision::Denied(_)));
        assert!(matches!(policy.check("api.stripe.org"), PolicyDecision::Denied(_)));
    }

    #[test]
    fn wildcard_matches_subdomains() {
        let policy = allow(&["*.stripe.com"]);
        assert_eq!(policy.check("api.stripe.com"), PolicyDecision::Allowed);
        assert_eq!(policy.check("events.stripe.com"), PolicyDecision::Allowed);
        assert_eq!(policy.check("foo.bar.stripe.com"), PolicyDecision::Allowed);
    }

    #[test]
    fn wildcard_does_not_match_apex() {
        // `*.stripe.com` should NOT match `stripe.com` itself —
        // require explicit listing.
        let policy = allow(&["*.stripe.com"]);
        assert!(matches!(policy.check("stripe.com"), PolicyDecision::Denied(_)));
    }

    #[test]
    fn wildcard_does_not_match_overlapping_suffix() {
        // `evilstripe.com` ends with `stripe.com` byte-wise, but
        // we require the dot before the suffix.
        let policy = allow(&["*.stripe.com"]);
        assert!(matches!(policy.check("evilstripe.com"), PolicyDecision::Denied(_)));
    }

    #[test]
    fn allowlist_check_is_case_insensitive() {
        let policy = allow(&["api.STRIPE.com"]);
        assert_eq!(policy.check("API.stripe.COM"), PolicyDecision::Allowed);
    }

    #[test]
    fn denylist_overrides_allowlist() {
        // Even if you literally allow metadata.google.internal,
        // the hard denylist wins.
        let policy = allow(&["metadata.google.internal"]);
        assert!(matches!(policy.check("metadata.google.internal"), PolicyDecision::Denied(_)));
    }

    #[test]
    fn denylist_overrides_wildcard_match() {
        // *.fly.dev should match an allowlist entry — but the hard
        // denylist for *.fly.dev wins.
        let policy = allow(&["*.fly.dev"]);
        assert!(matches!(policy.check("anyone.fly.dev"), PolicyDecision::Denied(_)));
    }

    #[test]
    fn malformed_wildcard_is_rejected() {
        let result = HostPolicy::from_patterns(&["foo.*.com".to_string()]);
        assert!(matches!(result, Err(PolicyError::UnsupportedWildcard(_))));
    }

    #[test]
    fn empty_pattern_is_rejected() {
        let result = HostPolicy::from_patterns(&["   ".to_string()]);
        assert!(matches!(result, Err(PolicyError::InvalidPattern(_))));
    }

    #[test]
    fn wildcard_without_suffix_is_rejected() {
        let result = HostPolicy::from_patterns(&["*.".to_string()]);
        assert!(matches!(result, Err(PolicyError::InvalidPattern(_))));
    }
}
