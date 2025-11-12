//! Access review data provider for collaboration system
//!
//! This module provides a UserDataProvider implementation that integrates
//! the access review system with the collaboration database.

use mockforge_core::security::access_review::{
    ApiTokenInfo, PrivilegedAccessInfo, UserAccessInfo,
};
use mockforge_core::security::{
    ApiTokenStorage, JustificationStorage, MfaStorage, UserDataProvider,
};
use mockforge_core::Error;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// User data provider for collaboration system
pub struct CollabUserDataProvider {
    db: Pool<Sqlite>,
    user_service: Arc<crate::user::UserService>,
    workspace_service: Arc<crate::workspace::WorkspaceService>,
    token_storage: Option<Arc<dyn ApiTokenStorage>>,
    mfa_storage: Option<Arc<dyn MfaStorage>>,
    justification_storage: Option<Arc<dyn JustificationStorage>>,
}

impl CollabUserDataProvider {
    /// Create a new user data provider
    pub fn new(
        db: Pool<Sqlite>,
        user_service: Arc<crate::user::UserService>,
        workspace_service: Arc<crate::workspace::WorkspaceService>,
    ) -> Self {
        Self {
            db,
            user_service,
            workspace_service,
            token_storage: None,
            mfa_storage: None,
            justification_storage: None,
        }
    }

    /// Create with optional storage backends
    pub fn with_storage(
        db: Pool<Sqlite>,
        user_service: Arc<crate::user::UserService>,
        workspace_service: Arc<crate::workspace::WorkspaceService>,
        token_storage: Option<Arc<dyn ApiTokenStorage>>,
        mfa_storage: Option<Arc<dyn MfaStorage>>,
        justification_storage: Option<Arc<dyn JustificationStorage>>,
    ) -> Self {
        Self {
            db,
            user_service,
            workspace_service,
            token_storage,
            mfa_storage,
            justification_storage,
        }
    }
}

#[async_trait::async_trait]
impl UserDataProvider for CollabUserDataProvider {
    async fn get_all_users(&self) -> Result<Vec<UserAccessInfo>, Error> {
        // Fetch all active users
        let users = sqlx::query_as!(
            crate::models::User,
            r#"
            SELECT id as "id: Uuid", username, email, password_hash, display_name, avatar_url,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
                   is_active as "is_active: bool"
            FROM users
            WHERE is_active = TRUE
            ORDER BY created_at
            "#,
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| Error::Generic(format!("Failed to fetch users: {}", e)))?;

        // For each user, get their workspace memberships and roles
        let mut user_access_list = Vec::new();

        for user in users {
            // Get all workspace memberships for this user
            let memberships = sqlx::query_as!(
                crate::models::WorkspaceMember,
                r#"
                SELECT
                    id as "id: Uuid",
                    workspace_id as "workspace_id: Uuid",
                    user_id as "user_id: Uuid",
                    role as "role: crate::models::UserRole",
                    joined_at as "joined_at: chrono::DateTime<chrono::Utc>",
                    last_activity as "last_activity: chrono::DateTime<chrono::Utc>"
                FROM workspace_members
                WHERE user_id = ?
                ORDER BY last_activity DESC
                "#,
                user.id
            )
            .fetch_all(&self.db)
            .await
            .map_err(|e| Error::Generic(format!("Failed to fetch memberships: {}", e)))?;

            // Collect roles and permissions
            let roles: Vec<String> = memberships
                .iter()
                .map(|m| format!("{:?}", m.role))
                .collect();

            // Map roles to permissions (simplified - in reality would use PermissionChecker)
            let permissions: Vec<String> = memberships
                .iter()
                .flat_map(|m| {
                    match m.role {
                        crate::models::UserRole::Admin => vec![
                            "WorkspaceCreate".to_string(),
                            "WorkspaceRead".to_string(),
                            "WorkspaceUpdate".to_string(),
                            "WorkspaceDelete".to_string(),
                            "WorkspaceManageMembers".to_string(),
                            "MockCreate".to_string(),
                            "MockRead".to_string(),
                            "MockUpdate".to_string(),
                            "MockDelete".to_string(),
                        ],
                        crate::models::UserRole::Editor => vec![
                            "MockCreate".to_string(),
                            "MockRead".to_string(),
                            "MockUpdate".to_string(),
                            "MockDelete".to_string(),
                        ],
                        crate::models::UserRole::Viewer => vec!["MockRead".to_string()],
                    }
                })
                .collect();

            // Get most recent activity
            let last_activity = memberships
                .iter()
                .map(|m| m.last_activity)
                .max();

            // Calculate days inactive
            let days_inactive = last_activity.map(|activity| {
                let duration = Utc::now() - activity;
                duration.num_days() as u64
            });

            // Access granted date is the earliest membership join date
            let access_granted = memberships
                .iter()
                .map(|m| m.joined_at)
                .min()
                .unwrap_or(user.created_at);

            user_access_list.push(UserAccessInfo {
                user_id: user.id,
                username: user.username,
                email: user.email,
                roles,
                permissions,
                last_login: last_activity, // Using last_activity as proxy for last login
                access_granted,
                days_inactive,
                is_active: user.is_active,
            });
        }

        Ok(user_access_list)
    }

