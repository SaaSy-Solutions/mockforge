//! HTTP handlers module

pub mod access_review;
pub mod change_management;
pub mod compliance_dashboard;
pub mod privileged_access;
pub mod risk_assessment;

pub use access_review::{
    access_review_router, AccessReviewState, ApproveAccessRequest, RevokeAccessRequest,
};
pub use change_management::{change_management_router, ChangeManagementState};
pub use compliance_dashboard::{compliance_dashboard_router, ComplianceDashboardState};
pub use privileged_access::{privileged_access_router, PrivilegedAccessState};
pub use risk_assessment::{risk_assessment_router, RiskAssessmentState};
