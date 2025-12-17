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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_log_store_creation() {
        let store = AuditLogStore::new(100);
        let stats = store.get_stats().await;
        assert_eq!(stats.total_actions, 0);
        assert_eq!(stats.successful_actions, 0);
        assert_eq!(stats.failed_actions, 0);
    }

    #[tokio::test]
    async fn test_audit_log_store_default() {
        let store = AuditLogStore::default();
        let stats = store.get_stats().await;
        assert_eq!(stats.total_actions, 0);
    }

    #[tokio::test]
    async fn test_record_audit_log() {
        let store = AuditLogStore::new(100);

        let log = create_audit_log(
            AdminActionType::ConfigLatencyUpdated,
            "Updated latency config".to_string(),
            Some("/api/config/latency".to_string()),
            true,
            None,
            None,
        );

        store.record(log).await;

        let stats = store.get_stats().await;
        assert_eq!(stats.total_actions, 1);
        assert_eq!(stats.successful_actions, 1);
        assert_eq!(stats.failed_actions, 0);
    }

    #[tokio::test]
    async fn test_record_failed_audit_log() {
        let store = AuditLogStore::new(100);

        let log = create_audit_log(
            AdminActionType::ConfigLatencyUpdated,
            "Failed to update latency config".to_string(),
            Some("/api/config/latency".to_string()),
            false,
            Some("Permission denied".to_string()),
            None,
        );

        store.record(log).await;

        let stats = store.get_stats().await;
        assert_eq!(stats.total_actions, 1);
        assert_eq!(stats.successful_actions, 0);
        assert_eq!(stats.failed_actions, 1);
    }

    #[tokio::test]
    async fn test_audit_log_with_metadata() {
        let store = AuditLogStore::new(100);

        let metadata = serde_json::json!({
            "old_value": 100,
            "new_value": 200,
            "reason": "Performance optimization"
        });

        let log = create_audit_log(
            AdminActionType::ConfigLatencyUpdated,
            "Updated latency config".to_string(),
            Some("/api/config/latency".to_string()),
            true,
            None,
            Some(metadata.clone()),
        );

        store.record(log).await;

        let logs = store.get_logs(None, None, None, None).await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].metadata, Some(metadata));
    }

    #[tokio::test]
    async fn test_max_logs_limit() {
        let store = AuditLogStore::new(5);

        // Add 10 logs
        for i in 0..10 {
            let log = create_audit_log(
                AdminActionType::ConfigLatencyUpdated,
                format!("Action {}", i),
                None,
                true,
                None,
                None,
            );
            store.record(log).await;
        }

        let logs = store.get_logs(None, None, None, None).await;
        assert_eq!(logs.len(), 5, "Should only keep last 5 logs");
    }

    #[tokio::test]
    async fn test_get_logs_filtering_by_action_type() {
        let store = AuditLogStore::new(100);

        // Add different action types
        store
            .record(create_audit_log(
                AdminActionType::ConfigLatencyUpdated,
                "Latency updated".to_string(),
                None,
                true,
                None,
                None,
            ))
            .await;

        store
            .record(create_audit_log(
                AdminActionType::FixtureCreated,
                "Fixture created".to_string(),
                None,
                true,
                None,
                None,
            ))
            .await;

        store
            .record(create_audit_log(
                AdminActionType::ConfigLatencyUpdated,
                "Latency updated again".to_string(),
                None,
                true,
                None,
                None,
            ))
            .await;

        let logs = store
            .get_logs(Some(AdminActionType::ConfigLatencyUpdated), None, None, None)
            .await;
        assert_eq!(logs.len(), 2);
        assert!(logs.iter().all(|log| log.action_type == AdminActionType::ConfigLatencyUpdated));
    }

    #[tokio::test]
    async fn test_get_logs_filtering_by_user() {
        let store = AuditLogStore::new(100);

        let mut log1 = create_audit_log(
            AdminActionType::ConfigLatencyUpdated,
            "Action by user1".to_string(),
            None,
            true,
            None,
            None,
        );
        log1.user_id = Some("user1".to_string());
        store.record(log1).await;

        let mut log2 = create_audit_log(
            AdminActionType::FixtureCreated,
            "Action by user2".to_string(),
            None,
            true,
            None,
            None,
        );
        log2.user_id = Some("user2".to_string());
        store.record(log2).await;

        let mut log3 = create_audit_log(
            AdminActionType::ConfigFaultsUpdated,
            "Action by user1".to_string(),
            None,
            true,
            None,
            None,
        );
        log3.user_id = Some("user1".to_string());
        store.record(log3).await;

        let logs = store.get_logs(None, Some("user1"), None, None).await;
        assert_eq!(logs.len(), 2);
        assert!(logs.iter().all(|log| log.user_id.as_deref() == Some("user1")));
    }

    #[tokio::test]
    async fn test_get_logs_with_limit_and_offset() {
        let store = AuditLogStore::new(100);

        // Add 10 logs
        for i in 0..10 {
            let log = create_audit_log(
                AdminActionType::ConfigLatencyUpdated,
                format!("Action {}", i),
                None,
                true,
                None,
                None,
            );
            store.record(log).await;
        }

        // Get logs with limit
        let logs = store.get_logs(None, None, Some(5), None).await;
        assert_eq!(logs.len(), 5);

        // Get logs with offset
        let logs = store.get_logs(None, None, Some(3), Some(2)).await;
        assert_eq!(logs.len(), 3);

        // Get logs with offset beyond limit
        let logs = store.get_logs(None, None, Some(5), Some(8)).await;
        assert_eq!(logs.len(), 2);
    }

    #[tokio::test]
    async fn test_logs_sorted_by_timestamp_newest_first() {
        let store = AuditLogStore::new(100);

        // Add logs with delays
        for i in 0..5 {
            let log = create_audit_log(
                AdminActionType::ConfigLatencyUpdated,
                format!("Action {}", i),
                None,
                true,
                None,
                None,
            );
            store.record(log).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let logs = store.get_logs(None, None, None, None).await;
        assert_eq!(logs.len(), 5);

        // Verify newest first
        for i in 0..logs.len() - 1 {
            assert!(logs[i].timestamp >= logs[i + 1].timestamp);
        }
    }

    #[tokio::test]
    async fn test_clear_logs() {
        let store = AuditLogStore::new(100);

        // Add logs
        for _ in 0..5 {
            store
                .record(create_audit_log(
                    AdminActionType::ConfigLatencyUpdated,
                    "Action".to_string(),
                    None,
                    true,
                    None,
                    None,
                ))
                .await;
        }

        let stats_before = store.get_stats().await;
        assert_eq!(stats_before.total_actions, 5);

        store.clear().await;

        let stats_after = store.get_stats().await;
        assert_eq!(stats_after.total_actions, 0);
    }

    #[tokio::test]
    async fn test_audit_stats_actions_by_type() {
        let store = AuditLogStore::new(100);

        // Add various action types
        store
            .record(create_audit_log(
                AdminActionType::ConfigLatencyUpdated,
                "".to_string(),
                None,
                true,
                None,
                None,
            ))
            .await;
        store
            .record(create_audit_log(
                AdminActionType::ConfigLatencyUpdated,
                "".to_string(),
                None,
                true,
                None,
                None,
            ))
            .await;
        store
            .record(create_audit_log(
                AdminActionType::FixtureCreated,
                "".to_string(),
                None,
                true,
                None,
                None,
            ))
            .await;
        store
            .record(create_audit_log(
                AdminActionType::RouteEnabled,
                "".to_string(),
                None,
                true,
                None,
                None,
            ))
            .await;
        store
            .record(create_audit_log(
                AdminActionType::FixtureCreated,
                "".to_string(),
                None,
                true,
                None,
                None,
            ))
            .await;

        let stats = store.get_stats().await;
        assert_eq!(stats.total_actions, 5);
        assert_eq!(stats.actions_by_type.get("ConfigLatencyUpdated"), Some(&2));
        assert_eq!(stats.actions_by_type.get("FixtureCreated"), Some(&2));
        assert_eq!(stats.actions_by_type.get("RouteEnabled"), Some(&1));
    }

    #[tokio::test]
    async fn test_audit_stats_most_recent_timestamp() {
        let store = AuditLogStore::new(100);

        // Add first log
        store
            .record(create_audit_log(
                AdminActionType::ConfigLatencyUpdated,
                "First".to_string(),
                None,
                true,
                None,
                None,
            ))
            .await;

        let stats1 = store.get_stats().await;
        let first_timestamp = stats1.most_recent_timestamp.unwrap();

        // Wait a bit and add another log
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        store
            .record(create_audit_log(
                AdminActionType::FixtureCreated,
                "Second".to_string(),
                None,
                true,
                None,
                None,
            ))
            .await;

        let stats2 = store.get_stats().await;
        let second_timestamp = stats2.most_recent_timestamp.unwrap();

        assert!(second_timestamp > first_timestamp);
    }

    #[test]
    fn test_create_audit_log_helper() {
        let log = create_audit_log(
            AdminActionType::UserCreated,
            "Created new user".to_string(),
            Some("users/123".to_string()),
            true,
            None,
            Some(serde_json::json!({"username": "testuser"})),
        );

        assert_eq!(log.action_type, AdminActionType::UserCreated);
        assert_eq!(log.description, "Created new user");
        assert_eq!(log.resource, Some("users/123".to_string()));
        assert!(log.success);
        assert_eq!(log.error_message, None);
        assert!(log.metadata.is_some());
        assert_eq!(log.user_id, None);
        assert_eq!(log.username, None);
        assert_eq!(log.ip_address, None);
        assert_eq!(log.user_agent, None);
    }

    #[test]
    fn test_admin_action_type_serialization() {
        let action = AdminActionType::ConfigLatencyUpdated;
        let serialized = serde_json::to_string(&action).unwrap();
        assert_eq!(serialized, "\"config_latency_updated\"");

        let deserialized: AdminActionType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, action);
    }

    #[test]
    fn test_audit_log_serialization() {
        let log = AdminAuditLog {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            action_type: AdminActionType::FixtureCreated,
            user_id: Some("user123".to_string()),
            username: Some("testuser".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            description: "Created fixture".to_string(),
            resource: Some("/fixtures/test".to_string()),
            success: true,
            error_message: None,
            metadata: Some(serde_json::json!({"key": "value"})),
        };

        let serialized = serde_json::to_string(&log).unwrap();
        let deserialized: AdminAuditLog = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, log.id);
        assert_eq!(deserialized.action_type, log.action_type);
        assert_eq!(deserialized.user_id, log.user_id);
        assert_eq!(deserialized.description, log.description);
    }

    #[test]
    fn test_all_admin_action_types_covered() {
        // Test that all action types can be serialized and deserialized
        let actions = vec![
            AdminActionType::ConfigLatencyUpdated,
            AdminActionType::ConfigFaultsUpdated,
            AdminActionType::ConfigProxyUpdated,
            AdminActionType::ConfigTrafficShapingUpdated,
            AdminActionType::ConfigValidationUpdated,
            AdminActionType::ServerRestarted,
            AdminActionType::ServerShutdown,
            AdminActionType::ServerStatusChecked,
            AdminActionType::LogsCleared,
            AdminActionType::LogsExported,
            AdminActionType::LogsFiltered,
            AdminActionType::FixtureCreated,
            AdminActionType::FixtureUpdated,
            AdminActionType::FixtureDeleted,
            AdminActionType::FixtureBulkDeleted,
            AdminActionType::FixtureMoved,
            AdminActionType::RouteEnabled,
            AdminActionType::RouteDisabled,
            AdminActionType::RouteCreated,
            AdminActionType::RouteDeleted,
            AdminActionType::RouteUpdated,
            AdminActionType::ServiceEnabled,
            AdminActionType::ServiceDisabled,
            AdminActionType::ServiceConfigUpdated,
            AdminActionType::MetricsExported,
            AdminActionType::MetricsConfigUpdated,
            AdminActionType::UserCreated,
            AdminActionType::UserUpdated,
            AdminActionType::UserDeleted,
            AdminActionType::RoleChanged,
            AdminActionType::PermissionGranted,
            AdminActionType::PermissionRevoked,
            AdminActionType::SystemConfigBackedUp,
            AdminActionType::SystemConfigRestored,
            AdminActionType::SystemHealthChecked,
            AdminActionType::ApiKeyCreated,
            AdminActionType::ApiKeyDeleted,
            AdminActionType::ApiKeyRotated,
            AdminActionType::SecurityPolicyUpdated,
        ];

        for action in actions {
            let serialized = serde_json::to_string(&action).unwrap();
            let deserialized: AdminActionType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, action);
        }
    }

    #[tokio::test]
    async fn test_global_audit_store_initialization() {
        let store1 = init_global_audit_store(100);
        let store2 = get_global_audit_store();

        assert!(store2.is_some());

        // Both should point to the same store
        let store2 = store2.unwrap();

        // Add log via store1
        store1
            .record(create_audit_log(
                AdminActionType::ConfigLatencyUpdated,
                "Test".to_string(),
                None,
                true,
                None,
                None,
            ))
            .await;

        // Should be visible via store2
        let stats = store2.get_stats().await;
        assert_eq!(stats.total_actions, 1);
    }

    #[tokio::test]
    async fn test_concurrent_audit_log_writes() {
        let store = Arc::new(AuditLogStore::new(1000));
        let mut handles = vec![];

        // Spawn multiple tasks writing logs concurrently
        for i in 0..10 {
            let store_clone = store.clone();
            let handle = tokio::spawn(async move {
                for j in 0..10 {
                    let log = create_audit_log(
                        AdminActionType::ConfigLatencyUpdated,
                        format!("Task {} - Log {}", i, j),
                        None,
                        true,
                        None,
                        None,
                    );
                    store_clone.record(log).await;
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        let stats = store.get_stats().await;
        assert_eq!(stats.total_actions, 100);
    }
}
