//! Standalone deserialization types mirroring the admin API JSON shapes.
//! Intentionally duplicated from `mockforge-ui` for decoupling — this crate
//! has zero internal `mockforge-*` dependencies.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── API envelope ─────────────────────────────────────────────────────

/// Generic API response wrapper used by all admin endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

// ── Dashboard ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub server_info: ServerInfo,
    pub system_info: DashboardSystemInfo,
    pub metrics: SimpleMetrics,
    pub servers: Vec<ServerStatus>,
    pub recent_logs: Vec<RequestLog>,
    pub system: SystemInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub version: String,
    #[serde(default)]
    pub build_time: String,
    #[serde(default)]
    pub git_sha: String,
    pub http_server: Option<String>,
    pub ws_server: Option<String>,
    pub grpc_server: Option<String>,
    pub graphql_server: Option<String>,
    #[serde(default)]
    pub api_enabled: bool,
    #[serde(default)]
    pub admin_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSystemInfo {
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub arch: String,
    #[serde(default)]
    pub uptime: u64,
    #[serde(default)]
    pub memory_usage: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleMetrics {
    #[serde(default)]
    pub total_requests: u64,
    #[serde(default)]
    pub active_requests: u64,
    #[serde(default)]
    pub average_response_time: f64,
    #[serde(default)]
    pub error_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub server_type: String,
    pub address: Option<String>,
    #[serde(default)]
    pub running: bool,
    pub start_time: Option<DateTime<Utc>>,
    pub uptime_seconds: Option<u64>,
    #[serde(default)]
    pub active_connections: u64,
    #[serde(default)]
    pub total_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub uptime_seconds: u64,
    #[serde(default)]
    pub memory_usage_mb: u64,
    #[serde(default)]
    pub cpu_usage_percent: f64,
    #[serde(default)]
    pub active_threads: usize,
    #[serde(default)]
    pub total_routes: usize,
    #[serde(default)]
    pub total_fixtures: usize,
}

// ── Request logs ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    #[serde(default)]
    pub response_time_ms: u64,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub response_size_bytes: u64,
    pub error_message: Option<String>,
}

// ── Routes ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub method: Option<String>,
    pub path: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub has_fixtures: bool,
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub request_count: u64,
    pub last_request: Option<DateTime<Utc>>,
    #[serde(default)]
    pub error_count: u64,
}

// ── Metrics ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsData {
    #[serde(default)]
    pub requests_by_endpoint: HashMap<String, u64>,
    #[serde(default)]
    pub response_time_percentiles: HashMap<String, u64>,
    pub endpoint_percentiles: Option<HashMap<String, HashMap<String, u64>>>,
    pub latency_over_time: Option<Vec<(DateTime<Utc>, u64)>>,
    #[serde(default)]
    pub error_rate_by_endpoint: HashMap<String, f64>,
    #[serde(default)]
    pub memory_usage_over_time: Vec<(DateTime<Utc>, u64)>,
    #[serde(default)]
    pub cpu_usage_over_time: Vec<(DateTime<Utc>, f64)>,
}

// ── Health ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub status: String,
    #[serde(default)]
    pub services: HashMap<String, String>,
    pub last_check: Option<DateTime<Utc>>,
    #[serde(default)]
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthProbe {
    pub status: String,
    #[serde(default)]
    pub checks: HashMap<String, serde_json::Value>,
}

// ── Config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigState {
    pub latency: LatencyConfig,
    pub faults: FaultConfig,
    pub proxy: ProxyConfig,
    pub traffic_shaping: TrafficShapingConfig,
    pub validation: ValidationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub base_ms: u64,
    #[serde(default)]
    pub jitter_ms: u64,
    #[serde(default)]
    pub tag_overrides: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub failure_rate: f64,
    #[serde(default)]
    pub status_codes: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default)]
    pub enabled: bool,
    pub upstream_url: Option<String>,
    #[serde(default)]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficShapingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub bandwidth: serde_json::Value,
    #[serde(default)]
    pub burst_loss: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub aggregate_errors: bool,
    #[serde(default)]
    pub validate_responses: bool,
    #[serde(default)]
    pub overrides: serde_json::Value,
}

// ── Plugins ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub healthy: bool,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
}

// ── Fixtures ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureInfo {
    pub id: String,
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub path: String,
    pub saved_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub file_size: u64,
    #[serde(default)]
    pub file_path: String,
    #[serde(default)]
    pub fingerprint: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

