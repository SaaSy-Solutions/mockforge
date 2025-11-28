//! Deceptive Canary Mode
//!
//! Routes a small percentage of team traffic to "deceptive deploys" by default, with opt-out.
//! Great for dogfooding realism in cloud deployments.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Deceptive canary configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DeceptiveCanaryConfig {
    /// Enable deceptive canary mode
    pub enabled: bool,
    /// Traffic percentage to route to deceptive deploy (0.0 to 1.0)
    pub traffic_percentage: f64,
    /// Team/user identification criteria
    pub team_identifiers: TeamIdentifiers,
    /// Opt-out header name (e.g., "X-Opt-Out-Canary")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opt_out_header: Option<String>,
    /// Opt-out query parameter name (e.g., "no-canary")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opt_out_query_param: Option<String>,
    /// Deceptive deploy URL to route to
    pub deceptive_deploy_url: String,
    /// Routing strategy for selecting which requests to route
    pub routing_strategy: CanaryRoutingStrategy,
    /// Statistics tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<CanaryStats>,
}

impl Default for DeceptiveCanaryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            traffic_percentage: 0.05, // 5% by default
            team_identifiers: TeamIdentifiers::default(),
            opt_out_header: Some("X-Opt-Out-Canary".to_string()),
            opt_out_query_param: Some("no-canary".to_string()),
            deceptive_deploy_url: String::new(),
            routing_strategy: CanaryRoutingStrategy::ConsistentHash,
            stats: Some(CanaryStats::default()),
        }
    }
}

/// Team/user identification criteria
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TeamIdentifiers {
    /// User agent patterns (regex patterns, "*" matches all)
    #[serde(default)]
    pub user_agents: Option<Vec<String>>,
    /// IP address ranges (CIDR notation or specific IPs)
    #[serde(default)]
    pub ip_ranges: Option<Vec<String>>,
    /// Header matching rules (header name -> value pattern)
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    /// Team names/IDs to match
    #[serde(default)]
    pub teams: Option<Vec<String>>,
}

/// Canary routing strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CanaryRoutingStrategy {
    /// Consistent hashing on user ID for consistent routing
    ConsistentHash,
    /// Random selection per request
    Random,
    /// Round-robin distribution
    RoundRobin,
}

/// Statistics for canary routing
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CanaryStats {
    /// Total requests processed
    pub total_requests: u64,
    /// Requests routed to canary
    pub canary_requests: u64,
    /// Requests that opted out
    pub opted_out_requests: u64,
    /// Requests that matched team criteria
    pub matched_requests: u64,
}

impl CanaryStats {
    /// Get canary routing percentage
    pub fn canary_percentage(&self) -> f64 {
        if self.matched_requests == 0 {
            return 0.0;
        }
        (self.canary_requests as f64 / self.matched_requests as f64) * 100.0
    }
}

