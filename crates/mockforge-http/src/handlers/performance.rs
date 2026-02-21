//! Performance Mode API handlers
//!
//! Provides HTTP endpoints for performance mode:
//! - Start/stop performance simulation
//! - Configure RPS profiles
//! - Add/remove bottlenecks
//! - Get performance metrics and snapshots

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use mockforge_performance::{
    bottleneck::BottleneckConfig,
    controller::RpsProfile,
    metrics::PerformanceSnapshot,
    simulator::{PerformanceSimulator, SimulatorConfig},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// State for performance mode handlers
#[derive(Clone)]
pub struct PerformanceState {
    /// Performance simulator
    pub simulator: Arc<RwLock<Option<Arc<PerformanceSimulator>>>>,
}

impl PerformanceState {
    /// Create new performance state
    pub fn new() -> Self {
        Self {
            simulator: Arc::new(RwLock::new(None)),
        }
    }
}

/// Request to start performance mode
#[derive(Debug, Deserialize)]
pub struct StartPerformanceRequest {
    /// Initial RPS
    pub initial_rps: f64,
    /// RPS profile (optional)
    pub rps_profile: Option<RpsProfile>,
    /// Bottlenecks (optional)
    pub bottlenecks: Option<Vec<BottleneckConfig>>,
}

/// Request to update RPS
#[derive(Debug, Deserialize)]
pub struct UpdateRpsRequest {
    /// Target RPS
    pub target_rps: f64,
}

/// Request to add bottleneck
#[derive(Debug, Deserialize)]
pub struct AddBottleneckRequest {
    /// Bottleneck configuration
    pub bottleneck: BottleneckConfig,
}

/// Response for performance operations
#[derive(Debug, Serialize)]
pub struct PerformanceResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Response message
    pub message: String,
    /// Performance snapshot (if available)
    pub snapshot: Option<PerformanceSnapshot>,
}

/// Start performance mode
/// POST /api/performance/start
pub async fn start_performance(
    State(state): State<PerformanceState>,
    Json(request): Json<StartPerformanceRequest>,
) -> Result<Json<PerformanceResponse>, StatusCode> {
    info!("Starting performance mode with RPS: {}", request.initial_rps);

    let config = SimulatorConfig::new(request.initial_rps).with_rps_profile(
        request.rps_profile.unwrap_or_else(|| RpsProfile::constant(request.initial_rps)),
    );

    let config = if let Some(bottlenecks) = request.bottlenecks {
        let mut cfg = config;
        for bottleneck in bottlenecks {
            cfg = cfg.with_bottleneck(bottleneck);
        }
        cfg
    } else {
        config
    };

    let simulator = Arc::new(PerformanceSimulator::new(config));
    simulator.start().await;

    {
        let mut sim = state.simulator.write().await;
        *sim = Some(simulator.clone());
    }

    let snapshot = simulator.get_snapshot().await;

    Ok(Json(PerformanceResponse {
        success: true,
        message: "Performance mode started".to_string(),
        snapshot: Some(snapshot),
    }))
}

/// Stop performance mode
/// POST /api/performance/stop
pub async fn stop_performance(
    State(state): State<PerformanceState>,
) -> Result<Json<PerformanceResponse>, StatusCode> {
    info!("Stopping performance mode");

    let simulator = {
        let mut sim = state.simulator.write().await;
        sim.take()
    };

    if let Some(sim) = simulator {
        sim.stop().await;
        Ok(Json(PerformanceResponse {
            success: true,
            message: "Performance mode stopped".to_string(),
            snapshot: None,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get current performance snapshot
/// GET /api/performance/snapshot
pub async fn get_performance_snapshot(
    State(state): State<PerformanceState>,
) -> Result<Json<PerformanceSnapshot>, StatusCode> {
    let simulator = {
        let sim = state.simulator.read().await;
        sim.clone()
    };

    if let Some(sim) = simulator {
        let snapshot = sim.get_snapshot().await;
        Ok(Json(snapshot))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Update target RPS
/// POST /api/performance/rps
pub async fn update_rps(
    State(state): State<PerformanceState>,
    Json(request): Json<UpdateRpsRequest>,
) -> Result<Json<PerformanceResponse>, StatusCode> {
    let simulator = {
        let sim = state.simulator.read().await;
        sim.clone()
    };

    if let Some(sim) = simulator {
        sim.rps_controller().set_target_rps(request.target_rps).await;
        let snapshot = sim.get_snapshot().await;

        Ok(Json(PerformanceResponse {
            success: true,
            message: format!("RPS updated to {}", request.target_rps),
            snapshot: Some(snapshot),
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Add bottleneck
/// POST /api/performance/bottlenecks
pub async fn add_bottleneck(
    State(state): State<PerformanceState>,
    Json(request): Json<AddBottleneckRequest>,
) -> Result<Json<PerformanceResponse>, StatusCode> {
    let simulator = {
        let sim = state.simulator.read().await;
        sim.clone()
    };

    if let Some(sim) = simulator {
        sim.bottleneck_simulator().add_bottleneck(request.bottleneck).await;
        let snapshot = sim.get_snapshot().await;

        Ok(Json(PerformanceResponse {
            success: true,
            message: "Bottleneck added".to_string(),
            snapshot: Some(snapshot),
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Remove all bottlenecks
/// DELETE /api/performance/bottlenecks
pub async fn clear_bottlenecks(
    State(state): State<PerformanceState>,
) -> Result<Json<PerformanceResponse>, StatusCode> {
    let simulator = {
        let sim = state.simulator.read().await;
        sim.clone()
    };

    if let Some(sim) = simulator {
        sim.bottleneck_simulator().clear_bottlenecks().await;
        let snapshot = sim.get_snapshot().await;

        Ok(Json(PerformanceResponse {
            success: true,
            message: "All bottlenecks cleared".to_string(),
            snapshot: Some(snapshot),
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get performance status
/// GET /api/performance/status
pub async fn get_performance_status(
    State(state): State<PerformanceState>,
) -> Json<serde_json::Value> {
    let simulator = {
        let sim = state.simulator.read().await;
        sim.clone()
    };

    if let Some(sim) = simulator {
        let is_running = sim.is_running().await;
        let target_rps = sim.rps_controller().get_target_rps().await;
        let current_rps = sim.rps_controller().get_current_rps().await;
        let bottlenecks = sim.bottleneck_simulator().get_bottlenecks().await;

        Json(serde_json::json!({
            "running": is_running,
            "target_rps": target_rps,
            "current_rps": current_rps,
            "bottlenecks": bottlenecks.len(),
            "bottleneck_types": bottlenecks.iter().map(|b| format!("{:?}", b.bottleneck_type)).collect::<Vec<_>>(),
        }))
    } else {
        Json(serde_json::json!({
            "running": false,
            "target_rps": 0.0,
            "current_rps": 0.0,
            "bottlenecks": 0,
            "bottleneck_types": Vec::<String>::new(),
        }))
    }
}

/// Create the Axum router for performance mode API
pub fn performance_router(state: PerformanceState) -> Router {
    Router::new()
        .route("/start", post(start_performance))
        .route("/stop", post(stop_performance))
        .route("/snapshot", get(get_performance_snapshot))
        .route("/rps", post(update_rps))
        .route("/bottlenecks", post(add_bottleneck))
        .route("/bottlenecks", delete(clear_bottlenecks))
        .route("/status", get(get_performance_status))
        .with_state(state)
}