// ── Smoke tests ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmokeTestResult {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub description: String,
    pub last_run: Option<DateTime<Utc>>,
    #[serde(default)]
    pub status: String,
    pub response_time_ms: Option<u64>,
    pub error_message: Option<String>,
    pub status_code: Option<u16>,
    pub duration_seconds: Option<f64>,
}

// ── Workspaces ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub active: bool,
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub environments: Vec<String>,
}

// ── Chaos ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosStatus {
    #[serde(default)]
    pub enabled: bool,
    pub active_scenario: Option<String>,
    #[serde(default)]
    pub settings: serde_json::Value,
}

// ── Time Travel ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTravelStatus {
    #[serde(default)]
    pub enabled: bool,
    pub current_time: Option<DateTime<Utc>>,
    pub time_scale: Option<f64>,
    #[serde(default)]
    pub scheduled_responses: u64,
}

// ── Chains ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainInfo {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub steps: Vec<serde_json::Value>,
    #[serde(default)]
    pub description: String,
}

// ── Audit ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub details: serde_json::Value,
}

// ── Analytics ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    #[serde(default)]
    pub total_requests: u64,
    #[serde(default)]
    pub unique_endpoints: u64,
    #[serde(default)]
    pub error_rate: f64,
    #[serde(default)]
    pub avg_response_time: f64,
    #[serde(default)]
    pub top_endpoints: Vec<EndpointStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointStat {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub count: u64,
    #[serde(default)]
    pub avg_time: f64,
}

// ── Recorder ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecorderStatus {
    #[serde(default)]
    pub recording: bool,
    #[serde(default)]
    pub recorded_count: u64,
}

// ── Verification ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    #[serde(default)]
    pub matched: bool,
    #[serde(default)]
    pub count: u64,
    #[serde(default)]
    pub details: serde_json::Value,
}

// ── World State ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateEntry {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub value: serde_json::Value,
    pub updated_at: Option<DateTime<Utc>>,
}

// ── Federation ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPeer {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub status: String,
    pub last_sync: Option<DateTime<Utc>>,
}

