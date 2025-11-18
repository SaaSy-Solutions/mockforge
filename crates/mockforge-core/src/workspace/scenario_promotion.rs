//! Scenario promotion workflow
//!
//! Handles promotion of scenarios between environments (dev → test → prod)
//! with version tracking and promotion history.

use crate::workspace::mock_environment::MockEnvironmentName;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Scenario promotion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioPromotionRequest {
    /// Scenario ID to promote
    pub scenario_id: String,
    /// Scenario version to promote
    pub scenario_version: String,
    /// Workspace ID
    pub workspace_id: String,
    /// Source environment
    pub from_environment: MockEnvironmentName,
    /// Target environment
    pub to_environment: MockEnvironmentName,
    /// Whether this requires approval
    pub requires_approval: bool,
    /// Reason why approval is required
    pub approval_required_reason: Option<String>,
    /// Comments from promoter
    pub comments: Option<String>,
}

/// Scenario promotion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioPromotionResult {
    /// Promotion ID
    pub promotion_id: String,
    /// Whether promotion was successful
    pub success: bool,
    /// Status message
    pub message: String,
    /// Whether approval is required
    pub requires_approval: bool,
    /// Promotion status
    pub status: PromotionStatus,
}

/// Promotion status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromotionStatus {
    /// Promotion is pending approval
    Pending,
    /// Promotion has been approved
    Approved,
    /// Promotion has been rejected
    Rejected,
    /// Promotion has been completed
    Completed,
    /// Promotion failed
    Failed,
}

/// Scenario promotion workflow manager
///
/// Manages the promotion workflow for scenarios between environments.
pub struct ScenarioPromotionWorkflow;

impl ScenarioPromotionWorkflow {
    /// Validate promotion path
    ///
    /// Ensures the promotion follows the correct path: dev → test → prod
    pub fn validate_promotion_path(
        from: MockEnvironmentName,
        to: MockEnvironmentName,
    ) -> Result<(), String> {
        match (from, to) {
            (MockEnvironmentName::Dev, MockEnvironmentName::Test) => Ok(()),
            (MockEnvironmentName::Test, MockEnvironmentName::Prod) => Ok(()),
            _ => Err(format!(
                "Invalid promotion path: {} → {}. Valid paths are: dev → test, test → prod",
                from.as_str(),
                to.as_str()
            )),
        }
    }

    /// Determine if promotion requires approval
    ///
    /// Checks if a scenario promotion requires approval based on:
    /// - High-impact tags (e.g., "auth", "billing", "high-impact")
    /// - Target environment (prod always requires approval)
    /// - Custom approval rules
    /// - Configuration settings
    pub fn requires_approval(
        scenario_tags: &[String],
        target_environment: MockEnvironmentName,
        approval_rules: &ApprovalRules,
    ) -> (bool, Option<String>) {
        // Prod promotions always require approval
        if target_environment == MockEnvironmentName::Prod && approval_rules.prod_requires_approval
        {
            return (true, Some("Production promotions require approval".to_string()));
        }

        // Check for high-impact tags
        for tag in scenario_tags {
            if approval_rules.high_impact_tags.contains(tag) {
                let reason = if target_environment == MockEnvironmentName::Prod {
                    format!("High-impact scenario tag '{}' requires approval for production", tag)
                } else if target_environment == MockEnvironmentName::Test
                    && approval_rules.dev_to_test_requires_approval
                {
                    format!(
                        "High-impact scenario tag '{}' requires approval for test environment",
                        tag
                    )
                } else if target_environment == MockEnvironmentName::Test {
                    format!("High-impact scenario tag '{}' requires approval", tag)
                } else {
                    format!("High-impact scenario tag '{}' requires approval", tag)
                };
                return (true, Some(reason));
            }
        }

        // Check custom rules
        for rule in &approval_rules.custom_rules {
            if rule.matches(scenario_tags, target_environment) {
                return (true, Some(rule.reason.clone()));
            }
        }

        // Dev → test promotions may require approval if configured
        if target_environment == MockEnvironmentName::Test
            && approval_rules.dev_to_test_requires_approval
            && !scenario_tags.is_empty()
        {
            // Check if any tag matches high-impact
            for tag in scenario_tags {
                if approval_rules.high_impact_tags.contains(tag) {
                    return (
                        true,
                        Some(format!(
                            "High-impact scenario tag '{}' requires approval for test environment",
                            tag
                        )),
                    );
                }
            }
        }

        (false, None)
    }

