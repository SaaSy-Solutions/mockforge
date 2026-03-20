//! Change Management System
//!
//! This module provides a formal change management process for system changes,
//! ensuring all changes are properly planned, approved, tested, and documented.

use crate::Error;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Change type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// Security enhancement
    Security,
    /// Feature addition
    Feature,
    /// Bug fix
    Bugfix,
    /// Infrastructure change
    Infrastructure,
    /// Configuration change
    Configuration,
}

/// Change priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum ChangePriority {
    /// Critical priority - immediate action required
    Critical,
    /// High priority - urgent action required
    High,
    /// Medium priority - action required
    Medium,
    /// Low priority - planned action
    Low,
}

/// Change urgency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeUrgency {
    /// Emergency - critical security fixes, system outages
    Emergency,
    /// High urgency
    High,
    /// Medium urgency
    Medium,
    /// Low urgency
    Low,
}

/// Change status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    /// Change request pending approval
    PendingApproval,
    /// Change approved, ready for implementation
    Approved,
    /// Change rejected
    Rejected,
    /// Change being implemented
    Implementing,
    /// Change completed
    Completed,
    /// Change cancelled
    Cancelled,
    /// Change rolled back
    RolledBack,
}

/// Change request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRequest {
    /// Change request ID (e.g., "CHG-2025-001")
    pub change_id: String,
    /// Change title
    pub title: String,
    /// Change description
    pub description: String,
    /// Requester user ID
    pub requester_id: Uuid,
    /// Request date
    pub request_date: DateTime<Utc>,
    /// Change type
    pub change_type: ChangeType,
    /// Change priority
    pub priority: ChangePriority,
    /// Change urgency
    pub urgency: ChangeUrgency,
    /// Affected systems
    pub affected_systems: Vec<String>,
    /// Impact scope
    pub impact_scope: Option<String>,
    /// Risk level
    pub risk_level: Option<String>,
    /// Rollback plan
    pub rollback_plan: Option<String>,
    /// Testing required
    pub testing_required: bool,
    /// Test plan
    pub test_plan: Option<String>,
    /// Test environment
    pub test_environment: Option<String>,
    /// Change status
    pub status: ChangeStatus,
    /// Approvers required
    pub approvers: Vec<String>,
    /// Approval status (map of approver -> approval status)
    pub approval_status: HashMap<String, ApprovalStatus>,
    /// Implementation plan
    pub implementation_plan: Option<String>,
    /// Scheduled implementation time
    pub scheduled_time: Option<DateTime<Utc>>,
    /// Implementation started time
    pub implementation_started: Option<DateTime<Utc>>,
    /// Implementation completed time
    pub implementation_completed: Option<DateTime<Utc>>,
    /// Test results
    pub test_results: Option<String>,
    /// Post-implementation review
    pub post_implementation_review: Option<String>,
    /// Change history
    pub history: Vec<ChangeHistoryEntry>,
}

/// Approval status for an approver
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    /// Pending approval
    Pending,
    /// Approved
    Approved,
    /// Rejected
    Rejected,
}

/// Change history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeHistoryEntry {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Action performed
    pub action: String,
    /// User who performed the action
    pub user_id: Uuid,
    /// Details
    pub details: String,
}

impl ChangeRequest {
    /// Create a new change request
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        change_id: String,
        title: String,
        description: String,
        requester_id: Uuid,
        change_type: ChangeType,
        priority: ChangePriority,
        urgency: ChangeUrgency,
        affected_systems: Vec<String>,
        testing_required: bool,
        approvers: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        let mut approval_status = HashMap::new();
        for approver in &approvers {
            approval_status.insert(approver.clone(), ApprovalStatus::Pending);
        }

