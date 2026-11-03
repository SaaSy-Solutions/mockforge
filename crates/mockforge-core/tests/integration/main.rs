//! Single integration-test binary for this crate (#1451).
//!
//! Each module was previously its own `tests/*.rs` binary; linking one
//! binary instead of one per file cuts CI link time. Files that mutate
//! process-global state (env vars, global subscribers/databases) stay as
//! separate binaries next to this directory.

mod chain_execution_e2e_test;
mod chaining_integration_tests;
mod codegen_integration_test;
mod concurrency_tests;
mod cross_protocol_tests;
mod custom_fixture_integration_test;
mod error_handling_tests;
mod latency_failure_edge_cases;
mod module_imports_test;
mod openapi_generator_tests;
mod optional_feature_basics;
mod output_control_tests;
mod overrides_edge_cases;
mod performance_edge_cases;
mod performance_regression_tests;
mod priority_handler_edge_cases;
mod prop_conditions_tests;
mod prop_routing_tests;
mod prop_tests;
mod protocol_registry_integration;
mod reality_hot_reload_test;
mod reality_integration_test;
mod record_replay_edge_cases;
mod request_chaining_edge_cases;
mod request_logger_edge_cases;
mod routing_edge_cases;
mod rule_explanation_tests;
mod schema_constraints;
mod stateful_handler_edge_cases;
mod swagger2_import;
mod sync_cross_platform_tests;
// These two files wrap their tests in a same-named inline module.
#[allow(clippy::module_inception)]
mod sync_tests;
mod test_openapi_routes;
mod traffic_shaping_edge_cases;
mod uuid_validation;
#[allow(clippy::module_inception)]
mod validation_tests;
