//! Trusted proxy and client IP extraction utilities
//!
//! This module provides secure client IP extraction that only trusts
//! X-Forwarded-For and similar headers when the request comes from
//! a trusted proxy.
//!
//! Configuration:
//! - `TRUSTED_PROXIES`: Comma-separated list of trusted proxy IP addresses or CIDR ranges
//!   Example: "10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,127.0.0.1"
//! - If not set, defaults to private network ranges (for typical cloud/container deployments)
//!
//! Security considerations:
//! - X-Forwarded-For can be spoofed by clients
//! - Only trust this header when the immediate connection is from a known proxy
//! - The last entry in X-Forwarded-For added by your proxy is the client IP

use axum::http::HeaderMap;
use std::net::IpAddr;
use std::sync::OnceLock;

/// Default trusted proxy ranges (private networks)
/// These are commonly used by load balancers and proxies in cloud environments
const DEFAULT_TRUSTED_PROXIES: &[&str] = &[
    "10.0.0.0/8",     // Private network
    "172.16.0.0/12",  // Private network
    "192.168.0.0/16", // Private network
    "127.0.0.0/8",    // Localhost
    "::1/128",        // IPv6 localhost
    "fc00::/7",       // IPv6 private
    "fe80::/10",      // IPv6 link-local
];

/// Cached trusted proxy configuration
static TRUSTED_PROXIES: OnceLock<Vec<IpNetwork>> = OnceLock::new();

/// A simple IP network representation for CIDR matching
#[derive(Debug, Clone)]
struct IpNetwork {
    network: IpAddr,
    prefix_len: u8,
}

impl IpNetwork {
    /// Parse a CIDR string like "192.168.0.0/16" or a plain IP like "10.0.0.1"
    fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.trim().split('/').collect();

        let network: IpAddr = parts[0].parse().ok()?;

        let prefix_len = if parts.len() > 1 {
            parts[1].parse().ok()?
        } else {
            // Default prefix for single IPs
            match network {
                IpAddr::V4(_) => 32,
                IpAddr::V6(_) => 128,
            }
        };

        Some(Self {
            network,
            prefix_len,
        })
    }

    /// Check if an IP address is within this network
    fn contains(&self, ip: &IpAddr) -> bool {
        match (&self.network, ip) {
            (IpAddr::V4(net), IpAddr::V4(addr)) => {
                let net_bits = u32::from(*net);
                let addr_bits = u32::from(*addr);
                let mask = if self.prefix_len >= 32 {
                    !0u32
                } else {
                    !0u32 << (32 - self.prefix_len)
                };
                (net_bits & mask) == (addr_bits & mask)
            }
            (IpAddr::V6(net), IpAddr::V6(addr)) => {
                let net_bits = u128::from(*net);
                let addr_bits = u128::from(*addr);
                let mask = if self.prefix_len >= 128 {
                    !0u128
                } else {
                    !0u128 << (128 - self.prefix_len)
                };
                (net_bits & mask) == (addr_bits & mask)
            }
            _ => false, // IPv4 and IPv6 don't match
        }
    }
}

/// Get the list of trusted proxy networks
fn get_trusted_proxies() -> &'static Vec<IpNetwork> {
    TRUSTED_PROXIES.get_or_init(|| {
        let proxy_list = std::env::var("TRUSTED_PROXIES")
            .map(|s| s.split(',').map(|p| p.to_string()).collect::<Vec<_>>())
            .unwrap_or_else(|_| DEFAULT_TRUSTED_PROXIES.iter().map(|s| s.to_string()).collect());

        let networks: Vec<IpNetwork> =
            proxy_list.iter().filter_map(|s| IpNetwork::parse(s)).collect();

        if networks.is_empty() {
            tracing::warn!("No trusted proxies configured, X-Forwarded-For will not be trusted");
        } else {
            tracing::info!("Trusted proxies configured: {} networks", networks.len());
        }

        networks
    })
}

/// Check if an IP address is from a trusted proxy
pub fn is_trusted_proxy(ip: &IpAddr) -> bool {
    get_trusted_proxies().iter().any(|net| net.contains(ip))
}

