//! Proxy configuration types and settings

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Migration mode for route handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum MigrationMode {
    /// Always use mock (ignore proxy even if rule matches)
    Mock,
    /// Proxy to real backend AND generate mock response for comparison
    Shadow,
    /// Always use real backend (proxy)
    Real,
    /// Use existing priority chain (default, backward compatible)
    #[default]
    Auto,
}

/// Configuration for proxy behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
    pub headers: HashMap<String, String>,
    /// Proxy prefix to strip from paths
    pub prefix: Option<String>,
    /// Whether to proxy by default
    pub passthrough_by_default: bool,
    /// Proxy rules
    pub rules: Vec<ProxyRule>,
    /// Whether migration features are enabled
    #[serde(default)]
    pub migration_enabled: bool,
    /// Group-level migration mode overrides
    /// Maps group name to migration mode
    #[serde(default)]
    pub migration_groups: HashMap<String, MigrationMode>,
    /// Request body replacement rules for browser proxy mode
    #[serde(default)]
    pub request_replacements: Vec<BodyTransformRule>,
    /// Response body replacement rules for browser proxy mode
    #[serde(default)]
    pub response_replacements: Vec<BodyTransformRule>,
    /// Allow absolute `http(s)://` URLs embedded in the stripped request
    /// path to be forwarded as-is (#1012 / MF-002). Off by default: the
    /// forward-proxy-by-path-vector was an open proxy/SSRF hole. Even when
    /// enabled, request-derived URLs still pass the egress guard.
    #[serde(default)]
    pub allow_absolute_url_upstream: bool,
    /// Explicit upstream allowlist overriding the egress denylist for
    /// request-derived URLs. Entries here re-open SSRF by design — only
    /// list hosts the proxy must genuinely reach.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_allowlist: Option<crate::egress::UpstreamAllowlist>,
}

/// Proxy routing rule
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
    /// Migration mode for this route (mock, shadow, real, auto)
    #[serde(default)]
    pub migration_mode: MigrationMode,
    /// Migration group this route belongs to (optional)
    #[serde(default)]
    pub migration_group: Option<String>,
    /// Conditional expression for proxying (JSONPath, JavaScript-like, or Rhai script)
    /// If provided, the request will only be proxied if the condition evaluates to true
    /// Examples:
    ///   - JSONPath: "$.user.role == 'admin'"
    ///   - Header check: "header[authorization] != ''"
    ///   - Query param: "query[env] == 'production'"
    ///   - Complex: "AND($.user.role == 'admin', header[x-forwarded-for] != '')"
    #[serde(default)]
    pub condition: Option<String>,
}

impl Default for ProxyRule {
    fn default() -> Self {
        Self {
            path_pattern: "/".to_string(),
            target_url: "http://localhost:9080".to_string(),
            enabled: true,
            pattern: "/".to_string(),
            upstream_url: "http://localhost:9080".to_string(),
            migration_mode: MigrationMode::Auto,
            migration_group: None,
            condition: None,
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
            headers: HashMap::new(),
            prefix: Some("/proxy/".to_string()),
            passthrough_by_default: true,
            rules: Vec::new(),
            migration_enabled: false,
            migration_groups: HashMap::new(),
            request_replacements: Vec::new(),
            response_replacements: Vec::new(),
            allow_absolute_url_upstream: false,
            upstream_allowlist: None,
        }
    }

    /// Get the effective migration mode for a path
    /// Checks group overrides first, then route-specific mode
    pub fn get_effective_migration_mode(&self, path: &str) -> Option<MigrationMode> {
        if !self.migration_enabled {
            return None;
        }

        // Find matching rule
        for rule in &self.rules {
            if rule.enabled && self.path_matches_pattern(&rule.path_pattern, path) {
                // Check group override first
                if let Some(ref group) = rule.migration_group {
                    if let Some(&group_mode) = self.migration_groups.get(group) {
                        return Some(group_mode);
                    }
                }
                // Return route-specific mode
                return Some(rule.migration_mode);
            }
        }

        None
    }

