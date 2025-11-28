//! Access review service for integrating with user management systems
//!
//! This service provides a bridge between the access review engine and actual user data
//! from the database, allowing the review system to work with real user information.

use crate::security::access_review::{
    AccessReviewEngine, ApiTokenInfo, PrivilegedAccessInfo, ReviewFrequency, UserAccessInfo,
};
use crate::Error;
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

/// Trait for user data providers
///
/// This trait allows the access review system to work with different user management
/// backends (database, LDAP, etc.)
#[async_trait::async_trait]
pub trait UserDataProvider: Send + Sync {
    /// Get all active users with their access information
    async fn get_all_users(&self) -> Result<Vec<UserAccessInfo>, Error>;

    /// Get privileged users (users with admin roles)
    async fn get_privileged_users(&self) -> Result<Vec<PrivilegedAccessInfo>, Error>;

    /// Get all API tokens for review
    async fn get_api_tokens(&self) -> Result<Vec<ApiTokenInfo>, Error>;

    /// Get user by ID
    async fn get_user(&self, user_id: Uuid) -> Result<Option<UserAccessInfo>, Error>;

    /// Get last login date for a user
    ///
    /// Returns None if the user has never logged in or if login tracking is not available
    async fn get_last_login(&self, user_id: Uuid) -> Result<Option<DateTime<Utc>>, Error>;

    /// Revoke user access (deactivate user)
    async fn revoke_user_access(&self, user_id: Uuid, reason: String) -> Result<(), Error>;

    /// Update user permissions/roles
    async fn update_user_permissions(
        &self,
        user_id: Uuid,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> Result<(), Error>;
}

/// Access review service
///
/// This service manages access reviews by coordinating between the review engine
/// and the user data provider.
pub struct AccessReviewService {
    engine: AccessReviewEngine,
    user_provider: Box<dyn UserDataProvider>,
}

impl AccessReviewService {
    /// Create a new access review service
    pub fn new(engine: AccessReviewEngine, user_provider: Box<dyn UserDataProvider>) -> Self {
        Self {
            engine,
            user_provider,
        }
    }

    /// Start a user access review
    ///
    /// Fetches all users from the provider and starts a review
    pub async fn start_user_access_review(&mut self) -> Result<String, Error> {
        let users = self.user_provider.get_all_users().await?;

        let review = self.engine.start_user_access_review(users).await?;

        Ok(review.review_id)
    }

    /// Start a privileged access review
    ///
    /// Fetches privileged users and starts a review
    pub async fn start_privileged_access_review(&mut self) -> Result<String, Error> {
        let privileged_users = self.user_provider.get_privileged_users().await?;

        // For now, we'll convert privileged users to user access info
        // In the future, we can add a dedicated privileged review method
        let users: Vec<UserAccessInfo> = privileged_users
            .into_iter()
            .map(|p| UserAccessInfo {
                user_id: p.user_id,
                username: p.username,
                email: "".to_string(), // Would need to fetch from user provider
                roles: p.roles,
                permissions: vec![], // Would need to fetch from permissions system
                last_login: p.last_privileged_action,
                access_granted: Utc::now() - Duration::days(90), // Placeholder
                days_inactive: p.last_privileged_action.map(|d| (Utc::now() - d).num_days() as u64),
                is_active: true,
            })
            .collect();

        let review = self.engine.start_user_access_review(users).await?;

        Ok(review.review_id)
    }

    /// Start an API token review
    ///
    /// Fetches all API tokens and starts a review
    pub async fn start_token_review(&mut self) -> Result<String, Error> {
        let tokens = self.user_provider.get_api_tokens().await?;

        // For now, token reviews are not fully implemented in the engine
        // This is a placeholder for future implementation
        Err(Error::Generic("Token review not yet implemented in review engine".to_string()))
    }

    /// Approve user access in a review
    pub async fn approve_user_access(
        &mut self,
        review_id: &str,
        user_id: Uuid,
        approved_by: Uuid,
        justification: Option<String>,
    ) -> Result<(), Error> {
        self.engine
            .approve_user_access(review_id, user_id, approved_by, justification)
            .map_err(|e| Error::Generic(e.to_string()))
    }

