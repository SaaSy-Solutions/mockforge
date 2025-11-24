//! Audit logging for Admin UI actions
//!
//! This module provides comprehensive audit logging for all administrative actions
//! performed through the Admin UI, ensuring compliance and security monitoring.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Admin action types that should be audited
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdminActionType {
    // Configuration changes
    ConfigLatencyUpdated,
    ConfigFaultsUpdated,
    ConfigProxyUpdated,
    ConfigTrafficShapingUpdated,
    ConfigValidationUpdated,

    // Server management
    ServerRestarted,
    ServerShutdown,
    ServerStatusChecked,

    // Log management
    LogsCleared,
    LogsExported,
    LogsFiltered,

    // Fixture management
    FixtureCreated,
    FixtureUpdated,
    FixtureDeleted,
    FixtureBulkDeleted,
    FixtureMoved,

    // Route management
    RouteEnabled,
    RouteDisabled,
    RouteCreated,
    RouteDeleted,
    RouteUpdated,

    // Service management
    ServiceEnabled,
    ServiceDisabled,
    ServiceConfigUpdated,

    // Metrics and monitoring
    MetricsExported,
    MetricsConfigUpdated,

    // User and access management
    UserCreated,
    UserUpdated,
    UserDeleted,
    RoleChanged,
    PermissionGranted,
    PermissionRevoked,

    // System operations
    SystemConfigBackedUp,
    SystemConfigRestored,
    SystemHealthChecked,

    // Security operations
    ApiKeyCreated,
    ApiKeyDeleted,
    ApiKeyRotated,
    SecurityPolicyUpdated,
}

/// Audit log entry for admin actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAuditLog {
    /// Unique audit log ID
    pub id: Uuid,
    /// Timestamp of the action
    pub timestamp: chrono::DateTime<Utc>,
    /// Type of action performed
    pub action_type: AdminActionType,
    /// User who performed the action (if authenticated)
    pub user_id: Option<String>,
    /// Username (if available)
    pub username: Option<String>,
    /// IP address of the requester
    pub ip_address: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
    /// Action description
    pub description: String,
    /// Resource affected (e.g., endpoint path, fixture ID)
    pub resource: Option<String>,
    /// Success status
    pub success: bool,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Audit log storage
#[derive(Debug, Clone)]
pub struct AuditLogStore {
    /// In-memory audit logs (max 10000 entries)
    logs: Arc<RwLock<Vec<AdminAuditLog>>>,
    /// Maximum number of logs to keep
    max_logs: usize,
}

impl Default for AuditLogStore {
    fn default() -> Self {
        Self::new(10000)
    }
}

impl AuditLogStore {
    /// Create a new audit log store
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
            max_logs,
        }
    }

    /// Record an audit log entry
    pub async fn record(&self, log: AdminAuditLog) {
        let mut logs = self.logs.write().await;
        logs.push(log.clone());

        // Trim to max_logs if necessary
        if logs.len() > self.max_logs {
            let remove_count = logs.len() - self.max_logs;
            logs.drain(0..remove_count);
        }

        // Log to tracing for external log aggregation
        if log.success {
            info!(
                action = ?log.action_type,
                user_id = ?log.user_id,
                resource = ?log.resource,
                "Admin action: {}",
                log.description
            );
        } else {
            warn!(
                action = ?log.action_type,
                user_id = ?log.user_id,
                resource = ?log.resource,
                error = ?log.error_message,
                "Admin action failed: {}",
                log.description
            );
        }
    }

    /// Get audit logs with optional filtering
    pub async fn get_logs(
        &self,
        action_type: Option<AdminActionType>,
        user_id: Option<&str>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Vec<AdminAuditLog> {
        let logs = self.logs.read().await;
        let mut filtered: Vec<_> = logs.iter().cloned().collect();

        // Filter by action type
        if let Some(action_type) = action_type {
            filtered.retain(|log| log.action_type == action_type);
        }

        // Filter by user ID
        if let Some(user_id) = user_id {
            filtered.retain(|log| log.user_id.as_deref() == Some(user_id));
        }

        // Sort by timestamp (newest first)
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply offset
        let start = offset.unwrap_or(0);
        let end = limit.map(|l| start + l).unwrap_or(filtered.len());

        filtered.into_iter().skip(start).take(end - start).collect()
    }

    /// Clear all audit logs
    pub async fn clear(&self) {
        let mut logs = self.logs.write().await;
        logs.clear();
    }

    /// Get statistics about audit logs
    pub async fn get_stats(&self) -> AuditLogStats {
        let logs = self.logs.read().await;

        let total_actions = logs.len();
        let successful_actions = logs.iter().filter(|log| log.success).count();
        let failed_actions = total_actions - successful_actions;

        // Count by action type
        let mut actions_by_type: HashMap<String, usize> = HashMap::new();
        for log in logs.iter() {
            let key = format!("{:?}", log.action_type);
            *actions_by_type.entry(key).or_insert(0) += 1;
        }

        // Get most recent action
        let most_recent = logs.iter().max_by_key(|log| log.timestamp).cloned();

        AuditLogStats {
            total_actions,
            successful_actions,
            failed_actions,
            actions_by_type,
            most_recent_timestamp: most_recent.map(|log| log.timestamp),
        }
    }
}

/// Audit log statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogStats {
    /// Total number of audit log entries
    pub total_actions: usize,
    /// Number of successful actions
    pub successful_actions: usize,
    /// Number of failed actions
    pub failed_actions: usize,
    /// Count of actions by type
    pub actions_by_type: HashMap<String, usize>,
    /// Timestamp of most recent action
    pub most_recent_timestamp: Option<chrono::DateTime<Utc>>,
}

/// Helper function to create an audit log entry
pub fn create_audit_log(
    action_type: AdminActionType,
    description: String,
    resource: Option<String>,
    success: bool,
    error_message: Option<String>,
    metadata: Option<serde_json::Value>,
) -> AdminAuditLog {
    AdminAuditLog {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        action_type,
        user_id: None,    // Will be set by middleware
        username: None,   // Will be set by middleware
        ip_address: None, // Will be set by middleware
        user_agent: None, // Will be set by middleware
        description,
        resource,
        success,
        error_message,
        metadata,
    }
}

/// Global audit log store instance
static GLOBAL_AUDIT_STORE: std::sync::OnceLock<Arc<AuditLogStore>> = std::sync::OnceLock::new();

/// Initialize the global audit log store
pub fn init_global_audit_store(max_logs: usize) -> Arc<AuditLogStore> {
    GLOBAL_AUDIT_STORE
        .get_or_init(|| Arc::new(AuditLogStore::new(max_logs)))
        .clone()
}

/// Get the global audit log store
pub fn get_global_audit_store() -> Option<Arc<AuditLogStore>> {
    GLOBAL_AUDIT_STORE.get().cloned()
}