/// Extract the real client IP address from request headers
///
/// This function safely extracts the client IP by:
/// 1. Checking if the connecting IP is from a trusted proxy
/// 2. Only trusting X-Forwarded-For if from a trusted proxy
/// 3. Using the first (leftmost) IP from X-Forwarded-For (original client)
/// 4. Falling back to the connecting IP if headers are not trusted
///
/// # Arguments
/// * `headers` - The HTTP headers from the request
/// * `connecting_ip` - The IP address of the immediate connection (from socket)
///
/// # Returns
/// The client IP address as a string
pub fn extract_client_ip(headers: &HeaderMap, connecting_ip: Option<&str>) -> String {
    // Parse the connecting IP if provided
    let connecting_addr: Option<IpAddr> = connecting_ip.and_then(|s| s.parse().ok());

    // Check if we should trust forwarded headers
    let trust_headers = connecting_addr.map(|ip| is_trusted_proxy(&ip)).unwrap_or(false);

    if trust_headers {
        // Try X-Forwarded-For first (most common)
        if let Some(forwarded_for) = headers.get("X-Forwarded-For") {
            if let Ok(value) = forwarded_for.to_str() {
                // X-Forwarded-For format: "client, proxy1, proxy2"
                // The first entry is the original client
                if let Some(client_ip) = value.split(',').next() {
                    let ip = client_ip.trim();
                    if !ip.is_empty() && ip.parse::<IpAddr>().is_ok() {
                        return ip.to_string();
                    }
                }
            }
        }

        // Try X-Real-IP (used by some proxies like nginx)
        if let Some(real_ip) = headers.get("X-Real-IP") {
            if let Ok(value) = real_ip.to_str() {
                let ip = value.trim();
                if !ip.is_empty() && ip.parse::<IpAddr>().is_ok() {
                    return ip.to_string();
                }
            }
        }
    }

    // Fall back to connecting IP or unknown
    connecting_ip.unwrap_or("unknown").to_string()
}

/// Extract client IP from headers only (when socket info is not available)
/// This is less secure and should only be used when you know the request
/// is coming from a trusted proxy.
///
/// **Warning**: This trusts X-Forwarded-For blindly. Only use in contexts
/// where you know the proxy is trusted (e.g., behind a load balancer).
pub fn extract_client_ip_from_headers(headers: &HeaderMap) -> String {
    // X-Forwarded-For
    if let Some(forwarded_for) = headers.get("X-Forwarded-For") {
        if let Ok(value) = forwarded_for.to_str() {
            if let Some(client_ip) = value.split(',').next() {
                let ip = client_ip.trim();
                if !ip.is_empty() {
                    return ip.to_string();
                }
            }
        }
    }

    // X-Real-IP
    if let Some(real_ip) = headers.get("X-Real-IP") {
        if let Ok(value) = real_ip.to_str() {
            let ip = value.trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }

    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_network_parse_ipv4() {
        let net = IpNetwork::parse("192.168.0.0/16").unwrap();
        assert_eq!(net.prefix_len, 16);
    }

    #[test]
    fn test_ip_network_parse_single_ip() {
        let net = IpNetwork::parse("10.0.0.1").unwrap();
        assert_eq!(net.prefix_len, 32);
    }

    #[test]
    fn test_ip_network_contains() {
        let net = IpNetwork::parse("192.168.0.0/16").unwrap();

        assert!(net.contains(&"192.168.1.1".parse().unwrap()));
        assert!(net.contains(&"192.168.255.255".parse().unwrap()));
        assert!(!net.contains(&"192.169.0.1".parse().unwrap()));
        assert!(!net.contains(&"10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn test_ip_network_contains_single() {
        let net = IpNetwork::parse("10.0.0.1").unwrap();

        assert!(net.contains(&"10.0.0.1".parse().unwrap()));
        assert!(!net.contains(&"10.0.0.2".parse().unwrap()));
    }

    #[test]
    fn test_extract_client_ip_trusted_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-For", "203.0.113.195, 70.41.3.18".parse().unwrap());

        // From a private IP (trusted by default)
        let ip = extract_client_ip(&headers, Some("10.0.0.1"));
        assert_eq!(ip, "203.0.113.195");
    }

    #[test]
    fn test_extract_client_ip_untrusted() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-For", "spoofed-ip".parse().unwrap());

        // From a public IP (not trusted)
        let ip = extract_client_ip(&headers, Some("8.8.8.8"));
        assert_eq!(ip, "8.8.8.8");
    }

    #[test]
    fn test_extract_client_ip_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Real-IP", "203.0.113.50".parse().unwrap());

        let ip = extract_client_ip(&headers, Some("192.168.1.1"));
        assert_eq!(ip, "203.0.113.50");
    }

    #[test]
    fn test_extract_client_ip_no_headers() {
        let headers = HeaderMap::new();

        let ip = extract_client_ip(&headers, Some("10.0.0.1"));
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_extract_client_ip_unknown() {
        let headers = HeaderMap::new();

        let ip = extract_client_ip(&headers, None);
        assert_eq!(ip, "unknown");
    }
}
