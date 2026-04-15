//! Render-verification tests for all TUI screens.
//!
//! Each test creates a screen, optionally feeds it sample data via `on_data()`,
//! then renders it into a headless `TestBackend` terminal. This catches panics,
//! layout errors, and rendering regressions without needing a real terminal.

#![allow(clippy::unwrap_used)]

use mockforge_tui::screens::Screen;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

/// Standard terminal size for render tests (80x24 is classic).
const WIDTH: u16 = 120;
const HEIGHT: u16 = 40;

/// Render a screen into a test terminal and return the buffer content as a string.
fn render_screen(screen: &dyn Screen) -> String {
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend).expect("failed to create test terminal");
    let area = Rect::new(0, 0, WIDTH, HEIGHT);

    terminal
        .draw(|frame| {
            screen.render(frame, area);
        })
        .expect("render should not fail");

    // Extract buffer content as a string for assertions
    let buffer = terminal.backend().buffer().clone();
    let mut output = String::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let cell = &buffer[(x, y)];
            output.push_str(cell.symbol());
        }
        output.push('\n');
    }
    output
}

// ── Sample data builders ──────────────────────────────────────────────

fn sample_dashboard_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "server_info": { "version": "0.3.73" },
        "system_info": {},
        "servers": [
            { "server_type": "HTTP", "running": true, "address": "127.0.0.1:3000" },
            { "server_type": "gRPC", "running": true, "address": "127.0.0.1:50051" }
        ],
        "metrics": {
            "total_requests": 5678,
            "average_response_time": 23.5,
            "error_rate": 0.01
        },
        "system": {
            "cpu_usage_percent": 45.0,
            "memory_usage_mb": 256,
            "active_threads": 4,
            "total_routes": 15,
            "total_fixtures": 5
        },
        "recent_logs": [
            {
                "id": "req-001",
                "method": "GET",
                "path": "/api/users",
                "status_code": 200,
                "response_time_ms": 12,
                "timestamp": "2025-01-01T14:23:01Z"
            },
            {
                "id": "req-002",
                "method": "POST",
                "path": "/api/items",
                "status_code": 201,
                "response_time_ms": 45,
                "timestamp": "2025-01-01T14:23:02Z"
            }
        ]
    }))
    .unwrap()
}

fn sample_routes_json() -> String {
    serde_json::to_string(&serde_json::json!([
        {
            "path": "/api/users",
            "method": "GET",
            "request_count": 100,
            "error_count": 2,
            "latency_ms": 45,
            "has_fixtures": true
        },
        {
            "path": "/api/items",
            "method": "POST",
            "request_count": 50,
            "error_count": 0,
            "latency_ms": null,
            "has_fixtures": false
        },
        {
            "path": "/api/orders/{id}",
            "method": "DELETE",
            "request_count": 10,
            "error_count": 1,
            "latency_ms": 120,
            "has_fixtures": true
        }
    ]))
    .unwrap()
}

fn sample_smoke_tests_json() -> String {
    serde_json::to_string(&serde_json::json!([
        {
            "id": "test-1",
            "name": "GET /health",
            "method": "GET",
            "path": "/health",
            "status": "passed",
            "response_time_ms": 5,
            "error_message": null
        },
        {
            "id": "test-2",
            "name": "POST /api/items",
            "method": "POST",
            "path": "/api/items",
            "status": "failed",
            "response_time_ms": 120,
            "error_message": "Expected 201, got 500"
        }
    ]))
    .unwrap()
}

fn sample_chaos_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "enabled": true,
        "active_scenario": "network_degradation",
        "active_scenarios": ["network_degradation"],
        "settings": {
            "latency_ms": 200,
            "failure_rate": 0.1
        }
    }))
    .unwrap()
}

fn sample_metrics_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "total_requests": 9999,
        "requests_per_second": 42.5,
        "average_response_time_ms": 15.3,
        "p50_ms": 10.0,
        "p95_ms": 50.0,
        "p99_ms": 120.0,
        "error_rate": 0.005,
        "active_connections": 12,
        "uptime_seconds": 86400
    }))
    .unwrap()
}

