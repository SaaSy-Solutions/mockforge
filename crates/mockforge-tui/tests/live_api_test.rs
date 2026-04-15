//! Live integration tests that verify the TUI client can parse actual server responses.
//! These tests require a running `MockForge` server at localhost:9080.

#![allow(clippy::unwrap_used)]

use mockforge_tui::api::client::MockForgeClient;

fn client() -> MockForgeClient {
    MockForgeClient::new("http://localhost:9080".into(), None).unwrap()
}

/// Check if the server is running before running tests.
async fn server_available() -> bool {
    client().ping().await
}

#[tokio::test]
async fn dashboard_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running at localhost:9080");
        return;
    }
    let result = client().get_dashboard().await;
    match &result {
        Ok(data) => {
            assert!(!data.server_info.version.is_empty());
            eprintln!("Dashboard OK: version={}", data.server_info.version);
        }
        Err(e) => panic!("Dashboard parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn routes_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_routes().await;
    match &result {
        Ok(routes) => eprintln!("Routes OK: {} routes", routes.len()),
        Err(e) => panic!("Routes parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn health_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_health().await;
    match &result {
        Ok(health) => {
            assert!(!health.status.is_empty());
            eprintln!("Health OK: status={}", health.status);
        }
        Err(e) => panic!("Health parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn config_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_config().await;
    match &result {
        Ok(_) => eprintln!("Config OK"),
        Err(e) => panic!("Config parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn metrics_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_metrics().await;
    match &result {
        Ok(_) => eprintln!("Metrics OK"),
        Err(e) => panic!("Metrics parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn logs_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_logs(Some(10)).await;
    match &result {
        Ok(logs) => eprintln!("Logs OK: {} entries", logs.len()),
        Err(e) => panic!("Logs parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn plugins_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_plugins().await;
    match &result {
        Ok(plugins) => eprintln!("Plugins OK: {} plugins", plugins.len()),
        Err(e) => panic!("Plugins parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn fixtures_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_fixtures().await;
    match &result {
        Ok(fixtures) => eprintln!("Fixtures OK: {} fixtures", fixtures.len()),
        Err(e) => panic!("Fixtures parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn server_info_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_server_info().await;
    match &result {
        Ok(info) => eprintln!("ServerInfo OK: admin_port={}", info.admin_port),
        Err(e) => panic!("ServerInfo parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn time_travel_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_time_travel_status().await;
    match &result {
        Ok(status) => eprintln!("TimeTravel OK: enabled={}", status.enabled),
        Err(e) => panic!("TimeTravel parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn analytics_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_analytics_summary().await;
    match &result {
        Ok(summary) => eprintln!("Analytics OK: rate={}", summary.request_rate),
        Err(e) => panic!("Analytics parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn audit_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_audit_logs().await;
    match &result {
        Ok(logs) => eprintln!("Audit OK: {} entries", logs.len()),
        Err(e) => panic!("Audit parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn workspaces_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_workspaces().await;
    match &result {
        Ok(ws) => eprintln!("Workspaces OK: {} workspaces", ws.len()),
        Err(e) => panic!("Workspaces parse failed: {e:#}"),
    }
}

#[tokio::test]
async fn smoke_tests_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_smoke_tests().await;
    match &result {
        Ok(tests) => eprintln!("SmokeTests OK: {} tests", tests.len()),
        Err(e) => panic!("SmokeTests parse failed: {e:#}"),
    }
}

// Endpoints that may not exist (SPA fallback) - should error gracefully, not panic
#[tokio::test]
async fn chaos_handles_missing_endpoint() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_chaos_status().await;
    // OK or error is fine, just shouldn't panic
    match &result {
        Ok(_) => eprintln!("Chaos: available"),
        Err(e) => eprintln!("Chaos: not available (expected) - {e}"),
    }
}

#[tokio::test]
async fn federation_handles_missing_endpoint() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_federation_peers().await;
    match &result {
        Ok(_) => eprintln!("Federation: available"),
        Err(e) => eprintln!("Federation: not available (expected) - {e}"),
    }
}

#[tokio::test]
async fn vbr_handles_missing_endpoint() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_vbr_status().await;
    match &result {
        Ok(_) => eprintln!("VBR: available"),
        Err(e) => eprintln!("VBR: not available (expected) - {e}"),
    }
}

#[tokio::test]
async fn recorder_handles_missing_endpoint() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_recorder_status().await;
    match &result {
        Ok(_) => eprintln!("Recorder: available"),
        Err(e) => eprintln!("Recorder: not available (expected) - {e}"),
    }
}

#[tokio::test]
async fn world_state_handles_missing_endpoint() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_world_state().await;
    match &result {
        Ok(_) => eprintln!("WorldState: available"),
        Err(e) => eprintln!("WorldState: not available (expected) - {e}"),
    }
}

#[tokio::test]
async fn contract_diff_parses_from_live_server() {
    if !server_available().await {
        eprintln!("SKIP: server not running");
        return;
    }
    let result = client().get_contract_diff_captures().await;
    match &result {
        Ok(captures) => eprintln!("ContractDiff OK: {} captures", captures.len()),
        Err(e) => panic!("ContractDiff parse failed: {e:#}"),
    }
}