    /// Revoke user access in a review
    ///
    /// This both updates the review and actually revokes the user's access
    pub async fn revoke_user_access(
        &mut self,
        review_id: &str,
        user_id: Uuid,
        revoked_by: Uuid,
        reason: String,
    ) -> Result<(), Error> {
        // Update the review
        self.engine
            .revoke_user_access(review_id, user_id, revoked_by, reason.clone())
            .map_err(|e| Error::Generic(e.to_string()))?;

        // Actually revoke the user's access
        self.user_provider.revoke_user_access(user_id, reason).await?;

        Ok(())
    }

    /// Update user permissions in a review
    ///
    /// This both updates the review and actually updates the user's permissions
    /// in the user management system.
    pub async fn update_user_permissions(
        &mut self,
        review_id: &str,
        user_id: Uuid,
        updated_by: Uuid,
        new_roles: Vec<String>,
        new_permissions: Vec<String>,
        reason: Option<String>,
    ) -> Result<(), Error> {
        // Update the review to track permission changes
        self.engine
            .update_user_permissions(
                review_id,
                user_id,
                updated_by,
                new_roles.clone(),
                new_permissions.clone(),
                reason.clone(),
            )
            .map_err(|e| Error::Generic(e.to_string()))?;

        // Actually update the user's permissions in the user management system
        self.user_provider
            .update_user_permissions(user_id, new_roles, new_permissions)
            .await?;

        Ok(())
    }

    /// Get review by ID
    pub fn get_review(
        &self,
        review_id: &str,
    ) -> Option<&crate::security::access_review::AccessReview> {
        self.engine.get_review(review_id)
    }

    /// Get all reviews
    pub fn get_all_reviews(&self) -> Vec<&crate::security::access_review::AccessReview> {
        self.engine.get_all_reviews()
    }

    /// Check for auto-revocations
    ///
    /// This should be called periodically to automatically revoke access
    /// for users whose review approvals have expired.
    pub async fn check_auto_revocations(&mut self) -> Result<Vec<(String, Uuid)>, Error> {
        let revoked = self.engine.check_auto_revocation();

        // Actually revoke access for auto-revoked users
        for (review_id, user_id) in &revoked {
            if let Some(review_item) =
                self.engine.get_review_items(review_id).and_then(|items| items.get(user_id))
            {
                let reason = review_item
                    .rejection_reason
                    .clone()
                    .unwrap_or_else(|| "Auto-revoked due to missing approval".to_string());

                if let Err(e) = self.user_provider.revoke_user_access(*user_id, reason).await {
                    tracing::error!("Failed to revoke access for user {}: {}", user_id, e);
                }
            }
        }

        Ok(revoked)
    }

    /// Get the review engine (for direct access if needed)
    pub fn engine(&self) -> &AccessReviewEngine {
        &self.engine
    }

    /// Get mutable access to the review engine
    pub fn engine_mut(&mut self) -> &mut AccessReviewEngine {
        &mut self.engine
    }
}

/// Check if a review is due based on frequency and last review date
pub fn is_review_due(frequency: ReviewFrequency, last_review_date: Option<DateTime<Utc>>) -> bool {
    let now = Utc::now();

    if let Some(last_review) = last_review_date {
        let next_review = frequency.next_review_date(last_review);
        now >= next_review
    } else {
        // No previous review, consider it due
        true
    }
}

/// Calculate days since last activity
pub fn days_since_last_activity(last_activity: Option<DateTime<Utc>>) -> Option<u64> {
    last_activity.map(|activity| {
        let duration = Utc::now() - activity;
        duration.num_days() as u64
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_review_due() {
        let frequency = ReviewFrequency::Quarterly;
        let last_review = Utc::now() - Duration::days(100);

        assert!(is_review_due(frequency, Some(last_review)));

        let recent_review = Utc::now() - Duration::days(10);
        assert!(!is_review_due(frequency, Some(recent_review)));

        // No previous review
        assert!(is_review_due(frequency, None));
    }

    #[test]
    fn test_days_since_last_activity() {
        let recent = Utc::now() - Duration::days(5);
        assert_eq!(days_since_last_activity(Some(recent)), Some(5));

        let old = Utc::now() - Duration::days(100);
        assert_eq!(days_since_last_activity(Some(old)), Some(100));

        assert_eq!(days_since_last_activity(None), None);
    }
}