fn sample_health_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "status": "healthy",
        "checks": {
            "http_server": { "status": "up" },
            "grpc_server": { "status": "up" },
            "memory": { "status": "ok", "value": "256MB" }
        }
    }))
    .unwrap()
}

fn sample_config_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "http_port": 3000,
        "grpc_port": 50051,
        "admin_port": 9080,
        "latency_enabled": true,
        "chaos_enabled": false,
        "template_expansion": true
    }))
    .unwrap()
}

fn sample_plugins_json() -> String {
    serde_json::to_string(&serde_json::json!([
        {
            "name": "auth-plugin",
            "version": "1.0.0",
            "enabled": true,
            "description": "Authentication plugin"
        }
    ]))
    .unwrap()
}

fn sample_fixtures_json() -> String {
    serde_json::to_string(&serde_json::json!([
        {
            "id": "fix-1",
            "route": "/api/users",
            "method": "GET",
            "name": "Default users response",
            "priority": 0
        }
    ]))
    .unwrap()
}

fn sample_chains_json() -> String {
    serde_json::to_string(&serde_json::json!([
        {
            "id": "chain-1",
            "name": "Login Flow",
            "steps": [{"action": "login"}],
            "description": "Simulates login"
        },
        {
            "id": "chain-2",
            "name": "CRUD Flow",
            "steps": [{"action": "create"}, {"action": "read"}],
            "description": "Create-read cycle"
        }
    ]))
    .unwrap()
}

fn sample_workspaces_json() -> String {
    serde_json::to_string(&serde_json::json!([
        {
            "id": "ws-1",
            "name": "Development",
            "description": "Dev workspace",
            "active": true,
            "environments": ["dev", "staging"]
        }
    ]))
    .unwrap()
}

fn sample_federation_json() -> String {
    serde_json::to_string(&serde_json::json!([
        {
            "id": "peer-1",
            "url": "http://peer1:9080",
            "status": "connected"
        }
    ]))
    .unwrap()
}

fn sample_analytics_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "total_requests": 10000,
        "top_routes": [
            { "path": "/api/users", "count": 5000 },
            { "path": "/api/items", "count": 3000 }
        ],
        "error_trends": []
    }))
    .unwrap()
}

fn sample_audit_json() -> String {
    serde_json::to_string(&serde_json::json!([
        {
            "id": "audit-1",
            "timestamp": "2025-01-01T14:23:01Z",
            "action": "config_change",
            "user": "admin",
            "details": "Changed latency settings"
        }
    ]))
    .unwrap()
}

fn sample_recorder_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "recording": false,
        "sessions": []
    }))
    .unwrap()
}

fn sample_time_travel_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "enabled": false,
        "snapshots": []
    }))
    .unwrap()
}

fn sample_world_state_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "entities": {},
        "counters": {}
    }))
    .unwrap()
}

fn sample_contract_diff_json() -> String {
    serde_json::to_string(&serde_json::json!([])).unwrap()
}

fn sample_import_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "formats": ["openapi", "postman", "har"],
        "recent_imports": []
    }))
    .unwrap()
}

fn sample_behavioral_cloning_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "enabled": false,
        "models": []
    }))
    .unwrap()
}

// ── Empty-state render tests (no data) ─────────────────────────────────
// Verify that every screen renders without panicking in its initial state.

