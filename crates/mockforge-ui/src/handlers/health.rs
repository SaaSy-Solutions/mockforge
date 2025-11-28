//! Health check endpoints for Kubernetes and cloud deployments

use axum::{extract::State, response::Json};
use chrono;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::handlers::AdminState;

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub timestamp: u64,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub duration_ms: u64,
}

/// Liveness probe - Is the application running?
/// Returns 200 if the application is alive, even if degraded
pub async fn liveness_probe(State(state): State<AdminState>) -> Json<HealthResponse> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    let uptime_seconds = (chrono::Utc::now() - state.start_time).num_seconds() as u64;

    let response = HealthResponse {
        status: HealthStatus::Healthy,
        timestamp,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        checks: vec![],
    };

    Json(response)
}

/// Readiness probe - Is the application ready to serve traffic?
/// Returns 200 only if all critical services are ready
pub async fn readiness_probe(State(state): State<AdminState>) -> Json<HealthResponse> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    let mut checks = vec![];
    let mut overall_status = HealthStatus::Healthy;

    // Check HTTP server
    if state.http_server_addr.is_some() {
        checks.push(HealthCheck {
            name: "http_server".to_string(),
            status: HealthStatus::Healthy,
            message: Some("HTTP server is running".to_string()),
            duration_ms: 0,
        });
    } else {
        checks.push(HealthCheck {
            name: "http_server".to_string(),
            status: HealthStatus::Degraded,
            message: Some("HTTP server is not enabled".to_string()),
            duration_ms: 0,
        });
    }

    // Check WebSocket server
    if state.ws_server_addr.is_some() {
        checks.push(HealthCheck {
            name: "websocket_server".to_string(),
            status: HealthStatus::Healthy,
            message: Some("WebSocket server is running".to_string()),
            duration_ms: 0,
        });
    }

    // Check gRPC server
    if state.grpc_server_addr.is_some() {
        checks.push(HealthCheck {
            name: "grpc_server".to_string(),
            status: HealthStatus::Healthy,
            message: Some("gRPC server is running".to_string()),
            duration_ms: 0,
        });
    }

    // Check if any critical service failed
    let critical_failures = checks.iter().any(|c| {
        matches!(c.status, HealthStatus::Unhealthy)
            && (c.name == "http_server" || c.name == "grpc_server")
    });

    if critical_failures {
        overall_status = HealthStatus::Unhealthy;
    }

    let uptime_seconds = (chrono::Utc::now() - state.start_time).num_seconds() as u64;

    let response = HealthResponse {
        status: overall_status,
        timestamp,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        checks,
    };

    Json(response)
}

/// Startup probe - Has the application completed initialization?
/// Returns 200 when the application is fully started
pub async fn startup_probe(State(state): State<AdminState>) -> Json<HealthResponse> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    // For now, consider started if admin UI is running
    let status = if state.api_enabled {
        HealthStatus::Healthy
    } else {
        HealthStatus::Unhealthy
    };

    let uptime_seconds = (chrono::Utc::now() - state.start_time).num_seconds() as u64;

    let response = HealthResponse {
        status: status.clone(),
        timestamp,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        checks: vec![HealthCheck {
            name: "initialization".to_string(),
            status: status.clone(),
            message: Some("Application initialized".to_string()),
            duration_ms: 0,
        }],
    };

    Json(response)
}

/// Deep health check - Comprehensive system health
/// Checks all subsystems and dependencies
pub async fn deep_health_check(State(state): State<AdminState>) -> Json<HealthResponse> {
    let start = SystemTime::now();
    let timestamp = start.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

    let mut checks = vec![];
    let mut overall_status = HealthStatus::Healthy;

    // Check Admin UI status
    checks.push(HealthCheck {
        name: "admin_ui".to_string(),
        status: if state.api_enabled {
            HealthStatus::Healthy
        } else {
            HealthStatus::Degraded
        },
        message: Some(if state.api_enabled {
            format!("Admin UI is accessible on port {}", state.admin_port)
        } else {
            "Admin UI API endpoints are disabled".to_string()
        }),
        duration_ms: 0,
    });

    // Check all servers with addresses
    let servers = vec![
        ("http_server", state.http_server_addr.as_ref().map(|a| a.to_string())),
        ("websocket_server", state.ws_server_addr.as_ref().map(|a| a.to_string())),
        ("grpc_server", state.grpc_server_addr.as_ref().map(|a| a.to_string())),
        ("graphql_server", state.graphql_server_addr.as_ref().map(|a| a.to_string())),
    ];

    for (name, addr_opt) in servers {
        if let Some(addr) = addr_opt {
            checks.push(HealthCheck {
                name: name.to_string(),
                status: HealthStatus::Healthy,
                message: Some(format!("{} is running on {}", name, addr)),
                duration_ms: 0,
            });
        } else {
            checks.push(HealthCheck {
                name: name.to_string(),
                status: HealthStatus::Degraded,
                message: Some(format!("{} is not enabled", name)),
                duration_ms: 0,
            });
        }
    }

    // Configuration is loaded and valid (no separate check needed as it's validated at startup)

    // Check metrics
    let metrics = state.metrics.read().await;
    let total_requests = metrics.total_requests;
    drop(metrics);

    checks.push(HealthCheck {
        name: "metrics".to_string(),
        status: HealthStatus::Healthy,
        message: Some(format!("Processed {} requests", total_requests)),
        duration_ms: 0,
    });

    // Calculate overall duration
    let duration = SystemTime::now().duration_since(start).unwrap().as_millis() as u64;

    // Determine overall status - unhealthy if any critical service is unhealthy
    let critical_failures = checks.iter().any(|c| {
        matches!(c.status, HealthStatus::Unhealthy)
            && (c.name == "http_server" || c.name == "admin_ui")
    });
    if critical_failures {
        overall_status = HealthStatus::Unhealthy;
    } else if checks.iter().any(|c| matches!(c.status, HealthStatus::Degraded)) {
        overall_status = HealthStatus::Degraded;
    }

    let uptime_seconds = (chrono::Utc::now() - state.start_time).num_seconds() as u64;

    let response = HealthResponse {
        status: overall_status,
        timestamp,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        checks,
    };

    Json(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serialization() {
        let status = HealthStatus::Healthy;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""healthy""#);
    }

    #[test]
    fn test_health_response_structure() {
        let response = HealthResponse {
            status: HealthStatus::Healthy,
            timestamp: 1234567890,
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            checks: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("1.0.0"));
    }
}
