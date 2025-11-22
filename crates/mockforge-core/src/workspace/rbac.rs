//! Environment-scoped RBAC for workspace environments
//!
//! Extends the base RBAC system to support environment-specific permissions,
//! allowing fine-grained control over who can modify settings in dev/test/prod.

use crate::workspace::mock_environment::MockEnvironmentName;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Permission type - using string for now since mockforge_collab may not be available
/// In the future, this could be replaced with mockforge_collab::permissions::Permission
pub type Permission = String;

/// Environment permission policy
///
/// Defines which roles are allowed to perform specific actions in specific environments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentPermissionPolicy {
    /// Unique identifier
    pub id: String,
    /// Organization ID (optional, for org-wide policies)
    pub org_id: Option<String>,
    /// Workspace ID (optional, for workspace-specific policies)
    pub workspace_id: Option<String>,
    /// Environment this policy applies to
    pub environment: MockEnvironmentName,
    /// Permission this policy controls
    pub permission: String, // Permission name as string (e.g., "ManageSettings")
    /// Roles allowed to perform this action in this environment
    pub allowed_roles: Vec<String>, // Role names as strings (e.g., "admin", "platform", "qa")
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl EnvironmentPermissionPolicy {
    /// Create a new environment permission policy
    pub fn new(
        environment: MockEnvironmentName,
        permission: Permission,
        allowed_roles: Vec<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            org_id: None,
            workspace_id: None,
            environment,
            permission: permission.to_string(),
            allowed_roles,
            created_at: chrono::Utc::now(),
        }
    }

    /// Check if a role is allowed for this policy
    pub fn allows_role(&self, role: &str) -> bool {
        self.allowed_roles.iter().any(|r| r.eq_ignore_ascii_case(role))
    }
}

/// Environment permission checker
///
/// Checks if a user has permission to perform an action in a specific environment,
/// considering both base permissions and environment-specific policies.
pub struct EnvironmentPermissionChecker {
    /// Environment-specific policies indexed by (environment, permission)
    policies: HashMap<(MockEnvironmentName, String), Vec<EnvironmentPermissionPolicy>>,
}

impl EnvironmentPermissionChecker {
    /// Create a new environment permission checker
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }

    /// Add a policy
    pub fn add_policy(&mut self, policy: EnvironmentPermissionPolicy) {
        let key = (policy.environment, policy.permission.clone());
        self.policies.entry(key).or_insert_with(Vec::new).push(policy);
    }

    /// Check if a role has permission in an environment
    ///
    /// Returns true if:
    /// 1. There's no environment-specific policy (fallback to base permission check)
    /// 2. There's a policy and the role is allowed
    pub fn has_permission(
        &self,
        role: &str,
        permission: Permission,
        environment: MockEnvironmentName,
    ) -> bool {
        let key = (environment, permission.to_string());

        if let Some(policies) = self.policies.get(&key) {
            // Check if any policy allows this role
            policies.iter().any(|policy| policy.allows_role(role))
        } else {
            // No environment-specific policy, fallback to base permission check
            // This should be handled by the base RBAC system
            true
        }
    }

    /// Get policies for an environment
    pub fn get_policies_for_environment(
        &self,
        environment: MockEnvironmentName,
    ) -> Vec<&EnvironmentPermissionPolicy> {
        self.policies
            .iter()
            .filter_map(|((env, _), policies)| {
                if *env == environment {
                    Some(policies.iter())
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }

    /// Get policies for a permission across all environments
    pub fn get_policies_for_permission(
        &self,
        permission: Permission,
    ) -> Vec<&EnvironmentPermissionPolicy> {
        let perm_str = permission.to_string();
        self.policies
            .iter()
            .filter_map(|((_, perm), policies)| {
                if *perm == perm_str {
                    Some(policies.iter())
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }
}

impl Default for EnvironmentPermissionChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to check environment-scoped permissions
///
/// This function combines base permission checking with environment-specific policies.
/// It should be called after the base permission check passes.
pub fn check_environment_permission(
    checker: &EnvironmentPermissionChecker,
    role: &str,
    permission: Permission,
    environment: Option<MockEnvironmentName>,
) -> bool {
    // If no environment is specified, use base permission check only
    let env = match environment {
        Some(e) => e,
        None => return true, // No environment restriction
    };

    // Check environment-specific policy
    checker.has_permission(role, permission, env)
}