    /// Check if a request should be proxied
    /// Respects migration mode: mock forces mock, real forces proxy, shadow forces proxy, auto uses existing logic
    /// This is a legacy method that doesn't evaluate conditions - use should_proxy_with_condition for conditional proxying
    pub fn should_proxy(&self, _method: &http::Method, path: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // Check migration mode if enabled
        if self.migration_enabled {
            if let Some(mode) = self.get_effective_migration_mode(path) {
                match mode {
                    MigrationMode::Mock => return false,  // Force mock
                    MigrationMode::Shadow => return true, // Force proxy (for shadow mode)
                    MigrationMode::Real => return true,   // Force proxy
                    MigrationMode::Auto => {
                        // Fall through to existing logic
                    }
                }
            }
        }

        // If there are rules, check if any rule matches (without condition evaluation)
        for rule in &self.rules {
            if rule.enabled && self.path_matches_pattern(&rule.path_pattern, path) {
                // If rule has a condition, we can't evaluate it here (no request context)
                // So we skip conditional rules in this legacy method
                if rule.condition.is_none() {
                    return true;
                }
            }
        }

        // If no rules match, check prefix logic
        match &self.prefix {
            None => true, // No prefix means proxy everything
            Some(prefix) => path.starts_with(prefix),
        }
    }

    /// Check if a request should be proxied with conditional evaluation
    /// This method evaluates conditions in proxy rules using request context
    pub fn should_proxy_with_condition(
        &self,
        method: &http::Method,
        uri: &http::Uri,
        headers: &http::HeaderMap,
        body: Option<&[u8]>,
    ) -> bool {
        use crate::conditional::find_matching_rule;

        if !self.enabled {
            return false;
        }

        let path = uri.path();

        // Check migration mode if enabled
        if self.migration_enabled {
            if let Some(mode) = self.get_effective_migration_mode(path) {
                match mode {
                    MigrationMode::Mock => return false,  // Force mock
                    MigrationMode::Shadow => return true, // Force proxy (for shadow mode)
                    MigrationMode::Real => return true,   // Force proxy
                    MigrationMode::Auto => {
                        // Fall through to conditional evaluation
                    }
                }
            }
        }

        // If there are rules, check if any rule matches with condition evaluation
        if !self.rules.is_empty()
            && find_matching_rule(&self.rules, method, uri, headers, body, |pattern, path| {
                self.path_matches_pattern(pattern, path)
            })
            .is_some()
        {
            return true;
        }

        // If no rules match, check prefix logic (only if no rules have conditions)
        let has_conditional_rules = self.rules.iter().any(|r| r.enabled && r.condition.is_some());
        if !has_conditional_rules {
            match &self.prefix {
                None => true, // No prefix means proxy everything
                Some(prefix) => path.starts_with(prefix),
            }
        } else {
            false // If we have conditional rules but none matched, don't proxy
        }
    }

    /// Check if a route should use shadow mode (proxy + generate mock)
    pub fn should_shadow(&self, path: &str) -> bool {
        if !self.migration_enabled {
            return false;
        }

        if let Some(mode) = self.get_effective_migration_mode(path) {
            return mode == MigrationMode::Shadow;
        }

        false
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
        if let Some(prefix) = pattern.strip_suffix("/*") {
            path.starts_with(prefix)
        } else {
            path == pattern
        }
    }

    /// Update migration mode for a specific route pattern
    /// Returns true if the rule was found and updated
    pub fn update_rule_migration_mode(&mut self, pattern: &str, mode: MigrationMode) -> bool {
        for rule in &mut self.rules {
            if rule.path_pattern == pattern || rule.pattern == pattern {
                rule.migration_mode = mode;
                return true;
            }
        }
        false
    }

    /// Update migration mode for an entire group
    /// This affects all routes that belong to the group
    pub fn update_group_migration_mode(&mut self, group: &str, mode: MigrationMode) {
        self.migration_groups.insert(group.to_string(), mode);
    }

    /// Toggle a route's migration mode through the stages: mock → shadow → real → mock
    /// Returns the new mode if the rule was found
    pub fn toggle_route_migration(&mut self, pattern: &str) -> Option<MigrationMode> {
        for rule in &mut self.rules {
            if rule.path_pattern == pattern || rule.pattern == pattern {
                rule.migration_mode = match rule.migration_mode {
                    MigrationMode::Mock => MigrationMode::Shadow,
                    MigrationMode::Shadow => MigrationMode::Real,
                    MigrationMode::Real => MigrationMode::Mock,
                    MigrationMode::Auto => MigrationMode::Mock, // Start migration from auto
                };
                return Some(rule.migration_mode);
            }
        }
        None
    }