        Self {
            change_id,
            title,
            description,
            requester_id,
            request_date: now,
            change_type,
            priority,
            urgency,
            affected_systems,
            impact_scope: None,
            risk_level: None,
            rollback_plan: None,
            testing_required,
            test_plan: None,
            test_environment: None,
            status: ChangeStatus::PendingApproval,
            approvers,
            approval_status,
            implementation_plan: None,
            scheduled_time: None,
            implementation_started: None,
            implementation_completed: None,
            test_results: None,
            post_implementation_review: None,
            history: vec![ChangeHistoryEntry {
                timestamp: now,
                action: "created".to_string(),
                user_id: requester_id,
                details: "Change request created".to_string(),
            }],
        }
    }

    /// Check if all approvals are complete
    pub fn is_fully_approved(&self) -> bool {
        self.approval_status.values().all(|status| *status == ApprovalStatus::Approved)
    }

    /// Check if any approval was rejected
    pub fn is_rejected(&self) -> bool {
        self.approval_status.values().any(|status| *status == ApprovalStatus::Rejected)
    }

    /// Add history entry
    pub fn add_history(&mut self, action: String, user_id: Uuid, details: String) {
        self.history.push(ChangeHistoryEntry {
            timestamp: Utc::now(),
            action,
            user_id,
            details,
        });
    }
}

/// Change management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ChangeManagementConfig {
    /// Whether change management is enabled
    pub enabled: bool,
    /// Approval workflow configuration
    pub approval_workflow: ApprovalWorkflowConfig,
    /// Testing requirements
    pub testing: TestingConfig,
    /// Notification configuration
    pub notifications: NotificationConfig,
}

/// Approval workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApprovalWorkflowConfig {
    /// Emergency change approvers
    pub emergency: ApprovalLevelConfig,
    /// High priority approvers
    pub high: ApprovalLevelConfig,
    /// Medium priority approvers
    pub medium: ApprovalLevelConfig,
    /// Low priority approvers
    pub low: ApprovalLevelConfig,
}

/// Approval level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApprovalLevelConfig {
    /// Required approvers
    pub approvers: Vec<String>,
    /// Approval timeout (in hours)
    pub approval_timeout_hours: u64,
}

/// Testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TestingConfig {
    /// Change types that require testing
    pub required_for: Vec<ChangeType>,
    /// Test environments
    pub test_environments: Vec<String>,
    /// Required test coverage percentage
    pub test_coverage_required: u8,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NotificationConfig {
    /// Whether notifications are enabled
    pub enabled: bool,
    /// Notification channels
    pub channels: Vec<String>,
    /// Recipients
    pub recipients: Vec<String>,
}

impl Default for ChangeManagementConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            approval_workflow: ApprovalWorkflowConfig {
                emergency: ApprovalLevelConfig {
                    approvers: vec![
                        "security-team-lead".to_string(),
                        "engineering-manager".to_string(),
                    ],
                    approval_timeout_hours: 1,
                },
                high: ApprovalLevelConfig {
                    approvers: vec![
                        "security-team".to_string(),
                        "engineering-manager".to_string(),
                        "change-manager".to_string(),
                    ],
                    approval_timeout_hours: 24,
                },
                medium: ApprovalLevelConfig {
                    approvers: vec![
                        "engineering-manager".to_string(),
                        "change-manager".to_string(),
                    ],
                    approval_timeout_hours: 72,
                },
                low: ApprovalLevelConfig {
                    approvers: vec!["change-manager".to_string()],
                    approval_timeout_hours: 168, // 7 days
                },
            },
            testing: TestingConfig {
                required_for: vec![ChangeType::Security, ChangeType::Infrastructure],
                test_environments: vec!["staging".to_string(), "production-like".to_string()],
                test_coverage_required: 80,
            },
            notifications: NotificationConfig {
                enabled: true,
                channels: vec!["email".to_string(), "slack".to_string()],
                recipients: vec![
                    "change-manager".to_string(),
                    "security-team".to_string(),
                    "engineering-team".to_string(),
                ],
            },
        }
    }
}

/// Change management engine
pub struct ChangeManagementEngine {
    config: ChangeManagementConfig,
    /// Active change requests
    changes: std::sync::Arc<tokio::sync::RwLock<HashMap<String, ChangeRequest>>>,
    /// Change ID counter
    change_id_counter: std::sync::Arc<tokio::sync::RwLock<u64>>,
}

