//! Kubernetes-native health check endpoints
//!
//! This module provides comprehensive health check endpoints following Kubernetes best practices:
//! - **Liveness probe**: Indicates if the container is alive
//! - **Readiness probe**: Indicates if the container is ready to accept traffic
//! - **Startup probe**: Indicates if the container has finished initialization
//!
//! These endpoints are essential for:
//! - Kubernetes deployment orchestration
//! - Load balancer health checks
//! - Service discovery integration
//! - Graceful shutdown coordination

use axum::{extract::State, http::StatusCode, response::Json, routing::get};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Service initialization status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Service is initializing (not ready)
    Initializing,
    /// Service is ready to accept traffic
    Ready,
    /// Service is shutting down (not accepting new requests)
    ShuttingDown,
    /// Service has failed and is unhealthy
    Failed,
}

impl ServiceStatus {
    /// Check if service is ready to accept traffic
    pub fn is_ready(&self) -> bool {
        matches!(self, ServiceStatus::Ready)
    }

    /// Check if service is alive (not failed)
    pub fn is_alive(&self) -> bool {
        !matches!(self, ServiceStatus::Failed)
    }
}

/// Health check manager for tracking service state
#[derive(Debug, Clone)]
pub struct HealthManager {
    /// Current service status
    status: Arc<RwLock<ServiceStatus>>,
    /// Server startup time
    start_time: Arc<Instant>,
    /// Service initialization deadline (timeout)
    init_deadline: Arc<Option<Instant>>,
    /// Shutdown signal for graceful termination
    shutdown_signal: Arc<RwLock<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl HealthManager {
    /// Create a new health manager
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(ServiceStatus::Initializing)),
            start_time: Arc::new(Instant::now()),
            init_deadline: Arc::new(None),
            shutdown_signal: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new health manager with initialization timeout
    pub fn with_init_timeout(timeout: Duration) -> Self {
        let deadline = Instant::now() + timeout;
        Self {
            status: Arc::new(RwLock::new(ServiceStatus::Initializing)),
            start_time: Arc::new(Instant::now()),
            init_deadline: Arc::new(Some(deadline)),
            shutdown_signal: Arc::new(RwLock::new(None)),
        }
    }

    /// Mark service as ready
    pub async fn set_ready(&self) {
        let mut status = self.status.write().await;
        *status = ServiceStatus::Ready;
        info!("Service marked as ready");
    }

    /// Mark service as failed
    pub async fn set_failed(&self, reason: &str) {
        let mut status = self.status.write().await;
        *status = ServiceStatus::Failed;
        error!("Service marked as failed: {}", reason);
    }

    /// Mark service as shutting down
    pub async fn set_shutting_down(&self) {
        let mut status = self.status.write().await;
        *status = ServiceStatus::ShuttingDown;
        info!("Service marked as shutting down");
    }

    /// Get current service status
    pub async fn get_status(&self) -> ServiceStatus {
        *self.status.read().await
    }

    /// Get server uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Check if initialization has timed out
    pub fn is_init_timeout(&self) -> bool {
        if let Some(deadline) = *self.init_deadline {
            Instant::now() > deadline
        } else {
            false
        }
    }

    /// Set shutdown signal receiver for graceful shutdown
    pub async fn set_shutdown_signal(&self, sender: tokio::sync::oneshot::Sender<()>) {
        let mut signal = self.shutdown_signal.write().await;
        *signal = Some(sender);
    }

    /// Trigger graceful shutdown
    pub async fn trigger_shutdown(&self) {
        self.set_shutting_down().await;
        let mut signal = self.shutdown_signal.write().await;
        if let Some(sender) = signal.take() {
            let _ = sender.send(());
            info!("Graceful shutdown signal sent");
        }
    }
}

impl Default for HealthManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    /// Service version
    pub version: String,
    /// Additional status information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HealthDetails>,
}

/// Detailed health information
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthDetails {
    /// Service initialization status
    pub initialization: String,
    /// Active connection count (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections: Option<u64>,
    /// Memory usage in bytes (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
}

