//! Automated access review engine for compliance
//!
//! This module provides automated access review functionality for:
//! - Quarterly user access reviews
//! - Monthly privileged access reviews
//! - Monthly API token reviews
//! - Quarterly resource access reviews
//!
//! Compliance: SOC 2 CC6 (Logical Access), ISO 27001 A.9.2 (User Access Management)

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Review frequency types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ReviewFrequency {
    /// Monthly reviews
    Monthly,
    /// Quarterly reviews
    Quarterly,
    /// Annual reviews
    Annually,
}

impl ReviewFrequency {
    /// Get the duration for this frequency
    pub fn duration(&self) -> Duration {
        match self {
            ReviewFrequency::Monthly => Duration::days(30),
            ReviewFrequency::Quarterly => Duration::days(90),
            ReviewFrequency::Annually => Duration::days(365),
        }
    }

    /// Calculate the next review date from a given date
    pub fn next_review_date(&self, from: DateTime<Utc>) -> DateTime<Utc> {
        from + self.duration()
    }
}

/// Review status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    /// Review is pending (not yet started)
    Pending,
    /// Review is in progress
    InProgress,
    /// Review is completed
    Completed,
    /// Review was cancelled
    Cancelled,
}

/// Review type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewType {
    /// User access review
    UserAccess,
    /// Privileged access review
    PrivilegedAccess,
    /// API token review
    ApiToken,
    /// Resource access review
    ResourceAccess,
}

/// User access information for review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccessInfo {
    /// User ID
    pub user_id: Uuid,
    /// Username
    pub username: String,
    /// Email address
    pub email: String,
    /// Current roles
    pub roles: Vec<String>,
    /// Permissions
    pub permissions: Vec<String>,
    /// Last login date
    pub last_login: Option<DateTime<Utc>>,
    /// Access granted date
    pub access_granted: DateTime<Utc>,
    /// Days since last activity
    pub days_inactive: Option<u64>,
    /// Whether user is active
    pub is_active: bool,
}

/// Privileged access information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegedAccessInfo {
    /// User ID
    pub user_id: Uuid,
    /// Username
    pub username: String,
    /// Privileged roles
    pub roles: Vec<String>,
    /// Whether MFA is enabled
    pub mfa_enabled: bool,
    /// Access justification
    pub justification: Option<String>,
    /// Justification expiration date
    pub justification_expires: Option<DateTime<Utc>>,
    /// Recent privileged actions count
    pub recent_actions_count: u64,
    /// Last privileged action date
    pub last_privileged_action: Option<DateTime<Utc>>,
}

/// API token information for review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTokenInfo {
    /// Token ID
    pub token_id: String,
    /// Token name/description
    pub name: Option<String>,
    /// Token owner user ID
    pub owner_id: Uuid,
    /// Token scopes/permissions
    pub scopes: Vec<String>,
    /// Creation date
    pub created_at: DateTime<Utc>,
    /// Last usage date
    pub last_used: Option<DateTime<Utc>>,
    /// Expiration date
    pub expires_at: Option<DateTime<Utc>>,
    /// Days since last use
    pub days_unused: Option<u64>,
    /// Whether token is active
    pub is_active: bool,
}

/// Resource access information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAccessInfo {
    /// Resource type
    pub resource_type: String,
    /// Resource ID
    pub resource_id: String,
    /// Users with access
    pub users_with_access: Vec<Uuid>,
    /// Access levels
    pub access_levels: HashMap<Uuid, String>,
    /// Last access date per user
    pub last_access: HashMap<Uuid, Option<DateTime<Utc>>>,
}

/// Review findings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFindings {
    /// Number of inactive users
    pub inactive_users: u32,
    /// Number of users with excessive permissions
    pub excessive_permissions: u32,
    /// Number of users with no recent access
    pub no_recent_access: u32,
    /// Number of privileged users without MFA
    pub privileged_without_mfa: u32,
    /// Number of unused tokens
    pub unused_tokens: u32,
    /// Number of tokens with excessive scopes
    pub excessive_scopes: u32,
    /// Number of tokens expiring soon
    pub expiring_soon: u32,
    /// Additional custom findings
    pub custom: HashMap<String, u32>,
}