    async fn get_privileged_users(&self) -> Result<Vec<PrivilegedAccessInfo>, Error> {
        // Get all users with admin role in any workspace
        let admin_members = sqlx::query_as!(
            crate::models::WorkspaceMember,
            r#"
            SELECT
                id as "id: Uuid",
                workspace_id as "workspace_id: Uuid",
                user_id as "user_id: Uuid",
                role as "role: crate::models::UserRole",
                joined_at as "joined_at: chrono::DateTime<chrono::Utc>",
                last_activity as "last_activity: chrono::DateTime<chrono::Utc>"
            FROM workspace_members
            WHERE role = 'admin'
            ORDER BY last_activity DESC
            "#,
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| Error::Generic(format!("Failed to fetch privileged users: {}", e)))?;

        // Group by user_id and collect roles
        use std::collections::HashMap;
        let mut user_roles: HashMap<Uuid, Vec<String>> = HashMap::new();
        let mut user_activities: HashMap<Uuid, Vec<DateTime<Utc>>> = HashMap::new();

        for member in &admin_members {
            user_roles
                .entry(member.user_id)
                .or_insert_with(Vec::new)
                .push(format!("{:?}", member.role));
            user_activities
                .entry(member.user_id)
                .or_insert_with(Vec::new)
                .push(member.last_activity);
        }

        // Get user details
        let mut privileged_list = Vec::new();

        for (user_id, roles) in user_roles {
            // Get user details
            let user = self.user_service.get_user(user_id).await
                .map_err(|e| Error::Generic(format!("Failed to get user {}: {}", user_id, e)))?;

            let activities = user_activities.get(&user_id).cloned().unwrap_or_default();
            let last_activity = activities.iter().max().copied();

            // Check MFA status
            let mfa_enabled = if let Some(ref mfa_storage) = self.mfa_storage {
                mfa_storage
                    .get_mfa_status(user_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|s| s.enabled)
                    .unwrap_or(false)
            } else {
                false
            };

            // Get justification
            let (justification, justification_expires) = if let Some(ref just_storage) = self.justification_storage {
                just_storage
                    .get_justification(user_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|j| (Some(j.justification), j.expires_at))
                    .unwrap_or((None, None))
            } else {
                (None, None)
            };

            privileged_list.push(PrivilegedAccessInfo {
                user_id,
                username: user.username,
                roles,
                mfa_enabled,
                justification,
                justification_expires,
                recent_actions_count: activities.len() as u64,
                last_privileged_action: last_activity,
            });
        }

        Ok(privileged_list)
    }

