//! Request handlers for the admin UI

use axum::{
    extract::{Query, State},
    response::{Html, Json},
};
use serde_json::json;
use std::collections::HashMap;

use crate::models::*;

/// Shared state for the admin UI
#[derive(Clone)]
pub struct AdminState {
    /// HTTP server address
    pub http_server_addr: Option<std::net::SocketAddr>,
    /// WebSocket server address
    pub ws_server_addr: Option<std::net::SocketAddr>,
    /// gRPC server address
    pub grpc_server_addr: Option<std::net::SocketAddr>,
    /// Start time
    pub start_time: chrono::DateTime<chrono::Utc>,
}

impl AdminState {
    /// Create new admin state
    pub fn new(
        http_server_addr: Option<std::net::SocketAddr>,
        ws_server_addr: Option<std::net::SocketAddr>,
        grpc_server_addr: Option<std::net::SocketAddr>,
    ) -> Self {
        Self {
            http_server_addr,
            ws_server_addr,
            grpc_server_addr,
            start_time: chrono::Utc::now(),
        }
    }
}

/// Serve the main admin interface
pub async fn serve_admin_html() -> Html<&'static str> {
    Html(crate::get_admin_html())
}

/// Serve admin CSS
pub async fn serve_admin_css() -> &'static str {
    crate::get_admin_css()
}

/// Serve admin JavaScript
pub async fn serve_admin_js() -> &'static str {
    crate::get_admin_js()
}

/// Get dashboard data
pub async fn get_dashboard(State(state): State<AdminState>) -> Json<ApiResponse<DashboardData>> {
    let uptime = (chrono::Utc::now() - state.start_time).num_seconds() as u64;

    // Mock data for demonstration - in real implementation, this would collect
    // actual metrics from the running servers
    let system_info = SystemInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        memory_usage_mb: 45,
        cpu_usage_percent: 2.3,
        active_threads: 8,
        total_routes: 12,
        total_fixtures: 5,
    };

    let servers = vec![
        ServerStatus {
            server_type: "HTTP".to_string(),
            address: state.http_server_addr.map(|addr| addr.to_string()),
            running: state.http_server_addr.is_some(),
            start_time: Some(state.start_time),
            uptime_seconds: Some(uptime),
            active_connections: 3,
            total_requests: 156,
        },
        ServerStatus {
            server_type: "WebSocket".to_string(),
            address: state.ws_server_addr.map(|addr| addr.to_string()),
            running: state.ws_server_addr.is_some(),
            start_time: Some(state.start_time),
            uptime_seconds: Some(uptime),
            active_connections: 2,
            total_requests: 89,
        },
        ServerStatus {
            server_type: "gRPC".to_string(),
            address: state.grpc_server_addr.map(|addr| addr.to_string()),
            running: state.grpc_server_addr.is_some(),
            start_time: Some(state.start_time),
            uptime_seconds: Some(uptime),
            active_connections: 1,
            total_requests: 34,
        },
    ];

    let routes = vec![
        RouteInfo {
            method: Some("GET".to_string()),
            path: "/api/users".to_string(),
            priority: 0,
            has_fixtures: true,
            latency_ms: Some(50),
            request_count: 45,
            last_request: Some(chrono::Utc::now() - chrono::Duration::minutes(5)),
            error_count: 0,
        },
        RouteInfo {
            method: Some("POST".to_string()),
            path: "/api/users".to_string(),
            priority: 0,
            has_fixtures: false,
            latency_ms: Some(75),
            request_count: 12,
            last_request: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            error_count: 1,
        },
    ];

    let recent_logs = vec![RequestLog {
        id: "req_123".to_string(),
        timestamp: chrono::Utc::now() - chrono::Duration::minutes(2),
        method: "GET".to_string(),
        path: "/api/users/123".to_string(),
        status_code: 200,
        response_time_ms: 45,
        client_ip: Some("127.0.0.1".to_string()),
        user_agent: Some("Mozilla/5.0".to_string()),
        headers: HashMap::from([
            ("accept".to_string(), "application/json".to_string()),
            ("authorization".to_string(), "Bearer token123".to_string()),
        ]),
        response_size_bytes: 1024,
        error_message: None,
    }];

    let dashboard = DashboardData {
        system: system_info,
        servers,
        routes,
        recent_logs,
        latency_profile: LatencyProfile {
            name: "default".to_string(),
            base_ms: 50,
            jitter_ms: 20,
            tag_overrides: HashMap::from([
                ("auth".to_string(), 100),
                ("analytics".to_string(), 200),
            ]),
        },
        fault_config: FaultConfig {
            enabled: false,
            failure_rate: 0.0,
            status_codes: vec![500, 502, 503],
            active_failures: 0,
        },
        proxy_config: ProxyConfig {
            enabled: false,
            upstream_url: Some("http://api.example.com".to_string()),
            timeout_seconds: 30,
            requests_proxied: 0,
        },
    };

    Json(ApiResponse::success(dashboard))
}

/// Get health check status
pub async fn get_health() -> Json<HealthCheck> {
    Json(
        HealthCheck::healthy()
            .with_service("http".to_string(), "healthy".to_string())
            .with_service("websocket".to_string(), "healthy".to_string())
            .with_service("grpc".to_string(), "healthy".to_string()),
    )
}

