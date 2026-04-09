//! Permission enum with 28 granular permissions
//!
//! Defines fine-grained permissions for MockForge Cloud resources
//!
//! Permissions are:
//! - Organization: OrgRead, OrgUpdate, OrgDelete, OrgManageMembers, OrgManageBilling
//! - Plugin: PluginRead, PluginPublish, PluginYank, PluginVerify
//! - Template: TemplateRead, TemplatePublish, TemplateUpdate, TemplateDelete
//! - Scenario: ScenarioRead, ScenarioPublish, ScenarioUpdate, ScenarioDelete
//! - Review: ReviewCreate, ReviewUpdate, ReviewDelete, ReviewModerate
//! - HostedMock: HostedMockRead, HostedMockCreate, HostedMockUpdate, HostedMockDelete, HostedMockMetrics
//! - Usage: UsageRead
//! - Admin: AdminAll (super-permission that grants all other permissions)

use serde::{Deserialize, Serialize};

/// Permission groups for authorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    // Organization permissions
    OrgRead,
    OrgUpdate,
    OrgDelete,
    OrgManageMembers,
    OrgManageBilling,

    // Plugin permissions
    PluginRead,
    PluginPublish,
    PluginYank,
    PluginVerify,

    // Template permissions
    TemplateRead,
    TemplatePublish,
    TemplateUpdate,
    TemplateDelete,

    // Scenario permissions
    ScenarioRead,
    ScenarioPublish,
    ScenarioUpdate,
    ScenarioDelete,

    // Review permissions
    ReviewCreate,
    ReviewUpdate,
    ReviewDelete,
    ReviewModerate,

    // Hosted Mock permissions
    HostedMockRead,
    HostedMockCreate,
    HostedMockUpdate,
    HostedMockDelete,
    HostedMockMetrics,

    // Usage permissions
    UsageRead,

    // Admin permissions (requires all permissions)
    AdminAll,
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Permission::OrgRead => write!(f, "org:read"),
            Permission::OrgUpdate => write!(f, "org:update"),
            Permission::OrgDelete => write!(f, "org:delete"),
            Permission::OrgManageMembers => write!(f, "org:manage_members"),
            Permission::OrgManageBilling => write!(f, "org:manage_billing"),
            Permission::PluginRead => write!(f, "plugin:read"),
            Permission::PluginPublish => write!(f, "plugin:publish"),
            Permission::PluginYank => write!(f, "plugin:yank"),
            Permission::PluginVerify => write!(f, "plugin:verify"),
            Permission::TemplateRead => write!(f, "template:read"),
            Permission::TemplatePublish => write!(f, "template:publish"),
            Permission::TemplateUpdate => write!(f, "template:update"),
            Permission::TemplateDelete => write!(f, "template:delete"),
            Permission::ScenarioRead => write!(f, "scenario:read"),
            Permission::ScenarioPublish => write!(f, "scenario:publish"),
            Permission::ScenarioUpdate => write!(f, "scenario:update"),
            Permission::ScenarioDelete => write!(f, "scenario:delete"),
            Permission::ReviewCreate => write!(f, "review:create"),
            Permission::ReviewUpdate => write!(f, "review:update"),
            Permission::ReviewDelete => write!(f, "review:delete"),
            Permission::ReviewModerate => write!(f, "review:moderate"),
            Permission::HostedMockRead => write!(f, "hosted_mock:read"),
            Permission::HostedMockCreate => write!(f, "hosted_mock:create"),
            Permission::HostedMockUpdate => write!(f, "hosted_mock:update"),
            Permission::HostedMockDelete => write!(f, "hosted_mock:delete"),
            Permission::HostedMockMetrics => write!(f, "hosted_mock:metrics"),
            Permission::UsageRead => write!(f, "usage:read"),
            Permission::AdminAll => write!(f, "admin:all"),
        }
    }
}

impl Permission {
    /// Check if a permission set grants a specific permission.
    /// Returns true if:
    /// - The exact permission is in the set, OR
    /// - AdminAll is in the set (super-permission that grants everything)
    pub fn is_granted(required: Permission, granted: &[Permission]) -> bool {
        granted.contains(&required) || granted.contains(&Permission::AdminAll)
    }