    /// Toggle a group's migration mode through the stages: mock → shadow → real → mock
    /// Returns the new mode
    pub fn toggle_group_migration(&mut self, group: &str) -> MigrationMode {
        let current_mode = self.migration_groups.get(group).copied().unwrap_or(MigrationMode::Auto);
        let new_mode = match current_mode {
            MigrationMode::Mock => MigrationMode::Shadow,
            MigrationMode::Shadow => MigrationMode::Real,
            MigrationMode::Real => MigrationMode::Mock,
            MigrationMode::Auto => MigrationMode::Mock, // Start migration from auto
        };
        self.migration_groups.insert(group.to_string(), new_mode);
        new_mode
    }

    /// Get all routes with their migration status
    pub fn get_migration_routes(&self) -> Vec<MigrationRouteInfo> {
        self.rules
            .iter()
            .map(|rule| {
                let effective_mode = if let Some(ref group) = rule.migration_group {
                    self.migration_groups.get(group).copied().unwrap_or(rule.migration_mode)
                } else {
                    rule.migration_mode
                };

                MigrationRouteInfo {
                    pattern: rule.path_pattern.clone(),
                    upstream_url: rule.target_url.clone(),
                    migration_mode: effective_mode,
                    route_mode: rule.migration_mode,
                    migration_group: rule.migration_group.clone(),
                    enabled: rule.enabled,
                }
            })
            .collect()
    }

    /// Get all migration groups with their status
    pub fn get_migration_groups(&self) -> HashMap<String, MigrationGroupInfo> {
        let mut group_info: HashMap<String, MigrationGroupInfo> = HashMap::new();

        // Collect all groups from rules
        for rule in &self.rules {
            if let Some(ref group) = rule.migration_group {
                let entry = group_info.entry(group.clone()).or_insert_with(|| MigrationGroupInfo {
                    name: group.clone(),
                    migration_mode: self
                        .migration_groups
                        .get(group)
                        .copied()
                        .unwrap_or(rule.migration_mode),
                    route_count: 0,
                });
                entry.route_count += 1;
            }
        }

        // Add groups that only exist in migration_groups (no routes yet)
        for (group_name, &mode) in &self.migration_groups {
            group_info.entry(group_name.clone()).or_insert_with(|| MigrationGroupInfo {
                name: group_name.clone(),
                migration_mode: mode,
                route_count: 0,
            });
        }

        group_info
    }
}

/// Information about a route's migration status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRouteInfo {
    /// Route pattern
    pub pattern: String,
    /// Upstream URL
    pub upstream_url: String,
    /// Effective migration mode (considering group overrides)
    pub migration_mode: MigrationMode,
    /// Route-specific migration mode
    pub route_mode: MigrationMode,
    /// Migration group this route belongs to (if any)
    pub migration_group: Option<String>,
    /// Whether the route is enabled
    pub enabled: bool,
}

/// Information about a migration group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationGroupInfo {
    /// Group name
    pub name: String,
    /// Current migration mode for the group
    pub migration_mode: MigrationMode,
    /// Number of routes in this group
    pub route_count: usize,
}

/// Body transformation rule for request/response replacement
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BodyTransformRule {
    /// URL pattern to match (supports wildcards like "/api/users/*")
    pub pattern: String,
    /// Optional status code filter for response rules (only applies to responses)
    #[serde(default)]
    pub status_codes: Vec<u16>,
    /// Body transformations to apply
    pub body_transforms: Vec<BodyTransform>,
    /// Whether this rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl BodyTransformRule {
    /// Check if this rule matches a URL
    pub fn matches_url(&self, url: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // Simple pattern matching - supports wildcards
        if self.pattern.ends_with("/*") {
            let prefix = &self.pattern[..self.pattern.len() - 2];
            url.starts_with(prefix)
        } else {
            url == self.pattern || url.starts_with(&self.pattern)
        }
    }

    /// Check if this rule matches a status code (for response rules)
    pub fn matches_status_code(&self, status_code: u16) -> bool {
        if self.status_codes.is_empty() {
            true // No filter means match all
        } else {
            self.status_codes.contains(&status_code)
        }
    }
}