/// Get request logs with optional filtering
pub async fn get_logs(
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<RequestLog>>> {
    let mut filter = LogFilter::default();

    if let Some(method) = params.get("method") {
        filter.method = Some(method.clone());
    }
    if let Some(path) = params.get("path") {
        filter.path_pattern = Some(path.clone());
    }
    if let Some(status) = params.get("status").and_then(|s| s.parse().ok()) {
        filter.status_code = Some(status);
    }
    if let Some(limit) = params.get("limit").and_then(|s| s.parse().ok()) {
        filter.limit = Some(limit);
    }

    // Mock logs for demonstration
    let logs = vec![
        RequestLog {
            id: "req_001".to_string(),
            timestamp: chrono::Utc::now() - chrono::Duration::minutes(1),
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            status_code: 200,
            response_time_ms: 45,
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("curl/7.68.0".to_string()),
            headers: HashMap::new(),
            response_size_bytes: 2048,
            error_message: None,
        },
        RequestLog {
            id: "req_002".to_string(),
            timestamp: chrono::Utc::now() - chrono::Duration::minutes(5),
            method: "POST".to_string(),
            path: "/api/users".to_string(),
            status_code: 201,
            response_time_ms: 120,
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("PostmanRuntime/7.28.4".to_string()),
            headers: HashMap::new(),
            response_size_bytes: 512,
            error_message: None,
        },
    ];

    Json(ApiResponse::success(logs))
}

/// Get metrics data
pub async fn get_metrics() -> Json<ApiResponse<MetricsData>> {
    let metrics = MetricsData {
        requests_by_endpoint: HashMap::from([
            ("/api/users".to_string(), 156),
            ("/api/products".to_string(), 89),
            ("/api/orders".to_string(), 34),
        ]),
        response_time_percentiles: HashMap::from([
            ("p50".to_string(), 45),
            ("p95".to_string(), 120),
            ("p99".to_string(), 250),
        ]),
        error_rate_by_endpoint: HashMap::from([
            ("/api/users".to_string(), 0.02),
            ("/api/products".to_string(), 0.01),
            ("/api/orders".to_string(), 0.0),
        ]),
        memory_usage_over_time: vec![
            (chrono::Utc::now() - chrono::Duration::minutes(10), 40),
            (chrono::Utc::now() - chrono::Duration::minutes(5), 45),
            (chrono::Utc::now(), 42),
        ],
        cpu_usage_over_time: vec![
            (chrono::Utc::now() - chrono::Duration::minutes(10), 1.5),
            (chrono::Utc::now() - chrono::Duration::minutes(5), 2.3),
            (chrono::Utc::now(), 1.8),
        ],
    };

    Json(ApiResponse::success(metrics))
}

/// Update latency profile
pub async fn update_latency(Json(update): Json<ConfigUpdate>) -> Json<ApiResponse<String>> {
    if update.config_type != "latency" {
        return Json(ApiResponse::error("Invalid config type".to_string()));
    }

    // In a real implementation, this would update the actual latency configuration
    tracing::info!("Updating latency profile: {:?}", update.data);

    Json(ApiResponse::success("Latency profile updated".to_string()))
}

/// Update fault injection configuration
pub async fn update_faults(Json(update): Json<ConfigUpdate>) -> Json<ApiResponse<String>> {
    if update.config_type != "faults" {
        return Json(ApiResponse::error("Invalid config type".to_string()));
    }

    // In a real implementation, this would update the actual fault configuration
    tracing::info!("Updating fault configuration: {:?}", update.data);

    Json(ApiResponse::success("Fault configuration updated".to_string()))
}

/// Update proxy configuration
pub async fn update_proxy(Json(update): Json<ConfigUpdate>) -> Json<ApiResponse<String>> {
    if update.config_type != "proxy" {
        return Json(ApiResponse::error("Invalid config type".to_string()));
    }

    // In a real implementation, this would update the actual proxy configuration
    tracing::info!("Updating proxy configuration: {:?}", update.data);

    Json(ApiResponse::success("Proxy configuration updated".to_string()))
}

/// Clear request logs
pub async fn clear_logs() -> Json<ApiResponse<String>> {
    // In a real implementation, this would clear the actual logs
    tracing::info!("Clearing request logs");

    Json(ApiResponse::success("Logs cleared".to_string()))
}

/// Restart servers
pub async fn restart_servers() -> Json<ApiResponse<String>> {
    // In a real implementation, this would restart the actual servers
    tracing::warn!("Server restart not implemented in demo mode");

    Json(ApiResponse::success("Servers restarted (demo)".to_string()))
}

/// Get server configuration
pub async fn get_config() -> Json<ApiResponse<serde_json::Value>> {
    let config = json!({
        "latency": {
            "enabled": true,
            "base_ms": 50,
            "jitter_ms": 20
        },
        "faults": {
            "enabled": false,
            "failure_rate": 0.0
        },
        "proxy": {
            "enabled": false,
            "upstream_url": "http://api.example.com"
        },
        "logging": {
            "level": "info",
            "max_logs": 1000
        }
    });

    Json(ApiResponse::success(config))
}

/// Get fixtures/replay data (compatibility with existing React admin UI)
pub async fn get_fixtures() -> Json<ApiResponse<Vec<serde_json::Value>>> {
    // Mock fixtures data for demonstration
    // In a real implementation, this would read from the fixtures directory
    let fixtures = vec![
        json!({
            "protocol": "http",
            "operation_id": "get_users",
            "saved_at": "2024-01-15T10:30:00Z",
            "path": "/api/users",
            "method": "GET"
        }),
        json!({
            "protocol": "http",
            "operation_id": "create_user",
            "saved_at": "2024-01-15T10:35:00Z",
            "path": "/api/users",
            "method": "POST"
        }),
        json!({
            "protocol": "websocket",
            "operation_id": "ws_chat",
            "saved_at": "2024-01-15T11:00:00Z",
            "path": "/ws",
            "method": "WS"
        }),
    ];

    Json(ApiResponse::success(fixtures))
}
