//! Core types for contract drift and budget management

use crate::ai_contract_diff::{ContractDiffResult, Mismatch, MismatchSeverity, MismatchType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for acceptable drift levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftBudget {
    /// Maximum number of breaking changes allowed
    pub max_breaking_changes: u32,
    /// Maximum number of non-breaking changes allowed
    pub max_non_breaking_changes: u32,
    /// Severity threshold for considering changes as breaking
    /// Changes at or above this severity are considered breaking
    pub severity_threshold: MismatchSeverity,
    /// Whether this budget is enabled
    pub enabled: bool,
}

impl Default for DriftBudget {
    fn default() -> Self {
        Self {
            max_breaking_changes: 0,
            max_non_breaking_changes: 10,
            severity_threshold: MismatchSeverity::High,
            enabled: true,
        }
    }
}

/// Global drift budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftBudgetConfig {
    /// Whether drift budget tracking is enabled
    pub enabled: bool,
    /// Default budget applied to all endpoints
    pub default_budget: Option<DriftBudget>,
    /// Per-endpoint budgets (key: "{method} {endpoint}")
    pub per_endpoint_budgets: HashMap<String, DriftBudget>,
    /// Breaking change detection rules
    pub breaking_change_rules: Vec<BreakingChangeRule>,
    /// Number of days to retain incident history
    pub incident_retention_days: u32,
}

impl Default for DriftBudgetConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_budget: Some(DriftBudget::default()),
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
    /// Number of breaking changes
    pub breaking_changes: u32,
    /// Number of non-breaking changes
    pub non_breaking_changes: u32,
    /// Mismatches that are considered breaking
    pub breaking_mismatches: Vec<Mismatch>,
    /// Mismatches that are non-breaking
    pub non_breaking_mismatches: Vec<Mismatch>,
    /// Current drift metrics
    pub metrics: DriftMetrics,
    /// Whether an incident should be created
    pub should_create_incident: bool,
}

impl DriftResult {
    /// Create a new drift result from contract diff result
    pub fn from_diff_result(
        diff_result: &ContractDiffResult,
        endpoint: String,
        method: String,
        budget: &DriftBudget,
        breaking_rules: &[BreakingChangeRule],
    ) -> Self {
        let mut breaking_mismatches = Vec::new();
        let mut non_breaking_mismatches = Vec::new();

        // Classify mismatches as breaking or non-breaking
        for mismatch in &diff_result.mismatches {
            let is_breaking = breaking_rules
                .iter()
                .filter(|rule| rule.enabled)
                .any(|rule| rule.matches(mismatch));

            if is_breaking {
                breaking_mismatches.push(mismatch.clone());
            } else {
                non_breaking_mismatches.push(mismatch.clone());
            }
        }

        let breaking_changes = breaking_mismatches.len() as u32;
        let non_breaking_changes = non_breaking_mismatches.len() as u32;
        let total_changes = breaking_changes + non_breaking_changes;

        // Check if budget is exceeded
        let budget_exceeded = breaking_changes > budget.max_breaking_changes
            || non_breaking_changes > budget.max_non_breaking_changes;

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
            non_breaking_changes,
            breaking_mismatches,
            non_breaking_mismatches,
            metrics,
            should_create_incident,
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
            (BreakingChangeRuleType::Severity, BreakingChangeRuleConfig::Severity { severity, include_higher }) => {
                if *include_higher {
                    mismatch.severity >= *severity
                } else {
                    mismatch.severity == *severity
                }
            }
            (BreakingChangeRuleType::MismatchType, BreakingChangeRuleConfig::MismatchType { mismatch_type }) => {
                mismatch.mismatch_type == *mismatch_type
            }
            (BreakingChangeRuleType::Custom, BreakingChangeRuleConfig::Custom { .. }) => {
                // Custom rules would need custom evaluation logic
                // For now, return false
                false
            }
            _ => false,
        }
    }
}