/// Deceptive canary router
///
/// Handles routing logic for deceptive canary mode.
pub struct DeceptiveCanaryRouter {
    config: DeceptiveCanaryConfig,
    round_robin_counter: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl DeceptiveCanaryRouter {
    /// Create a new deceptive canary router
    pub fn new(config: DeceptiveCanaryConfig) -> Self {
        Self {
            config,
            round_robin_counter: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Check if a request should be routed to deceptive deploy
    ///
    /// # Arguments
    /// * `user_agent` - User agent string from request
    /// * `ip_address` - Client IP address
    /// * `headers` - Request headers
    /// * `query_params` - Query parameters
    /// * `user_id` - Optional user ID for consistent hashing
    ///
    /// # Returns
    /// True if request should be routed to deceptive deploy
    pub fn should_route_to_canary(
        &self,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
        headers: &HashMap<String, String>,
        query_params: &HashMap<String, String>,
        user_id: Option<&str>,
    ) -> bool {
        // Check if canary is enabled
        if !self.config.enabled {
            return false;
        }

        // Check opt-out mechanisms
        if let Some(opt_out_header) = &self.config.opt_out_header {
            if headers.get(opt_out_header).is_some() {
                return false;
            }
        }

        if let Some(opt_out_param) = &self.config.opt_out_query_param {
            if query_params.get(opt_out_param).is_some() {
                return false;
            }
        }

        // Check if request matches team criteria
        if !self.matches_team_criteria(user_agent, ip_address, headers) {
            return false;
        }

        // Apply routing strategy

        match self.config.routing_strategy {
            CanaryRoutingStrategy::ConsistentHash => {
                self.consistent_hash_route(user_id, ip_address)
            }
            CanaryRoutingStrategy::Random => self.random_route(),
            CanaryRoutingStrategy::RoundRobin => self.round_robin_route(),
        }
    }

    /// Check if request matches team identification criteria
    fn matches_team_criteria(
        &self,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
        headers: &HashMap<String, String>,
    ) -> bool {
        // Check user agent
        if let Some(user_agents) = &self.config.team_identifiers.user_agents {
            if let Some(ua) = user_agent {
                let matches = user_agents.iter().any(|pattern| {
                    if pattern == "*" {
                        true
                    } else {
                        // Simple substring match (could be enhanced with regex)
                        ua.contains(pattern)
                    }
                });
                if !matches {
                    return false;
                }
            } else if !user_agents.contains(&"*".to_string()) {
                return false;
            }
        }

        // Check IP ranges
        if let Some(ip_ranges) = &self.config.team_identifiers.ip_ranges {
            if let Some(ip) = ip_address {
                let matches = ip_ranges.iter().any(|range| {
                    if range == "*" {
                        true
                    } else {
                        // Simple prefix match (could be enhanced with CIDR parsing)
                        ip.starts_with(range) || range == ip
                    }
                });
                if !matches {
                    return false;
                }
            } else if !ip_ranges.contains(&"*".to_string()) {
                return false;
            }
        }

        // Check headers
        if let Some(header_rules) = &self.config.team_identifiers.headers {
            for (header_name, expected_value) in header_rules {
                if let Some(actual_value) = headers.get(header_name) {
                    if actual_value != expected_value && expected_value != "*" {
                        return false;
                    }
                } else if expected_value != "*" {
                    return false;
                }
            }
        }

        true
    }

    /// Consistent hash routing
    fn consistent_hash_route(&self, user_id: Option<&str>, ip_address: Option<&str>) -> bool {
        // Use user_id if available, otherwise fall back to IP
        let hash_input = user_id.unwrap_or_else(|| ip_address.unwrap_or("default"));

        // Simple hash function
        let mut hasher = DefaultHasher::new();
        hash_input.hash(&mut hasher);
        let hash = hasher.finish();

        // Convert to percentage (0.0 to 1.0)
        let percentage = (hash % 10000) as f64 / 10000.0;

        percentage < self.config.traffic_percentage
    }

    /// Random routing
    fn random_route(&self) -> bool {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_value: f64 = rng.gen();
        random_value < self.config.traffic_percentage
    }

    /// Round-robin routing
    fn round_robin_route(&self) -> bool {
        let counter = self.round_robin_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let cycle_size = (1.0 / self.config.traffic_percentage) as u64;
        counter.is_multiple_of(cycle_size)
    }

    /// Get current configuration
    pub fn config(&self) -> &DeceptiveCanaryConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: DeceptiveCanaryConfig) {
        self.config = config;
    }

    /// Get routing statistics
    pub fn stats(&self) -> Option<&CanaryStats> {
        self.config.stats.as_ref()
    }

    /// Update statistics (thread-safe)
    pub fn record_request(&self, routed: bool, opted_out: bool, matched: bool) {
        if let Some(stats) = &self.config.stats {
            // Note: This is a simplified implementation
            // In production, you'd want to use atomic counters or a proper stats collector
            // For now, stats are stored in config which is not thread-safe for updates
            // This would need to be refactored to use Arc<RwLock<CanaryStats>> for thread-safety
        }
    }
}

impl Default for DeceptiveCanaryRouter {
    fn default() -> Self {
        Self::new(DeceptiveCanaryConfig::default())
    }
}