/// Actions taken during review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewActions {
    /// Number of users revoked
    pub users_revoked: u32,
    /// Number of permissions reduced
    pub permissions_reduced: u32,
    /// Number of MFA enforced
    pub mfa_enforced: u32,
    /// Number of tokens revoked
    pub tokens_revoked: u32,
    /// Number of tokens rotated
    pub tokens_rotated: u32,
    /// Number of scopes reduced
    pub scopes_reduced: u32,
    /// Additional custom actions
    pub custom: HashMap<String, u32>,
}

/// Access review record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessReview {
    /// Review ID
    pub review_id: String,
    /// Review type
    pub review_type: ReviewType,
    /// Review status
    pub status: ReviewStatus,
    /// Review date
    pub review_date: DateTime<Utc>,
    /// Due date for completion
    pub due_date: DateTime<Utc>,
    /// Total items reviewed
    pub total_items: u32,
    /// Items reviewed
    pub items_reviewed: u32,
    /// Review findings
    pub findings: ReviewFindings,
    /// Actions taken
    pub actions_taken: ReviewActions,
    /// Pending approvals count
    pub pending_approvals: u32,
    /// Next review date
    pub next_review_date: DateTime<Utc>,
    /// Review metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// User access review item (for approval workflow)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserReviewItem {
    /// Review ID
    pub review_id: String,
    /// User ID
    pub user_id: Uuid,
    /// User access information
    pub access_info: UserAccessInfo,
    /// Review status (pending, approved, rejected)
    pub status: String,
    /// Manager user ID (who should review)
    pub manager_id: Option<Uuid>,
    /// Approval deadline
    pub approval_deadline: Option<DateTime<Utc>>,
    /// Approved by
    pub approved_by: Option<Uuid>,
    /// Approved at
    pub approved_at: Option<DateTime<Utc>>,
    /// Rejection reason
    pub rejection_reason: Option<String>,
}

/// Access review configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AccessReviewConfig {
    /// Whether access review is enabled
    pub enabled: bool,
    /// User access review configuration
    pub user_review: UserReviewConfig,
    /// Privileged access review configuration
    pub privileged_review: PrivilegedReviewConfig,
    /// API token review configuration
    pub token_review: TokenReviewConfig,
    /// Resource access review configuration
    pub resource_review: ResourceReviewConfig,
    /// Notification configuration
    pub notifications: NotificationConfig,
}

/// User access review configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UserReviewConfig {
    /// Whether user review is enabled
    pub enabled: bool,
    /// Review frequency
    pub frequency: ReviewFrequency,
    /// Inactive threshold in days
    pub inactive_threshold_days: u64,
    /// Auto-revoke inactive users
    pub auto_revoke_inactive: bool,
    /// Require manager approval
    pub require_manager_approval: bool,
    /// Approval timeout in days
    pub approval_timeout_days: u64,
}

/// Privileged access review configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrivilegedReviewConfig {
    /// Whether privileged review is enabled
    pub enabled: bool,
    /// Review frequency
    pub frequency: ReviewFrequency,
    /// Require MFA
    pub require_mfa: bool,
    /// Require justification
    pub require_justification: bool,
    /// Alert on privilege escalation
    pub alert_on_escalation: bool,
}

/// API token review configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TokenReviewConfig {
    /// Whether token review is enabled
    pub enabled: bool,
    /// Review frequency
    pub frequency: ReviewFrequency,
    /// Unused threshold in days
    pub unused_threshold_days: u64,
    /// Auto-revoke unused tokens
    pub auto_revoke_unused: bool,
    /// Rotation threshold in days (before expiration)
    pub rotation_threshold_days: u64,
}

/// Resource access review configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ResourceReviewConfig {
    /// Whether resource review is enabled
    pub enabled: bool,
    /// Review frequency
    pub frequency: ReviewFrequency,
    /// List of sensitive resources
    pub sensitive_resources: Vec<String>,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NotificationConfig {
    /// Whether notifications are enabled
    pub enabled: bool,
    /// Notification channels (email, slack, etc.)
    pub channels: Vec<String>,
    /// Recipients (security_team, compliance_team, managers)
    pub recipients: Vec<String>,
}

