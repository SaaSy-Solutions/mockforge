//! Status page handlers
//!
//! Provides public status information for MockForge Cloud services

use axum::{extract::State, Json};
use chrono::Utc;
use serde::Serialize;

use crate::{error::ApiResult, AppState};

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: String, // "operational", "degraded", "down"
    pub timestamp: String,
    pub services: Vec<ServiceStatus>,
    pub incidents: Vec<Incident>,
}

#[derive(Debug, Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub status: String, // "operational", "degraded", "down"
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Incident {
    pub id: String,
    pub title: String,
    pub status: String, // "resolved", "investigating", "monitoring"
    pub started_at: String,
    pub resolved_at: Option<String>,
    pub impact: String, // "minor", "major", "critical"
}

/// Get public status page information
pub async fn get_status(State(state): State<AppState>) -> ApiResult<Json<StatusResponse>> {
    let pool = state.db.pool();

    // Check service health
    let mut services = Vec::new();

    // Check database
    let db_status = match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => ("operational", None),
        Err(_) => ("down", Some("Database connection failed".to_string())),
    };
    services.push(ServiceStatus {
        name: "Database".to_string(),
        status: db_status.0.to_string(),
        message: db_status.1,
    });

    // Check Redis (if configured)
    if let Some(redis) = &state.redis {
        let redis_status = match redis.ping().await {
            Ok(_) => ("operational", None),
            Err(_) => ("degraded", Some("Redis connection failed".to_string())),
        };
        services.push(ServiceStatus {
            name: "Redis".to_string(),
            status: redis_status.0.to_string(),
            message: redis_status.1,
        });
    } else {
        services.push(ServiceStatus {
            name: "Redis".to_string(),
            status: "operational".to_string(),
            message: Some("Not configured".to_string()),
        });
    }

    // Check storage
    let storage_status = match state.storage.health_check().await {
        Ok(_) => ("operational", None),
        Err(_) => ("degraded", Some("Storage connection failed".to_string())),
    };
    services.push(ServiceStatus {
        name: "Storage".to_string(),
        status: storage_status.0.to_string(),
        message: storage_status.1,
    });

    // API service is always operational if we can respond
    services.push(ServiceStatus {
        name: "API".to_string(),
        status: "operational".to_string(),
        message: None,
    });

    // Determine overall status
    let overall_status = if services.iter().any(|s| s.status == "down") {
        "down"
    } else if services.iter().any(|s| s.status == "degraded") {
        "degraded"
    } else {
        "operational"
    };

    // For now, return empty incidents list
    // In production, this would query an incidents table
    let incidents = Vec::new();

    Ok(Json(StatusResponse {
        status: overall_status.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        services,
        incidents,
    }))
}
