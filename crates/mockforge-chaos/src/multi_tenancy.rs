//! Multi-Tenancy Support
//!
//! Provides isolation and resource management for multiple tenants in MockForge.
//! Supports tenant-specific configurations, quotas, and access controls.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Multi-tenancy errors
#[derive(Error, Debug)]
pub enum MultiTenancyError {
    #[error("Tenant not found: {0}")]
    TenantNotFound(String),

    #[error("Tenant already exists: {0}")]
    TenantAlreadyExists(String),

    #[error("Access denied for tenant {tenant}: {reason}")]
    AccessDenied { tenant: String, reason: String },

    #[error("Quota exceeded for tenant {tenant}: {quota_type}")]
    QuotaExceeded { tenant: String, quota_type: String },

    #[error("Invalid tenant configuration: {0}")]
    InvalidConfig(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Unauthorized access: {0}")]
    Unauthorized(String),
}

pub type Result<T> = std::result::Result<T, MultiTenancyError>;

/// Tenant plan/tier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum TenantPlan {
    Free,
    Starter,
    Professional,
    Enterprise,
}

/// Resource quotas for a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    /// Maximum number of active scenarios
    pub max_scenarios: usize,
    /// Maximum number of concurrent executions
    pub max_concurrent_executions: usize,
    /// Maximum number of orchestrations
    pub max_orchestrations: usize,
    /// Maximum number of templates
    pub max_templates: usize,
    /// Maximum number of API requests per minute
    pub max_requests_per_minute: usize,
    /// Maximum storage in MB
    pub max_storage_mb: usize,
    /// Maximum number of users per tenant
    pub max_users: usize,
    /// Maximum duration for chaos experiments in seconds
    pub max_experiment_duration_secs: u64,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            max_scenarios: 10,
            max_concurrent_executions: 3,
            max_orchestrations: 5,
            max_templates: 10,
            max_requests_per_minute: 100,
            max_storage_mb: 100,
            max_users: 5,
            max_experiment_duration_secs: 3600, // 1 hour
        }
    }
}

impl ResourceQuota {
    /// Get quotas for a specific plan
    pub fn for_plan(plan: &TenantPlan) -> Self {
        match plan {
            TenantPlan::Free => Self {
                max_scenarios: 5,
                max_concurrent_executions: 1,
                max_orchestrations: 3,
                max_templates: 5,
                max_requests_per_minute: 50,
                max_storage_mb: 50,
                max_users: 1,
                max_experiment_duration_secs: 600, // 10 minutes
            },
            TenantPlan::Starter => Self {
                max_scenarios: 20,
                max_concurrent_executions: 5,
                max_orchestrations: 10,
                max_templates: 20,
                max_requests_per_minute: 200,
                max_storage_mb: 500,
                max_users: 5,
                max_experiment_duration_secs: 3600, // 1 hour
            },
            TenantPlan::Professional => Self {
                max_scenarios: 100,
                max_concurrent_executions: 20,
                max_orchestrations: 50,
                max_templates: 100,
                max_requests_per_minute: 1000,
                max_storage_mb: 5000,
                max_users: 25,
                max_experiment_duration_secs: 14400, // 4 hours
            },
            TenantPlan::Enterprise => Self {
                max_scenarios: usize::MAX,
                max_concurrent_executions: 100,
                max_orchestrations: usize::MAX,
                max_templates: usize::MAX,
                max_requests_per_minute: 10000,
                max_storage_mb: 50000,
                max_users: usize::MAX,
                max_experiment_duration_secs: 86400, // 24 hours
            },
        }
    }
}

/// Current resource usage for a tenant
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceUsage {
    pub scenarios: usize,
    pub concurrent_executions: usize,
    pub orchestrations: usize,
    pub templates: usize,
    pub storage_mb: usize,
    pub users: usize,
    pub requests_this_minute: usize,
    pub last_request_minute: DateTime<Utc>,
}

/// Tenant permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantPermissions {
    /// Can create chaos scenarios
    pub can_create_scenarios: bool,
    /// Can execute scenarios
    pub can_execute_scenarios: bool,
    /// Can access observability data
    pub can_view_observability: bool,
    /// Can manage resilience patterns
    pub can_manage_resilience: bool,
    /// Can access advanced features
    pub can_use_advanced_features: bool,
    /// Can integrate with external systems
    pub can_integrate_external: bool,
    /// Can use ML features
    pub can_use_ml_features: bool,
    /// Can manage users
    pub can_manage_users: bool,
    /// Custom permissions
    pub custom_permissions: HashSet<String>,
}

