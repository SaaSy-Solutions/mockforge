//! Single integration-test binary for this crate (#1451).
//!
//! Each module was previously its own `tests/*.rs` binary; linking one
//! binary instead of one per file cuts CI link time. Files that mutate
//! process-global state (env vars, global subscribers/databases) stay as
//! separate binaries next to this directory.

mod auth_crate_compat;
mod behavioral_cloning_extraction;
mod chaos_handler_routes;
mod deceptive_deploy_test;
mod drift_health_database_routes;
mod drift_health_routes;
mod drift_pipeline_routes;
mod error_scenarios_tests;
mod failure_scenarios_test;
mod fault_injection_e2e;
mod fixture_record_replay_e2e;
mod graceful_shutdown;
mod health_route_precedence;
mod intelligence_legacy_module_paths;
mod management_token_recording;
mod optional_feature_routes;
mod overrides_live_e2e;
mod response_status_2xx_e2e;
mod risk_simulation_routes;
mod security_tests;
mod snapshot_diff_routes;
mod spec_import_test;
mod token_integration_test;
mod workspace_runtime_activation;