impl Default for AccessReviewConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            user_review: UserReviewConfig {
                enabled: true,
                frequency: ReviewFrequency::Quarterly,
                inactive_threshold_days: 90,
                auto_revoke_inactive: true,
                require_manager_approval: true,
                approval_timeout_days: 30,
            },
            privileged_review: PrivilegedReviewConfig {
                enabled: true,
                frequency: ReviewFrequency::Monthly,
                require_mfa: true,
                require_justification: true,
                alert_on_escalation: true,
            },
            token_review: TokenReviewConfig {
                enabled: true,
                frequency: ReviewFrequency::Monthly,
                unused_threshold_days: 90,
                auto_revoke_unused: true,
                rotation_threshold_days: 30,
            },
            resource_review: ResourceReviewConfig {
                enabled: true,
                frequency: ReviewFrequency::Quarterly,
                sensitive_resources: vec![
                    "billing".to_string(),
                    "user_data".to_string(),
                    "audit_logs".to_string(),
                    "security_settings".to_string(),
                ],
            },
            notifications: NotificationConfig {
                enabled: true,
                channels: vec!["email".to_string()],
                recipients: vec!["security_team".to_string(), "compliance_team".to_string()],
            },
        }
    }
}

/// Access review engine
///
/// This engine manages automated access reviews according to the configured schedule
/// and policies. It generates reports, tracks approvals, and can automatically
/// revoke access when needed.
pub struct AccessReviewEngine {
    config: AccessReviewConfig,
    /// Active reviews (review_id -> review)
    active_reviews: HashMap<String, AccessReview>,
    /// User review items (review_id -> user_id -> item)
    user_review_items: HashMap<String, HashMap<Uuid, UserReviewItem>>,
}

impl AccessReviewEngine {
    /// Create a new access review engine
    pub fn new(config: AccessReviewConfig) -> Self {
        Self {
            config,
            active_reviews: HashMap::new(),
            user_review_items: HashMap::new(),
        }
    }

    /// Generate a review ID based on type and date
    pub fn generate_review_id(&self, review_type: ReviewType, date: DateTime<Utc>) -> String {
        let type_str = match review_type {
            ReviewType::UserAccess => "user",
            ReviewType::PrivilegedAccess => "privileged",
            ReviewType::ApiToken => "token",
            ReviewType::ResourceAccess => "resource",
        };

        let date_str = date.format("%Y-%m-%d");
        format!("review-{}-{}", date_str, type_str)
    }

    /// Start a user access review
    ///
    /// This generates a review report and creates review items for each user
    /// that needs to be reviewed.
    pub async fn start_user_access_review(
        &mut self,
        users: Vec<UserAccessInfo>,
    ) -> Result<AccessReview, crate::Error> {
        if !self.config.enabled || !self.config.user_review.enabled {
            return Err(crate::Error::Generic("User access review is not enabled".to_string()));
        }

        let now = Utc::now();
        let review_id = self.generate_review_id(ReviewType::UserAccess, now);
        let due_date = now + Duration::days(self.config.user_review.approval_timeout_days as i64);
        let next_review = self.config.user_review.frequency.next_review_date(now);

        // Analyze users and generate findings
        let mut findings = ReviewFindings {
            inactive_users: 0,
            excessive_permissions: 0,
            no_recent_access: 0,
            privileged_without_mfa: 0,
            unused_tokens: 0,
            excessive_scopes: 0,
            expiring_soon: 0,
            custom: HashMap::new(),
        };

        let mut review_items = HashMap::new();

        for user in &users {
            // Check for inactive users
            if let Some(days) = user.days_inactive {
                if days > self.config.user_review.inactive_threshold_days {
                    findings.inactive_users += 1;
                }
            }

            // Check for no recent access
            if user.last_login.is_none() || user.last_login.unwrap() < now - Duration::days(90) {
                findings.no_recent_access += 1;
            }

            // Check for excessive permissions (heuristic: more than 10 permissions)
            if user.permissions.len() > 10 {
                findings.excessive_permissions += 1;
            }

            // Create review item
            let review_item = UserReviewItem {
                review_id: review_id.clone(),
                user_id: user.user_id,
                access_info: user.clone(),
                status: "pending".to_string(),
                manager_id: None, // Would be populated from user's manager relationship
                approval_deadline: Some(due_date),
                approved_by: None,
                approved_at: None,
                rejection_reason: None,
            };

            review_items.insert(user.user_id, review_item);
        }

        let review = AccessReview {
            review_id: review_id.clone(),
            review_type: ReviewType::UserAccess,
            status: ReviewStatus::InProgress,
            review_date: now,
            due_date,
            total_items: users.len() as u32,
            items_reviewed: 0,
            findings: findings.clone(),
            actions_taken: ReviewActions {
                users_revoked: 0,
                permissions_reduced: 0,
                mfa_enforced: 0,
                tokens_revoked: 0,
                tokens_rotated: 0,
                scopes_reduced: 0,
                custom: HashMap::new(),
            },
            pending_approvals: review_items.len() as u32,
            next_review_date: next_review,
            metadata: HashMap::new(),
        };

        self.active_reviews.insert(review_id.clone(), review.clone());
        self.user_review_items.insert(review_id, review_items);

        Ok(review)
    }