impl TenantPermissions {
    /// Get permissions for a specific plan
    pub fn for_plan(plan: &TenantPlan) -> Self {
        match plan {
            TenantPlan::Free => Self {
                can_create_scenarios: true,
                can_execute_scenarios: true,
                can_view_observability: false,
                can_manage_resilience: false,
                can_use_advanced_features: false,
                can_integrate_external: false,
                can_use_ml_features: false,
                can_manage_users: false,
                custom_permissions: HashSet::new(),
            },
            TenantPlan::Starter => Self {
                can_create_scenarios: true,
                can_execute_scenarios: true,
                can_view_observability: true,
                can_manage_resilience: true,
                can_use_advanced_features: false,
                can_integrate_external: false,
                can_use_ml_features: false,
                can_manage_users: true,
                custom_permissions: HashSet::new(),
            },
            TenantPlan::Professional => Self {
                can_create_scenarios: true,
                can_execute_scenarios: true,
                can_view_observability: true,
                can_manage_resilience: true,
                can_use_advanced_features: true,
                can_integrate_external: true,
                can_use_ml_features: true,
                can_manage_users: true,
                custom_permissions: HashSet::new(),
            },
            TenantPlan::Enterprise => Self {
                can_create_scenarios: true,
                can_execute_scenarios: true,
                can_view_observability: true,
                can_manage_resilience: true,
                can_use_advanced_features: true,
                can_integrate_external: true,
                can_use_ml_features: true,
                can_manage_users: true,
                custom_permissions: HashSet::new(),
            },
        }
    }

    /// Check if tenant has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        match permission {
            "create_scenarios" => self.can_create_scenarios,
            "execute_scenarios" => self.can_execute_scenarios,
            "view_observability" => self.can_view_observability,
            "manage_resilience" => self.can_manage_resilience,
            "use_advanced_features" => self.can_use_advanced_features,
            "integrate_external" => self.can_integrate_external,
            "use_ml_features" => self.can_use_ml_features,
            "manage_users" => self.can_manage_users,
            custom => self.custom_permissions.contains(custom),
        }
    }
}

/// Tenant information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub plan: TenantPlan,
    pub quota: ResourceQuota,
    pub usage: ResourceUsage,
    pub permissions: TenantPermissions,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub enabled: bool,
}

