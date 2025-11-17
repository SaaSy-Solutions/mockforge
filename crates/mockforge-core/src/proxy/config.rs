//! Proxy configuration types and settings

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Migration mode for route handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum MigrationMode {
    /// Always use mock (ignore proxy even if rule matches)
    Mock,
    /// Proxy to real backend AND generate mock response for comparison
    Shadow,
    /// Always use real backend (proxy)
    Real,
    /// Use existing priority chain (default, backward compatible)
    Auto,
}

impl Default for MigrationMode {
    fn default() -> Self {
        Self::Auto
    }
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
    pub fn should_proxy(&self, _method: &axum::http::Method, path: &str) -> bool {
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
        method: &axum::http::Method,
        uri: &axum::http::Uri,
        headers: &axum::http::HeaderMap,
        body: Option<&[u8]>,
    ) -> bool {
        use crate::proxy::conditional::find_matching_rule;

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
        if !self.rules.is_empty() {
            if find_matching_rule(&self.rules, method, uri, headers, body, |pattern, path| {
                self.path_matches_pattern(pattern, path)
            })
            .is_some()
            {
                return true;
            }
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
pub enum TransformOperation {
    /// Replace the value at the path
    Replace,
    /// Add a new field at the path
    Add,
    /// Remove the field at the path
    Remove,
}

impl Default for TransformOperation {
    fn default() -> Self {
        Self::Replace
    }
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
        }
    }
}
