//! Single integration-test binary for this crate (#1451).
//!
//! Each module was previously its own `tests/*.rs` binary; linking one
//! binary instead of one per file cuts CI link time. Files that mutate
//! process-global state (env vars, global subscribers/databases) stay as
//! separate binaries next to this directory.

// Shared by cloud_verification_e2e and hosted_capture_workspaces_e2e; loaded
// once here because a binary may not load the same file as two modules.
#[path = "support/cloud_verification_fixture.rs"]
mod fixture;

mod audit_integrity_e2e;
mod audit_pii_guard;
mod billing_bypass_guard_e2e;
mod cloud_ai_contract_diff_e2e;
mod cloud_conformance_e2e;
mod cloud_verification_e2e;
mod cross_tenant_chaos_capture_e2e;
mod diff_finding_incident_e2e;
mod entitlement_subscription_gate_e2e;
mod federation_scenarios_e2e;
mod free_hosted_quota;
mod free_hosted_upgrade;
mod hosted_capture_workspaces_e2e;
mod hosted_mock_overrides_e2e;
mod hosted_mock_rls_routes_e2e;
mod marketplace_e2e;
mod paid_flow_e2e;
mod query_token_e2e;
mod rls_coverage_e2e;
mod rls_join_coverage_e2e;
mod seed_credentials_neutralised;
mod signup_email_identity;
mod signup_flow_e2e;
mod sso_discover;
mod tenant_isolation_rls;
mod test_generation_quota_gate_e2e;
mod token_scope_e2e;
mod usage_limits_e2e;
mod workspace_content_e2e;
