//! Security module for MockForge
//!
//! This module provides security event tracking, SIEM integration, and security monitoring
//! capabilities for compliance with SOC 2 and ISO 27001 requirements.

pub mod access_review;
pub mod access_review_global;
pub mod access_review_notifications;
pub mod access_review_scheduler;
pub mod access_review_service;
pub mod api_tokens;
pub mod emitter;
pub mod events;
pub mod justification_storage;
pub mod mfa_tracking;
pub mod change_management;
pub mod change_management_global;
pub mod compliance_dashboard;
pub mod compliance_dashboard_global;
pub mod privileged_access;
pub mod privileged_access_global;
pub mod risk_assessment;
pub mod risk_assessment_global;
pub mod siem;

pub use access_review::{
    AccessReview, AccessReviewConfig, AccessReviewEngine, ApiTokenInfo, PrivilegedAccessInfo,
    ResourceAccessInfo, ReviewFrequency, ReviewStatus, ReviewType, UserAccessInfo, UserReviewConfig,
    UserReviewItem,
};
pub use access_review_global::{get_global_access_review_service, init_global_access_review_service, is_access_review_service_initialized};
pub use access_review_scheduler::AccessReviewScheduler;
pub use access_review_service::{AccessReviewService, UserDataProvider};
pub use api_tokens::{ApiTokenStorage, InMemoryApiTokenStorage};
pub use emitter::{emit_security_event, emit_security_event_async, init_global_siem_emitter, is_siem_emitter_initialized};
pub use events::{SecurityEvent, SecurityEventSeverity, SecurityEventType, EventActor, EventTarget, EventOutcome};
pub use justification_storage::{AccessJustification, JustificationStorage, InMemoryJustificationStorage};
pub use mfa_tracking::{MfaMethod, MfaStatus, MfaStorage, InMemoryMfaStorage};
pub use change_management::{ApprovalStatus, ChangeHistoryEntry, ChangeManagementConfig, ChangeManagementEngine, ChangePriority, ChangeRequest, ChangeStatus, ChangeType, ChangeUrgency};
pub use change_management_global::{get_global_change_management_engine, init_global_change_management_engine, is_change_management_engine_initialized};
pub use compliance_dashboard::{AlertSummary, AlertType, ComplianceAlert, ComplianceDashboardConfig, ComplianceDashboardData, ComplianceDashboardEngine, ComplianceGap, ComplianceStandard, ControlCategory, ControlEffectiveness, GapSeverity, GapStatus, GapSummary, RemediationStatus};
pub use compliance_dashboard_global::{get_global_compliance_dashboard_engine, init_global_compliance_dashboard_engine, is_compliance_dashboard_engine_initialized};
pub use privileged_access::{PrivilegedAccessConfig, PrivilegedAccessManager, PrivilegedAccessRequest, PrivilegedAction, PrivilegedActionType, PrivilegedRole, PrivilegedSession, RequestStatus};
pub use privileged_access_global::{get_global_privileged_access_manager, init_global_privileged_access_manager, is_privileged_access_manager_initialized};
pub use risk_assessment::{Impact, Likelihood, Risk, RiskAssessmentConfig, RiskAssessmentEngine, RiskCategory, RiskLevel, RiskReviewFrequency, RiskSummary, TreatmentOption, TreatmentStatus};
pub use risk_assessment_global::{get_global_risk_assessment_engine, init_global_risk_assessment_engine, is_risk_assessment_engine_initialized};
pub use siem::{SiemEmitter, SiemConfig, SiemDestination};
