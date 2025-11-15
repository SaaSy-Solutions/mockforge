//! HTTP handlers module

pub mod access_review;
pub mod auth_helpers;
pub mod change_management;
pub mod compliance_dashboard;
pub mod consent;
pub mod oauth2_server;
pub mod privileged_access;
pub mod risk_assessment;
pub mod risk_simulation;
pub mod token_lifecycle;

pub use access_review::{
    access_review_router, AccessReviewState, ApproveAccessRequest, RevokeAccessRequest,
};
pub use change_management::{change_management_router, ChangeManagementState};
pub use compliance_dashboard::{compliance_dashboard_router, ComplianceDashboardState};
pub use privileged_access::{privileged_access_router, PrivilegedAccessState};
pub use risk_assessment::{risk_assessment_router, RiskAssessmentState};
pub use token_lifecycle::{token_lifecycle_router, TokenLifecycleState};
