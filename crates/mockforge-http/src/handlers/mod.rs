//! HTTP handlers module

pub mod ab_testing;
pub mod access_review;
pub mod ai_studio;
pub mod auth_helpers;
#[cfg(feature = "behavioral-cloning")]
pub mod behavioral_cloning;
pub mod change_management;
pub mod compliance_dashboard;
pub mod consent;
pub mod consistency;
pub mod consumer_contracts;
pub mod contract_health;
pub mod deceptive_canary;
pub mod drift_budget;
pub mod failure_designer;
pub mod fidelity;
pub mod forecasting;
pub mod incident_replay;
pub mod oauth2_server;
pub mod performance;
#[cfg(feature = "pipelines")]
pub mod pipelines;
pub mod pr_generation;
pub mod privileged_access;
pub mod protocol_contracts;
pub mod risk_assessment;
pub mod risk_simulation;
pub mod scenario_studio;
pub mod semantic_drift;
pub mod snapshot_diff;
pub mod snapshots;
pub mod threat_modeling;
pub mod token_lifecycle;
pub mod webhook_test;
pub mod world_state;
pub mod xray;

pub use ab_testing::{ab_testing_router, ABTestingState};
pub use access_review::{
    access_review_router, AccessReviewState, ApproveAccessRequest, RevokeAccessRequest,
};
pub use ai_studio::{ai_studio_router, AiStudioState};
pub use auth_helpers::OptionalAuthClaims;
#[cfg(feature = "behavioral-cloning")]
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
pub use contract_health::{contract_health_router, ContractHealthState};
pub use deceptive_canary::{get_canary_config, get_canary_stats, update_canary_config};
pub use drift_budget::{drift_budget_router, DriftBudgetState};
pub use failure_designer::{
    generate_scenario, preview_config, validate_rule, FailureDesignerState,
};
pub use fidelity::{calculate_fidelity, fidelity_router, get_fidelity, FidelityState};
pub use forecasting::{forecasting_router, ForecastingState};
pub use incident_replay::{
    generate_replay, import_and_generate, import_incident, IncidentReplayState,
};
pub use performance::{performance_router, PerformanceState};
#[cfg(feature = "pipelines")]
pub use pipelines::{pipeline_router, PipelineState};
pub use pr_generation::{pr_generation_router, PRGenerationState};
pub use privileged_access::{privileged_access_router, PrivilegedAccessState};
pub use protocol_contracts::{protocol_contracts_router, ProtocolContractState};
pub use risk_assessment::{risk_assessment_router, RiskAssessmentState};
pub use scenario_studio::{scenario_studio_router, ScenarioStudioState};
pub use semantic_drift::{semantic_drift_router, SemanticDriftState};
pub use snapshots::{snapshot_router, SnapshotState};
pub use threat_modeling::{threat_modeling_router, ThreatModelingState};
pub use token_lifecycle::{token_lifecycle_router, TokenLifecycleState};
pub use webhook_test::{webhook_test_router, WebhookTestState};
pub use world_state::{world_state_router, WorldStateState};
pub use xray::xray_router;