    /// Start an API token access review.
    pub async fn start_api_token_review(
        &mut self,
        tokens: Vec<ApiTokenInfo>,
    ) -> Result<AccessReview, crate::Error> {
        if !self.config.enabled || !self.config.token_review.enabled {
            return Err(crate::Error::Generic("API token review is not enabled".to_string()));
        }

        let now = Utc::now();
        let review_id = self.generate_review_id(ReviewType::ApiToken, now);
        let due_date = now + Duration::days(14);
        let next_review = self.config.token_review.frequency.next_review_date(now);

        let mut findings = ReviewFindings {
            inactive_users: 0,
            excessive_permissions: 0,
            no_recent_access: 0,
            privileged_without_mfa: 0,
            unused_tokens: 0,
            excessive_scopes: 0,
            expiring_soon: 0,
            custom: HashMap::new(),
        };

        for token in &tokens {
            if token
                .days_unused
                .is_some_and(|days| days > self.config.token_review.unused_threshold_days)
            {
                findings.unused_tokens += 1;
            }

            if token.scopes.len() > 5 {
                findings.excessive_scopes += 1;
            }

            if token.expires_at.is_some_and(|expires| {
                expires <= now + Duration::days(self.config.token_review.rotation_threshold_days as i64)
            }) {
                findings.expiring_soon += 1;
            }
        }

        let mut metadata = HashMap::new();
        metadata.insert(
            "token_ids".to_string(),
            serde_json::json!(tokens.iter().map(|t| t.token_id.clone()).collect::<Vec<_>>()),
        );

        let review = AccessReview {
            review_id: review_id.clone(),
            review_type: ReviewType::ApiToken,
            status: ReviewStatus::InProgress,
            review_date: now,
            due_date,
            total_items: tokens.len() as u32,
            items_reviewed: 0,
            findings,
            actions_taken: ReviewActions {
                users_revoked: 0,
                permissions_reduced: 0,
                mfa_enforced: 0,
                tokens_revoked: 0,
                tokens_rotated: 0,
                scopes_reduced: 0,
                custom: HashMap::new(),
            },
            pending_approvals: tokens.len() as u32,
            next_review_date: next_review,
            metadata,
        };

        self.active_reviews.insert(review_id, review.clone());
        Ok(review)
    }

    /// Start a resource access review.
    pub async fn start_resource_access_review(
        &mut self,
        resources: Vec<ResourceAccessInfo>,
    ) -> Result<AccessReview, crate::Error> {
        if !self.config.enabled || !self.config.resource_review.enabled {
            return Err(crate::Error::Generic(
                "Resource access review is not enabled".to_string(),
            ));
        }

        let now = Utc::now();
        let review_id = self.generate_review_id(ReviewType::ResourceAccess, now);
        let due_date = now + Duration::days(30);
        let next_review = self.config.resource_review.frequency.next_review_date(now);
        let stale_threshold = now - Duration::days(self.config.user_review.inactive_threshold_days as i64);

        let mut findings = ReviewFindings {
            inactive_users: 0,
            excessive_permissions: 0,
            no_recent_access: 0,
            privileged_without_mfa: 0,
            unused_tokens: 0,
            excessive_scopes: 0,
            expiring_soon: 0,
            custom: HashMap::new(),
        };

        let mut sensitive_resource_count = 0u32;
        for resource in &resources {
            if self
                .config
                .resource_review
                .sensitive_resources
                .iter()
                .any(|r| r == &resource.resource_type)
            {
                sensitive_resource_count += 1;
            }

            let stale_accesses = resource
                .last_access
                .values()
                .filter_map(|d| *d)
                .filter(|d| *d < stale_threshold)
                .count() as u32;
            findings.no_recent_access += stale_accesses;

            if resource.users_with_access.len() > 20 {
                findings.excessive_permissions += 1;
            }
        }

        findings
            .custom
            .insert("sensitive_resources_reviewed".to_string(), sensitive_resource_count);

        let review = AccessReview {
            review_id: review_id.clone(),
            review_type: ReviewType::ResourceAccess,
            status: ReviewStatus::InProgress,
            review_date: now,
            due_date,
            total_items: resources.len() as u32,
            items_reviewed: 0,
            findings,
            actions_taken: ReviewActions {
                users_revoked: 0,
                permissions_reduced: 0,
                mfa_enforced: 0,
                tokens_revoked: 0,
                tokens_rotated: 0,
                scopes_reduced: 0,
                custom: HashMap::new(),
            },
            pending_approvals: resources.len() as u32,
            next_review_date: next_review,
            metadata: HashMap::new(),
        };

        self.active_reviews.insert(review_id, review.clone());
        Ok(review)
    }

