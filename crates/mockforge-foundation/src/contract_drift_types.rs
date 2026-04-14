//! Core types for contract drift and budget management
//!
//! Extracted from `mockforge-core::contract_drift::types` as part of the
//! foundation crate split.
//!
//! `DriftResult` remains in mockforge-core because it has cross-module references
//! to `fitness::FitnessTestResult` and `consumer_mapping::ConsumerImpact`.

use crate::contract_diff_types::{Mismatch, MismatchSeverity, MismatchType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for acceptable drift levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DriftBudget {
    /// Maximum number of breaking changes allowed (used if percentage not set)
    #[serde(default)]
    pub max_breaking_changes: u32,
    /// Maximum number of non-breaking changes allowed (used if percentage not set)
    #[serde(default)]
    pub max_non_breaking_changes: u32,
    /// Maximum percentage of field churn allowed (0.0-100.0)
    /// If set, this takes precedence over absolute counts
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_field_churn_percent: Option<f64>,
    /// Time window in days for percentage calculations (e.g., 30 for monthly)
    /// Only used when max_field_churn_percent is set
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_window_days: Option<u32>,
    /// Severity threshold for considering changes as breaking
    /// Changes at or above this severity are considered breaking
    #[serde(default)]
    pub severity_threshold: MismatchSeverity,
    /// Whether this budget is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for DriftBudget {
    fn default() -> Self {
        Self {
            max_breaking_changes: 0,
            max_non_breaking_changes: 10,
            max_field_churn_percent: None,
            time_window_days: None,
            severity_threshold: MismatchSeverity::High,
            enabled: true,
        }
    }
}

/// Global drift budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DriftBudgetConfig {
    /// Whether drift budget tracking is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Default budget applied to all endpoints
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_budget: Option<DriftBudget>,
    /// Per-workspace budgets (key: workspace_id)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_workspace_budgets: HashMap<String, DriftBudget>,
    /// Per-service budgets (key: service_name or OpenAPI tag)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_service_budgets: HashMap<String, DriftBudget>,
    /// Per-tag budgets (key: OpenAPI tag name)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_tag_budgets: HashMap<String, DriftBudget>,
    /// Per-endpoint budgets (key: "{method} {endpoint}")
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_endpoint_budgets: HashMap<String, DriftBudget>,
    /// Breaking change detection rules
    #[serde(default)]
    pub breaking_change_rules: Vec<BreakingChangeRule>,
    /// Number of days to retain incident history
    #[serde(default = "default_retention_days")]
    pub incident_retention_days: u32,
}

fn default_retention_days() -> u32 {
    90
}

impl Default for DriftBudgetConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_budget: Some(DriftBudget::default()),
            per_workspace_budgets: HashMap::new(),
            per_service_budgets: HashMap::new(),
            per_tag_budgets: HashMap::new(),
            per_endpoint_budgets: HashMap::new(),
            breaking_change_rules: vec![
                BreakingChangeRule {
                    rule_type: BreakingChangeRuleType::Severity,
                    config: BreakingChangeRuleConfig::Severity {
                        severity: MismatchSeverity::High,
                        include_higher: true,
                    },
                    enabled: true,
                },
                BreakingChangeRule {
                    rule_type: BreakingChangeRuleType::MismatchType,
                    config: BreakingChangeRuleConfig::MismatchType {
                        mismatch_type: MismatchType::MissingRequiredField,
                    },
                    enabled: true,
                },
            ],
            incident_retention_days: 90,
        }
    }
}

/// Rule for detecting breaking changes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BreakingChangeRule {
    /// Type of rule
    pub rule_type: BreakingChangeRuleType,
    /// Rule configuration
    pub config: BreakingChangeRuleConfig,
    /// Whether this rule is enabled
    pub enabled: bool,
}

/// Type of breaking change rule
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum BreakingChangeRuleType {
    /// Rule based on mismatch severity
    Severity,
    /// Rule based on mismatch type
    MismatchType,
    /// Custom rule with JSON configuration
    Custom,
}

/// Configuration for breaking change rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BreakingChangeRuleConfig {
    /// Severity-based rule
    Severity {
        /// Minimum severity level
        severity: MismatchSeverity,
        /// Whether to include higher severities
        include_higher: bool,
    },
    /// Mismatch type-based rule
    MismatchType {
        /// Mismatch type that is considered breaking
        mismatch_type: MismatchType,
    },
    /// Custom rule with JSON configuration
    Custom {
        /// Custom rule configuration (JSON)
        config: serde_json::Value,
    },
}

/// Current drift metrics for an endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftMetrics {
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Number of breaking changes detected
    pub breaking_changes: u32,
    /// Number of non-breaking changes detected
    pub non_breaking_changes: u32,
    /// Total number of changes
    pub total_changes: u32,
    /// Whether budget is exceeded
    pub budget_exceeded: bool,
    /// Last updated timestamp
    pub last_updated: i64,
}

