//! Proxy configuration types and settings

use serde::{Deserialize, Serialize};

/// Configuration for proxy behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Whether the proxy is enabled
    pub enabled: bool,
    /// Target URL to proxy requests to
    pub target_url: Option<String>,
    /// Timeout for proxy requests in seconds
    pub timeout_seconds: u64,
    /// Whether to follow redirects
    pub follow_redirects: bool,
    /// Additional headers to add to proxied requests
    pub headers: std::collections::HashMap<String, String>,
    /// Proxy prefix to strip from paths
    pub prefix: Option<String>,
    /// Whether to proxy by default
    pub passthrough_by_default: bool,
    /// Proxy rules
    pub rules: Vec<ProxyRule>,
}

/// Proxy routing rule
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyRule {
    /// Path pattern to match
    pub path_pattern: String,
    /// Target URL for this rule
    pub target_url: String,
    /// Whether this rule is enabled
    pub enabled: bool,
    /// Pattern for matching (alias for path_pattern)
    pub pattern: String,
    /// Upstream URL (alias for target_url)
    pub upstream_url: String,
}

impl Default for ProxyRule {
    fn default() -> Self {
        Self {
            path_pattern: "/".to_string(),
            target_url: "http://localhost:8080".to_string(),
            enabled: true,
            pattern: "/".to_string(),
            upstream_url: "http://localhost:8080".to_string(),
        }
    }
}

impl ProxyConfig {
    /// Create a new proxy configuration
    pub fn new(upstream_url: String) -> Self {
        Self {
            enabled: true,
            target_url: Some(upstream_url),
            timeout_seconds: 30,
            follow_redirects: true,
            headers: std::collections::HashMap::new(),
            prefix: Some("/proxy/".to_string()),
            passthrough_by_default: true,
            rules: Vec::new(),
        }
    }

    /// Check if a request should be proxied
    pub fn should_proxy(&self, _method: &axum::http::Method, path: &str) -> bool {
        if !self.enabled {
            return false;
        }
        
        // If there are rules, check if any rule matches
        for rule in &self.rules {
            if rule.enabled && self.path_matches_pattern(&rule.path_pattern, path) {
                return true;
            }
        }
        
        // If no rules match, check prefix logic
        match &self.prefix {
            None => true, // No prefix means proxy everything
            Some(prefix) => path.starts_with(prefix),
        }
    }

    /// Get the upstream URL for a specific path
    pub fn get_upstream_url(&self, path: &str) -> String {
        // Check rules first
        for rule in &self.rules {
            if rule.enabled && self.path_matches_pattern(&rule.path_pattern, path) {
                return rule.target_url.clone();
            }
        }
        
        // If no rule matches, use the default target URL
        if let Some(base_url) = &self.target_url {
            base_url.clone()
        } else {
            path.to_string()
        }
    }

    /// Strip the proxy prefix from a path
    pub fn strip_prefix(&self, path: &str) -> String {
        match &self.prefix {
            Some(prefix) => {
                if path.starts_with(prefix) {
                    let stripped = path.strip_prefix(prefix).unwrap_or(path);
                    // Ensure the result starts with a slash
                    if stripped.starts_with('/') {
                        stripped.to_string()
                    } else {
                        format!("/{}", stripped)
                    }
                } else {
                    path.to_string()
                }
            }
            None => path.to_string(), // No prefix to strip
        }
    }
    
    /// Check if a path matches a pattern (supports wildcards)
    fn path_matches_pattern(&self, pattern: &str, path: &str) -> bool {
        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 2];
            path.starts_with(prefix)
        } else {
            path == pattern
        }
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            target_url: None,
            timeout_seconds: 30,
            follow_redirects: true,
            headers: std::collections::HashMap::new(),
            prefix: None,
            passthrough_by_default: false,
            rules: Vec::new(),
        }
    }
}