/// Liveness probe endpoint
///
/// Kubernetes uses this to determine if the container should be restarted.
/// Returns 200 if the service is alive, 503 if it has failed.
///
/// This should be a lightweight check that doesn't depend on external services.
async fn liveness_probe(
    State(health): State<Arc<HealthManager>>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let status = health.get_status().await;
    let uptime = health.uptime_seconds();

    // Liveness checks if the process is alive (not failed)
    if status.is_alive() {
        let response = HealthResponse {
            status: "alive".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            uptime_seconds: uptime,
            version: env!("CARGO_PKG_VERSION").to_string(),
            details: None,
        };
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// Readiness probe endpoint
///
/// Kubernetes uses this to determine if the container is ready to receive traffic.
/// Returns 200 if the service is ready, 503 if it's not ready or shutting down.
///
/// This checks if the service has completed initialization and is ready to serve requests.
async fn readiness_probe(
    State(health): State<Arc<HealthManager>>,
) -> Result<Json<HealthResponse>, (StatusCode, Json<HealthResponse>)> {
    let status = health.get_status().await;
    let uptime = health.uptime_seconds();

    // Readiness checks if the service is ready to accept traffic
    if status.is_ready() {
        let response = HealthResponse {
            status: "ready".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            uptime_seconds: uptime,
            version: env!("CARGO_PKG_VERSION").to_string(),
            details: Some(HealthDetails {
                initialization: "complete".to_string(),
                connections: None,
                memory_bytes: None,
            }),
        };
        Ok(Json(response))
    } else {
        let details = match status {
            ServiceStatus::Initializing => {
                if health.is_init_timeout() {
                    "initialization_timeout".to_string()
                } else {
                    "initializing".to_string()
                }
            }
            ServiceStatus::ShuttingDown => "shutting_down".to_string(),
            ServiceStatus::Failed => "failed".to_string(),
            ServiceStatus::Ready => unreachable!(),
        };

        let response = HealthResponse {
            status: "not_ready".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            uptime_seconds: uptime,
            version: env!("CARGO_PKG_VERSION").to_string(),
            details: Some(HealthDetails {
                initialization: details,
                connections: None,
                memory_bytes: None,
            }),
        };

        Err((StatusCode::SERVICE_UNAVAILABLE, Json(response)))
    }
}

/// Startup probe endpoint
///
/// Kubernetes uses this to determine if the container has finished initialization.
/// This is useful for services that take a long time to start.
/// Returns 200 once initialization is complete, 503 while still initializing.
async fn startup_probe(
    State(health): State<Arc<HealthManager>>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let status = health.get_status().await;
    let uptime = health.uptime_seconds();

    // Startup probe checks if initialization is complete
    match status {
        ServiceStatus::Ready => {
            let response = HealthResponse {
                status: "startup_complete".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                uptime_seconds: uptime,
                version: env!("CARGO_PKG_VERSION").to_string(),
                details: Some(HealthDetails {
                    initialization: "complete".to_string(),
                    connections: None,
                    memory_bytes: None,
                }),
            };
            Ok(Json(response))
        }
        ServiceStatus::Initializing => {
            if health.is_init_timeout() {
                warn!("Startup probe: initialization timeout exceeded");
                Err(StatusCode::SERVICE_UNAVAILABLE)
            } else {
                debug!("Startup probe: still initializing");
                Err(StatusCode::SERVICE_UNAVAILABLE)
            }
        }
        ServiceStatus::Failed => Err(StatusCode::SERVICE_UNAVAILABLE),
        ServiceStatus::ShuttingDown => {
            // During shutdown, startup probe should return ready (service was started)
            let response = HealthResponse {
                status: "startup_complete".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                uptime_seconds: uptime,
                version: env!("CARGO_PKG_VERSION").to_string(),
                details: Some(HealthDetails {
                    initialization: "complete".to_string(),
                    connections: None,
                    memory_bytes: None,
                }),
            };
            Ok(Json(response))
        }
    }
}

/// Combined health check endpoint (backwards compatibility)
///
/// This endpoint provides a general health check that combines liveness and readiness.
/// For Kubernetes deployments, prefer using the specific probe endpoints.
async fn health_check(
    State(health): State<Arc<HealthManager>>,
) -> Result<Json<HealthResponse>, (StatusCode, Json<HealthResponse>)> {
    let status = health.get_status().await;
    let uptime = health.uptime_seconds();

    if status.is_ready() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            uptime_seconds: uptime,
            version: env!("CARGO_PKG_VERSION").to_string(),
            details: Some(HealthDetails {
                initialization: "complete".to_string(),
                connections: None,
                memory_bytes: None,
            }),
        };
        Ok(Json(response))
    } else {
        let status_str = match status {
            ServiceStatus::Initializing => "initializing",
            ServiceStatus::ShuttingDown => "shutting_down",
            ServiceStatus::Failed => "failed",
            ServiceStatus::Ready => unreachable!(),
        };

        let response = HealthResponse {
            status: status_str.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            uptime_seconds: uptime,
            version: env!("CARGO_PKG_VERSION").to_string(),
            details: Some(HealthDetails {
                initialization: status_str.to_string(),
                connections: None,
                memory_bytes: None,
            }),
        };

        Err((StatusCode::SERVICE_UNAVAILABLE, Json(response)))
    }
}