    /// Approve a user's access in a review
    pub fn approve_user_access(
        &mut self,
        review_id: &str,
        user_id: Uuid,
        approved_by: Uuid,
        justification: Option<String>,
    ) -> Result<(), crate::Error> {
        let review = self
            .active_reviews
            .get_mut(review_id)
            .ok_or_else(|| crate::Error::Generic(format!("Review {} not found", review_id)))?;

        let items = self.user_review_items.get_mut(review_id).ok_or_else(|| {
            crate::Error::Generic(format!("Review items for {} not found", review_id))
        })?;

        let item = items.get_mut(&user_id).ok_or_else(|| {
            crate::Error::Generic(format!("User {} not found in review", user_id))
        })?;

        item.status = "approved".to_string();
        item.approved_by = Some(approved_by);
        item.approved_at = Some(Utc::now());

        review.items_reviewed += 1;
        review.pending_approvals = review.pending_approvals.saturating_sub(1);

        // Add justification to metadata if provided
        if let Some(just) = justification {
            review
                .metadata
                .insert(format!("justification_{}", user_id), serde_json::json!(just));
        }

        Ok(())
    }

    /// Revoke a user's access in a review
    pub fn revoke_user_access(
        &mut self,
        review_id: &str,
        user_id: Uuid,
        _revoked_by: Uuid,
        reason: String,
    ) -> Result<(), crate::Error> {
        let review = self
            .active_reviews
            .get_mut(review_id)
            .ok_or_else(|| crate::Error::Generic(format!("Review {} not found", review_id)))?;

        let items = self.user_review_items.get_mut(review_id).ok_or_else(|| {
            crate::Error::Generic(format!("Review items for {} not found", review_id))
        })?;

        let item = items.get_mut(&user_id).ok_or_else(|| {
            crate::Error::Generic(format!("User {} not found in review", user_id))
        })?;

        item.status = "revoked".to_string();
        item.rejection_reason = Some(reason.clone());

        review.items_reviewed += 1;
        review.pending_approvals = review.pending_approvals.saturating_sub(1);
        review.actions_taken.users_revoked += 1;

        // Add revocation reason to metadata
        review
            .metadata
            .insert(format!("revocation_reason_{}", user_id), serde_json::json!(reason));

        Ok(())
    }

