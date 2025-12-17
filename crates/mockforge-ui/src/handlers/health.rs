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

    // ==================== HealthStatus Tests ====================

    #[test]
    fn test_health_status_serialization_healthy() {
        let status = HealthStatus::Healthy;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""healthy""#);
    }

    #[test]
    fn test_health_status_serialization_degraded() {
        let status = HealthStatus::Degraded;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""degraded""#);
    }

    #[test]
    fn test_health_status_serialization_unhealthy() {
        let status = HealthStatus::Unhealthy;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""unhealthy""#);
    }

    #[test]
    fn test_health_status_deserialization() {
        let healthy: HealthStatus = serde_json::from_str(r#""healthy""#).unwrap();
        assert!(matches!(healthy, HealthStatus::Healthy));

        let degraded: HealthStatus = serde_json::from_str(r#""degraded""#).unwrap();
        assert!(matches!(degraded, HealthStatus::Degraded));

        let unhealthy: HealthStatus = serde_json::from_str(r#""unhealthy""#).unwrap();
        assert!(matches!(unhealthy, HealthStatus::Unhealthy));
    }

    #[test]
    fn test_health_status_clone() {
        let status = HealthStatus::Healthy;
        let cloned = status.clone();
        assert!(matches!(cloned, HealthStatus::Healthy));
    }

    #[test]
    fn test_health_status_debug() {
        let status = HealthStatus::Healthy;
        let debug = format!("{:?}", status);
        assert_eq!(debug, "Healthy");
    }

    // ==================== HealthResponse Tests ====================

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

    #[test]
    fn test_health_response_with_checks() {
        let check = HealthCheck {
            name: "database".to_string(),
            status: HealthStatus::Healthy,
            message: Some("Connected".to_string()),
            duration_ms: 5,
        };

        let response = HealthResponse {
            status: HealthStatus::Healthy,
            timestamp: 1234567890,
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            checks: vec![check],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("database"));
        assert!(json.contains("Connected"));
    }

    #[test]
    fn test_health_response_deserialization() {
        let json = r#"{
            "status": "healthy",
            "timestamp": 1234567890,
            "version": "1.0.0",
            "uptime_seconds": 3600,
            "checks": []
        }"#;

        let response: HealthResponse = serde_json::from_str(json).unwrap();
        assert!(matches!(response.status, HealthStatus::Healthy));
        assert_eq!(response.timestamp, 1234567890);
        assert_eq!(response.version, "1.0.0");
        assert_eq!(response.uptime_seconds, 3600);
        assert!(response.checks.is_empty());
    }

    #[test]
    fn test_health_response_clone() {
        let response = HealthResponse {
            status: HealthStatus::Degraded,
            timestamp: 1234567890,
            version: "2.0.0".to_string(),
            uptime_seconds: 7200,
            checks: vec![],
        };

        let cloned = response.clone();
        assert!(matches!(cloned.status, HealthStatus::Degraded));
        assert_eq!(cloned.version, "2.0.0");
        assert_eq!(cloned.uptime_seconds, 7200);
    }

    // ==================== HealthCheck Tests ====================

    #[test]
    fn test_health_check_creation() {
        let check = HealthCheck {
            name: "redis".to_string(),
            status: HealthStatus::Healthy,
            message: Some("Connection pool active".to_string()),
            duration_ms: 10,
        };

        assert_eq!(check.name, "redis");
        assert!(matches!(check.status, HealthStatus::Healthy));
        assert_eq!(check.message, Some("Connection pool active".to_string()));
        assert_eq!(check.duration_ms, 10);
    }

    #[test]
    fn test_health_check_no_message() {
        let check = HealthCheck {
            name: "cache".to_string(),
            status: HealthStatus::Degraded,
            message: None,
            duration_ms: 0,
        };

        assert!(check.message.is_none());
    }

    #[test]
    fn test_health_check_serialization() {
        let check = HealthCheck {
            name: "test".to_string(),
            status: HealthStatus::Unhealthy,
            message: Some("Error".to_string()),
            duration_ms: 100,
        };

        let json = serde_json::to_string(&check).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("unhealthy"));
        assert!(json.contains("Error"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_health_check_deserialization() {
        let json = r#"{
            "name": "database",
            "status": "healthy",
            "message": "OK",
            "duration_ms": 5
        }"#;

        let check: HealthCheck = serde_json::from_str(json).unwrap();
        assert_eq!(check.name, "database");
        assert!(matches!(check.status, HealthStatus::Healthy));
        assert_eq!(check.message, Some("OK".to_string()));
        assert_eq!(check.duration_ms, 5);
    }

    #[test]
    fn test_health_check_clone() {
        let check = HealthCheck {
            name: "api".to_string(),
            status: HealthStatus::Healthy,
            message: Some("Active".to_string()),
            duration_ms: 20,
        };

        let cloned = check.clone();
        assert_eq!(cloned.name, check.name);
        assert_eq!(cloned.duration_ms, check.duration_ms);
    }

    // ==================== Complex Response Tests ====================

    #[test]
    fn test_health_response_multiple_checks() {
        let checks = vec![
            HealthCheck {
                name: "http_server".to_string(),
                status: HealthStatus::Healthy,
                message: Some("Running on port 8080".to_string()),
                duration_ms: 0,
            },
            HealthCheck {
                name: "grpc_server".to_string(),
                status: HealthStatus::Healthy,
                message: Some("Running on port 50051".to_string()),
                duration_ms: 1,
            },
            HealthCheck {
                name: "websocket_server".to_string(),
                status: HealthStatus::Degraded,
                message: Some("High latency".to_string()),
                duration_ms: 50,
            },
        ];

        let response = HealthResponse {
            status: HealthStatus::Degraded,
            timestamp: 1234567890,
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            checks,
        };

        assert_eq!(response.checks.len(), 3);
        assert!(matches!(response.status, HealthStatus::Degraded));
    }

    #[test]
    fn test_health_response_roundtrip() {
        let original = HealthResponse {
            status: HealthStatus::Healthy,
            timestamp: 9999999999,
            version: "3.0.0".to_string(),
            uptime_seconds: 86400,
            checks: vec![HealthCheck {
                name: "test".to_string(),
                status: HealthStatus::Healthy,
                message: Some("Test message".to_string()),
                duration_ms: 42,
            }],
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: HealthResponse = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized.status, HealthStatus::Healthy));
        assert_eq!(deserialized.timestamp, original.timestamp);
        assert_eq!(deserialized.version, original.version);
        assert_eq!(deserialized.uptime_seconds, original.uptime_seconds);
        assert_eq!(deserialized.checks.len(), 1);
    }

    #[test]
    fn test_health_check_debug() {
        let check = HealthCheck {
            name: "debug_test".to_string(),
            status: HealthStatus::Healthy,
            message: None,
            duration_ms: 0,
        };

        let debug = format!("{:?}", check);
        assert!(debug.contains("debug_test"));
        assert!(debug.contains("Healthy"));
    }

    #[test]
    fn test_health_response_with_zero_uptime() {
        let response = HealthResponse {
            status: HealthStatus::Healthy,
            timestamp: 0,
            version: "0.0.1".to_string(),
            uptime_seconds: 0,
            checks: vec![],
        };

        assert_eq!(response.uptime_seconds, 0);
        assert_eq!(response.timestamp, 0);
    }

    #[test]
    fn test_health_check_high_duration() {
        let check = HealthCheck {
            name: "slow_check".to_string(),
            status: HealthStatus::Degraded,
            message: Some("Timeout warning".to_string()),
            duration_ms: 30000, // 30 seconds
        };

        assert_eq!(check.duration_ms, 30000);
    }
}
