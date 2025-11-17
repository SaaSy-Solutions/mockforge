//! HTTP handlers module

pub mod ab_testing;
pub mod access_review;
pub mod auth_helpers;
pub mod behavioral_cloning;
pub mod change_management;
pub mod compliance_dashboard;
pub mod consent;
pub mod consistency;
pub mod consumer_contracts;
pub mod deceptive_canary;
pub mod drift_budget;
pub mod failure_designer;
pub mod fidelity;
pub mod incident_replay;
pub mod oauth2_server;
pub mod pr_generation;
pub mod privileged_access;
pub mod risk_assessment;
pub mod risk_simulation;
pub mod scenario_studio;
pub mod snapshots;
pub mod token_lifecycle;
pub mod webhook_test;
pub mod xray;

pub use ab_testing::{ab_testing_router, ABTestingState};
pub use access_review::{
    access_review_router, AccessReviewState, ApproveAccessRequest, RevokeAccessRequest,
};
pub use auth_helpers::OptionalAuthClaims;
pub use behavioral_cloning::{
    apply_amplification, behavioral_cloning_router, build_probability_model, discover_sequences,
    generate_sequence_scenario, get_probability_model, get_rare_edges, get_sequence,
    list_probability_models, list_sequences, sample_latency, sample_status_code,
    BehavioralCloningState,
};
pub use change_management::{change_management_router, ChangeManagementState};
pub use compliance_dashboard::{compliance_dashboard_router, ComplianceDashboardState};
pub use consistency::{consistency_router, ConsistencyState};
pub use consumer_contracts::{consumer_contracts_router, ConsumerContractsState};
pub use deceptive_canary::{get_canary_config, get_canary_stats, update_canary_config};
pub use drift_budget::{drift_budget_router, DriftBudgetState};
pub use failure_designer::{
    generate_scenario, preview_config, validate_rule, FailureDesignerState,
};
pub use fidelity::{calculate_fidelity, fidelity_router, get_fidelity, FidelityState};
pub use incident_replay::{
    generate_replay, import_and_generate, import_incident, IncidentReplayState,
};
pub use pr_generation::{pr_generation_router, PRGenerationState};
pub use privileged_access::{privileged_access_router, PrivilegedAccessState};
pub use risk_assessment::{risk_assessment_router, RiskAssessmentState};
pub use scenario_studio::{scenario_studio_router, ScenarioStudioState};
pub use snapshots::{snapshot_router, SnapshotState};
pub use token_lifecycle::{token_lifecycle_router, TokenLifecycleState};
pub use webhook_test::{webhook_test_router, WebhookTestState};
pub use xray::xray_router;
