//! Privileged Access Management
//!
//! This module provides comprehensive privileged access management including:
//! - MFA enforcement
//! - Access justification tracking
//! - Privileged action monitoring
//! - Session management
//! - Automatic revocation

use crate::security::{
    justification_storage::{AccessJustification, JustificationStorage},
    mfa_tracking::{MfaStatus, MfaStorage},
};
use crate::Error;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Privileged role types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrivilegedRole {
    /// Admin role - full system access
    Admin,
    /// Owner role - organization ownership
    Owner,
    /// Service account - automated system access
    ServiceAccount,
}

/// Privileged access request status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestStatus {
    /// Request pending manager approval
    PendingManager,
    /// Request pending security review
    PendingSecurity,
    /// Request approved
    Approved,
    /// Request denied
    Denied,
    /// Request cancelled
    Cancelled,
}

/// Privileged action type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegedActionType {
    /// User management actions
    UserCreate,
    /// User deletion
    UserDelete,
    /// User modification
    UserModify,
    /// Role assignment
    RoleAssign,
    /// Role revocation
    RoleRevoke,
    /// Role escalation
    RoleEscalate,
    /// Permission grant
    PermissionGrant,
    /// Permission revocation
    PermissionRevoke,
    /// Configuration modification
    ConfigModify,
    /// Security policy change
    SecurityPolicyChange,
    /// Security setting change
    SecuritySettingChange,
    /// Audit log access
    AuditLogAccess,
    /// Data export
    DataExport,
    /// Data deletion
    DataDelete,
    /// Other privileged actions
    Other,
}

/// Privileged access request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegedAccessRequest {
    /// Request ID
    pub request_id: Uuid,
    /// User requesting access
    pub user_id: Uuid,
    /// Requested role
    pub requested_role: PrivilegedRole,
    /// Justification text
    pub justification: String,
    /// Business need description
    pub business_need: Option<String>,
    /// Manager who approved (if applicable)
    pub manager_approval: Option<Uuid>,
    /// Security team approval
    pub security_approval: Option<Uuid>,
    /// Request status
    pub status: RequestStatus,
    /// Request creation date
    pub created_at: DateTime<Utc>,
    /// Last update date
    pub updated_at: DateTime<Utc>,
    /// Access expiration date (if approved)
    pub expires_at: Option<DateTime<Utc>>,
}

impl PrivilegedAccessRequest {
    /// Create a new privileged access request
    pub fn new(
        user_id: Uuid,
        requested_role: PrivilegedRole,
        justification: String,
        business_need: Option<String>,
        manager_approval: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            request_id: Uuid::new_v4(),
            user_id,
            requested_role,
            justification,
            business_need,
            manager_approval,
            security_approval: None,
            status: RequestStatus::PendingManager,
            created_at: now,
            updated_at: now,
            expires_at: None,
        }
    }

    /// Check if request is approved
    pub fn is_approved(&self) -> bool {
        self.status == RequestStatus::Approved
    }

    /// Check if request is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Utc::now() > exp)
            .unwrap_or(false)
    }
}


/// Privileged action record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegedAction {
    /// Action ID
    pub action_id: Uuid,
    /// User who performed the action
    pub user_id: Uuid,
    /// Action type
    pub action_type: PrivilegedActionType,
    /// Resource affected
    pub resource: Option<String>,
    /// Action details
    pub details: Option<String>,
    /// IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Privileged session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegedSession {
    /// Session ID
    pub session_id: String,
    /// User ID
    pub user_id: Uuid,
    /// Role
    pub role: PrivilegedRole,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Last activity time
    pub last_activity: DateTime<Utc>,
    /// IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Whether session is active
    pub is_active: bool,
}

/// Privileged access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegedAccessConfig {
    /// Require MFA for privileged users
    pub require_mfa: bool,
    /// MFA grace period in days
    pub mfa_grace_period_days: u64,
    /// Auto-suspend if MFA not enabled
    pub auto_suspend_no_mfa: bool,
    /// Session timeout in minutes
    pub session_timeout_minutes: u64,
    /// Max concurrent sessions
    pub max_concurrent_sessions: u32,
    /// Record sensitive actions
    pub record_sensitive_actions: bool,
    /// Monitor activity
    pub monitor_activity: bool,
    /// Sensitive action types that require alerting
    pub sensitive_actions: Vec<PrivilegedActionType>,
}