impl Tenant {
    /// Create a new tenant
    pub fn new(name: String, plan: TenantPlan) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            plan: plan.clone(),
            quota: ResourceQuota::for_plan(&plan),
            usage: ResourceUsage::default(),
            permissions: TenantPermissions::for_plan(&plan),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
            enabled: true,
        }
    }

    /// Check if tenant can perform action within quota
    pub fn check_quota(&self, resource_type: &str) -> Result<()> {
        if !self.enabled {
            return Err(MultiTenancyError::AccessDenied {
                tenant: self.id.clone(),
                reason: "Tenant is disabled".to_string(),
            });
        }

        match resource_type {
            "scenario" => {
                if self.usage.scenarios >= self.quota.max_scenarios {
                    return Err(MultiTenancyError::QuotaExceeded {
                        tenant: self.id.clone(),
                        quota_type: "scenarios".to_string(),
                    });
                }
            }
            "execution" => {
                if self.usage.concurrent_executions >= self.quota.max_concurrent_executions {
                    return Err(MultiTenancyError::QuotaExceeded {
                        tenant: self.id.clone(),
                        quota_type: "concurrent_executions".to_string(),
                    });
                }
            }
            "orchestration" => {
                if self.usage.orchestrations >= self.quota.max_orchestrations {
                    return Err(MultiTenancyError::QuotaExceeded {
                        tenant: self.id.clone(),
                        quota_type: "orchestrations".to_string(),
                    });
                }
            }
            "template" => {
                if self.usage.templates >= self.quota.max_templates {
                    return Err(MultiTenancyError::QuotaExceeded {
                        tenant: self.id.clone(),
                        quota_type: "templates".to_string(),
                    });
                }
            }
            "user" => {
                if self.usage.users >= self.quota.max_users {
                    return Err(MultiTenancyError::QuotaExceeded {
                        tenant: self.id.clone(),
                        quota_type: "users".to_string(),
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Check rate limiting
    pub fn check_rate_limit(&mut self) -> Result<()> {
        let now = Utc::now();
        let current_minute = now.format("%Y-%m-%d %H:%M").to_string();
        let last_minute = self.usage.last_request_minute.format("%Y-%m-%d %H:%M").to_string();

        if current_minute != last_minute {
            // Reset counter for new minute
            self.usage.requests_this_minute = 0;
            self.usage.last_request_minute = now;
        }

        if self.usage.requests_this_minute >= self.quota.max_requests_per_minute {
            return Err(MultiTenancyError::QuotaExceeded {
                tenant: self.id.clone(),
                quota_type: "requests_per_minute".to_string(),
            });
        }

        self.usage.requests_this_minute += 1;
        Ok(())
    }
}

/// Multi-tenancy manager
pub struct TenantManager {
    tenants: Arc<RwLock<HashMap<String, Tenant>>>,
    name_to_id: Arc<RwLock<HashMap<String, String>>>,
}

impl TenantManager {
    /// Create a new tenant manager
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
            name_to_id: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new tenant
    pub fn create_tenant(&self, name: String, plan: TenantPlan) -> Result<Tenant> {
        let mut name_map = self.name_to_id.write();

        if name_map.contains_key(&name) {
            return Err(MultiTenancyError::TenantAlreadyExists(name));
        }

        let tenant = Tenant::new(name.clone(), plan);
        let tenant_id = tenant.id.clone();

        let mut tenants = self.tenants.write();
        tenants.insert(tenant_id.clone(), tenant.clone());
        name_map.insert(name, tenant_id);

        Ok(tenant)
    }

    /// Get tenant by ID
    pub fn get_tenant(&self, tenant_id: &str) -> Result<Tenant> {
        let tenants = self.tenants.read();
        tenants
            .get(tenant_id)
            .cloned()
            .ok_or_else(|| MultiTenancyError::TenantNotFound(tenant_id.to_string()))
    }

    /// Get tenant by name
    pub fn get_tenant_by_name(&self, name: &str) -> Result<Tenant> {
        let name_map = self.name_to_id.read();
        let tenant_id = name_map
            .get(name)
            .ok_or_else(|| MultiTenancyError::TenantNotFound(name.to_string()))?;

        self.get_tenant(tenant_id)
    }

    /// Update tenant
    pub fn update_tenant(&self, tenant: Tenant) -> Result<()> {
        let mut tenants = self.tenants.write();

        if !tenants.contains_key(&tenant.id) {
            return Err(MultiTenancyError::TenantNotFound(tenant.id.clone()));
        }

        tenants.insert(tenant.id.clone(), tenant);
        Ok(())
    }

    /// Delete tenant
    pub fn delete_tenant(&self, tenant_id: &str) -> Result<()> {
        let mut tenants = self.tenants.write();
        let tenant = tenants
            .remove(tenant_id)
            .ok_or_else(|| MultiTenancyError::TenantNotFound(tenant_id.to_string()))?;

        let mut name_map = self.name_to_id.write();
        name_map.remove(&tenant.name);

        Ok(())
    }

    /// List all tenants
    pub fn list_tenants(&self) -> Vec<Tenant> {
        let tenants = self.tenants.read();
        tenants.values().cloned().collect()
    }

    /// Increment usage counter
    pub fn increment_usage(&self, tenant_id: &str, resource_type: &str) -> Result<()> {
        let mut tenants = self.tenants.write();
        let tenant = tenants
            .get_mut(tenant_id)
            .ok_or_else(|| MultiTenancyError::TenantNotFound(tenant_id.to_string()))?;

        match resource_type {
            "scenario" => tenant.usage.scenarios += 1,
            "execution" => tenant.usage.concurrent_executions += 1,
            "orchestration" => tenant.usage.orchestrations += 1,
            "template" => tenant.usage.templates += 1,
            "user" => tenant.usage.users += 1,
            _ => {}
        }

        Ok(())
    }

    /// Decrement usage counter
    pub fn decrement_usage(&self, tenant_id: &str, resource_type: &str) -> Result<()> {
        let mut tenants = self.tenants.write();
        let tenant = tenants
            .get_mut(tenant_id)
            .ok_or_else(|| MultiTenancyError::TenantNotFound(tenant_id.to_string()))?;

        match resource_type {
            "scenario" => {
                if tenant.usage.scenarios > 0 {
                    tenant.usage.scenarios -= 1;
                }
            }
            "execution" => {
                if tenant.usage.concurrent_executions > 0 {
                    tenant.usage.concurrent_executions -= 1;
                }
            }
            "orchestration" => {
                if tenant.usage.orchestrations > 0 {
                    tenant.usage.orchestrations -= 1;
                }
            }
            "template" => {
                if tenant.usage.templates > 0 {
                    tenant.usage.templates -= 1;
                }
            }
            "user" => {
                if tenant.usage.users > 0 {
                    tenant.usage.users -= 1;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Check permission for tenant
    pub fn check_permission(&self, tenant_id: &str, permission: &str) -> Result<()> {
        let tenant = self.get_tenant(tenant_id)?;

        if !tenant.enabled {
            return Err(MultiTenancyError::AccessDenied {
                tenant: tenant_id.to_string(),
                reason: "Tenant is disabled".to_string(),
            });
        }

        if !tenant.permissions.has_permission(permission) {
            return Err(MultiTenancyError::AccessDenied {
                tenant: tenant_id.to_string(),
                reason: format!("Missing permission: {}", permission),
            });
        }

        Ok(())
    }

    /// Check quota and increment if allowed
    pub fn check_and_increment(&self, tenant_id: &str, resource_type: &str) -> Result<()> {
        let tenant = self.get_tenant(tenant_id)?;
        tenant.check_quota(resource_type)?;
        self.increment_usage(tenant_id, resource_type)?;
        Ok(())
    }

    /// Upgrade tenant plan
    pub fn upgrade_plan(&self, tenant_id: &str, new_plan: TenantPlan) -> Result<()> {
        let mut tenant = self.get_tenant(tenant_id)?;

        if new_plan <= tenant.plan {
            return Err(MultiTenancyError::InvalidConfig(
                "New plan must be higher than current plan".to_string(),
            ));
        }

        tenant.plan = new_plan.clone();
        tenant.quota = ResourceQuota::for_plan(&new_plan);
        tenant.permissions = TenantPermissions::for_plan(&new_plan);
        tenant.updated_at = Utc::now();

        self.update_tenant(tenant)?;
        Ok(())
    }

    /// Disable tenant
    pub fn disable_tenant(&self, tenant_id: &str) -> Result<()> {
        let mut tenant = self.get_tenant(tenant_id)?;
        tenant.enabled = false;
        tenant.updated_at = Utc::now();
        self.update_tenant(tenant)?;
        Ok(())
    }

    /// Enable tenant
    pub fn enable_tenant(&self, tenant_id: &str) -> Result<()> {
        let mut tenant = self.get_tenant(tenant_id)?;
        tenant.enabled = true;
        tenant.updated_at = Utc::now();
        self.update_tenant(tenant)?;
        Ok(())
    }
}

impl Default for TenantManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_creation() {
        let manager = TenantManager::new();
        let tenant = manager.create_tenant("test-tenant".to_string(), TenantPlan::Starter).unwrap();

        assert_eq!(tenant.name, "test-tenant");
        assert_eq!(tenant.plan, TenantPlan::Starter);
        assert!(tenant.enabled);
    }

    #[test]
    fn test_duplicate_tenant() {
        let manager = TenantManager::new();
        manager.create_tenant("test-tenant".to_string(), TenantPlan::Free).unwrap();

        let result = manager.create_tenant("test-tenant".to_string(), TenantPlan::Free);
        assert!(result.is_err());
    }

    #[test]
    fn test_quota_checking() {
        let tenant = Tenant::new("test".to_string(), TenantPlan::Free);

        // Should be OK initially
        assert!(tenant.check_quota("scenario").is_ok());

        // Simulate exceeding quota
        let mut tenant_with_usage = tenant.clone();
        tenant_with_usage.usage.scenarios = tenant_with_usage.quota.max_scenarios;

        assert!(tenant_with_usage.check_quota("scenario").is_err());
    }

    #[test]
    fn test_permission_checking() {
        let free_tenant = Tenant::new("free".to_string(), TenantPlan::Free);
        let pro_tenant = Tenant::new("pro".to_string(), TenantPlan::Professional);

        assert!(!free_tenant.permissions.has_permission("use_ml_features"));
        assert!(pro_tenant.permissions.has_permission("use_ml_features"));
    }

    #[test]
    fn test_plan_upgrade() {
        let manager = TenantManager::new();
        let tenant = manager.create_tenant("test".to_string(), TenantPlan::Free).unwrap();

        manager.upgrade_plan(&tenant.id, TenantPlan::Professional).unwrap();

        let updated = manager.get_tenant(&tenant.id).unwrap();
        assert_eq!(updated.plan, TenantPlan::Professional);
        assert!(updated.permissions.has_permission("use_ml_features"));
    }

    #[test]
    fn test_usage_tracking() {
        let manager = TenantManager::new();
        let tenant = manager.create_tenant("test".to_string(), TenantPlan::Starter).unwrap();

        manager.increment_usage(&tenant.id, "scenario").unwrap();
        manager.increment_usage(&tenant.id, "scenario").unwrap();

        let updated = manager.get_tenant(&tenant.id).unwrap();
        assert_eq!(updated.usage.scenarios, 2);

        manager.decrement_usage(&tenant.id, "scenario").unwrap();
        let updated = manager.get_tenant(&tenant.id).unwrap();
        assert_eq!(updated.usage.scenarios, 1);
    }
}