impl ChangeManagementEngine {
    /// Create a new change management engine
    pub fn new(config: ChangeManagementConfig) -> Self {
        Self {
            config,
            changes: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            change_id_counter: std::sync::Arc::new(tokio::sync::RwLock::new(0)),
        }
    }

    /// Generate next change ID
    async fn generate_change_id(&self) -> String {
        let now = Utc::now();
        let year = now.format("%Y").to_string();
        let mut counter = self.change_id_counter.write().await;
        *counter += 1;
        format!("CHG-{}-{:03}", year, *counter)
    }

    /// Get approvers for a priority level
    fn get_approvers_for_priority(&self, priority: ChangePriority) -> Vec<String> {
        match priority {
            ChangePriority::Critical => self.config.approval_workflow.emergency.approvers.clone(),
            ChangePriority::High => self.config.approval_workflow.high.approvers.clone(),
            ChangePriority::Medium => self.config.approval_workflow.medium.approvers.clone(),
            ChangePriority::Low => self.config.approval_workflow.low.approvers.clone(),
        }
    }

    /// Create a new change request
    #[allow(clippy::too_many_arguments)]
    pub async fn create_change_request(
        &self,
        title: String,
        description: String,
        requester_id: Uuid,
        change_type: ChangeType,
        priority: ChangePriority,
        urgency: ChangeUrgency,
        affected_systems: Vec<String>,
        testing_required: bool,
        test_plan: Option<String>,
        test_environment: Option<String>,
        rollback_plan: Option<String>,
        impact_scope: Option<String>,
        risk_level: Option<String>,
    ) -> Result<ChangeRequest, Error> {
        let change_id = self.generate_change_id().await;
        let approvers = self.get_approvers_for_priority(priority);

        let mut change = ChangeRequest::new(
            change_id,
            title,
            description,
            requester_id,
            change_type,
            priority,
            urgency,
            affected_systems,
            testing_required,
            approvers,
        );

        change.test_plan = test_plan;
        change.test_environment = test_environment;
        change.rollback_plan = rollback_plan;
        change.impact_scope = impact_scope;
        change.risk_level = risk_level;

        let change_id = change.change_id.clone();
        let mut changes = self.changes.write().await;
        changes.insert(change_id, change.clone());

        Ok(change)
    }

    /// Approve a change request
    pub async fn approve_change(
        &self,
        change_id: &str,
        approver: &str,
        approver_id: Uuid,
        comments: Option<String>,
        conditions: Option<Vec<String>>,
    ) -> Result<(), Error> {
        let mut changes = self.changes.write().await;
        let change = changes
            .get_mut(change_id)
            .ok_or_else(|| Error::Generic("Change request not found".to_string()))?;

        if change.status != ChangeStatus::PendingApproval {
            return Err(Error::Generic("Change request is not pending approval".to_string()));
        }

        if !change.approvers.contains(&approver.to_string()) {
            return Err(Error::Generic("User is not an approver for this change".to_string()));
        }

        change.approval_status.insert(approver.to_string(), ApprovalStatus::Approved);

        let details = format!(
            "Change approved by {}{}{}",
            approver,
            comments.map(|c| format!(" - {}", c)).unwrap_or_default(),
            conditions
                .map(|conds| format!(" - Conditions: {}", conds.join(", ")))
                .unwrap_or_default()
        );
        change.add_history("approved".to_string(), approver_id, details);

        // Check if all approvals are complete
        if change.is_fully_approved() {
            change.status = ChangeStatus::Approved;
            change.add_history(
                "all_approvals_complete".to_string(),
                approver_id,
                "All approvals received, change ready for implementation".to_string(),
            );
        }

        Ok(())
    }

    /// Reject a change request
    pub async fn reject_change(
        &self,
        change_id: &str,
        approver: &str,
        approver_id: Uuid,
        reason: String,
    ) -> Result<(), Error> {
        let mut changes = self.changes.write().await;
        let change = changes
            .get_mut(change_id)
            .ok_or_else(|| Error::Generic("Change request not found".to_string()))?;

        if change.status != ChangeStatus::PendingApproval {
            return Err(Error::Generic("Change request is not pending approval".to_string()));
        }

        change.approval_status.insert(approver.to_string(), ApprovalStatus::Rejected);
        change.status = ChangeStatus::Rejected;
        change.add_history(
            "rejected".to_string(),
            approver_id,
            format!("Change rejected: {}", reason),
        );

        Ok(())
    }

