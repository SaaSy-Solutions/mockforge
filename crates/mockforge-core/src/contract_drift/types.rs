//! Core types for contract drift and budget management

use crate::ai_contract_diff::{ContractDiffResult, Mismatch, MismatchSeverity, MismatchType};
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
    /// Service names can be explicit or match OpenAPI operation tags
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_service_budgets: HashMap<String, DriftBudget>,
    /// Per-tag budgets (key: OpenAPI tag name)
    /// Alternative to per_service_budgets, uses OpenAPI tags directly
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
                // Default rules: Critical and High severity are breaking
                BreakingChangeRule {
                    rule_type: BreakingChangeRuleType::Severity,
                    config: BreakingChangeRuleConfig::Severity {
                        severity: MismatchSeverity::High,
                        include_higher: true,
                    },
                    enabled: true,
                },
                // Missing required fields are always breaking
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
    pub fitness_test_results: Vec<crate::contract_drift::fitness::FitnessTestResult>,
    /// Consumer impact analysis (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumer_impact: Option<crate::contract_drift::consumer_mapping::ConsumerImpact>,
}

impl DriftResult {
    /// Create a new drift result from contract diff result
    ///
    /// `baseline_field_count` is optional and used for percentage-based budget calculations.
    /// If provided, it represents the historical baseline field count for the endpoint.
    pub fn from_diff_result(
        diff_result: &ContractDiffResult,
        endpoint: String,
        method: String,
        budget: &DriftBudget,
        breaking_rules: &[BreakingChangeRule],
        baseline_field_count: Option<f64>,
    ) -> Self {
        // Use three-way classification for better categorization
        use crate::contract_drift::BreakingChangeDetector;
        let detector = BreakingChangeDetector::new(breaking_rules.to_vec());
        let (non_breaking_mismatches, potentially_breaking_mismatches, breaking_mismatches) =
            detector.classify_three_way(&diff_result.mismatches);

        let breaking_changes = breaking_mismatches.len() as u32;
        let potentially_breaking_changes = potentially_breaking_mismatches.len() as u32;
        let non_breaking_changes = non_breaking_mismatches.len() as u32;
        let total_changes = breaking_changes + potentially_breaking_changes + non_breaking_changes;

        // Check if budget is exceeded
        // If percentage-based budget is set, use that; otherwise use absolute counts
        let budget_exceeded = if let Some(max_churn_percent) = budget.max_field_churn_percent {
            // Calculate field churn percentage using baseline from field tracker
            if let Some(baseline) = baseline_field_count {
                if baseline > 0.0 {
                    let churn_percent = (total_changes as f64 / baseline) * 100.0;
                    churn_percent > max_churn_percent
                        || breaking_changes > budget.max_breaking_changes
                } else {
                    // If baseline is 0, any changes represent 100% churn
                    total_changes > 0
                        && (100.0 > max_churn_percent
                            || breaking_changes > budget.max_breaking_changes)
                }
            } else {
                // No baseline available - fall back to absolute counts
                breaking_changes > budget.max_breaking_changes
                    || non_breaking_changes > budget.max_non_breaking_changes
            }
        } else {
            // Use absolute counts
            breaking_changes > budget.max_breaking_changes
                || non_breaking_changes > budget.max_non_breaking_changes
        };

        // Determine if incident should be created
        let should_create_incident = budget_exceeded || breaking_changes > 0;

        let metrics = DriftMetrics {
            endpoint: endpoint.clone(),
            method: method.clone(),
            breaking_changes,
            non_breaking_changes,
            total_changes,
            budget_exceeded,
            last_updated: chrono::Utc::now().timestamp(),
        };

        Self {
            budget_exceeded,
            breaking_changes,
            potentially_breaking_changes,
            non_breaking_changes,
            breaking_mismatches,
            potentially_breaking_mismatches,
            non_breaking_mismatches,
            metrics,
            should_create_incident,
            fitness_test_results: Vec::new(),
            consumer_impact: None,
        }
    }
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
                    // Reverse comparison: Critical < High < Medium < Low < Info in enum order
                    // But we want Critical > High > Medium > Low > Info for severity
                    // So we check if severity is <= mismatch.severity (reversed)
                    mismatch.severity <= *severity
                } else {
                    mismatch.severity == *severity
                }
            }
            (
                BreakingChangeRuleType::MismatchType,
                BreakingChangeRuleConfig::MismatchType { mismatch_type },
            ) => mismatch.mismatch_type == *mismatch_type,
            (BreakingChangeRuleType::Custom, BreakingChangeRuleConfig::Custom { config }) => {
                // Custom rules evaluate JSON config against mismatch fields.
                // Supported config keys:
                //   "path_contains": string — match if mismatch.path contains this substring
                //   "path_regex": string — match if mismatch.path matches this regex
                //   "description_contains": string — match if description contains substring
                //   "severity": string — match exact severity level
                //   "mismatch_type": string — match exact mismatch type
                //   "method": string — match exact HTTP method
                let mut matched = true;

                if let Some(path_contains) = config.get("path_contains").and_then(|v| v.as_str()) {
                    if !mismatch.path.contains(path_contains) {
                        matched = false;
                    }
                }

                if matched {
                    if let Some(desc_contains) =
                        config.get("description_contains").and_then(|v| v.as_str())
                    {
                        if !mismatch.description.contains(desc_contains) {
                            matched = false;
                        }
                    }
                }

                if matched {
                    if let Some(severity_str) = config.get("severity").and_then(|v| v.as_str()) {
                        let severity_matches = match severity_str.to_lowercase().as_str() {
                            "critical" => mismatch.severity == MismatchSeverity::Critical,
                            "high" => mismatch.severity == MismatchSeverity::High,
                            "medium" => mismatch.severity == MismatchSeverity::Medium,
                            "low" => mismatch.severity == MismatchSeverity::Low,
                            "info" => mismatch.severity == MismatchSeverity::Info,
                            _ => true, // Unknown severity string, don't filter
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
