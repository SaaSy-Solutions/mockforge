//! Field-level and entity-level reality mixing
//!
//! This module provides per-field and per-entity reality source configuration,
//! enabling fine-grained control over which fields use real vs mock vs recorded data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Reality source for a field or entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RealitySource {
    /// Use real upstream data
    Real,
    /// Use mock/synthetic data
    Mock,
    /// Use recorded production data
    Recorded,
    /// Use synthetic/generated data
    Synthetic,
}

impl RealitySource {
    /// Convert to blend ratio (0.0 = mock, 1.0 = real)
    pub fn to_blend_ratio(&self) -> f64 {
        match self {
            RealitySource::Real => 1.0,
            RealitySource::Mock => 0.0,
            RealitySource::Recorded => 0.5, // Recorded is between mock and real
            RealitySource::Synthetic => 0.0, // Synthetic is like mock
        }
    }
}

/// Field pattern for matching JSON paths
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FieldPattern {
    /// JSON path pattern (e.g., "id", "email", "*.currency", "user.pii.*")
    pub path: String,
    /// Reality source to use for matching fields
    pub source: RealitySource,
    /// Optional priority (higher = more specific, checked first)
    #[serde(default)]
    pub priority: i32,
}

impl FieldPattern {
    /// Check if a JSON path matches this pattern
    ///
    /// Supports:
    /// - Exact match: "id" matches "id"
    /// - Wildcard suffix: "*.currency" matches "user.currency", "order.currency"
    /// - Wildcard prefix: "user.*" matches "user.id", "user.email"
    /// - Full wildcard: "*" matches everything
    pub fn matches(&self, json_path: &str) -> bool {
        if self.path == "*" {
            return true;
        }

        // Check for wildcard patterns
        if self.path.ends_with(".*") {
            let prefix = &self.path[..self.path.len() - 2];
            return json_path.starts_with(prefix) && json_path.len() > prefix.len();
        }

        if self.path.starts_with("*.") {
            let suffix = &self.path[2..];
            return json_path.ends_with(suffix) && json_path.len() > suffix.len();
        }

        // Exact match
        self.path == json_path
    }
}

/// Entity-level reality rule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EntityRealityRule {
    /// Entity type (e.g., "user", "order", "currency")
    pub entity_type: String,
    /// Reality source to use for this entity
    pub source: RealitySource,
    /// Optional field overrides within this entity
    #[serde(default)]
    pub field_overrides: HashMap<String, RealitySource>,
}

/// Field reality configuration
///
/// Configures per-field and per-entity reality sources for fine-grained
/// control over data blending.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FieldRealityConfig {
    /// Whether field-level mixing is enabled
    #[serde(default)]
    pub enabled: bool,
    /// Field patterns for matching JSON paths
    #[serde(default)]
    pub field_patterns: Vec<FieldPattern>,
    /// Entity-level rules
    #[serde(default)]
    pub entity_rules: HashMap<String, EntityRealityRule>,
    /// Default reality source when no pattern matches
    #[serde(default)]
    pub default_source: Option<RealitySource>,
}

impl FieldRealityConfig {
    /// Create a new field reality config
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable field-level mixing
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Add a field pattern
    pub fn add_field_pattern(mut self, pattern: FieldPattern) -> Self {
        // Sort by priority (higher priority first)
        self.field_patterns.push(pattern);
        self.field_patterns.sort_by(|a, b| b.priority.cmp(&a.priority));
        self
    }

    /// Add an entity rule
    pub fn add_entity_rule(mut self, entity_type: String, rule: EntityRealityRule) -> Self {
        self.entity_rules.insert(entity_type, rule);
        self
    }

    /// Get the reality source for a JSON path
    ///
    /// Checks in order:
    /// 1. Field patterns (by priority)
    /// 2. Entity rules
    /// 3. Default source
    ///
    /// Returns None if no match and no default
    pub fn get_source_for_path(&self, json_path: &str) -> Option<RealitySource> {
        if !self.enabled {
            return None;
        }

        // Check field patterns (already sorted by priority)
        for pattern in &self.field_patterns {
            if pattern.matches(json_path) {
                return Some(pattern.source);
            }
        }

        // Check entity rules
        // Extract entity type from path (first segment)
        if let Some(dot_pos) = json_path.find('.') {
            let entity_type = &json_path[..dot_pos];
            if let Some(rule) = self.entity_rules.get(entity_type) {
                // Check for field override
                let field = &json_path[dot_pos + 1..];
                if let Some(override_source) = rule.field_overrides.get(field) {
                    return Some(*override_source);
                }
                return Some(rule.source);
            }
        } else {
            // Single segment path - check if it's an entity type
            if let Some(rule) = self.entity_rules.get(json_path) {
                return Some(rule.source);
            }
        }

        // Return default if set
        self.default_source
    }

    /// Get the blend ratio for a JSON path
    ///
    /// Returns the blend ratio (0.0 to 1.0) for the given path,
    /// or None if field mixing is disabled or no pattern matches.
    pub fn get_blend_ratio_for_path(&self, json_path: &str) -> Option<f64> {
        self.get_source_for_path(json_path).map(|source| source.to_blend_ratio())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_pattern_exact_match() {
        let pattern = FieldPattern {
            path: "id".to_string(),
            source: RealitySource::Recorded,
            priority: 0,
        };
        assert!(pattern.matches("id"));
        assert!(!pattern.matches("email"));
    }

    #[test]
    fn test_field_pattern_wildcard_suffix() {
        let pattern = FieldPattern {
            path: "*.currency".to_string(),
            source: RealitySource::Real,
            priority: 0,
        };
        assert!(pattern.matches("user.currency"));
        assert!(pattern.matches("order.currency"));
        assert!(!pattern.matches("currency"));
    }

    #[test]
    fn test_field_pattern_wildcard_prefix() {
        let pattern = FieldPattern {
            path: "user.*".to_string(),
            source: RealitySource::Synthetic,
            priority: 0,
        };
        assert!(pattern.matches("user.id"));
        assert!(pattern.matches("user.email"));
        assert!(!pattern.matches("order.id"));
    }

    #[test]
    fn test_field_reality_config_path_matching() {
        let mut config = FieldRealityConfig::new().enable();
        config = config.add_field_pattern(FieldPattern {
            path: "id".to_string(),
            source: RealitySource::Recorded,
            priority: 10,
        });
        config = config.add_field_pattern(FieldPattern {
            path: "*.pii".to_string(),
            source: RealitySource::Synthetic,
            priority: 5,
        });

        assert_eq!(config.get_source_for_path("id"), Some(RealitySource::Recorded));
        assert_eq!(config.get_source_for_path("user.pii"), Some(RealitySource::Synthetic));
    }

    #[test]
    fn test_entity_rule() {
        let mut config = FieldRealityConfig::new().enable();
        let mut rule = EntityRealityRule {
            entity_type: "currency".to_string(),
            source: RealitySource::Real,
            field_overrides: HashMap::new(),
        };
        rule.field_overrides.insert("rate".to_string(), RealitySource::Real);
        config = config.add_entity_rule("currency".to_string(), rule);

        assert_eq!(config.get_source_for_path("currency"), Some(RealitySource::Real));
        assert_eq!(config.get_source_for_path("currency.rate"), Some(RealitySource::Real));
    }
}