impl BreakingChangeRule {
    /// Check if a mismatch matches this rule
    pub fn matches(&self, mismatch: &Mismatch) -> bool {
        if !self.enabled {
            return false;
        }

        match (&self.rule_type, &self.config) {
            (
                BreakingChangeRuleType::Severity,
                BreakingChangeRuleConfig::Severity {
                    severity,
                    include_higher,
                },
            ) => {
                if *include_higher {
                    mismatch.severity <= *severity
                } else {
                    mismatch.severity == *severity
                }
            }
            (
                BreakingChangeRuleType::MismatchType,
                BreakingChangeRuleConfig::MismatchType { mismatch_type },
            ) => &mismatch.mismatch_type == mismatch_type,
            (BreakingChangeRuleType::Custom, BreakingChangeRuleConfig::Custom { config }) => {
                // Custom rule matching — delegated to configuration JSON
                let mut matched = true;

                if let Some(path_pattern) = config.get("path_pattern").and_then(|v| v.as_str()) {
                    if !mismatch.path.contains(path_pattern) {
                        matched = false;
                    }
                }

                if matched {
                    if let Some(severity_str) = config.get("severity").and_then(|v| v.as_str()) {
                        let severity_matches = match severity_str {
                            "critical" => mismatch.severity == MismatchSeverity::Critical,
                            "high" => mismatch.severity == MismatchSeverity::High,
                            "medium" => mismatch.severity == MismatchSeverity::Medium,
                            "low" => mismatch.severity == MismatchSeverity::Low,
                            "info" => mismatch.severity == MismatchSeverity::Info,
                            _ => true,
                        };
                        if !severity_matches {
                            matched = false;
                        }
                    }
                }

                if matched {
                    if let Some(method_str) = config.get("method").and_then(|v| v.as_str()) {
                        if let Some(ref mismatch_method) = mismatch.method {
                            if !mismatch_method.eq_ignore_ascii_case(method_str) {
                                matched = false;
                            }
                        } else {
                            matched = false;
                        }
                    }
                }

                matched
            }
            _ => false,
        }
    }
}

// ============================================================================
// Fitness test result (pure data; evaluator logic stays in core)
// ============================================================================

/// Result of evaluating a fitness function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessTestResult {
    /// ID of the fitness function that was evaluated
    pub function_id: String,
    /// Name of the fitness function
    pub function_name: String,
    /// Whether the test passed
    pub passed: bool,
    /// Human-readable message about the result
    pub message: String,
    /// Metrics collected during evaluation
    pub metrics: HashMap<String, f64>,
}

// ============================================================================
// Consumer mapping types (pure data; analyzer logic stays in core)
// ============================================================================

/// Type of consuming application
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AppType {
    /// Web application
    Web,
    /// Mobile application (iOS)
    #[serde(rename = "mobile_ios")]
    MobileIos,
    /// Mobile application (Android)
    #[serde(rename = "mobile_android")]
    MobileAndroid,
    /// Internal tool or service
    InternalTool,
    /// CLI tool
    Cli,
    /// Other/unknown
    Other,
}

impl std::fmt::Display for AppType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppType::Web => write!(f, "Web App"),
            AppType::MobileIos => write!(f, "Mobile App (iOS)"),
            AppType::MobileAndroid => write!(f, "Mobile App (Android)"),
            AppType::InternalTool => write!(f, "Internal Tool"),
            AppType::Cli => write!(f, "CLI Tool"),
            AppType::Other => write!(f, "Other"),
        }
    }
}

/// A consuming application that uses SDK methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ConsumingApp {
    /// Unique identifier for the app
    pub app_id: String,
    /// Human-readable name
    pub app_name: String,
    /// Type of application
    pub app_type: AppType,
    /// Optional repository URL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_url: Option<String>,
    /// Timestamp when this app was last updated
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<i64>,
    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// An SDK method that calls an endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SDKMethod {
    /// SDK name (e.g., "typescript-sdk", "python-sdk")
    pub sdk_name: String,
    /// Method name (e.g., "getUser", "createOrder")
    pub method_name: String,
    /// List of consuming apps that use this SDK method
    #[serde(default)]
    pub consuming_apps: Vec<ConsumingApp>,
}

/// Mapping from endpoint to SDK methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerMapping {
    /// Endpoint path or operation ID
    pub endpoint: String,
    /// HTTP method or protocol identifier
    pub method: String,
    /// SDK methods that call this endpoint/operation
    #[serde(default)]
    pub sdk_methods: Vec<SDKMethod>,
    /// Timestamp when this mapping was created
    #[serde(default)]
    pub created_at: i64,
    /// Timestamp when this mapping was last updated
    #[serde(default)]
    pub updated_at: i64,
}

/// Impact analysis result showing which consumers are affected by drift
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerImpact {
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// SDK methods that are affected
    pub affected_sdk_methods: Vec<SDKMethod>,
    /// Applications that are affected
    pub affected_apps: Vec<ConsumingApp>,
    /// Human-readable impact summary
    pub impact_summary: String,
}

// ============================================================================
// DriftResult (pure data; constructors / evaluator helpers stay in core)
// ============================================================================

/// Result of drift budget evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftResult {
    /// Whether budget is exceeded
    pub budget_exceeded: bool,
    /// Number of breaking changes (definitely breaking)
    pub breaking_changes: u32,
    /// Number of potentially breaking changes (requires review)
    #[serde(default)]
    pub potentially_breaking_changes: u32,
    /// Number of non-breaking changes
    pub non_breaking_changes: u32,
    /// Mismatches that are considered breaking (definitely breaking)
    pub breaking_mismatches: Vec<Mismatch>,
    /// Mismatches that are potentially breaking (requires review)
    #[serde(default)]
    pub potentially_breaking_mismatches: Vec<Mismatch>,
    /// Mismatches that are non-breaking
    pub non_breaking_mismatches: Vec<Mismatch>,
    /// Current drift metrics
    pub metrics: DriftMetrics,
    /// Whether an incident should be created
    pub should_create_incident: bool,
    /// Results from fitness function tests
    #[serde(default)]
    pub fitness_test_results: Vec<FitnessTestResult>,
    /// Consumer impact analysis (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumer_impact: Option<ConsumerImpact>,
}
