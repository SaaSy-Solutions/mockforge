//! Health check endpoints
//!
//! Provides health check patterns:
//! - `/health` - Legacy endpoint, basic health check
//! - `/health/live` - Liveness probe, confirms the service is running
//! - `/health/ready` - Readiness probe, confirms all dependencies are healthy
//! - `/health/circuits` - Circuit breaker status for external services

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::AppState;

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ComponentStatus {
    Healthy,
    Unhealthy,
    Degraded,
}

/// Individual component health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: ComponentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

/// Overall health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: ComponentStatus,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<std::collections::HashMap<String, ComponentHealth>>,
}

/// Legacy health check endpoint - returns basic status
/// GET /health
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Liveness probe - confirms the service is running
/// Used by Kubernetes/container orchestrators to determine if the container should be restarted
/// GET /health/live
pub async fn liveness_check() -> (StatusCode, Json<HealthResponse>) {
    let response = HealthResponse {
        status: ComponentStatus::Healthy,
        version: env!("CARGO_PKG_VERSION").to_string(),
        components: None,
    };
    (StatusCode::OK, Json(response))
}

/// Readiness probe - confirms the service is ready to accept traffic
/// Checks all critical dependencies (database, Redis, S3, etc.)
/// GET /health/ready
pub async fn readiness_check(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let mut components = std::collections::HashMap::new();
    let mut overall_status = ComponentStatus::Healthy;

    // Check database connectivity
    let db_health = check_database(&state).await;
    if db_health.status == ComponentStatus::Unhealthy {
        overall_status = ComponentStatus::Unhealthy;
    }
    components.insert("database".to_string(), db_health);

    // Check Redis connectivity (if configured)
    let redis_health = check_redis(&state).await;
    match redis_health.status {
        ComponentStatus::Unhealthy => {
            // Redis being down when 2FA is enabled is critical
            if state.config.two_factor_enabled.unwrap_or(false) {
                overall_status = ComponentStatus::Unhealthy;
            } else if overall_status == ComponentStatus::Healthy {
                overall_status = ComponentStatus::Degraded;
            }
        }
        ComponentStatus::Degraded => {
            if overall_status == ComponentStatus::Healthy {
                overall_status = ComponentStatus::Degraded;
            }
        }
        ComponentStatus::Healthy => {}
    }
    components.insert("redis".to_string(), redis_health);

    // Check S3/storage connectivity
    let storage_health = check_storage(&state).await;
    if storage_health.status == ComponentStatus::Unhealthy {
        // Storage being down is critical for plugin operations
        overall_status = ComponentStatus::Unhealthy;
    } else if storage_health.status == ComponentStatus::Degraded
        && overall_status == ComponentStatus::Healthy
    {
        overall_status = ComponentStatus::Degraded;
    }
    components.insert("storage".to_string(), storage_health);

    // Check email configuration (non-critical)
    let email_health = check_email_config();
    if email_health.status == ComponentStatus::Degraded
        && overall_status == ComponentStatus::Healthy
    {
        // Email config issues are degraded, not unhealthy
        overall_status = ComponentStatus::Degraded;
    }
    components.insert("email".to_string(), email_health);

    let response = HealthResponse {
        status: overall_status.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        components: Some(components),
    };

    let status_code = match overall_status {
        ComponentStatus::Healthy => StatusCode::OK,
        ComponentStatus::Degraded => StatusCode::OK,
        ComponentStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(response))
}

/// Check database connectivity by executing a simple query
async fn check_database(state: &AppState) -> ComponentHealth {
    let start = std::time::Instant::now();

    // Execute a simple query to verify database connectivity
    let result: Result<(i32,), sqlx::Error> =
        sqlx::query_as("SELECT 1").fetch_one(state.db.pool()).await;

    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(_) => ComponentHealth {
            status: ComponentStatus::Healthy,
            message: None,
            latency_ms: Some(latency_ms),
        },
        Err(e) => {
            tracing::error!(error = %e, "Database health check failed");
            ComponentHealth {
                status: ComponentStatus::Unhealthy,
                message: Some("Database connection failed".to_string()),
                latency_ms: Some(latency_ms),
            }
        }
    }
}

/// Check Redis connectivity
async fn check_redis(state: &AppState) -> ComponentHealth {
    match &state.redis {
        Some(redis) => {
            let start = std::time::Instant::now();
            let result = redis.ping().await;
            let latency_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(_) => ComponentHealth {
                    status: ComponentStatus::Healthy,
                    message: None,
                    latency_ms: Some(latency_ms),
                },
                Err(e) => {
                    tracing::error!(error = %e, "Redis health check failed");
                    ComponentHealth {
                        status: ComponentStatus::Unhealthy,
                        message: Some("Redis connection failed".to_string()),
                        latency_ms: Some(latency_ms),
                    }
                }
            }
        }
        None => ComponentHealth {
            status: ComponentStatus::Degraded,
            message: Some("Redis not configured".to_string()),
            latency_ms: None,
        },
    }
}

