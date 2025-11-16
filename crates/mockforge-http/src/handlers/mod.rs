//! HTTP handlers module

pub mod access_review;
pub mod auth_helpers;
pub mod change_management;
pub mod compliance_dashboard;
pub mod consent;
pub mod consumer_contracts;
pub mod deceptive_canary;
pub mod drift_budget;
pub mod failure_designer;
pub mod fidelity;
pub mod incident_replay;
pub mod oauth2_server;
pub mod pr_generation;
pub mod privileged_access;
pub mod webhook_test;
pub mod risk_assessment;
pub mod risk_simulation;
pub mod token_lifecycle;

pub use access_review::{
    access_review_router, AccessReviewState, ApproveAccessRequest, RevokeAccessRequest,
};
pub use auth_helpers::OptionalAuthClaims;
pub use change_management::{change_management_router, ChangeManagementState};
pub use compliance_dashboard::{compliance_dashboard_router, ComplianceDashboardState};
pub use consumer_contracts::{consumer_contracts_router, ConsumerContractsState};
pub use deceptive_canary::{get_canary_config, get_canary_stats, update_canary_config};
pub use drift_budget::{drift_budget_router, DriftBudgetState};
pub use failure_designer::{
    generate_scenario, preview_config, validate_rule, FailureDesignerState,
};
pub use fidelity::{calculate_fidelity, get_fidelity};
pub use incident_replay::{
    generate_replay, import_and_generate, import_incident, IncidentReplayState,
};
pub use privileged_access::{privileged_access_router, PrivilegedAccessState};
pub use pr_generation::{pr_generation_router, PRGenerationState};
pub use risk_assessment::{risk_assessment_router, RiskAssessmentState};
pub use webhook_test::{webhook_test_router, WebhookTestState};
pub use token_lifecycle::{token_lifecycle_router, TokenLifecycleState};
