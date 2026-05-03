//! Per-request matcher for conditional chaos injection.
//!
//! Lets fault injection (and other chaos features) be gated on properties of the
//! incoming request — source IP/CIDR, header presence or value, request body size,
//! and `Transfer-Encoding: chunked`. An empty matcher matches every request.
//!
//! AND semantics: every populated field must match. Within a list (e.g. `source_ips`,
//! `headers`), the field matches if **any** entry matches.

use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;

/// Header presence / exact-value filter.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeaderMatch {
    /// Header name (case-insensitive).
    pub name: String,
    /// Optional exact value. `None` = match on presence only.
    #[serde(default)]
    pub value: Option<String>,
}

/// Request properties that gate whether a chaos action fires.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestMatcher {
    /// Match if the client IP falls in any of these CIDR ranges (e.g.
    /// `"192.168.0.0/16"`, `"10.0.0.5/32"`, `"::1/128"`). A bare IP without a
    /// prefix is treated as `/32` (v4) or `/128` (v6). Empty list = any IP.
    #[serde(default)]
    pub source_ips: Vec<String>,

    /// Required headers. All entries must be satisfied (AND across the list).
    #[serde(default)]
    pub headers: Vec<HeaderMatch>,

    /// Minimum request body size in bytes (inclusive).
    #[serde(default)]
    pub min_body_size_bytes: Option<usize>,

    /// Maximum request body size in bytes (inclusive).
    #[serde(default)]
    pub max_body_size_bytes: Option<usize>,

    /// `Some(true)` matches only requests with `Transfer-Encoding: chunked`,
    /// `Some(false)` matches only requests **without** chunked encoding,
    /// `None` is don't-care.
    #[serde(default)]
    pub chunked_only: Option<bool>,
}

impl RequestMatcher {
    /// True when no field is configured — matches every request.
    pub fn is_empty(&self) -> bool {
        self.source_ips.is_empty()
            && self.headers.is_empty()
            && self.min_body_size_bytes.is_none()
            && self.max_body_size_bytes.is_none()
            && self.chunked_only.is_none()
    }

    /// Evaluate the matcher against extracted request properties.
    ///
    /// `client_ip` should be the resolved client IP string (already de-proxied if
    /// applicable). `headers` is an iterator over `(name, value)` pairs (header
    /// names should be lowercase). `body_size` is the request body size in bytes
    /// or `None` if not yet known. `is_chunked` reflects `Transfer-Encoding: chunked`.
    pub fn matches<'a, I>(
        &self,
        client_ip: Option<&str>,
        headers: I,
        body_size: Option<usize>,
        is_chunked: bool,
    ) -> bool
    where
        I: IntoIterator<Item = (&'a str, &'a str)> + Clone,
    {
        if self.is_empty() {
            return true;
        }

        if !self.source_ips.is_empty() {
            let ok = client_ip
                .and_then(|s| IpAddr::from_str(s).ok())
                .map(|ip| self.source_ips.iter().any(|cidr| ip_in_cidr(ip, cidr)))
                .unwrap_or(false);
            if !ok {
                return false;
            }
        }

        for hm in &self.headers {
            let needle = hm.name.to_ascii_lowercase();
            let mut found = false;
            for (k, v) in headers.clone() {
                if k.eq_ignore_ascii_case(&needle) {
                    match &hm.value {
                        None => {
                            found = true;
                            break;
                        }
                        Some(expected) if v == expected => {
                            found = true;
                            break;
                        }
                        _ => continue,
                    }
                }
            }
            if !found {
                return false;
            }
        }

        if let Some(min) = self.min_body_size_bytes {
            if body_size.unwrap_or(0) < min {
                return false;
            }
        }
        if let Some(max) = self.max_body_size_bytes {
            if body_size.unwrap_or(0) > max {
                return false;
            }
        }

        if let Some(want) = self.chunked_only {
            if want != is_chunked {
                return false;
            }
        }

        true
    }
}