/// Check S3/storage connectivity
async fn check_storage(state: &AppState) -> ComponentHealth {
    let start = std::time::Instant::now();
    let result = state.storage.health_check().await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(_) => ComponentHealth {
            status: ComponentStatus::Healthy,
            message: None,
            latency_ms: Some(latency_ms),
        },
        Err(e) => {
            tracing::error!(error = %e, "Storage health check failed");
            ComponentHealth {
                status: ComponentStatus::Unhealthy,
                message: Some("Storage connection failed".to_string()),
                latency_ms: Some(latency_ms),
            }
        }
    }
}

/// Circuit breaker status for a single service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStatus {
    pub service: String,
    pub state: String,
    pub total_calls: u64,
    pub total_failures: u64,
    pub total_rejections: u64,
}

/// Circuit breaker status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakersResponse {
    pub circuits: Vec<CircuitBreakerStatus>,
    pub all_closed: bool,
}

/// Circuit breaker status endpoint
/// GET /health/circuits
pub async fn circuit_breaker_status(
    State(state): State<AppState>,
) -> Json<CircuitBreakersResponse> {
    let states = state.circuit_breakers.all_states().await;
    let metrics = state.circuit_breakers.all_metrics().await;

    let circuits: Vec<CircuitBreakerStatus> = states
        .into_iter()
        .map(|(name, circuit_state)| {
            let metric = metrics.iter().find(|m| m.name == name);
            CircuitBreakerStatus {
                service: name,
                state: circuit_state.to_string(),
                total_calls: metric.map(|m| m.total_calls).unwrap_or(0),
                total_failures: metric.map(|m| m.total_failures).unwrap_or(0),
                total_rejections: metric.map(|m| m.total_rejections).unwrap_or(0),
            }
        })
        .collect();

    let all_closed = circuits.iter().all(|c| c.state == "CLOSED");

    Json(CircuitBreakersResponse {
        circuits,
        all_closed,
    })
}