/// Create health check router with all probe endpoints
pub fn health_router(health_manager: Arc<HealthManager>) -> axum::Router {
    use axum::Router;
    Router::new()
        .route("/health", get(health_check))
        .route("/health/live", get(liveness_probe))
        .route("/health/ready", get(readiness_probe))
        .route("/health/startup", get(startup_probe))
        .with_state(health_manager)
}

/// Create health check router with custom prefix
pub fn health_router_with_prefix(health_manager: Arc<HealthManager>, prefix: &str) -> axum::Router {
    use axum::Router;
    Router::new()
        .route(&format!("{}/health", prefix), get(health_check))
        .route(&format!("{}/health/live", prefix), get(liveness_probe))
        .route(&format!("{}/health/ready", prefix), get(readiness_probe))
        .route(&format!("{}/health/startup", prefix), get(startup_probe))
        .with_state(health_manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_liveness_probe_alive() {
        let health = Arc::new(HealthManager::new());
        health.set_ready().await;

        let app = health_router(health.clone());
        let response = app
            .oneshot(Request::builder().uri("/health/live").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_liveness_probe_failed() {
        let health = Arc::new(HealthManager::new());
        health.set_failed("test failure").await;

        let app = health_router(health.clone());
        let response = app
            .oneshot(Request::builder().uri("/health/live").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_readiness_probe_ready() {
        let health = Arc::new(HealthManager::new());
        health.set_ready().await;

        let app = health_router(health.clone());
        let response = app
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_readiness_probe_initializing() {
        let health = Arc::new(HealthManager::new());
        // Status is Initializing by default

        let app = health_router(health.clone());
        let response = app
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_startup_probe_ready() {
        let health = Arc::new(HealthManager::new());
        health.set_ready().await;

        let app = health_router(health.clone());
        let response = app
            .oneshot(Request::builder().uri("/health/startup").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_startup_probe_initializing() {
        let health = Arc::new(HealthManager::new());
        // Status is Initializing by default

        let app = health_router(health.clone());
        let response = app
            .oneshot(Request::builder().uri("/health/startup").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_health_check_ready() {
        let health = Arc::new(HealthManager::new());
        health.set_ready().await;

        let app = health_router(health.clone());
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_service_status() {
        assert!(ServiceStatus::Ready.is_ready());
        assert!(!ServiceStatus::Initializing.is_ready());
        assert!(!ServiceStatus::ShuttingDown.is_ready());
        assert!(!ServiceStatus::Failed.is_ready());

        assert!(ServiceStatus::Ready.is_alive());
        assert!(ServiceStatus::Initializing.is_alive());
        assert!(ServiceStatus::ShuttingDown.is_alive());
        assert!(!ServiceStatus::Failed.is_alive());
    }
}