    /// Update user permissions in a review
    ///
    /// This method updates the user's permissions/roles as part of a review action.
    /// It tracks the permission change in the review and updates the review item.
    pub fn update_user_permissions(
        &mut self,
        review_id: &str,
        user_id: Uuid,
        updated_by: Uuid,
        new_roles: Vec<String>,
        new_permissions: Vec<String>,
        reason: Option<String>,
    ) -> Result<(), crate::Error> {
        let review = self
            .active_reviews
            .get_mut(review_id)
            .ok_or_else(|| crate::Error::Generic(format!("Review {} not found", review_id)))?;

        let items = self.user_review_items.get_mut(review_id).ok_or_else(|| {
            crate::Error::Generic(format!("Review items for {} not found", review_id))
        })?;

        let item = items.get_mut(&user_id).ok_or_else(|| {
            crate::Error::Generic(format!("User {} not found in review", user_id))
        })?;

        // Store old permissions for tracking
        let old_roles = item.access_info.roles.clone();
        let old_permissions = item.access_info.permissions.clone();

        // Update the access info
        item.access_info.roles = new_roles.clone();
        item.access_info.permissions = new_permissions.clone();

        // Mark as reviewed if permissions were reduced
        let roles_reduced = new_roles.len() < old_roles.len();
        let permissions_reduced = new_permissions.len() < old_permissions.len();

        if roles_reduced || permissions_reduced {
            item.status = "permissions_updated".to_string();
            review.items_reviewed += 1;
            review.pending_approvals = review.pending_approvals.saturating_sub(1);
            review.actions_taken.permissions_reduced += 1;
        }

        // Store permission change metadata
        let change_metadata = serde_json::json!({
            "updated_by": updated_by.to_string(),
            "old_roles": old_roles,
            "new_roles": new_roles,
            "old_permissions": old_permissions,
            "new_permissions": new_permissions,
            "reason": reason,
            "updated_at": Utc::now(),
        });
        review
            .metadata
            .insert(format!("permission_update_{}", user_id), change_metadata);

        Ok(())
    }

    /// Get review items for a review
    pub fn get_review_items(&self, review_id: &str) -> Option<&HashMap<Uuid, UserReviewItem>> {
        self.user_review_items.get(review_id)
    }

    /// Get a review by ID
    pub fn get_review(&self, review_id: &str) -> Option<&AccessReview> {
        self.active_reviews.get(review_id)
    }

    /// Get all active reviews
    pub fn get_all_reviews(&self) -> Vec<&AccessReview> {
        self.active_reviews.values().collect()
    }

    /// Check for reviews that need auto-revocation
    ///
    /// This checks all pending review items and automatically revokes access
    /// for items that have exceeded their approval deadline.
    pub fn check_auto_revocation(&mut self) -> Vec<(String, Uuid)> {
        let now = Utc::now();
        let mut revoked = Vec::new();

        for (review_id, items) in &mut self.user_review_items {
            let review = match self.active_reviews.get_mut(review_id) {
                Some(r) => r,
                None => continue,
            };

            if !self.config.user_review.auto_revoke_inactive {
                continue;
            }

            for (user_id, item) in items.iter_mut() {
                if item.status == "pending" {
                    if let Some(deadline) = item.approval_deadline {
                        if now > deadline {
                            // Auto-revoke
                            item.status = "auto_revoked".to_string();
                            item.rejection_reason = Some(
                                "Access automatically revoked due to missing approval within deadline".to_string(),
                            );

                            review.items_reviewed += 1;
                            review.pending_approvals = review.pending_approvals.saturating_sub(1);
                            review.actions_taken.users_revoked += 1;

                            revoked.push((review_id.clone(), *user_id));
                        }
                    }
                }
            }
        }

        revoked
    }

