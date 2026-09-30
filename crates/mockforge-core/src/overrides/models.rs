//! Override data models and types
//!
//! This module defines the core data structures for the override system:
//! - OverrideRule: Configuration for applying overrides (the wire format)
//! - OverrideMode: How patches are applied
//! - PatchOp: Individual patch operations
//! - Overrides: A validated rule set with its targets compiled

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Configuration for a single override rule
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OverrideRule {
    /// Human-readable label shown in the admin UI
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Disabled rules are kept in the set but never applied
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Target selectors for matching operations:
    /// - "operation:opId" - match by operation ID
    /// - "tag:Tag" - match by OpenAPI tag
    /// - "regex:pattern" - match the operation ID by regex
    /// - "path:pattern" - match the path template by regex
    /// - "*" - match every operation
    pub targets: Vec<String>,
    /// JSON patch operations to apply when this rule matches
    pub patch: Vec<PatchOp>,
    /// Optional condition expression (JSONPath/XPath) that must evaluate to true
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// Override mode for applying patches: "replace" (default) or "merge"
    #[serde(default = "default_mode")]
    pub mode: OverrideMode,
    /// Whether to apply post-templating expansion after patching
    #[serde(default)]
    pub post_templating: bool,
}

/// Override mode for applying patches
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum OverrideMode {
    /// Replace values (default JSON patch behavior)
    #[serde(rename = "replace")]
    Replace,
    /// Merge objects and arrays instead of replacing
    #[serde(rename = "merge")]
    Merge,
}

/// JSON patch operation (RFC 6902 format)
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "op")]
pub enum PatchOp {
    /// Add a new value at the specified path
    #[serde(rename = "add")]
    Add {
        /// JSON pointer path to add the value
        path: String,
        /// Value to add
        value: Value,
    },
    /// Replace the value at the specified path
    #[serde(rename = "replace")]
    Replace {
        /// JSON pointer path to replace
        path: String,
        /// New value
        value: Value,
    },
    /// Remove the value at the specified path
    #[serde(rename = "remove")]
    Remove {
        /// JSON pointer path to remove
        path: String,
    },
}

impl PatchOp {
    /// The JSON pointer this operation acts on
    pub fn path(&self) -> &str {
        match self {
            PatchOp::Add { path, .. }
            | PatchOp::Replace { path, .. }
            | PatchOp::Remove { path } => path,
        }
    }
}

/// A parsed rule target
#[derive(Debug, Clone)]
pub enum Target {
    /// `operation:<id>`
    Operation(String),
    /// `tag:<tag>`
    Tag(String),
    /// `regex:<re>`, matched against the operation ID
    OperationRegex(Regex),
    /// `path:<re>`, matched against the path template
    PathRegex(Regex),
    /// `*`
    Any,
}

impl Target {
    fn parse(raw: &str) -> Result<Self, String> {
        let regex = |pattern: &str| Regex::new(pattern).map_err(|e| e.to_string());
        if raw == "*" {
            Ok(Target::Any)
        } else if let Some(id) = raw.strip_prefix("operation:") {
            Ok(Target::Operation(id.to_string()))
        } else if let Some(tag) = raw.strip_prefix("tag:") {
            Ok(Target::Tag(tag.to_string()))
        } else if let Some(pattern) = raw.strip_prefix("regex:") {
            regex(pattern).map(Target::OperationRegex)
        } else if let Some(pattern) = raw.strip_prefix("path:") {
            regex(pattern).map(Target::PathRegex)
        } else {
            Err("expected `operation:`, `tag:`, `regex:`, `path:`, or `*`".to_string())
        }
    }

    /// Whether this target selects the given operation
    pub fn matches(&self, operation_id: &str, tags: &[String], path: &str) -> bool {
        match self {
            Target::Operation(id) => id == operation_id,
            Target::Tag(tag) => tags.iter().any(|t| t == tag),
            Target::OperationRegex(re) => re.is_match(operation_id),
            Target::PathRegex(re) => re.is_match(path),
            Target::Any => true,
        }
    }
}

/// Why a rule set was rejected
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum OverridesError {
    /// A rule has no targets, so it could never apply
    #[error("rule {rule}: at least one target is required")]
    NoTargets {
        /// Index of the offending rule
        rule: usize,
    },
    /// A target string could not be parsed
    #[error("rule {rule}: invalid target `{target}`: {reason}")]
    InvalidTarget {
        /// Index of the offending rule
        rule: usize,
        /// The target as written
        target: String,
        /// What was wrong with it
        reason: String,
    },
    /// A patch path is not a JSON pointer
    #[error("rule {rule}: patch path `{path}` is not a JSON pointer (use `/field/sub`, or `` for the whole body)")]
    InvalidPointer {
        /// Index of the offending rule
        rule: usize,
        /// The path as written
        path: String,
    },
}

/// A validated rule set. Build it with [`Overrides::new`]; every target is
/// parsed up front so matching never fails at request time.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(try_from = "OverridesWire", into = "OverridesWire")]
pub struct Overrides {
    rules: Vec<OverrideRule>,
    targets: Vec<Vec<Target>>,
}

#[derive(Serialize, Deserialize)]
struct OverridesWire {
    rules: Vec<OverrideRule>,
}

impl TryFrom<OverridesWire> for Overrides {
    type Error = OverridesError;

    fn try_from(wire: OverridesWire) -> Result<Self, Self::Error> {
        Overrides::new(wire.rules)
    }
}

impl From<Overrides> for OverridesWire {
    fn from(overrides: Overrides) -> Self {
        OverridesWire {
            rules: overrides.rules,
        }
    }
}

impl Overrides {
    /// Validate a rule set and compile its targets
    pub fn new(rules: Vec<OverrideRule>) -> Result<Self, OverridesError> {
        let mut targets = Vec::with_capacity(rules.len());
        for (index, rule) in rules.iter().enumerate() {
            if rule.targets.is_empty() {
                return Err(OverridesError::NoTargets { rule: index });
            }
            let parsed = rule
                .targets
                .iter()
                .map(|raw| {
                    Target::parse(raw).map_err(|reason| OverridesError::InvalidTarget {
                        rule: index,
                        target: raw.clone(),
                        reason,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if let Some(op) =
                rule.patch.iter().find(|op| op.path().parse::<jsonptr::PointerBuf>().is_err())
            {
                return Err(OverridesError::InvalidPointer {
                    rule: index,
                    path: op.path().to_string(),
                });
            }
            targets.push(parsed);
        }
        Ok(Self { rules, targets })
    }

    /// The rules in this set, in application order
    pub fn rules(&self) -> &[OverrideRule] {
        &self.rules
    }

    /// Each rule paired with its compiled targets
    pub(crate) fn compiled(&self) -> impl Iterator<Item = (&OverrideRule, &[Target])> {
        self.rules.iter().zip(self.targets.iter().map(Vec::as_slice))
    }
}

fn default_enabled() -> bool {
    true
}

fn default_mode() -> OverrideMode {
    OverrideMode::Replace
}
