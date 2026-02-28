//! Configuration types for Reality Continuum
//!
//! Defines the configuration structures for blending mock and real data sources,
//! including transition modes, schedules, and merge strategies.

use crate::protocol_abstraction::Protocol;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transition mode for blend ratio progression
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TransitionMode {
    /// Time-based progression using virtual clock
    TimeBased,
    /// Manual configuration (blend ratio set explicitly)
    #[default]
    Manual,
    /// Scheduled progression with fixed timeline
    Scheduled,
}

/// Merge strategy for blending responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum MergeStrategy {
    /// Field-level intelligent merge (deep merge objects, combine arrays)
    #[default]
    FieldLevel,
    /// Weighted selection (return mock with X% probability, real with (100-X)%)
    Weighted,
    /// Response body blending (merge arrays, average numeric fields)
    BodyBlend,
}

/// Configuration for Reality Continuum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ContinuumConfig {
    /// Whether the continuum feature is enabled
    #[serde(default = "default_false")]
    pub enabled: bool,
    /// Default blend ratio (0.0 = 100% mock, 1.0 = 100% real)
    #[serde(default = "default_blend_ratio")]
    pub default_ratio: f64,
    /// Transition mode for blend ratio progression
    #[serde(default)]
    pub transition_mode: TransitionMode,
    /// Time schedule for time-based transitions (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_schedule: Option<super::schedule::TimeSchedule>,
    /// Merge strategy for blending responses
    #[serde(default)]
    pub merge_strategy: MergeStrategy,
    /// Per-route blend ratio overrides
    #[serde(default)]
    pub routes: Vec<ContinuumRule>,
    /// Group-level blend ratio overrides
    #[serde(default)]
    pub groups: HashMap<String, f64>,
    /// Field-level reality mixing configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_mixing: Option<super::field_mixer::FieldRealityConfig>,
    /// Cross-protocol state sharing configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_protocol_state: Option<CrossProtocolStateConfig>,
}

fn default_false() -> bool {
    false
}

fn default_blend_ratio() -> f64 {
    0.0 // Start with 100% mock
}

impl Default for ContinuumConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_ratio: 0.0,
            transition_mode: TransitionMode::Manual,
            time_schedule: None,
            merge_strategy: MergeStrategy::FieldLevel,
            routes: Vec::new(),
            groups: HashMap::new(),
            field_mixing: None,
            cross_protocol_state: None,
        }
    }
}

impl ContinuumConfig {
    /// Create a new continuum configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable the continuum feature
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Set the default blend ratio
    pub fn with_default_ratio(mut self, ratio: f64) -> Self {
        self.default_ratio = ratio.clamp(0.0, 1.0);
        self
    }

    /// Set the transition mode
    pub fn with_transition_mode(mut self, mode: TransitionMode) -> Self {
        self.transition_mode = mode;
        self
    }

    /// Set the time schedule
    pub fn with_time_schedule(mut self, schedule: super::schedule::TimeSchedule) -> Self {
        self.time_schedule = Some(schedule);
        self
    }

    /// Set the merge strategy
    pub fn with_merge_strategy(mut self, strategy: MergeStrategy) -> Self {
        self.merge_strategy = strategy;
        self
    }

    /// Add a route-specific rule
    pub fn add_route(mut self, rule: ContinuumRule) -> Self {
        self.routes.push(rule);
        self
    }

    /// Set a group-level blend ratio
    pub fn set_group_ratio(mut self, group: String, ratio: f64) -> Self {
        self.groups.insert(group, ratio.clamp(0.0, 1.0));
        self
    }

    /// Set cross-protocol state configuration
    pub fn with_cross_protocol_state(mut self, config: CrossProtocolStateConfig) -> Self {
        self.cross_protocol_state = Some(config);
        self
    }
}