/// True if `ip` belongs to the given CIDR (or equals the bare IP).
fn ip_in_cidr(ip: IpAddr, cidr: &str) -> bool {
    if let Ok(net) = IpNet::from_str(cidr) {
        return net.contains(&ip);
    }
    if let Ok(single) = IpAddr::from_str(cidr) {
        return single == ip;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn iter(v: &[(String, String)]) -> impl IntoIterator<Item = (&str, &str)> + Clone {
        v.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect::<Vec<_>>()
    }

    #[test]
    fn empty_matcher_matches_everything() {
        let m = RequestMatcher::default();
        assert!(m.is_empty());
        let hs = h(&[]);
        assert!(m.matches(None, iter(&hs), None, false));
        assert!(m.matches(Some("8.8.8.8"), iter(&hs), Some(1024), true));
    }

    #[test]
    fn cidr_v4_match() {
        let m = RequestMatcher {
            source_ips: vec!["10.0.0.0/8".into()],
            ..Default::default()
        };
        let hs = h(&[]);
        assert!(m.matches(Some("10.5.6.7"), iter(&hs), None, false));
        assert!(!m.matches(Some("11.0.0.1"), iter(&hs), None, false));
        assert!(!m.matches(None, iter(&hs), None, false));
    }

    #[test]
    fn bare_ip_treated_as_host() {
        let m = RequestMatcher {
            source_ips: vec!["127.0.0.1".into()],
            ..Default::default()
        };
        let hs = h(&[]);
        assert!(m.matches(Some("127.0.0.1"), iter(&hs), None, false));
        assert!(!m.matches(Some("127.0.0.2"), iter(&hs), None, false));
    }

    #[test]
    fn cidr_v6_match() {
        let m = RequestMatcher {
            source_ips: vec!["2001:db8::/32".into()],
            ..Default::default()
        };
        let hs = h(&[]);
        assert!(m.matches(Some("2001:db8::1"), iter(&hs), None, false));
        assert!(!m.matches(Some("2001:db9::1"), iter(&hs), None, false));
    }

    #[test]
    fn header_presence_only() {
        let m = RequestMatcher {
            headers: vec![HeaderMatch {
                name: "x-test".into(),
                value: None,
            }],
            ..Default::default()
        };
        let with = h(&[("x-test", "anything")]);
        let without = h(&[("x-other", "v")]);
        assert!(m.matches(None, iter(&with), None, false));
        assert!(!m.matches(None, iter(&without), None, false));
    }

    #[test]
    fn header_exact_value_case_insensitive_name() {
        let m = RequestMatcher {
            headers: vec![HeaderMatch {
                name: "X-Test".into(),
                value: Some("abc".into()),
            }],
            ..Default::default()
        };
        let good = h(&[("x-test", "abc")]);
        let bad = h(&[("x-test", "xyz")]);
        assert!(m.matches(None, iter(&good), None, false));
        assert!(!m.matches(None, iter(&bad), None, false));
    }

    #[test]
    fn body_size_threshold() {
        let m = RequestMatcher {
            min_body_size_bytes: Some(1024),
            ..Default::default()
        };
        let hs = h(&[]);
        assert!(m.matches(None, iter(&hs), Some(2048), false));
        assert!(!m.matches(None, iter(&hs), Some(512), false));
        assert!(!m.matches(None, iter(&hs), None, false));

        let m2 = RequestMatcher {
            max_body_size_bytes: Some(1024),
            ..Default::default()
        };
        assert!(m2.matches(None, iter(&hs), Some(512), false));
        assert!(!m2.matches(None, iter(&hs), Some(2048), false));
    }

    #[test]
    fn chunked_only() {
        let m_chunked = RequestMatcher {
            chunked_only: Some(true),
            ..Default::default()
        };
        let m_unchunked = RequestMatcher {
            chunked_only: Some(false),
            ..Default::default()
        };
        let hs = h(&[]);
        assert!(m_chunked.matches(None, iter(&hs), None, true));
        assert!(!m_chunked.matches(None, iter(&hs), None, false));
        assert!(!m_unchunked.matches(None, iter(&hs), None, true));
        assert!(m_unchunked.matches(None, iter(&hs), None, false));
    }

    #[test]
    fn and_semantics_across_fields() {
        let m = RequestMatcher {
            source_ips: vec!["10.0.0.0/8".into()],
            headers: vec![HeaderMatch {
                name: "x-test".into(),
                value: Some("yes".into()),
            }],
            min_body_size_bytes: Some(100),
            chunked_only: Some(true),
            ..Default::default()
        };
        let hs = h(&[("x-test", "yes")]);
        assert!(m.matches(Some("10.1.1.1"), iter(&hs), Some(200), true));
        // Wrong IP
        assert!(!m.matches(Some("8.8.8.8"), iter(&hs), Some(200), true));
        // Wrong header value
        let bad_hs = h(&[("x-test", "no")]);
        assert!(!m.matches(Some("10.1.1.1"), iter(&bad_hs), Some(200), true));
        // Body too small
        assert!(!m.matches(Some("10.1.1.1"), iter(&hs), Some(50), true));
        // Not chunked
        assert!(!m.matches(Some("10.1.1.1"), iter(&hs), Some(200), false));
    }

    #[test]
    fn invalid_cidr_does_not_panic() {
        let m = RequestMatcher {
            source_ips: vec!["not-an-ip".into()],
            ..Default::default()
        };
        let hs = h(&[]);
        assert!(!m.matches(Some("1.2.3.4"), iter(&hs), None, false));
    }
}