/// Check email service configuration
fn check_email_config() -> ComponentHealth {
    let provider = std::env::var("EMAIL_PROVIDER").unwrap_or_else(|_| "disabled".to_string());

    match provider.to_lowercase().as_str() {
        "disabled" => ComponentHealth {
            status: ComponentStatus::Degraded,
            message: Some("Email service disabled".to_string()),
            latency_ms: None,
        },
        "postmark" | "brevo" | "sendinblue" => {
            // Check if API key is configured
            if std::env::var("EMAIL_API_KEY").is_ok() {
                ComponentHealth {
                    status: ComponentStatus::Healthy,
                    message: Some(format!("Provider: {}", provider)),
                    latency_ms: None,
                }
            } else {
                ComponentHealth {
                    status: ComponentStatus::Degraded,
                    message: Some(format!("{} configured but EMAIL_API_KEY missing", provider)),
                    latency_ms: None,
                }
            }
        }
        "smtp" => {
            // Check if SMTP host is configured
            if std::env::var("SMTP_HOST").is_ok() {
                ComponentHealth {
                    status: ComponentStatus::Healthy,
                    message: Some("Provider: SMTP".to_string()),
                    latency_ms: None,
                }
            } else {
                ComponentHealth {
                    status: ComponentStatus::Degraded,
                    message: Some("SMTP configured but SMTP_HOST missing".to_string()),
                    latency_ms: None,
                }
            }
        }
        _ => ComponentHealth {
            status: ComponentStatus::Degraded,
            message: Some(format!("Unknown email provider: {}", provider)),
            latency_ms: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_status_serialization() {
        assert_eq!(serde_json::to_string(&ComponentStatus::Healthy).unwrap(), "\"healthy\"");
        assert_eq!(serde_json::to_string(&ComponentStatus::Unhealthy).unwrap(), "\"unhealthy\"");
        assert_eq!(serde_json::to_string(&ComponentStatus::Degraded).unwrap(), "\"degraded\"");
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: ComponentStatus::Healthy,
            version: "1.0.0".to_string(),
            components: None,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["version"], "1.0.0");
        assert!(json.get("components").is_none());
    }

    #[test]
    fn test_health_response_with_components() {
        let mut components = std::collections::HashMap::new();
        components.insert(
            "database".to_string(),
            ComponentHealth {
                status: ComponentStatus::Healthy,
                message: None,
                latency_ms: Some(5),
            },
        );

        let response = HealthResponse {
            status: ComponentStatus::Healthy,
            version: "1.0.0".to_string(),
            components: Some(components),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert!(json.get("components").is_some());
        assert_eq!(json["components"]["database"]["status"], "healthy");
        assert_eq!(json["components"]["database"]["latency_ms"], 5);
    }

    #[test]
    fn test_component_health_without_optional_fields() {
        let health = ComponentHealth {
            status: ComponentStatus::Healthy,
            message: None,
            latency_ms: None,
        };

        let json = serde_json::to_value(&health).unwrap();
        assert_eq!(json["status"], "healthy");
        assert!(json.get("message").is_none());
        assert!(json.get("latency_ms").is_none());
    }

    #[tokio::test]
    async fn test_legacy_health_check() {
        let response = health_check().await;
        assert_eq!(response.0["status"], "ok");
        assert!(response.0.get("version").is_some());
    }

    #[tokio::test]
    async fn test_liveness_check() {
        let (status, response) = liveness_check().await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.status, ComponentStatus::Healthy);
    }

    #[test]
    fn test_check_email_config_disabled() {
        std::env::remove_var("EMAIL_PROVIDER");
        let health = check_email_config();
        assert_eq!(health.status, ComponentStatus::Degraded);
        assert!(health.message.as_ref().unwrap().contains("disabled"));
    }

    #[test]
    fn test_check_email_config_postmark_with_key() {
        std::env::set_var("EMAIL_PROVIDER", "postmark");
        std::env::set_var("EMAIL_API_KEY", "test-key");
        let health = check_email_config();
        assert_eq!(health.status, ComponentStatus::Healthy);
        assert!(health.message.as_ref().unwrap().contains("postmark"));
        std::env::remove_var("EMAIL_PROVIDER");
        std::env::remove_var("EMAIL_API_KEY");
    }

    #[test]
    fn test_check_email_config_postmark_without_key() {
        std::env::set_var("EMAIL_PROVIDER", "postmark");
        std::env::remove_var("EMAIL_API_KEY");
        let health = check_email_config();
        assert_eq!(health.status, ComponentStatus::Degraded);
        assert!(health.message.as_ref().unwrap().contains("EMAIL_API_KEY missing"));
        std::env::remove_var("EMAIL_PROVIDER");
    }

    #[test]
    fn test_check_email_config_smtp_with_host() {
        std::env::set_var("EMAIL_PROVIDER", "smtp");
        std::env::set_var("SMTP_HOST", "localhost");
        let health = check_email_config();
        assert_eq!(health.status, ComponentStatus::Healthy);
        assert!(health.message.as_ref().unwrap().contains("SMTP"));
        std::env::remove_var("EMAIL_PROVIDER");
        std::env::remove_var("SMTP_HOST");
    }

    #[test]
    fn test_check_email_config_smtp_without_host() {
        std::env::set_var("EMAIL_PROVIDER", "smtp");
        std::env::remove_var("SMTP_HOST");
        let health = check_email_config();
        assert_eq!(health.status, ComponentStatus::Degraded);
        assert!(health.message.as_ref().unwrap().contains("SMTP_HOST missing"));
        std::env::remove_var("EMAIL_PROVIDER");
    }

    #[test]
    fn test_check_email_config_unknown_provider() {
        std::env::set_var("EMAIL_PROVIDER", "unknown");
        let health = check_email_config();
        assert_eq!(health.status, ComponentStatus::Degraded);
        assert!(health.message.as_ref().unwrap().contains("Unknown"));
        std::env::remove_var("EMAIL_PROVIDER");
    }

    #[test]
    fn test_health_response_with_all_components() {
        let mut components = std::collections::HashMap::new();
        components.insert(
            "database".to_string(),
            ComponentHealth {
                status: ComponentStatus::Healthy,
                message: None,
                latency_ms: Some(5),
            },
        );
        components.insert(
            "redis".to_string(),
            ComponentHealth {
                status: ComponentStatus::Degraded,
                message: Some("Redis not configured".to_string()),
                latency_ms: None,
            },
        );
        components.insert(
            "storage".to_string(),
            ComponentHealth {
                status: ComponentStatus::Healthy,
                message: None,
                latency_ms: Some(15),
            },
        );
        components.insert(
            "email".to_string(),
            ComponentHealth {
                status: ComponentStatus::Healthy,
                message: Some("Provider: SMTP".to_string()),
                latency_ms: None,
            },
        );

        let response = HealthResponse {
            status: ComponentStatus::Degraded,
            version: "1.0.0".to_string(),
            components: Some(components),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["status"], "degraded");
        assert_eq!(json["components"]["database"]["status"], "healthy");
        assert_eq!(json["components"]["redis"]["status"], "degraded");
        assert_eq!(json["components"]["storage"]["status"], "healthy");
        assert_eq!(json["components"]["email"]["status"], "healthy");
    }
}