/// Cross-protocol state sharing configuration
///
/// Ensures that HTTP, WebSocket, gRPC, TCP, and webhooks all use the same
/// backing persona graph and unified state when configured.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CrossProtocolStateConfig {
    /// State model identifier (e.g., "ecommerce_v1", "finance_v1")
    ///
    /// This identifies the shared state model that defines how personas
    /// and entities are related across protocols.
    pub state_model: String,

    /// List of protocols that should share state
    ///
    /// When a protocol is included, it will use the same persona graph
    /// and unified state as other protocols in this list.
    #[serde(default)]
    pub share_state_across: Vec<Protocol>,

    /// Whether cross-protocol state sharing is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for CrossProtocolStateConfig {
    fn default() -> Self {
        Self {
            state_model: "default".to_string(),
            share_state_across: vec![Protocol::Http, Protocol::WebSocket, Protocol::Grpc],
            enabled: true,
        }
    }
}

impl CrossProtocolStateConfig {
    /// Create a new cross-protocol state configuration
    pub fn new(state_model: String) -> Self {
        Self {
            state_model,
            share_state_across: Vec::new(),
            enabled: true,
        }
    }

    /// Add a protocol to share state across
    pub fn add_protocol(mut self, protocol: Protocol) -> Self {
        if !self.share_state_across.contains(&protocol) {
            self.share_state_across.push(protocol);
        }
        self
    }

    /// Set the list of protocols to share state across
    pub fn with_protocols(mut self, protocols: Vec<Protocol>) -> Self {
        self.share_state_across = protocols;
        self
    }

    /// Check if a protocol should share state
    pub fn should_share_state(&self, protocol: &Protocol) -> bool {
        self.enabled && self.share_state_across.contains(protocol)
    }
}

/// Rule for per-route continuum configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ContinuumRule {
    /// Path pattern to match (supports wildcards like "/api/users/*")
    pub pattern: String,
    /// Blend ratio for this route (0.0 = 100% mock, 1.0 = 100% real)
    pub ratio: f64,
    /// Optional migration group this route belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Whether this rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl ContinuumRule {
    /// Create a new continuum rule
    pub fn new(pattern: String, ratio: f64) -> Self {
        Self {
            pattern,
            ratio: ratio.clamp(0.0, 1.0),
            group: None,
            enabled: true,
        }
    }

    /// Set the migration group
    pub fn with_group(mut self, group: String) -> Self {
        self.group = Some(group);
        self
    }

    /// Check if a path matches this rule's pattern
    pub fn matches_path(&self, path: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // Simple pattern matching - supports wildcards
        if self.pattern.ends_with("/*") {
            let prefix = &self.pattern[..self.pattern.len() - 2];
            // For wildcard patterns, path must start with prefix and have at least one more segment
            if let Some(remaining) = path.strip_prefix(prefix) {
                // Must have at least one segment after the prefix (not just a trailing slash)
                !remaining.is_empty() && remaining != "/"
            } else {
                false
            }
        } else {
            // Exact match only - no prefix matching for non-wildcard patterns
            path == self.pattern
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_continuum_config_default() {
        let config = ContinuumConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.default_ratio, 0.0);
        assert_eq!(config.transition_mode, TransitionMode::Manual);
    }

    #[test]
    fn test_continuum_config_builder() {
        let config = ContinuumConfig::new()
            .enable()
            .with_default_ratio(0.5)
            .with_transition_mode(TransitionMode::TimeBased);

        assert!(config.enabled);
        assert_eq!(config.default_ratio, 0.5);
        assert_eq!(config.transition_mode, TransitionMode::TimeBased);
    }

    #[test]
    fn test_continuum_rule_matching() {
        let rule = ContinuumRule::new("/api/users/*".to_string(), 0.5);
        assert!(rule.matches_path("/api/users/123"));
        assert!(rule.matches_path("/api/users/456"));
        assert!(!rule.matches_path("/api/orders/123"));

        let exact_rule = ContinuumRule::new("/api/health".to_string(), 0.0);
        assert!(exact_rule.matches_path("/api/health"));
        assert!(!exact_rule.matches_path("/api/health/check"));
    }

    #[test]
    fn test_ratio_clamping() {
        let rule = ContinuumRule::new("/test".to_string(), 1.5);
        assert_eq!(rule.ratio, 1.0);

        let rule = ContinuumRule::new("/test".to_string(), -0.5);
        assert_eq!(rule.ratio, 0.0);
    }
}