    /// Start change implementation
    pub async fn start_implementation(
        &self,
        change_id: &str,
        implementer_id: Uuid,
        implementation_plan: String,
        scheduled_time: Option<DateTime<Utc>>,
    ) -> Result<(), Error> {
        let mut changes = self.changes.write().await;
        let change = changes
            .get_mut(change_id)
            .ok_or_else(|| Error::Generic("Change request not found".to_string()))?;

        if change.status != ChangeStatus::Approved {
            return Err(Error::Generic(
                "Change request must be approved before implementation".to_string(),
            ));
        }

        change.status = ChangeStatus::Implementing;
        change.implementation_plan = Some(implementation_plan);
        change.scheduled_time = scheduled_time;
        change.implementation_started = Some(Utc::now());

        change.add_history(
            "implementation_started".to_string(),
            implementer_id,
            "Change implementation started".to_string(),
        );

        Ok(())
    }

    /// Complete change implementation
    pub async fn complete_change(
        &self,
        change_id: &str,
        implementer_id: Uuid,
        test_results: Option<String>,
        post_implementation_review: Option<String>,
    ) -> Result<(), Error> {
        let mut changes = self.changes.write().await;
        let change = changes
            .get_mut(change_id)
            .ok_or_else(|| Error::Generic("Change request not found".to_string()))?;

        if change.status != ChangeStatus::Implementing {
            return Err(Error::Generic(
                "Change request must be in implementing status".to_string(),
            ));
        }

        change.status = ChangeStatus::Completed;
        change.implementation_completed = Some(Utc::now());
        change.test_results = test_results;
        change.post_implementation_review = post_implementation_review;

        change.add_history(
            "completed".to_string(),
            implementer_id,
            "Change implementation completed".to_string(),
        );

        Ok(())
    }

    /// Get change request by ID
    pub async fn get_change(&self, change_id: &str) -> Result<Option<ChangeRequest>, Error> {
        let changes = self.changes.read().await;
        Ok(changes.get(change_id).cloned())
    }

    /// Get all change requests
    pub async fn get_all_changes(&self) -> Result<Vec<ChangeRequest>, Error> {
        let changes = self.changes.read().await;
        Ok(changes.values().cloned().collect())
    }

    /// Get changes by status
    pub async fn get_changes_by_status(
        &self,
        status: ChangeStatus,
    ) -> Result<Vec<ChangeRequest>, Error> {
        let changes = self.changes.read().await;
        Ok(changes.values().filter(|c| c.status == status).cloned().collect())
    }

    /// Get changes by requester
    pub async fn get_changes_by_requester(
        &self,
        requester_id: Uuid,
    ) -> Result<Vec<ChangeRequest>, Error> {
        let changes = self.changes.read().await;
        Ok(changes.values().filter(|c| c.requester_id == requester_id).cloned().collect())
    }

    /// Cancel a change request
    pub async fn cancel_change(
        &self,
        change_id: &str,
        user_id: Uuid,
        reason: String,
    ) -> Result<(), Error> {
        let mut changes = self.changes.write().await;
        let change = changes
            .get_mut(change_id)
            .ok_or_else(|| Error::Generic("Change request not found".to_string()))?;

        change.status = ChangeStatus::Cancelled;
        change.add_history(
            "cancelled".to_string(),
            user_id,
            format!("Change cancelled: {}", reason),
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_change_request_creation() {
        let config = ChangeManagementConfig::default();
        let engine = ChangeManagementEngine::new(config);

        let change = engine
            .create_change_request(
                "Test Change".to_string(),
                "Test description".to_string(),
                Uuid::new_v4(),
                ChangeType::Security,
                ChangePriority::High,
                ChangeUrgency::High,
                vec!["system1".to_string()],
                true,
                Some("Test plan".to_string()),
                Some("staging".to_string()),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(change.status, ChangeStatus::PendingApproval);
        assert!(!change.approvers.is_empty());
    }
}
