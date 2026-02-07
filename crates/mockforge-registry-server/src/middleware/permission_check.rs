//! Permission checking for API handlers
//!
//! Provides centralized permission checking for granular access control.
//! Uses the Permission enum and OrgRole::get_permissions() to determine
//! whether a user has a specific permission within an organization.

use crate::{
    error::ApiError,
    models::{OrgMember, OrgRole},
    AppState,
};
use uuid::Uuid;

use super::permissions::Permission;

/// Centralized permission checker bound to application state.
///
/// Usage in handlers:
/// ```ignore
/// let checker = PermissionChecker::new(&state);
/// checker.require_permission(user_id, org_id, Permission::PluginPublish).await?;
/// ```
pub struct PermissionChecker<'a> {
    state: &'a AppState,
}

impl<'a> PermissionChecker<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    /// Check if a user has a specific permission within an organization.
    ///
    /// Returns `Ok(true)` if the permission is granted, `Ok(false)` otherwise.
    /// Returns `Err` on database errors.
    pub async fn has_permission(
        &self,
        user_id: Uuid,
        org_id: Uuid,
        required: Permission,
    ) -> Result<bool, ApiError> {
        let role = self.resolve_role(user_id, org_id).await?;

        match role {
            Some(role) => {
                let granted = role.get_permissions();
                Ok(Permission::is_granted(required, &granted))
            }
            // Not a member of the org — no permissions
            None => Ok(false),
        }
    }

    /// Require a specific permission, returning `ApiError::PermissionDenied` if not granted.
    pub async fn require_permission(
        &self,
        user_id: Uuid,
        org_id: Uuid,
        required: Permission,
    ) -> Result<(), ApiError> {
        if self.has_permission(user_id, org_id, required).await? {
            Ok(())
        } else {
            Err(ApiError::PermissionDenied)
        }
    }

    /// Get all permissions for a user within a specific organization.
    pub async fn get_permissions(
        &self,
        user_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<Permission>, ApiError> {
        let role = self.resolve_role(user_id, org_id).await?;

        match role {
            Some(role) => Ok(role.get_permissions()),
            None => Ok(Vec::new()),
        }
    }

    /// Resolve a user's role within an organization.
    /// Returns `None` if the user is not a member.
    async fn resolve_role(&self, user_id: Uuid, org_id: Uuid) -> Result<Option<OrgRole>, ApiError> {
        let member = OrgMember::find(self.state.db.pool(), org_id, user_id)
            .await
            .map_err(|e| ApiError::Database(e))?;

        Ok(member.map(|m| m.role()))
    }
}