/// Individual body transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BodyTransform {
    /// JSONPath expression to target (e.g., "$.userId", "$.email")
    pub path: String,
    /// Replacement value (supports template expansion like "{{uuid}}", "{{faker.email}}")
    pub replace: String,
    /// Operation to perform
    #[serde(default)]
    pub operation: TransformOperation,
}

/// Transform operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TransformOperation {
    /// Replace the value at the path
    #[default]
    Replace,
    /// Add a new field at the path
    Add,
    /// Remove the field at the path
    Remove,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            target_url: None,
            timeout_seconds: 30,
            follow_redirects: true,
            headers: HashMap::new(),
            prefix: None,
            passthrough_by_default: false,
            rules: Vec::new(),
            migration_enabled: false,
            migration_groups: HashMap::new(),
            request_replacements: Vec::new(),
            response_replacements: Vec::new(),
            allow_absolute_url_upstream: false,
            upstream_allowlist: None,
        }
    }
}

#[cfg(test)]
mod proxy_config_tests {
    use super::*;
    use http::{HeaderMap, HeaderValue, Method, Uri};
    use serde_json::json;

    fn rule(pattern: &str, target: &str) -> ProxyRule {
        ProxyRule {
            path_pattern: pattern.to_string(),
            target_url: target.to_string(),
            pattern: pattern.to_string(),
            upstream_url: target.to_string(),
            ..Default::default()
        }
    }

    fn config_with(rules: Vec<ProxyRule>) -> ProxyConfig {
        ProxyConfig {
            enabled: true,
            target_url: Some("http://default-upstream".to_string()),
            rules,
            ..Default::default()
        }
    }

    // ─── Defaults ───────────────────────────────────────────────────────────

    #[test]
    fn new_sets_proxy_defaults() {
        let config = ProxyConfig::new("http://upstream.example.com".to_string());
        assert!(config.enabled);
        assert_eq!(config.target_url.as_deref(), Some("http://upstream.example.com"));
        assert_eq!(config.prefix.as_deref(), Some("/proxy/"));
        assert!(config.passthrough_by_default);
        assert_eq!(config.timeout_seconds, 30);
        assert!(config.follow_redirects);
    }