#[test]
fn dashboard_renders_empty() {
    use mockforge_tui::screens::dashboard::DashboardScreen;
    let screen = DashboardScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn logs_renders_empty() {
    use mockforge_tui::screens::logs::LogsScreen;
    let screen = LogsScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn routes_renders_empty() {
    use mockforge_tui::screens::routes::RoutesScreen;
    let screen = RoutesScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn metrics_renders_empty() {
    use mockforge_tui::screens::metrics::MetricsScreen;
    let screen = MetricsScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn config_renders_empty() {
    use mockforge_tui::screens::config::ConfigScreen;
    let screen = ConfigScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn chaos_renders_empty() {
    use mockforge_tui::screens::chaos::ChaosScreen;
    let screen = ChaosScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn workspaces_renders_empty() {
    use mockforge_tui::screens::workspaces::WorkspacesScreen;
    let screen = WorkspacesScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn plugins_renders_empty() {
    use mockforge_tui::screens::plugins::PluginsScreen;
    let screen = PluginsScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn fixtures_renders_empty() {
    use mockforge_tui::screens::fixtures::FixturesScreen;
    let screen = FixturesScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn health_renders_empty() {
    use mockforge_tui::screens::health::HealthScreen;
    let screen = HealthScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn smoke_tests_renders_empty() {
    use mockforge_tui::screens::smoke_tests::SmokeTestsScreen;
    let screen = SmokeTestsScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn time_travel_renders_empty() {
    use mockforge_tui::screens::time_travel::TimeTravelScreen;
    let screen = TimeTravelScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn chains_renders_empty() {
    use mockforge_tui::screens::chains::ChainsScreen;
    let screen = ChainsScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn verification_renders_empty() {
    use mockforge_tui::screens::verification::VerificationScreen;
    let screen = VerificationScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn analytics_renders_empty() {
    use mockforge_tui::screens::analytics::AnalyticsScreen;
    let screen = AnalyticsScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn recorder_renders_empty() {
    use mockforge_tui::screens::recorder::RecorderScreen;
    let screen = RecorderScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn import_renders_empty() {
    use mockforge_tui::screens::import::ImportScreen;
    let screen = ImportScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn audit_renders_empty() {
    use mockforge_tui::screens::audit::AuditScreen;
    let screen = AuditScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn world_state_renders_empty() {
    use mockforge_tui::screens::world_state::WorldStateScreen;
    let screen = WorldStateScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn contract_diff_renders_empty() {
    use mockforge_tui::screens::contract_diff::ContractDiffScreen;
    let screen = ContractDiffScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn federation_renders_empty() {
    use mockforge_tui::screens::federation::FederationScreen;
    let screen = FederationScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

#[test]
fn behavioral_cloning_renders_empty() {
    use mockforge_tui::screens::behavioral_cloning::BehavioralCloningScreen;
    let screen = BehavioralCloningScreen::new();
    let output = render_screen(&screen);
    assert!(!output.is_empty());
}

// ── Data-loaded render tests ───────────────────────────────────────────
// Verify screens render correctly after receiving sample API data.

#[test]
fn dashboard_renders_with_data() {
    use mockforge_tui::screens::dashboard::DashboardScreen;
    let mut screen = DashboardScreen::new();
    screen.on_data(&sample_dashboard_json());
    let output = render_screen(&screen);
    // Dashboard should show server info and metrics
    assert!(
        output.contains("HTTP") || output.contains("5678") || output.contains("0.3.73"),
        "Dashboard should display server info, metrics, or version"
    );
}

#[test]
fn routes_renders_with_data() {
    use mockforge_tui::screens::routes::RoutesScreen;
    let mut screen = RoutesScreen::new();
    screen.on_data(&sample_routes_json());
    let output = render_screen(&screen);
    assert!(
        output.contains("/api/users") || output.contains("GET"),
        "Routes screen should display route paths or methods"
    );
}

#[test]
fn smoke_tests_renders_with_data() {
    use mockforge_tui::screens::smoke_tests::SmokeTestsScreen;
    let mut screen = SmokeTestsScreen::new();
    screen.on_data(&sample_smoke_tests_json());
    let output = render_screen(&screen);
    assert!(
        output.contains("health") || output.contains("passed") || output.contains("failed"),
        "Smoke tests screen should display test results"
    );
}

#[test]
fn chaos_renders_with_data() {
    use mockforge_tui::screens::chaos::ChaosScreen;
    let mut screen = ChaosScreen::new();
    screen.on_data(&sample_chaos_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "Chaos screen should render content with data");
}

#[test]
fn metrics_renders_with_data() {
    use mockforge_tui::screens::metrics::MetricsScreen;
    let mut screen = MetricsScreen::new();
    screen.on_data(&sample_metrics_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "Metrics screen should render content with data");
}

#[test]
fn health_renders_with_data() {
    use mockforge_tui::screens::health::HealthScreen;
    let mut screen = HealthScreen::new();
    screen.on_data(&sample_health_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "Health screen should render content with data");
}

#[test]
fn config_renders_with_data() {
    use mockforge_tui::screens::config::ConfigScreen;
    let mut screen = ConfigScreen::new();
    screen.on_data(&sample_config_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "Config screen should render content with data");
}

#[test]
fn plugins_renders_with_data() {
    use mockforge_tui::screens::plugins::PluginsScreen;
    let mut screen = PluginsScreen::new();
    screen.on_data(&sample_plugins_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "Plugins screen should render content with data");
}

#[test]
fn fixtures_renders_with_data() {
    use mockforge_tui::screens::fixtures::FixturesScreen;
    let mut screen = FixturesScreen::new();
    screen.on_data(&sample_fixtures_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "Fixtures screen should render content with data");
}

#[test]
fn chains_renders_with_data() {
    use mockforge_tui::screens::chains::ChainsScreen;
    let mut screen = ChainsScreen::new();
    screen.on_data(&sample_chains_json());
    let output = render_screen(&screen);
    assert!(
        output.contains("Login") || output.contains("CRUD"),
        "Chains screen should display chain names"
    );
}

#[test]
fn workspaces_renders_with_data() {
    use mockforge_tui::screens::workspaces::WorkspacesScreen;
    let mut screen = WorkspacesScreen::new();
    screen.on_data(&sample_workspaces_json());
    let output = render_screen(&screen);
    assert!(
        output.contains("Development") || output.contains("ws-1"),
        "Workspaces screen should display workspace info"
    );
}

#[test]
fn federation_renders_with_data() {
    use mockforge_tui::screens::federation::FederationScreen;
    let mut screen = FederationScreen::new();
    screen.on_data(&sample_federation_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "Federation screen should render content with data");
}

#[test]
fn analytics_renders_with_data() {
    use mockforge_tui::screens::analytics::AnalyticsScreen;
    let mut screen = AnalyticsScreen::new();
    screen.on_data(&sample_analytics_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "Analytics screen should render content with data");
}

#[test]
fn audit_renders_with_data() {
    use mockforge_tui::screens::audit::AuditScreen;
    let mut screen = AuditScreen::new();
    screen.on_data(&sample_audit_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "Audit screen should render content with data");
}

#[test]
fn recorder_renders_with_data() {
    use mockforge_tui::screens::recorder::RecorderScreen;
    let mut screen = RecorderScreen::new();
    screen.on_data(&sample_recorder_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "Recorder screen should render content with data");
}

#[test]
fn time_travel_renders_with_data() {
    use mockforge_tui::screens::time_travel::TimeTravelScreen;
    let mut screen = TimeTravelScreen::new();
    screen.on_data(&sample_time_travel_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "Time Travel screen should render content with data");
}

#[test]
fn world_state_renders_with_data() {
    use mockforge_tui::screens::world_state::WorldStateScreen;
    let mut screen = WorldStateScreen::new();
    screen.on_data(&sample_world_state_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "World State screen should render content with data");
}

#[test]
fn contract_diff_renders_with_data() {
    use mockforge_tui::screens::contract_diff::ContractDiffScreen;
    let mut screen = ContractDiffScreen::new();
    screen.on_data(&sample_contract_diff_json());
    let output = render_screen(&screen);
    assert!(
        !output.trim().is_empty(),
        "Contract Diff screen should render content with data"
    );
}

#[test]
fn import_renders_with_data() {
    use mockforge_tui::screens::import::ImportScreen;
    let mut screen = ImportScreen::new();
    screen.on_data(&sample_import_json());
    let output = render_screen(&screen);
    assert!(!output.trim().is_empty(), "Import screen should render content with data");
}

#[test]
fn behavioral_cloning_renders_with_data() {
    use mockforge_tui::screens::behavioral_cloning::BehavioralCloningScreen;
    let mut screen = BehavioralCloningScreen::new();
    screen.on_data(&sample_behavioral_cloning_json());
    let output = render_screen(&screen);
    assert!(
        !output.trim().is_empty(),
        "Behavioral Cloning screen should render content with data"
    );
}

// ── Error-state render tests ───────────────────────────────────────────
// Verify that screens handle and render error states gracefully.

#[test]
fn dashboard_renders_after_error() {
    use mockforge_tui::screens::dashboard::DashboardScreen;
    let mut screen = DashboardScreen::new();
    screen.on_error("Connection refused");
    let output = render_screen(&screen);
    assert!(!output.is_empty(), "Dashboard should render even with an error");
}

#[test]
fn routes_renders_after_parse_error() {
    use mockforge_tui::screens::routes::RoutesScreen;
    let mut screen = RoutesScreen::new();
    screen.on_data("invalid json {{{");
    let output = render_screen(&screen);
    assert!(!output.is_empty(), "Routes should render even after parse error");
}

#[test]
fn dashboard_renders_after_bad_data() {
    use mockforge_tui::screens::dashboard::DashboardScreen;
    let mut screen = DashboardScreen::new();
    screen.on_data("not valid json");
    let output = render_screen(&screen);
    assert!(!output.is_empty(), "Dashboard should render even after bad data");
}

// ── Small terminal render tests ────────────────────────────────────────
// Verify screens don't panic when rendered in a very small terminal.

#[test]
fn dashboard_renders_small_terminal() {
    use mockforge_tui::screens::dashboard::DashboardScreen;
    let mut screen = DashboardScreen::new();
    screen.on_data(&sample_dashboard_json());

    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            screen.render(frame, Rect::new(0, 0, 40, 10));
        })
        .expect("render in small terminal should not panic");
}

#[test]
fn routes_renders_small_terminal() {
    use mockforge_tui::screens::routes::RoutesScreen;
    let mut screen = RoutesScreen::new();
    screen.on_data(&sample_routes_json());

    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            screen.render(frame, Rect::new(0, 0, 40, 10));
        })
        .expect("render in small terminal should not panic");
}

// ── Title tests ────────────────────────────────────────────────────────

#[test]
fn all_screens_have_nonempty_titles() {
    use mockforge_tui::screens::{
        analytics::AnalyticsScreen, audit::AuditScreen,
        behavioral_cloning::BehavioralCloningScreen, chains::ChainsScreen, chaos::ChaosScreen,
        config::ConfigScreen, contract_diff::ContractDiffScreen, dashboard::DashboardScreen,
        federation::FederationScreen, fixtures::FixturesScreen, health::HealthScreen,
        import::ImportScreen, logs::LogsScreen, metrics::MetricsScreen, plugins::PluginsScreen,
        recorder::RecorderScreen, routes::RoutesScreen, smoke_tests::SmokeTestsScreen,
        time_travel::TimeTravelScreen, verification::VerificationScreen,
        workspaces::WorkspacesScreen, world_state::WorldStateScreen,
    };

    let screens: Vec<Box<dyn Screen>> = vec![
        Box::new(DashboardScreen::new()),
        Box::new(LogsScreen::new()),
        Box::new(RoutesScreen::new()),
        Box::new(MetricsScreen::new()),
        Box::new(ConfigScreen::new()),
        Box::new(ChaosScreen::new()),
        Box::new(WorkspacesScreen::new()),
        Box::new(PluginsScreen::new()),
        Box::new(FixturesScreen::new()),
        Box::new(HealthScreen::new()),
        Box::new(SmokeTestsScreen::new()),
        Box::new(TimeTravelScreen::new()),
        Box::new(ChainsScreen::new()),
        Box::new(VerificationScreen::new()),
        Box::new(AnalyticsScreen::new()),
        Box::new(RecorderScreen::new()),
        Box::new(ImportScreen::new()),
        Box::new(AuditScreen::new()),
        Box::new(WorldStateScreen::new()),
        Box::new(ContractDiffScreen::new()),
        Box::new(FederationScreen::new()),
        Box::new(BehavioralCloningScreen::new()),
    ];

    for screen in &screens {
        let title = screen.title();
        assert!(!title.is_empty(), "Screen title should not be empty");
    }

    // Verify we tested all 22 screens
    assert_eq!(screens.len(), 22, "Should test all 22 screens");
}