    async fn get_api_tokens(&self) -> Result<Vec<ApiTokenInfo>, Error> {
        if let Some(ref storage) = self.token_storage {
            storage.get_all_tokens().await
        } else {
            // No token storage configured, return empty list
            Ok(Vec::new())
        }
    }

    async fn get_user(&self, user_id: Uuid) -> Result<Option<UserAccessInfo>, Error> {
        let user = match self.user_service.get_user(user_id).await {
            Ok(u) => u,
            Err(_) => return Ok(None),
        };

        // Get memberships
        let memberships = sqlx::query_as!(
            crate::models::WorkspaceMember,
            r#"
            SELECT
                id as "id: Uuid",
                workspace_id as "workspace_id: Uuid",
                user_id as "user_id: Uuid",
                role as "role: crate::models::UserRole",
                joined_at as "joined_at: chrono::DateTime<chrono::Utc>",
                last_activity as "last_activity: chrono::DateTime<chrono::Utc>"
            FROM workspace_members
            WHERE user_id = ?
            "#,
            user_id
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| Error::Generic(format!("Failed to fetch memberships: {}", e)))?;

        let roles: Vec<String> = memberships
            .iter()
            .map(|m| format!("{:?}", m.role))
            .collect();

        let permissions: Vec<String> = memberships
            .iter()
            .flat_map(|m| {
                match m.role {
                    crate::models::UserRole::Admin => vec![
                        "WorkspaceCreate".to_string(),
                        "WorkspaceRead".to_string(),
                        "WorkspaceUpdate".to_string(),
                        "WorkspaceDelete".to_string(),
                        "WorkspaceManageMembers".to_string(),
                        "MockCreate".to_string(),
                        "MockRead".to_string(),
                        "MockUpdate".to_string(),
                        "MockDelete".to_string(),
                    ],
                    crate::models::UserRole::Editor => vec![
                        "MockCreate".to_string(),
                        "MockRead".to_string(),
                        "MockUpdate".to_string(),
                        "MockDelete".to_string(),
                    ],
                    crate::models::UserRole::Viewer => vec!["MockRead".to_string()],
                }
            })
            .collect();

        let last_activity = memberships
            .iter()
            .map(|m| m.last_activity)
            .max();

        let days_inactive = last_activity.map(|activity| {
            let duration = Utc::now() - activity;
            duration.num_days() as u64
        });

        let access_granted = memberships
            .iter()
            .map(|m| m.joined_at)
            .min()
            .unwrap_or(user.created_at);

        Ok(Some(UserAccessInfo {
            user_id: user.id,
            username: user.username,
            email: user.email,
            roles,
            permissions,
            last_login: last_activity,
            access_granted,
            days_inactive,
            is_active: user.is_active,
        }))
    }

    async fn get_last_login(&self, user_id: Uuid) -> Result<Option<DateTime<Utc>>, Error> {
        // Use last_activity from workspace_members as proxy for last login
        let result = sqlx::query!(
            r#"
            SELECT MAX(last_activity) as last_activity
            FROM workspace_members
            WHERE user_id = ?
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Error::Generic(format!("Failed to get last login: {}", e)))?;

        Ok(result.and_then(|r| r.last_activity))
    }

    async fn revoke_user_access(&self, user_id: Uuid, reason: String) -> Result<(), Error> {
        // Deactivate the user
        self.user_service
            .deactivate_user(user_id)
            .await
            .map_err(|e| Error::Generic(format!("Failed to revoke access: {}", e)))?;

        tracing::info!("Revoked access for user {}: {}", user_id, reason);

        Ok(())
    }

    async fn update_user_permissions(
        &self,
        user_id: Uuid,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> Result<(), Error> {
        // TODO: Implement permission updates
        // This would involve updating workspace memberships
        tracing::warn!(
            "update_user_permissions not yet fully implemented for user {}",
            user_id
        );
        Ok(())
    }
}