impl Default for PrivilegedAccessConfig {
    fn default() -> Self {
        Self {
            require_mfa: true,
            mfa_grace_period_days: 7,
            auto_suspend_no_mfa: true,
            session_timeout_minutes: 30,
            max_concurrent_sessions: 2,
            record_sensitive_actions: true,
            monitor_activity: true,
            sensitive_actions: vec![
                PrivilegedActionType::UserDelete,
                PrivilegedActionType::RoleEscalate,
                PrivilegedActionType::SecurityPolicyChange,
                PrivilegedActionType::DataExport,
                PrivilegedActionType::AuditLogAccess,
            ],
        }
    }
}

/// Privileged access manager
///
/// Manages privileged access requests, monitoring, and enforcement
pub struct PrivilegedAccessManager {
    config: PrivilegedAccessConfig,
    mfa_storage: Option<Arc<dyn MfaStorage>>,
    justification_storage: Option<Arc<dyn JustificationStorage>>,
    /// Active privileged sessions
    sessions: Arc<RwLock<HashMap<String, PrivilegedSession>>>,
    /// Privileged actions log
    actions: Arc<RwLock<Vec<PrivilegedAction>>>,
    /// Active access requests
    requests: Arc<RwLock<HashMap<Uuid, PrivilegedAccessRequest>>>,
}

