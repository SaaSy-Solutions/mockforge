//! Scenario promotion workflow
//!
//! Handles promotion of scenarios between environments (dev → test → prod)
//! with version tracking and promotion history.

use crate::pillars::{parse_pillar_tags_from_scenario_tags, Pillar};
use crate::workspace::mock_environment::MockEnvironmentName;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Entity type that can be promoted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromotionEntityType {
    /// Scenario promotion
    Scenario,
    /// Persona promotion
    Persona,
    /// Configuration promotion (reality, chaos, drift budget)
    Config,
}

impl std::fmt::Display for PromotionEntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromotionEntityType::Scenario => write!(f, "scenario"),
            PromotionEntityType::Persona => write!(f, "persona"),
            PromotionEntityType::Config => write!(f, "config"),
        }
    }
}

/// Generic promotion request that supports scenarios, personas, and configs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionRequest {
    /// Entity type being promoted
    pub entity_type: PromotionEntityType,
    /// Entity ID to promote (scenario ID, persona ID, or "config" for config promotion)
    pub entity_id: String,
    /// Entity version (for scenarios/personas) or config snapshot ID (for configs)
    pub entity_version: Option<String>,
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
    /// Additional metadata for the promotion (e.g., config changes diff)
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Scenario promotion request (backward compatibility)
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