// ── Contract Diff ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDiffCapture {
    pub id: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub diff_status: String,
    pub captured_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_api_response_success() {
        let json = r#"{
            "success": true,
            "data": "hello",
            "error": null
        }"#;
        let resp: ApiResponse<String> = serde_json::from_str(json).unwrap();
        assert!(resp.success);
        assert_eq!(resp.data.unwrap(), "hello");
        assert!(resp.error.is_none());
    }

    #[test]
    fn deserialize_api_response_error() {
        let json = r#"{
            "success": false,
            "data": null,
            "error": "something went wrong"
        }"#;
        let resp: ApiResponse<String> = serde_json::from_str(json).unwrap();
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert_eq!(resp.error.unwrap(), "something went wrong");
    }

    #[test]
    fn deserialize_server_info_minimal() {
        let json = r#"{
            "version": "0.3.31"
        }"#;
        let info: ServerInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.version, "0.3.31");
        assert!(info.build_time.is_empty());
        assert!(info.git_sha.is_empty());
        assert!(info.http_server.is_none());
        assert!(!info.api_enabled);
        assert_eq!(info.admin_port, 0);
    }

    #[test]
    fn deserialize_server_info_full() {
        let json = r#"{
            "version": "0.3.31",
            "build_time": "2025-01-15T10:00:00Z",
            "git_sha": "abc123",
            "http_server": "http://0.0.0.0:3000",
            "ws_server": "ws://0.0.0.0:3001",
            "grpc_server": null,
            "graphql_server": null,
            "api_enabled": true,
            "admin_port": 9080
        }"#;
        let info: ServerInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.version, "0.3.31");
        assert_eq!(info.http_server.unwrap(), "http://0.0.0.0:3000");
        assert!(info.api_enabled);
        assert_eq!(info.admin_port, 9080);
    }

    #[test]
    fn deserialize_request_log() {
        let json = r#"{
            "id": "req-001",
            "timestamp": "2025-06-15T12:00:00Z",
            "method": "GET",
            "path": "/api/users",
            "status_code": 200,
            "response_time_ms": 42,
            "client_ip": "127.0.0.1",
            "user_agent": "curl/8.0",
            "headers": {"content-type": "application/json"},
            "response_size_bytes": 1024,
            "error_message": null
        }"#;
        let log: RequestLog = serde_json::from_str(json).unwrap();
        assert_eq!(log.id, "req-001");
        assert_eq!(log.method, "GET");
        assert_eq!(log.path, "/api/users");
        assert_eq!(log.status_code, 200);
        assert_eq!(log.response_time_ms, 42);
        assert_eq!(log.client_ip.unwrap(), "127.0.0.1");
        assert_eq!(log.headers.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn deserialize_request_log_minimal() {
        let json = r#"{
            "id": "req-002",
            "timestamp": "2025-06-15T12:00:00Z",
            "method": "POST",
            "path": "/api/data",
            "status_code": 500
        }"#;
        let log: RequestLog = serde_json::from_str(json).unwrap();
        assert_eq!(log.id, "req-002");
        assert_eq!(log.status_code, 500);
        assert_eq!(log.response_time_ms, 0);
        assert!(log.client_ip.is_none());
        assert!(log.headers.is_empty());
    }

    #[test]
    fn deserialize_route_info() {
        let json = r#"{
            "method": "GET",
            "path": "/api/users/{id}",
            "priority": 10,
            "has_fixtures": true,
            "latency_ms": 50,
            "request_count": 100,
            "last_request": "2025-06-15T12:30:00Z",
            "error_count": 2
        }"#;
        let route: RouteInfo = serde_json::from_str(json).unwrap();
        assert_eq!(route.method.unwrap(), "GET");
        assert_eq!(route.path, "/api/users/{id}");
        assert_eq!(route.priority, 10);
        assert!(route.has_fixtures);
        assert_eq!(route.latency_ms.unwrap(), 50);
        assert_eq!(route.request_count, 100);
        assert_eq!(route.error_count, 2);
    }

    #[test]
    fn deserialize_route_info_minimal() {
        let json = r#"{
            "path": "/health"
        }"#;
        let route: RouteInfo = serde_json::from_str(json).unwrap();
        assert!(route.method.is_none());
        assert_eq!(route.path, "/health");
        assert_eq!(route.priority, 0);
        assert!(!route.has_fixtures);
        assert!(route.latency_ms.is_none());
    }

    #[test]
    fn deserialize_health_check() {
        let json = r#"{
            "status": "healthy",
            "services": {"http": "up", "grpc": "up"},
            "last_check": "2025-06-15T12:00:00Z",
            "issues": []
        }"#;
        let health: HealthCheck = serde_json::from_str(json).unwrap();
        assert_eq!(health.status, "healthy");
        assert_eq!(health.services.len(), 2);
        assert!(health.issues.is_empty());
    }

    #[test]
    fn deserialize_health_check_with_issues() {
        let json = r#"{
            "status": "degraded",
            "issues": ["kafka disconnected", "high latency"]
        }"#;
        let health: HealthCheck = serde_json::from_str(json).unwrap();
        assert_eq!(health.status, "degraded");
        assert_eq!(health.issues.len(), 2);
        assert_eq!(health.issues[0], "kafka disconnected");
    }

    #[test]
    fn deserialize_plugin_info() {
        let json = r#"{
            "id": "plugin-001",
            "name": "response-graphql",
            "version": "1.0.0",
            "types": ["response", "graphql"],
            "status": "active",
            "healthy": true,
            "description": "GraphQL response plugin",
            "author": "MockForge"
        }"#;
        let plugin: PluginInfo = serde_json::from_str(json).unwrap();
        assert_eq!(plugin.id, "plugin-001");
        assert_eq!(plugin.name, "response-graphql");
        assert!(plugin.healthy);
        assert_eq!(plugin.types.len(), 2);
    }

    #[test]
    fn deserialize_config_state() {
        let json = r#"{
            "latency": {
                "enabled": true,
                "base_ms": 100,
                "jitter_ms": 20,
                "tag_overrides": {"fast": 10}
            },
            "faults": {
                "enabled": false,
                "failure_rate": 0.05,
                "status_codes": [500, 503]
            },
            "proxy": {
                "enabled": false,
                "upstream_url": null,
                "timeout_seconds": 30
            },
            "traffic_shaping": {
                "enabled": false,
                "bandwidth": {},
                "burst_loss": {}
            },
            "validation": {
                "mode": "strict",
                "aggregate_errors": true,
                "validate_responses": false,
                "overrides": {}
            }
        }"#;
        let config: ConfigState = serde_json::from_str(json).unwrap();
        assert!(config.latency.enabled);
        assert_eq!(config.latency.base_ms, 100);
        assert_eq!(config.latency.jitter_ms, 20);
        assert!(!config.faults.enabled);
        assert_eq!(config.faults.status_codes, vec![500, 503]);
        assert!(!config.proxy.enabled);
        assert_eq!(config.validation.mode, "strict");
    }

    #[test]
    fn deserialize_fixture_info() {
        let json = r#"{
            "id": "fix-001",
            "protocol": "http",
            "method": "GET",
            "path": "/api/users",
            "saved_at": "2025-06-15T10:00:00Z",
            "file_size": 2048,
            "file_path": "/fixtures/users.json",
            "fingerprint": "abc123def",
            "metadata": {"tag": "v1"}
        }"#;
        let fixture: FixtureInfo = serde_json::from_str(json).unwrap();
        assert_eq!(fixture.id, "fix-001");
        assert_eq!(fixture.protocol, "http");
        assert_eq!(fixture.file_size, 2048);
    }

    #[test]
    fn deserialize_smoke_test_result() {
        let json = r#"{
            "id": "smoke-001",
            "name": "Health endpoint",
            "method": "GET",
            "path": "/health",
            "description": "Verify health endpoint returns 200",
            "last_run": "2025-06-15T12:00:00Z",
            "status": "passed",
            "response_time_ms": 15,
            "status_code": 200
        }"#;
        let result: SmokeTestResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.id, "smoke-001");
        assert_eq!(result.status, "passed");
        assert_eq!(result.status_code.unwrap(), 200);
    }

    #[test]
    fn deserialize_workspace_info() {
        let json = r#"{
            "id": "ws-001",
            "name": "default",
            "description": "Default workspace",
            "active": true,
            "created_at": "2025-01-01T00:00:00Z",
            "environments": ["dev", "staging"]
        }"#;
        let ws: WorkspaceInfo = serde_json::from_str(json).unwrap();
        assert_eq!(ws.id, "ws-001");
        assert!(ws.active);
        assert_eq!(ws.environments, vec!["dev", "staging"]);
    }

    #[test]
    fn deserialize_chaos_status() {
        let json = r#"{
            "enabled": true,
            "active_scenario": "network-partition",
            "settings": {"probability": 0.1}
        }"#;
        let chaos: ChaosStatus = serde_json::from_str(json).unwrap();
        assert!(chaos.enabled);
        assert_eq!(chaos.active_scenario.unwrap(), "network-partition");
    }

    #[test]
    fn deserialize_time_travel_status() {
        let json = r#"{
            "enabled": true,
            "current_time": "2025-01-01T00:00:00Z",
            "time_scale": 2.0,
            "scheduled_responses": 5
        }"#;
        let tt: TimeTravelStatus = serde_json::from_str(json).unwrap();
        assert!(tt.enabled);
        assert!((tt.time_scale.unwrap() - 2.0).abs() < f64::EPSILON);
        assert_eq!(tt.scheduled_responses, 5);
    }

    #[test]
    fn deserialize_chain_info() {
        let json = r#"{
            "id": "chain-001",
            "name": "User flow",
            "steps": [{"action": "create"}, {"action": "read"}],
            "description": "End-to-end user CRUD"
        }"#;
        let chain: ChainInfo = serde_json::from_str(json).unwrap();
        assert_eq!(chain.id, "chain-001");
        assert_eq!(chain.steps.len(), 2);
    }

    #[test]
    fn deserialize_audit_entry() {
        let json = r#"{
            "id": "audit-001",
            "timestamp": "2025-06-15T12:00:00Z",
            "action": "config.update",
            "user": "admin",
            "details": {"field": "latency_ms", "old": 50, "new": 100}
        }"#;
        let entry: AuditEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.id, "audit-001");
        assert_eq!(entry.action, "config.update");
        assert_eq!(entry.user, "admin");
    }

    #[test]
    fn deserialize_analytics_summary() {
        let json = r#"{
            "total_requests": 10000,
            "unique_endpoints": 25,
            "error_rate": 0.02,
            "avg_response_time": 45.5,
            "top_endpoints": [
                {"endpoint": "/api/users", "count": 5000, "avg_time": 30.0},
                {"endpoint": "/api/orders", "count": 3000, "avg_time": 50.0}
            ]
        }"#;
        let summary: AnalyticsSummary = serde_json::from_str(json).unwrap();
        assert_eq!(summary.total_requests, 10000);
        assert_eq!(summary.unique_endpoints, 25);
        assert_eq!(summary.top_endpoints.len(), 2);
        assert_eq!(summary.top_endpoints[0].count, 5000);
    }

    #[test]
    fn deserialize_recorder_status() {
        let json = r#"{
            "recording": true,
            "recorded_count": 42
        }"#;
        let recorder: RecorderStatus = serde_json::from_str(json).unwrap();
        assert!(recorder.recording);
        assert_eq!(recorder.recorded_count, 42);
    }

    #[test]
    fn deserialize_verification_result() {
        let json = r#"{
            "matched": true,
            "count": 3,
            "details": {"methods": ["GET", "POST"]}
        }"#;
        let result: VerificationResult = serde_json::from_str(json).unwrap();
        assert!(result.matched);
        assert_eq!(result.count, 3);
    }

    #[test]
    fn deserialize_world_state_entry() {
        let json = r#"{
            "key": "user.count",
            "value": 42,
            "updated_at": "2025-06-15T12:00:00Z"
        }"#;
        let entry: WorldStateEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.key, "user.count");
        assert_eq!(entry.value, serde_json::json!(42));
    }

    #[test]
    fn deserialize_federation_peer() {
        let json = r#"{
            "id": "peer-001",
            "url": "http://peer1:9080",
            "status": "connected",
            "last_sync": "2025-06-15T12:00:00Z"
        }"#;
        let peer: FederationPeer = serde_json::from_str(json).unwrap();
        assert_eq!(peer.id, "peer-001");
        assert_eq!(peer.url, "http://peer1:9080");
        assert_eq!(peer.status, "connected");
    }

    #[test]
    fn deserialize_contract_diff_capture() {
        let json = r#"{
            "id": "diff-001",
            "path": "/api/users",
            "method": "GET",
            "diff_status": "changed",
            "captured_at": "2025-06-15T12:00:00Z"
        }"#;
        let capture: ContractDiffCapture = serde_json::from_str(json).unwrap();
        assert_eq!(capture.id, "diff-001");
        assert_eq!(capture.diff_status, "changed");
    }

    #[test]
    fn deserialize_metrics_data() {
        let json = r#"{
            "requests_by_endpoint": {"/api/users": 100, "/api/orders": 50},
            "response_time_percentiles": {"p50": 20, "p99": 200},
            "error_rate_by_endpoint": {"/api/users": 0.01}
        }"#;
        let metrics: MetricsData = serde_json::from_str(json).unwrap();
        assert_eq!(metrics.requests_by_endpoint.len(), 2);
        assert_eq!(*metrics.requests_by_endpoint.get("/api/users").unwrap(), 100);
        assert_eq!(metrics.response_time_percentiles.len(), 2);
    }

    #[test]
    fn deserialize_system_info() {
        let json = r#"{
            "version": "0.3.31",
            "uptime_seconds": 3600,
            "memory_usage_mb": 128,
            "cpu_usage_percent": 15.5,
            "active_threads": 8,
            "total_routes": 42,
            "total_fixtures": 10
        }"#;
        let sys: SystemInfo = serde_json::from_str(json).unwrap();
        assert_eq!(sys.version, "0.3.31");
        assert_eq!(sys.uptime_seconds, 3600);
        assert_eq!(sys.total_routes, 42);
    }

    #[test]
    fn deserialize_health_probe() {
        let json = r#"{
            "status": "ok",
            "checks": {"db": true, "redis": "connected"}
        }"#;
        let probe: HealthProbe = serde_json::from_str(json).unwrap();
        assert_eq!(probe.status, "ok");
        assert_eq!(probe.checks.len(), 2);
    }

    #[test]
    fn roundtrip_serialize_deserialize() {
        let original = RecorderStatus {
            recording: true,
            recorded_count: 99,
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: RecorderStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.recording, original.recording);
        assert_eq!(deserialized.recorded_count, original.recorded_count);
    }

    #[test]
    fn api_response_with_complex_data() {
        let json = r#"{
            "success": true,
            "data": [
                {"method": "GET", "path": "/users/{id}", "priority": 1, "has_fixtures": false, "request_count": 0, "error_count": 0},
                {"method": "POST", "path": "/users", "priority": 2, "has_fixtures": true, "request_count": 5, "error_count": 1}
            ],
            "error": null
        }"#;
        let resp: ApiResponse<Vec<RouteInfo>> = serde_json::from_str(json).unwrap();
        assert!(resp.success);
        let routes = resp.data.unwrap();
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].path, "/users/{id}");
        assert_eq!(routes[1].request_count, 5);
    }
}