impl PrivilegedAccessManager {
    /// Create a new privileged access manager
    pub fn new(
        config: PrivilegedAccessConfig,
        mfa_storage: Option<Arc<dyn MfaStorage>>,
        justification_storage: Option<Arc<dyn JustificationStorage>>,
    ) -> Self {
        Self {
            config,
            mfa_storage,
            justification_storage,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            actions: Arc::new(RwLock::new(Vec::new())),
            requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Request privileged access
    pub async fn request_privileged_access(
        &self,
        user_id: Uuid,
        requested_role: PrivilegedRole,
        justification: String,
        business_need: Option<String>,
        manager_approval: Option<Uuid>,
    ) -> Result<PrivilegedAccessRequest, Error> {
        let request = PrivilegedAccessRequest::new(
            user_id,
            requested_role,
            justification,
            business_need,
            manager_approval,
        );

        let mut requests = self.requests.write().await;
        requests.insert(request.request_id, request.clone());

        Ok(request)
    }

    /// Approve privileged access request (manager approval)
    pub async fn approve_manager(
        &self,
        request_id: Uuid,
        approver_id: Uuid,
    ) -> Result<(), Error> {
        let mut requests = self.requests.write().await;
        let request = requests
            .get_mut(&request_id)
            .ok_or_else(|| Error::Generic("Request not found".to_string()))?;

        if request.status != RequestStatus::PendingManager {
            return Err(Error::Generic("Request is not pending manager approval".to_string()));
        }

        request.manager_approval = Some(approver_id);
        request.status = RequestStatus::PendingSecurity;
        request.updated_at = Utc::now();

        Ok(())
    }

    /// Approve privileged access request (security approval)
    pub async fn approve_security(
        &self,
        request_id: Uuid,
        approver_id: Uuid,
        expiration_days: u64,
    ) -> Result<(), Error> {
        let mut requests = self.requests.write().await;
        let request = requests
            .get_mut(&request_id)
            .ok_or_else(|| Error::Generic("Request not found".to_string()))?;

        if request.status != RequestStatus::PendingSecurity {
            return Err(Error::Generic("Request is not pending security approval".to_string()));
        }

        request.security_approval = Some(approver_id);
        request.status = RequestStatus::Approved;
        request.expires_at = Some(Utc::now() + Duration::days(expiration_days as i64));
        request.updated_at = Utc::now();

        // Store justification
        if let Some(ref just_storage) = self.justification_storage {
            let justification = AccessJustification::new(
                request.user_id,
                request.justification.clone(),
                request.business_need.clone(),
                request.manager_approval,
                request.expires_at,
            );
            just_storage.set_justification(justification).await?;
        }

        Ok(())
    }

    /// Deny privileged access request
    pub async fn deny_request(
        &self,
        request_id: Uuid,
        reason: String,
    ) -> Result<(), Error> {
        let mut requests = self.requests.write().await;
        let request = requests
            .get_mut(&request_id)
            .ok_or_else(|| Error::Generic("Request not found".to_string()))?;

        request.status = RequestStatus::Denied;
        request.updated_at = Utc::now();

        Ok(())
    }

    /// Check MFA compliance for a user
    pub async fn check_mfa_compliance(&self, user_id: Uuid) -> Result<bool, Error> {
        if !self.config.require_mfa {
            return Ok(true);
        }

        if let Some(ref mfa_storage) = self.mfa_storage {
            let mfa_status = mfa_storage.get_mfa_status(user_id).await?;
            Ok(mfa_status.map(|s| s.enabled).unwrap_or(false))
        } else {
            // No MFA storage configured, assume compliant
            Ok(true)
        }
    }

    /// Record a privileged action
    pub async fn record_action(
        &self,
        user_id: Uuid,
        action_type: PrivilegedActionType,
        resource: Option<String>,
        details: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
        session_id: Option<String>,
    ) -> Result<Uuid, Error> {
        let action = PrivilegedAction {
            action_id: Uuid::new_v4(),
            user_id,
            action_type,
            resource,
            details,
            ip_address,
            user_agent,
            session_id,
            timestamp: Utc::now(),
        };

        let mut actions = self.actions.write().await;
        actions.push(action.clone());

        // Check if this is a sensitive action that requires alerting
        if self.config.sensitive_actions.contains(&action_type) {
            // TODO: Emit security event for sensitive action
        }

        Ok(action.action_id)
    }

    /// Start a privileged session
    pub async fn start_session(
        &self,
        session_id: String,
        user_id: Uuid,
        role: PrivilegedRole,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(), Error> {
        // Check MFA compliance
        if !self.check_mfa_compliance(user_id).await? {
            if self.config.auto_suspend_no_mfa {
                return Err(Error::Generic("MFA not enabled for privileged user".to_string()));
            }
        }

        // Check concurrent session limit
        let sessions = self.sessions.read().await;
        let active_sessions = sessions
            .values()
            .filter(|s| s.user_id == user_id && s.is_active)
            .count();

        if active_sessions >= self.config.max_concurrent_sessions as usize {
            return Err(Error::Generic("Maximum concurrent sessions reached".to_string()));
        }
        drop(sessions);

        let session = PrivilegedSession {
            session_id: session_id.clone(),
            user_id,
            role,
            started_at: Utc::now(),
            last_activity: Utc::now(),
            ip_address,
            user_agent,
            is_active: true,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, session);

        Ok(())
    }

    /// Update session activity
    pub async fn update_session_activity(&self, session_id: &str) -> Result<(), Error> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = Utc::now();
        }
        Ok(())
    }

    /// End a privileged session
    pub async fn end_session(&self, session_id: &str) -> Result<(), Error> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.is_active = false;
        }
        Ok(())
    }

    /// Check for expired sessions and clean them up
    pub async fn cleanup_expired_sessions(&self) -> Result<Vec<String>, Error> {
        let timeout = Duration::minutes(self.config.session_timeout_minutes as i64);
        let now = Utc::now();
        let mut expired = Vec::new();

        let mut sessions = self.sessions.write().await;
        for (session_id, session) in sessions.iter_mut() {
            if session.is_active && (now - session.last_activity) > timeout {
                session.is_active = false;
                expired.push(session_id.clone());
            }
        }

        Ok(expired)
    }

    /// Get all privileged actions for a user
    pub async fn get_user_actions(&self, user_id: Uuid) -> Result<Vec<PrivilegedAction>, Error> {
        let actions = self.actions.read().await;
        Ok(actions
            .iter()
            .filter(|a| a.user_id == user_id)
            .cloned()
            .collect())
    }

    /// Get all active privileged sessions
    pub async fn get_active_sessions(&self) -> Result<Vec<PrivilegedSession>, Error> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .values()
            .filter(|s| s.is_active)
            .cloned()
            .collect())
    }

    /// Get access request by ID
    pub async fn get_request(&self, request_id: Uuid) -> Result<Option<PrivilegedAccessRequest>, Error> {
        let requests = self.requests.read().await;
        Ok(requests.get(&request_id).cloned())
    }

    /// Get all requests for a user
    pub async fn get_user_requests(&self, user_id: Uuid) -> Result<Vec<PrivilegedAccessRequest>, Error> {
        let requests = self.requests.read().await;
        Ok(requests
            .values()
            .filter(|r| r.user_id == user_id)
            .cloned()
            .collect())
    }
}

// Required for Arc usage
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_privileged_access_request() {
        let manager = PrivilegedAccessManager::new(
            PrivilegedAccessConfig::default(),
            None,
            None,
        );

        let request = manager
            .request_privileged_access(
                Uuid::new_v4(),
                PrivilegedRole::Admin,
                "Required for system administration".to_string(),
                Some("Manage production infrastructure".to_string()),
                Some(Uuid::new_v4()),
            )
            .await
            .unwrap();

        assert_eq!(request.status, RequestStatus::PendingManager);
        assert!(request.manager_approval.is_some());
    }
}