    /// Get next environment in promotion path
    pub fn next_environment(current: MockEnvironmentName) -> Option<MockEnvironmentName> {
        current.next()
    }

    /// Get previous environment in promotion path
    pub fn previous_environment(current: MockEnvironmentName) -> Option<MockEnvironmentName> {
        current.previous()
    }
}

/// Approval rules configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRules {
    /// Tags that require approval (e.g., "auth", "billing", "high-impact")
    pub high_impact_tags: Vec<String>,
    /// Custom approval rules
    pub custom_rules: Vec<CustomApprovalRule>,
    /// Whether test → prod promotions always require approval
    #[serde(default = "default_true")]
    pub prod_requires_approval: bool,
    /// Whether dev → test promotions require approval for high-impact scenarios
    #[serde(default = "default_false")]
    pub dev_to_test_requires_approval: bool,
    /// Minimum number of approvers required for high-impact changes
    #[serde(default = "default_min_approvers")]
    pub min_approvers: usize,
}

fn default_false() -> bool {
    false
}

fn default_min_approvers() -> usize {
    1
}

fn default_true() -> bool {
    true
}

impl Default for ApprovalRules {
    fn default() -> Self {
        Self {
            high_impact_tags: vec![
                "auth".to_string(),
                "billing".to_string(),
                "payment".to_string(),
                "high-impact".to_string(),
                "security".to_string(),
                "pii".to_string(),
            ],
            custom_rules: Vec::new(),
            prod_requires_approval: true,
            dev_to_test_requires_approval: false,
            min_approvers: 1,
        }
    }
}

/// Custom approval rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomApprovalRule {
    /// Rule name
    pub name: String,
    /// Tags that trigger this rule
    pub matching_tags: Vec<String>,
    /// Environments where this rule applies
    pub environments: Vec<MockEnvironmentName>,
    /// Reason for requiring approval
    pub reason: String,
}

impl CustomApprovalRule {
    /// Check if this rule matches the given tags and environment
    pub fn matches(&self, scenario_tags: &[String], environment: MockEnvironmentName) -> bool {
        // Check if environment matches
        if !self.environments.is_empty() && !self.environments.contains(&environment) {
            return false;
        }

        // Check if any tag matches
        for tag in scenario_tags {
            if self.matching_tags.contains(tag) {
                return true;
            }
        }

        false
    }
}

/// Promotion history for a scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionHistory {
    /// Scenario ID
    pub scenario_id: String,
    /// Workspace ID
    pub workspace_id: String,
    /// List of promotions in chronological order
    pub promotions: Vec<PromotionHistoryEntry>,
}

/// Single promotion history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionHistoryEntry {
    /// Promotion ID
    pub promotion_id: String,
    /// Scenario version
    pub scenario_version: String,
    /// From environment
    pub from_environment: MockEnvironmentName,
    /// To environment
    pub to_environment: MockEnvironmentName,
    /// Promoted by user ID
    pub promoted_by: String,
    /// Approved by user ID (if applicable)
    pub approved_by: Option<String>,
    /// Status
    pub status: PromotionStatus,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Comments
    pub comments: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_promotion_path() {
        assert!(ScenarioPromotionWorkflow::validate_promotion_path(
            MockEnvironmentName::Dev,
            MockEnvironmentName::Test
        )
        .is_ok());
        assert!(ScenarioPromotionWorkflow::validate_promotion_path(
            MockEnvironmentName::Test,
            MockEnvironmentName::Prod
        )
        .is_ok());
        assert!(ScenarioPromotionWorkflow::validate_promotion_path(
            MockEnvironmentName::Dev,
            MockEnvironmentName::Prod
        )
        .is_err());
    }

    #[test]
    fn test_requires_approval() {
        let rules = ApprovalRules::default();
        let tags = vec!["auth".to_string()];

        let (requires, reason) =
            ScenarioPromotionWorkflow::requires_approval(&tags, MockEnvironmentName::Test, &rules);
        assert!(requires);
        assert!(reason.is_some());

        let tags = vec!["normal".to_string()];
        let (requires, _) =
            ScenarioPromotionWorkflow::requires_approval(&tags, MockEnvironmentName::Test, &rules);
        assert!(!requires);
    }
}