    #[test]
    fn default_config_is_disabled_without_prefix() {
        let config = ProxyConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.target_url, None);
        assert_eq!(config.prefix, None);
        assert!(!config.passthrough_by_default);
    }

    // ─── Migration mode resolution ──────────────────────────────────────────

    #[test]
    fn migration_disabled_yields_no_mode() {
        let mut config = config_with(vec![rule("/api/*", "http://a")]);
        config.migration_enabled = false;
        assert_eq!(config.get_effective_migration_mode("/api/users"), None);
    }

    #[test]
    fn group_override_wins_over_route_mode() {
        let mut grouped = rule("/api/*", "http://a");
        grouped.migration_mode = MigrationMode::Real;
        grouped.migration_group = Some("billing".to_string());

        let mut config = config_with(vec![grouped]);
        config.migration_enabled = true;
        // No group override → route-specific mode.
        assert_eq!(config.get_effective_migration_mode("/api/users"), Some(MigrationMode::Real));
        // Group override takes precedence.
        config.update_group_migration_mode("billing", MigrationMode::Shadow);
        assert_eq!(config.get_effective_migration_mode("/api/users"), Some(MigrationMode::Shadow));
    }

    #[test]
    fn first_matching_rule_wins() {
        let mut first = rule("/api/*", "http://a");
        first.migration_mode = MigrationMode::Mock;
        let mut second = rule("/api/users", "http://b");
        second.migration_mode = MigrationMode::Real;

        let mut config = config_with(vec![first, second]);
        config.migration_enabled = true;
        assert_eq!(config.get_effective_migration_mode("/api/users"), Some(MigrationMode::Mock));
    }

    // ─── Proxy decisions ────────────────────────────────────────────────────

    #[test]
    fn should_proxy_respects_enabled_flag() {
        let mut config = config_with(vec![rule("/api/*", "http://a")]);
        config.enabled = false;
        assert!(!config.should_proxy(&Method::GET, "/api/users"));
    }

    #[test]
    fn migration_modes_force_mock_or_proxy() {
        let mut mock_rule = rule("/mock/*", "http://a");
        mock_rule.migration_mode = MigrationMode::Mock;
        let mut real_rule = rule("/real/*", "http://b");
        real_rule.migration_mode = MigrationMode::Real;
        let mut shadow_rule = rule("/shadow/*", "http://c");
        shadow_rule.migration_mode = MigrationMode::Shadow;

        let mut config = config_with(vec![mock_rule, real_rule, shadow_rule]);
        config.migration_enabled = true;

        assert!(!config.should_proxy(&Method::GET, "/mock/x"), "Mock forces mock");
        assert!(config.should_proxy(&Method::GET, "/real/x"), "Real forces proxy");
        assert!(config.should_proxy(&Method::GET, "/shadow/x"), "Shadow forces proxy");
    }

    #[test]
    fn should_proxy_skips_conditional_rules_and_falls_back_to_prefix() {
        let mut conditional = rule("/api/*", "http://a");
        conditional.condition = Some("header[x-test] == 'yes'".to_string());

        let mut config = config_with(vec![conditional]);
        config.prefix = Some("/proxy/".to_string());

        // The legacy method cannot evaluate conditions, so the conditional
        // rule is skipped and the prefix decides.
        assert!(!config.should_proxy(&Method::GET, "/api/users"));
        assert!(config.should_proxy(&Method::GET, "/proxy/other"));
    }

    #[test]
    fn prefix_logic_proxies_everything_when_unset() {
        let mut config = config_with(vec![]);
        config.prefix = None;
        assert!(config.should_proxy(&Method::GET, "/any/path"));
        config.prefix = Some("/proxy/".to_string());
        assert!(config.should_proxy(&Method::GET, "/proxy/any"));
        assert!(!config.should_proxy(&Method::GET, "/other/any"));
    }

    #[test]
    fn conditional_rules_evaluate_request_context() {
        let mut conditional = rule("/api/*", "http://a");
        conditional.condition = Some("header[x-test] != ''".to_string());
        let config = config_with(vec![conditional]);

        let uri = Uri::from_static("/api/users");
        let mut headers = HeaderMap::new();
        headers.insert("x-test", HeaderValue::from_static("yes"));
        assert!(config.should_proxy_with_condition(&Method::GET, &uri, &headers, None));

        // Condition false and conditional rules exist → no prefix fallback.
        let empty_headers = HeaderMap::new();
        assert!(!config.should_proxy_with_condition(&Method::GET, &uri, &empty_headers, None));
    }

    #[test]
    fn shadow_mode_detected_only_for_shadow_rules() {
        let mut shadow = rule("/shadow/*", "http://a");
        shadow.migration_mode = MigrationMode::Shadow;
        let mut real = rule("/real/*", "http://b");
        real.migration_mode = MigrationMode::Real;

        let mut config = config_with(vec![shadow, real]);
        assert!(!config.should_shadow("/shadow/x"), "migration disabled");
        config.migration_enabled = true;
        assert!(config.should_shadow("/shadow/x"));
        assert!(!config.should_shadow("/real/x"));
    }

    // ─── Upstream and prefix resolution ─────────────────────────────────────

    #[test]
    fn upstream_url_prefers_matching_rule() {
        let config = config_with(vec![rule("/api/*", "http://api-upstream")]);
        assert_eq!(config.get_upstream_url("/api/users"), "http://api-upstream");
        assert_eq!(config.get_upstream_url("/other"), "http://default-upstream");

        let mut no_target = config_with(vec![]);
        no_target.target_url = None;
        assert_eq!(no_target.get_upstream_url("/fallback"), "/fallback");
    }

    #[test]
    fn strip_prefix_normalizes_leading_slash() {
        let mut config = config_with(vec![]);
        config.prefix = Some("/proxy/".to_string());
        assert_eq!(config.strip_prefix("/proxy/api/users"), "/api/users");
        // Non-matching paths pass through unchanged.
        assert_eq!(config.strip_prefix("/other"), "/other");

        // A prefix without trailing slash still yields a leading slash.
        config.prefix = Some("/proxy".to_string());
        assert_eq!(config.strip_prefix("/proxyusers"), "/users");

        config.prefix = None;
        assert_eq!(config.strip_prefix("/anything"), "/anything");
    }

    #[test]
    fn path_patterns_support_wildcards() {
        let config = config_with(vec![rule("/api/*", "http://a")]);
        assert!(config.path_matches_pattern("/api/*", "/api/users"));
        assert!(config.path_matches_pattern("/api/*", "/api"));
        assert!(!config.path_matches_pattern("/api/*", "/other"));
        assert!(config.path_matches_pattern("/health", "/health"));
        assert!(!config.path_matches_pattern("/health", "/health/extra"));
    }

    // ─── Rule and group mutation ────────────────────────────────────────────

    #[test]
    fn rule_migration_mode_updates_by_either_pattern_alias() {
        let mut config = config_with(vec![rule("/api/*", "http://a")]);
        config.migration_enabled = true;
        assert!(config.update_rule_migration_mode("/api/*", MigrationMode::Real));
        assert_eq!(config.get_effective_migration_mode("/api/users"), Some(MigrationMode::Real));
        // Unknown pattern reports not-found.
        assert!(!config.update_rule_migration_mode("/nope", MigrationMode::Mock));
    }

    #[test]
    fn route_toggle_cycles_through_migration_stages() {
        let mut config = config_with(vec![rule("/api/*", "http://a")]);
        // Auto starts the cycle at Mock.
        assert_eq!(config.toggle_route_migration("/api/*"), Some(MigrationMode::Mock));
        assert_eq!(config.toggle_route_migration("/api/*"), Some(MigrationMode::Shadow));
        assert_eq!(config.toggle_route_migration("/api/*"), Some(MigrationMode::Real));
        assert_eq!(config.toggle_route_migration("/api/*"), Some(MigrationMode::Mock));
        assert_eq!(config.toggle_route_migration("/unknown"), None);
    }

    #[test]
    fn group_toggle_defaults_to_auto_when_absent() {
        let mut config = config_with(vec![]);
        assert_eq!(config.toggle_group_migration("billing"), MigrationMode::Mock);
        assert_eq!(config.toggle_group_migration("billing"), MigrationMode::Shadow);
    }

    // ─── Migration reporting ────────────────────────────────────────────────

    #[test]
    fn migration_routes_report_effective_modes() {
        let mut grouped = rule("/api/*", "http://a");
        grouped.migration_mode = MigrationMode::Real;
        grouped.migration_group = Some("billing".to_string());
        let plain = rule("/plain", "http://b");

        let mut config = config_with(vec![grouped, plain]);
        config.update_group_migration_mode("billing", MigrationMode::Shadow);

        let routes = config.get_migration_routes();
        assert_eq!(routes.len(), 2);
        let grouped_info = routes.iter().find(|r| r.pattern == "/api/*").unwrap();
        assert_eq!(grouped_info.migration_mode, MigrationMode::Shadow);
        assert_eq!(grouped_info.route_mode, MigrationMode::Real);
        assert_eq!(grouped_info.migration_group.as_deref(), Some("billing"));
        let plain_info = routes.iter().find(|r| r.pattern == "/plain").unwrap();
        assert_eq!(plain_info.migration_mode, plain_info.route_mode);
    }

    #[test]
    fn migration_groups_include_routeless_entries() {
        let mut grouped = rule("/api/*", "http://a");
        grouped.migration_group = Some("billing".to_string());
        let mut config = config_with(vec![grouped]);
        config.update_group_migration_mode("empty", MigrationMode::Real);

        let groups = config.get_migration_groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups["billing"].route_count, 1);
        assert_eq!(groups["empty"].route_count, 0);
        assert_eq!(groups["empty"].migration_mode, MigrationMode::Real);
    }

    // ─── MigrationMode serialization ────────────────────────────────────────

    #[test]
    fn migration_mode_serializes_lowercase() {
        assert_eq!(serde_json::to_value(MigrationMode::Shadow).unwrap(), json!("shadow"));
        assert_eq!(
            serde_json::from_value::<MigrationMode>(json!("real")).unwrap(),
            MigrationMode::Real
        );
        // A rule JSON without migration_mode falls back to the Auto default.
        let rule: ProxyRule = serde_json::from_value(json!({
            "path_pattern": "/a",
            "target_url": "http://a",
            "enabled": true,
            "pattern": "/a",
            "upstream_url": "http://a"
        }))
        .unwrap();
        assert_eq!(rule.migration_mode, MigrationMode::Auto);
    }
}