    /// Complete a review
    pub fn complete_review(&mut self, review_id: &str) -> Result<(), crate::Error> {
        let review = self
            .active_reviews
            .get_mut(review_id)
            .ok_or_else(|| crate::Error::Generic(format!("Review {} not found", review_id)))?;

        review.status = ReviewStatus::Completed;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_frequency_duration() {
        assert_eq!(ReviewFrequency::Monthly.duration(), Duration::days(30));
        assert_eq!(ReviewFrequency::Quarterly.duration(), Duration::days(90));
        assert_eq!(ReviewFrequency::Annually.duration(), Duration::days(365));
    }

    #[test]
    fn test_generate_review_id() {
        let config = AccessReviewConfig::default();
        let engine = AccessReviewEngine::new(config);
        let date = Utc::now();
        let id = engine.generate_review_id(ReviewType::UserAccess, date);
        assert!(id.starts_with("review-"));
        assert!(id.contains("user"));
    }

    #[tokio::test]
    async fn test_start_user_access_review() {
        let mut config = AccessReviewConfig::default();
        config.enabled = true;
        config.user_review.enabled = true;

        let mut engine = AccessReviewEngine::new(config);

        let users = vec![
            UserAccessInfo {
                user_id: Uuid::new_v4(),
                username: "user1".to_string(),
                email: "user1@example.com".to_string(),
                roles: vec!["editor".to_string()],
                permissions: vec!["read".to_string(), "write".to_string()],
                last_login: Some(Utc::now() - Duration::days(10)),
                access_granted: Utc::now() - Duration::days(100),
                days_inactive: Some(10),
                is_active: true,
            },
            UserAccessInfo {
                user_id: Uuid::new_v4(),
                username: "user2".to_string(),
                email: "user2@example.com".to_string(),
                roles: vec!["admin".to_string()],
                permissions: (0..15).map(|i| format!("perm{}", i)).collect(),
                last_login: Some(Utc::now() - Duration::days(120)),
                access_granted: Utc::now() - Duration::days(200),
                days_inactive: Some(120),
                is_active: true,
            },
        ];

        let review = engine.start_user_access_review(users).await.unwrap();
        assert_eq!(review.review_type, ReviewType::UserAccess);
        assert_eq!(review.total_items, 2);
        assert!(review.findings.inactive_users > 0);
        assert!(review.findings.excessive_permissions > 0);
    }

    #[test]
    fn test_approve_user_access() {
        let mut config = AccessReviewConfig::default();
        config.enabled = true;
        config.user_review.enabled = true;

        let mut engine = AccessReviewEngine::new(config);

        let user = UserAccessInfo {
            user_id: Uuid::new_v4(),
            username: "user1".to_string(),
            email: "user1@example.com".to_string(),
            roles: vec!["editor".to_string()],
            permissions: vec!["read".to_string()],
            last_login: Some(Utc::now()),
            access_granted: Utc::now() - Duration::days(10),
            days_inactive: Some(0),
            is_active: true,
        };

        // Start review
        let review =
            futures::executor::block_on(engine.start_user_access_review(vec![user.clone()]))
                .unwrap();
        let review_id = review.review_id.clone();

        // Approve access
        let approver_id = Uuid::new_v4();
        engine.approve_user_access(&review_id, user.user_id, approver_id, None).unwrap();

        let review = engine.get_review(&review_id).unwrap();
        assert_eq!(review.items_reviewed, 1);
        assert_eq!(review.pending_approvals, 0);
    }

    #[test]
    fn test_revoke_user_access() {
        let mut config = AccessReviewConfig::default();
        config.enabled = true;
        config.user_review.enabled = true;

        let mut engine = AccessReviewEngine::new(config);

        let user = UserAccessInfo {
            user_id: Uuid::new_v4(),
            username: "user1".to_string(),
            email: "user1@example.com".to_string(),
            roles: vec!["editor".to_string()],
            permissions: vec!["read".to_string()],
            last_login: Some(Utc::now()),
            access_granted: Utc::now() - Duration::days(10),
            days_inactive: Some(0),
            is_active: true,
        };

        // Start review
        let review =
            futures::executor::block_on(engine.start_user_access_review(vec![user.clone()]))
                .unwrap();
        let review_id = review.review_id.clone();

        // Revoke access
        let revoker_id = Uuid::new_v4();
        engine
            .revoke_user_access(
                &review_id,
                user.user_id,
                revoker_id,
                "No longer needed".to_string(),
            )
            .unwrap();

        let review = engine.get_review(&review_id).unwrap();
        assert_eq!(review.actions_taken.users_revoked, 1);
    }

    #[tokio::test]
    async fn test_start_resource_access_review() {
        let mut config = AccessReviewConfig::default();
        config.enabled = true;
        config.resource_review.enabled = true;

        let mut engine = AccessReviewEngine::new(config);
        let user_id = Uuid::new_v4();
        let mut access_levels = HashMap::new();
        access_levels.insert(user_id, "admin".to_string());
        let mut last_access = HashMap::new();
        last_access.insert(user_id, Some(Utc::now() - Duration::days(120)));

        let resources = vec![ResourceAccessInfo {
            resource_type: "billing".to_string(),
            resource_id: "res-1".to_string(),
            users_with_access: vec![user_id],
            access_levels,
            last_access,
        }];

        let review = engine.start_resource_access_review(resources).await.unwrap();
        assert_eq!(review.review_type, ReviewType::ResourceAccess);
        assert_eq!(review.total_items, 1);
        assert_eq!(
            review.findings.custom.get("sensitive_resources_reviewed"),
            Some(&1)
        );
        assert!(review.findings.no_recent_access >= 1);
    }
}