    /// Get permission category for UI grouping
    pub fn category(&self) -> PermissionCategory {
        match self {
            // Organization
            Permission::OrgRead
            | Permission::OrgUpdate
            | Permission::OrgDelete
            | Permission::OrgManageMembers
            | Permission::OrgManageBilling => PermissionCategory::Organization,
            // Plugin
            Permission::PluginRead
            | Permission::PluginPublish
            | Permission::PluginYank
            | Permission::PluginVerify => PermissionCategory::Plugin,
            // Template
            Permission::TemplateRead
            | Permission::TemplatePublish
            | Permission::TemplateUpdate
            | Permission::TemplateDelete => PermissionCategory::Template,
            // Scenario
            Permission::ScenarioRead
            | Permission::ScenarioPublish
            | Permission::ScenarioUpdate
            | Permission::ScenarioDelete => PermissionCategory::Scenario,
            // Review
            Permission::ReviewCreate
            | Permission::ReviewUpdate
            | Permission::ReviewDelete
            | Permission::ReviewModerate => PermissionCategory::Review,
            // Hosted Mock
            Permission::HostedMockRead
            | Permission::HostedMockCreate
            | Permission::HostedMockUpdate
            | Permission::HostedMockDelete
            | Permission::HostedMockMetrics => PermissionCategory::HostedMock,
            // Usage
            Permission::UsageRead => PermissionCategory::Usage,
            // Admin
            Permission::AdminAll => PermissionCategory::Admin,
        }
    }
}

/// Permission categories for UI display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionCategory {
    Organization,
    Plugin,
    Template,
    Scenario,
    Review,
    HostedMock,
    Usage,
    Admin,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_display() {
        assert_eq!(Permission::OrgRead.to_string(), "org:read");
        assert_eq!(Permission::PluginPublish.to_string(), "plugin:publish");
        assert_eq!(Permission::HostedMockMetrics.to_string(), "hosted_mock:metrics");
        assert_eq!(Permission::AdminAll.to_string(), "admin:all");
    }

    #[test]
    fn test_permission_serde_roundtrip() {
        let perm = Permission::OrgManageMembers;
        let json = serde_json::to_string(&perm).unwrap();
        assert_eq!(json, "\"org_manage_members\"");
        let deserialized: Permission = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, perm);
    }

    #[test]
    fn test_permission_category_mapping() {
        assert_eq!(Permission::OrgRead.category(), PermissionCategory::Organization);
        assert_eq!(Permission::OrgDelete.category(), PermissionCategory::Organization);
        assert_eq!(Permission::PluginYank.category(), PermissionCategory::Plugin);
        assert_eq!(Permission::TemplateUpdate.category(), PermissionCategory::Template);
        assert_eq!(Permission::ScenarioPublish.category(), PermissionCategory::Scenario);
        assert_eq!(Permission::ReviewModerate.category(), PermissionCategory::Review);
        assert_eq!(Permission::HostedMockCreate.category(), PermissionCategory::HostedMock);
        assert_eq!(Permission::UsageRead.category(), PermissionCategory::Usage);
        assert_eq!(Permission::AdminAll.category(), PermissionCategory::Admin);
    }

    #[test]
    fn test_is_granted_exact_match() {
        let perms = vec![Permission::OrgRead, Permission::PluginPublish];
        assert!(Permission::is_granted(Permission::OrgRead, &perms));
        assert!(Permission::is_granted(Permission::PluginPublish, &perms));
        assert!(!Permission::is_granted(Permission::OrgDelete, &perms));
    }

    #[test]
    fn test_is_granted_admin_all_bypasses() {
        let perms = vec![Permission::AdminAll];
        // AdminAll should grant any permission
        assert!(Permission::is_granted(Permission::OrgDelete, &perms));
        assert!(Permission::is_granted(Permission::PluginYank, &perms));
        assert!(Permission::is_granted(Permission::HostedMockDelete, &perms));
    }

    #[test]
    fn test_is_granted_empty_set() {
        let perms: Vec<Permission> = vec![];
        assert!(!Permission::is_granted(Permission::OrgRead, &perms));
    }

    #[test]
    fn test_permission_category_serde_roundtrip() {
        let cat = PermissionCategory::HostedMock;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"hosted_mock\"");
        let deserialized: PermissionCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, cat);
    }

    #[test]
    fn test_permission_copy_and_eq() {
        let p1 = Permission::OrgRead;
        let p2 = p1; // Copy
        assert_eq!(p1, p2);
        assert_ne!(p1, Permission::OrgUpdate);
    }

    #[test]
    fn test_permission_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Permission::OrgRead);
        set.insert(Permission::OrgRead); // duplicate
        set.insert(Permission::PluginPublish);
        assert_eq!(set.len(), 2);
    }
}
