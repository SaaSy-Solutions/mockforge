//! Permission checking and role-based access control

use crate::error::{CollabError, Result};
use crate::models::UserRole;
use serde::{Deserialize, Serialize};

/// Permission types in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Permission {
    // Workspace permissions
    WorkspaceCreate,
    WorkspaceRead,
    WorkspaceUpdate,
    WorkspaceDelete,
    WorkspaceArchive,
    WorkspaceManageMembers,

    // Mock/Route permissions
    MockCreate,
    MockRead,
    MockUpdate,
    MockDelete,

    // Collaboration permissions
    InviteMembers,
    RemoveMembers,
    ChangeRoles,

    // History permissions
    ViewHistory,
    CreateSnapshot,
    RestoreSnapshot,

    // Settings permissions
    ManageSettings,
    ManageIntegrations,
}

/// Role permissions mapping
pub struct RolePermissions;

impl RolePermissions {
    /// Get all permissions for a role
    pub fn get_permissions(role: UserRole) -> Vec<Permission> {
        match role {
            UserRole::Admin => vec![
                // Full access to everything
                Permission::WorkspaceCreate,
                Permission::WorkspaceRead,
                Permission::WorkspaceUpdate,
                Permission::WorkspaceDelete,
                Permission::WorkspaceArchive,
                Permission::WorkspaceManageMembers,
                Permission::MockCreate,
                Permission::MockRead,
                Permission::MockUpdate,
                Permission::MockDelete,
                Permission::InviteMembers,
                Permission::RemoveMembers,
                Permission::ChangeRoles,
                Permission::ViewHistory,
                Permission::CreateSnapshot,
                Permission::RestoreSnapshot,
                Permission::ManageSettings,
                Permission::ManageIntegrations,
            ],
            UserRole::Editor => vec![
                // Can edit but not manage workspace or members
                Permission::WorkspaceRead,
                Permission::MockCreate,
                Permission::MockRead,
                Permission::MockUpdate,
                Permission::MockDelete,
                Permission::ViewHistory,
                Permission::CreateSnapshot,
            ],
            UserRole::Viewer => vec![
                // Read-only access
                Permission::WorkspaceRead,
                Permission::MockRead,
                Permission::ViewHistory,
            ],
        }
    }

    /// Check if a role has a specific permission
    pub fn has_permission(role: UserRole, permission: Permission) -> bool {
        Self::get_permissions(role).contains(&permission)
    }
}

/// Permission checker for authorization
pub struct PermissionChecker;

impl PermissionChecker {
    /// Check if a user has permission to perform an action
    pub fn check(user_role: UserRole, required_permission: Permission) -> Result<()> {
        if RolePermissions::has_permission(user_role, required_permission) {
            Ok(())
        } else {
            Err(CollabError::AuthorizationFailed(format!(
                "Role {:?} does not have permission {:?}",
                user_role, required_permission
            )))
        }
    }

    /// Check multiple permissions (must have all)
    pub fn check_all(user_role: UserRole, required_permissions: &[Permission]) -> Result<()> {
        for permission in required_permissions {
            Self::check(user_role, *permission)?;
        }
        Ok(())
    }

    /// Check multiple permissions (must have at least one)
    pub fn check_any(user_role: UserRole, required_permissions: &[Permission]) -> Result<()> {
        for permission in required_permissions {
            if RolePermissions::has_permission(user_role, *permission) {
                return Ok(());
            }
        }
        Err(CollabError::AuthorizationFailed(format!(
            "Role {:?} does not have any of the required permissions",
            user_role
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_permissions() {
        let permissions = RolePermissions::get_permissions(UserRole::Admin);
        assert!(permissions.contains(&Permission::WorkspaceDelete));
        assert!(permissions.contains(&Permission::MockCreate));
        assert!(permissions.contains(&Permission::ChangeRoles));
    }

    #[test]
    fn test_editor_permissions() {
        let permissions = RolePermissions::get_permissions(UserRole::Editor);
        assert!(permissions.contains(&Permission::MockCreate));
        assert!(permissions.contains(&Permission::MockUpdate));
        assert!(!permissions.contains(&Permission::WorkspaceDelete));
        assert!(!permissions.contains(&Permission::ChangeRoles));
    }

    #[test]
    fn test_viewer_permissions() {
        let permissions = RolePermissions::get_permissions(UserRole::Viewer);
        assert!(permissions.contains(&Permission::WorkspaceRead));
        assert!(permissions.contains(&Permission::MockRead));
        assert!(!permissions.contains(&Permission::MockCreate));
        assert!(!permissions.contains(&Permission::MockUpdate));
    }

    #[test]
    fn test_permission_check() {
        assert!(PermissionChecker::check(UserRole::Admin, Permission::WorkspaceDelete).is_ok());
        assert!(PermissionChecker::check(UserRole::Editor, Permission::MockCreate).is_ok());
        assert!(PermissionChecker::check(UserRole::Viewer, Permission::MockCreate).is_err());
    }

    #[test]
    fn test_check_all() {
        let permissions = vec![Permission::MockRead, Permission::MockCreate];
        assert!(PermissionChecker::check_all(UserRole::Editor, &permissions).is_ok());
        assert!(PermissionChecker::check_all(UserRole::Viewer, &permissions).is_err());
    }

    #[test]
    fn test_check_any() {
        let permissions = vec![Permission::MockCreate, Permission::WorkspaceDelete];
        assert!(PermissionChecker::check_any(UserRole::Editor, &permissions).is_ok());

        let admin_only = vec![Permission::WorkspaceDelete, Permission::ChangeRoles];
        assert!(PermissionChecker::check_any(UserRole::Viewer, &admin_only).is_err());
    }
}