impl From<ScenarioPromotionRequest> for PromotionRequest {
    fn from(req: ScenarioPromotionRequest) -> Self {
        Self {
            entity_type: PromotionEntityType::Scenario,
            entity_id: req.scenario_id,
            entity_version: Some(req.scenario_version),
            workspace_id: req.workspace_id,
            from_environment: req.from_environment,
            to_environment: req.to_environment,
            requires_approval: req.requires_approval,
            approval_required_reason: req.approval_required_reason,
            comments: req.comments,
            metadata: HashMap::new(),
        }
    }
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

impl std::fmt::Display for PromotionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromotionStatus::Pending => write!(f, "pending"),
            PromotionStatus::Approved => write!(f, "approved"),
            PromotionStatus::Rejected => write!(f, "rejected"),
            PromotionStatus::Completed => write!(f, "completed"),
            PromotionStatus::Failed => write!(f, "failed"),
        }
    }
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
    /// - Pillar tags (e.g., "[Cloud][Contracts][Reality]")
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

        // Parse pillar tags from scenario tags
        let pillar_tags = parse_pillar_tags_from_scenario_tags(scenario_tags);

        // Check for high-impact pillar tag combinations
        if !pillar_tags.is_empty() {
            // Check if any pillar tag combination matches high-impact patterns
            for pattern in &approval_rules.high_impact_pillar_patterns {
                if Self::matches_pillar_pattern(&pillar_tags, pattern) {
                    let pillar_names: Vec<String> =
                        pillar_tags.iter().map(|p| p.display_name()).collect();
                    let reason = if target_environment == MockEnvironmentName::Prod {
                        format!(
                            "High-impact pillar tag combination {} requires approval for production",
                            pillar_names.join("")
                        )
                    } else if target_environment == MockEnvironmentName::Test
                        && approval_rules.dev_to_test_requires_approval
                    {
                        format!(
                            "High-impact pillar tag combination {} requires approval for test environment",
                            pillar_names.join("")
                        )
                    } else {
                        format!(
                            "High-impact pillar tag combination {} requires approval",
                            pillar_names.join("")
                        )
                    };
                    return (true, Some(reason));
                }
            }

            // Check if specific pillar tags require approval
            for required_pillar in &approval_rules.require_approval_pillars {
                if pillar_tags.contains(required_pillar) {
                    let reason = if target_environment == MockEnvironmentName::Prod {
                        format!(
                            "Pillar tag {} requires approval for production",
                            required_pillar.display_name()
                        )
                    } else if target_environment == MockEnvironmentName::Test
                        && approval_rules.dev_to_test_requires_approval
                    {
                        format!(
                            "Pillar tag {} requires approval for test environment",
                            required_pillar.display_name()
                        )
                    } else {
                        format!("Pillar tag {} requires approval", required_pillar.display_name())
                    };
                    return (true, Some(reason));
                }
            }
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

    /// Check if pillar tags match a pattern
    ///
    /// A pattern is a set of required pillars. The tags match if they contain
    /// all pillars in the pattern (and possibly more).
    fn matches_pillar_pattern(tags: &[Pillar], pattern: &[Pillar]) -> bool {
        // All pillars in the pattern must be present in tags
        pattern.iter().all(|required_pillar| tags.contains(required_pillar))
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
    /// Pillar tags that require approval (e.g., [Cloud], [Contracts])
    /// If a scenario has any of these pillar tags, approval is required
    #[serde(default)]
    pub require_approval_pillars: Vec<Pillar>,
    /// Pillar tag combinations that require approval
    /// Each pattern is a set of pillars that, when all present together, trigger approval
    /// Example: [[Cloud, Contracts, Reality]] means all three must be present
    #[serde(default)]
    pub high_impact_pillar_patterns: Vec<Vec<Pillar>>,
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
            // Pillar tags that always require approval
            require_approval_pillars: vec![
                // Cloud + Contracts + Reality combination is high-impact
                // (handled by high_impact_pillar_patterns below)
            ],
            // High-impact pillar tag combinations
            // Scenarios tagged with [Cloud][Contracts][Reality] require approval
            high_impact_pillar_patterns: vec![vec![
                Pillar::Cloud,
                Pillar::Contracts,
                Pillar::Reality,
            ]],
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

/// Promotion history for an entity (scenario, persona, or config)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionHistory {
    /// Entity type
    pub entity_type: PromotionEntityType,
    /// Entity ID
    pub entity_id: String,
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
    /// Entity type
    pub entity_type: PromotionEntityType,
    /// Entity ID
    pub entity_id: String,
    /// Entity version (for scenarios/personas) or config snapshot ID (for configs)
    pub entity_version: Option<String>,
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
    /// GitOps PR URL if created
    pub pr_url: Option<String>,
    /// Additional metadata (e.g., config changes diff)
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
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

    #[test]
    fn test_requires_approval_with_pillar_tags() {
        let rules = ApprovalRules::default();

        // Test with [Cloud][Contracts][Reality] combination (high-impact pattern)
        let tags = vec!["[Cloud][Contracts][Reality]".to_string()];
        let (requires, reason) =
            ScenarioPromotionWorkflow::requires_approval(&tags, MockEnvironmentName::Test, &rules);
        assert!(requires, "Should require approval for Cloud+Contracts+Reality combination");
        assert!(reason.is_some());
        assert!(reason.unwrap().contains("pillar tag combination"));

        // Test with individual pillar tags (should not require approval by default)
        let tags2 = vec!["[Cloud]".to_string()];
        let (requires2, _) =
            ScenarioPromotionWorkflow::requires_approval(&tags2, MockEnvironmentName::Test, &rules);
        assert!(!requires2, "Single pillar tag should not require approval by default");

        // Test with partial combination (should not match pattern)
        let tags3 = vec!["[Cloud][Contracts]".to_string()];
        let (requires3, _) =
            ScenarioPromotionWorkflow::requires_approval(&tags3, MockEnvironmentName::Test, &rules);
        assert!(!requires3, "Partial pillar combination should not require approval");
    }

    #[test]
    fn test_matches_pillar_pattern() {
        use crate::pillars::Pillar;

        let tags = vec![Pillar::Cloud, Pillar::Contracts, Pillar::Reality];
        let pattern = vec![Pillar::Cloud, Pillar::Contracts, Pillar::Reality];
        assert!(ScenarioPromotionWorkflow::matches_pillar_pattern(&tags, &pattern));

        let tags2 = vec![
            Pillar::Cloud,
            Pillar::Contracts,
            Pillar::Reality,
            Pillar::Ai,
        ];
        let pattern2 = vec![Pillar::Cloud, Pillar::Contracts];
        assert!(ScenarioPromotionWorkflow::matches_pillar_pattern(&tags2, &pattern2));

        let tags3 = vec![Pillar::Cloud, Pillar::Contracts];
        let pattern3 = vec![Pillar::Cloud, Pillar::Contracts, Pillar::Reality];
        assert!(!ScenarioPromotionWorkflow::matches_pillar_pattern(&tags3, &pattern3));
    }
}
