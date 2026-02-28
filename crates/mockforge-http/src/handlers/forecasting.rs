//! HTTP handlers for API change forecasting
//!
//! This module provides endpoints for querying and managing forecasts.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_core::contract_drift::forecasting::{ChangeForecast, Forecaster};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(feature = "database")]
use chrono::{DateTime, Utc};
#[cfg(feature = "database")]
use mockforge_core::contract_drift::forecasting::SeasonalPattern;
#[cfg(feature = "database")]
use uuid::Uuid;

use crate::database::Database;

/// Helper function to map database row to ChangeForecast
#[cfg(feature = "database")]
fn map_row_to_change_forecast(row: &sqlx::postgres::PgRow) -> Result<ChangeForecast, sqlx::Error> {
    use sqlx::Row;

    let service_id: Option<String> = row.try_get("service_id")?;
    let service_name: Option<String> = row.try_get("service_name")?;
    let endpoint: String = row.try_get("endpoint")?;
    let method: String = row.try_get("method")?;
    let forecast_window_days: i32 = row.try_get("forecast_window_days")?;
    let predicted_change_probability: f64 = row.try_get("predicted_change_probability")?;
    let predicted_break_probability: f64 = row.try_get("predicted_break_probability")?;
    let next_expected_change_date: Option<DateTime<Utc>> =
        row.try_get("next_expected_change_date")?;
    let next_expected_break_date: Option<DateTime<Utc>> =
        row.try_get("next_expected_break_date")?;
    let volatility_score: f64 = row.try_get("volatility_score")?;
    let confidence: f64 = row.try_get("confidence")?;
    let seasonal_patterns_json: serde_json::Value =
        row.try_get("seasonal_patterns").unwrap_or_default();
    let predicted_at: DateTime<Utc> = row.try_get("predicted_at")?;
    let expires_at: DateTime<Utc> = row.try_get("expires_at")?;

    // Parse seasonal patterns from JSONB
    let seasonal_patterns: Vec<SeasonalPattern> = if seasonal_patterns_json.is_array() {
        serde_json::from_value(seasonal_patterns_json).unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(ChangeForecast {
        service_id,
        service_name,
        endpoint,
        method,
        forecast_window_days: forecast_window_days as u32,
        predicted_change_probability,
        predicted_break_probability,
        next_expected_change_date,
        next_expected_break_date,
        volatility_score,
        confidence,
        seasonal_patterns,
        predicted_at,
        expires_at,
    })
}

/// State for forecasting handlers
#[derive(Clone)]
pub struct ForecastingState {
    /// Forecaster engine
    pub forecaster: Arc<Forecaster>,
    /// Database connection (optional)
    pub database: Option<Database>,
}

/// Query parameters for listing forecasts
#[derive(Debug, Deserialize)]
pub struct ListForecastsQuery {
    /// Workspace ID filter
    pub workspace_id: Option<String>,
    /// Service ID filter
    pub service_id: Option<String>,
    /// Endpoint filter
    pub endpoint: Option<String>,
    /// Method filter
    pub method: Option<String>,
    /// Forecast window (30, 90, or 180 days)
    pub window_days: Option<u32>,
}

/// Response for forecast list
#[derive(Debug, Serialize)]
pub struct ForecastListResponse {
    /// Forecasts
    pub forecasts: Vec<ChangeForecast>,
    /// Total count
    pub total: usize,
}

/// Get forecasts
///
/// GET /api/v1/forecasts
#[cfg(feature = "database")]
pub async fn list_forecasts(
    State(state): State<ForecastingState>,
    Query(params): Query<ListForecastsQuery>,
) -> Result<Json<ForecastListResponse>, StatusCode> {
    let pool = match state.database.as_ref().and_then(|db| db.pool()) {
        Some(pool) => pool,
        None => {
            return Ok(Json(ForecastListResponse {
                forecasts: Vec::new(),
                total: 0,
            }));
        }
    };

    // Build query with filters
    let mut query = String::from(
        "SELECT id, workspace_id, service_id, service_name, endpoint, method,
         forecast_window_days, predicted_change_probability, predicted_break_probability,
         next_expected_change_date, next_expected_break_date, volatility_score, confidence,
         seasonal_patterns, predicted_at, expires_at
         FROM api_change_forecasts WHERE expires_at > NOW()",
    );

    let mut bind_index = 1;

    if params.workspace_id.is_some() {
        query.push_str(&format!(" AND workspace_id = ${}", bind_index));
        bind_index += 1;
    }

    if params.service_id.is_some() {
        query.push_str(&format!(" AND service_id = ${}", bind_index));
        bind_index += 1;
    }

    if params.endpoint.is_some() {
        query.push_str(&format!(" AND endpoint = ${}", bind_index));
        bind_index += 1;
    }

    if params.method.is_some() {
        query.push_str(&format!(" AND method = ${}", bind_index));
        bind_index += 1;
    }

    if let Some(window) = params.window_days {
        query.push_str(&format!(" AND forecast_window_days = ${}", bind_index));
        bind_index += 1;
    }

    query.push_str(" ORDER BY predicted_at DESC LIMIT 100");

    // Build query with proper bindings using sqlx
    let mut query_builder = sqlx::query(&query);

    if let Some(ws_id) = &params.workspace_id {
        let uuid = Uuid::parse_str(ws_id).ok();
        query_builder = query_builder.bind(uuid);
    }

    if let Some(svc_id) = &params.service_id {
        query_builder = query_builder.bind(svc_id);
    }

    if let Some(ep) = &params.endpoint {
        query_builder = query_builder.bind(ep);
    }

    if let Some(m) = &params.method {
        query_builder = query_builder.bind(m);
    }

    if let Some(window) = params.window_days {
        query_builder = query_builder.bind(window as i32);
    }

    // Execute query
    let rows = query_builder.fetch_all(pool).await.map_err(|e| {
        tracing::error!("Failed to query forecasts: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Map rows to ChangeForecast
    let mut forecasts = Vec::new();
    for row in rows {
        match map_row_to_change_forecast(&row) {
            Ok(forecast) => forecasts.push(forecast),
            Err(e) => {
                tracing::warn!("Failed to map forecast row: {}", e);
                continue;
            }
        }
    }

    let total = forecasts.len();
    Ok(Json(ForecastListResponse { forecasts, total }))
}

/// List forecasts (no database)
///
/// GET /api/v1/forecasts
#[cfg(not(feature = "database"))]
pub async fn list_forecasts(
    State(_state): State<ForecastingState>,
    Query(_params): Query<ListForecastsQuery>,
) -> Result<Json<ForecastListResponse>, StatusCode> {
    Ok(Json(ForecastListResponse {
        forecasts: Vec::new(),
        total: 0,
    }))
}

/// Get service-level forecasts
///
/// GET /api/v1/forecasts/service/{service_id}
#[cfg(feature = "database")]
pub async fn get_service_forecasts(
    State(state): State<ForecastingState>,
    Path(service_id): Path<String>,
    Query(_params): Query<ListForecastsQuery>,
) -> Result<Json<ForecastListResponse>, StatusCode> {
    let pool = match state.database.as_ref().and_then(|db| db.pool()) {
        Some(pool) => pool,
        None => {
            return Ok(Json(ForecastListResponse {
                forecasts: Vec::new(),
                total: 0,
            }));
        }
    };

    // Query forecasts for this service
    let rows = sqlx::query(
        "SELECT * FROM api_change_forecasts
         WHERE service_id = $1 AND expires_at > NOW()
         ORDER BY predicted_at DESC LIMIT 50",
    )
    .bind(&service_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query service forecasts: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Map rows to forecasts
    let mut forecasts = Vec::new();
    for row in rows {
        match map_row_to_change_forecast(&row) {
            Ok(forecast) => forecasts.push(forecast),
            Err(e) => {
                tracing::warn!("Failed to map service forecast row: {}", e);
                continue;
            }
        }
    }

    let total = forecasts.len();
    Ok(Json(ForecastListResponse { forecasts, total }))
}

/// Get service-level forecasts (no database)
///
/// GET /api/v1/forecasts/service/{service_id}
#[cfg(not(feature = "database"))]
pub async fn get_service_forecasts(
    State(_state): State<ForecastingState>,
    Path(_service_id): Path<String>,
    Query(_params): Query<ListForecastsQuery>,
) -> Result<Json<ForecastListResponse>, StatusCode> {
    Ok(Json(ForecastListResponse {
        forecasts: Vec::new(),
        total: 0,
    }))
}

/// Get endpoint-level forecasts
///
/// GET /api/v1/forecasts/endpoint/{endpoint}
#[cfg(feature = "database")]
pub async fn get_endpoint_forecasts(
    State(state): State<ForecastingState>,
    Path(endpoint): Path<String>,
    Query(params): Query<ListForecastsQuery>,
) -> Result<Json<ForecastListResponse>, StatusCode> {
    let pool = match state.database.as_ref().and_then(|db| db.pool()) {
        Some(pool) => pool,
        None => {
            return Ok(Json(ForecastListResponse {
                forecasts: Vec::new(),
                total: 0,
            }));
        }
    };

    let method = params.method.as_deref().unwrap_or("%");

    let rows = sqlx::query(
        "SELECT * FROM api_change_forecasts
         WHERE endpoint = $1 AND method LIKE $2 AND expires_at > NOW()
         ORDER BY predicted_at DESC LIMIT 50",
    )
    .bind(&endpoint)
    .bind(method)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query endpoint forecasts: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Map rows to forecasts
    let mut forecasts = Vec::new();
    for row in rows {
        match map_row_to_change_forecast(&row) {
            Ok(forecast) => forecasts.push(forecast),
            Err(e) => {
                tracing::warn!("Failed to map endpoint forecast row: {}", e);
                continue;
            }
        }
    }

    let total = forecasts.len();
    Ok(Json(ForecastListResponse { forecasts, total }))
}

/// Get endpoint-level forecasts (no database)
///
/// GET /api/v1/forecasts/endpoint/{endpoint}
#[cfg(not(feature = "database"))]
pub async fn get_endpoint_forecasts(
    State(_state): State<ForecastingState>,
    Path(_endpoint): Path<String>,
    Query(_params): Query<ListForecastsQuery>,
) -> Result<Json<ForecastListResponse>, StatusCode> {
    Ok(Json(ForecastListResponse {
        forecasts: Vec::new(),
        total: 0,
    }))
}

/// Request to refresh forecasts
#[derive(Debug, Deserialize)]
pub struct RefreshForecastsRequest {
    /// Workspace ID
    pub workspace_id: Option<String>,
    /// Service ID
    pub service_id: Option<String>,
    /// Endpoint (optional)
    pub endpoint: Option<String>,
    /// Method (optional)
    pub method: Option<String>,
}

/// Refresh forecasts
///
/// POST /api/v1/forecasts/refresh
#[cfg(feature = "database")]
pub async fn refresh_forecasts(
    State(state): State<ForecastingState>,
    Json(request): Json<RefreshForecastsRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pool = match state.database.as_ref().and_then(|db| db.pool()) {
        Some(pool) => pool,
        None => {
            return Ok(Json(serde_json::json!({
                "success": false,
                "error": "Database not available"
            })));
        }
    };

    // Query historical incidents for forecasting
    let mut incident_query = String::from(
        "SELECT id, workspace_id, endpoint, method, incident_type, severity, status,
         detected_at, details, created_at, updated_at
         FROM drift_incidents WHERE 1=1",
    );

    if let Some(ws_id) = &request.workspace_id {
        incident_query.push_str(" AND workspace_id = $1");
    }

    // Execute query to get incidents
    let rows = sqlx::query(&incident_query).fetch_all(pool).await.map_err(|e| {
        tracing::error!("Failed to query drift incidents: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Map rows to DriftIncident and generate forecasts
    use mockforge_core::incidents::types::{IncidentSeverity, IncidentStatus, IncidentType};
    use sqlx::Row;
    let mut incidents = Vec::new();
    for row in rows {
        let id: uuid::Uuid = row.try_get("id").map_err(|e| {
            tracing::error!("Failed to get id from row: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        let workspace_id: Option<uuid::Uuid> = row.try_get("workspace_id").ok();
        let endpoint: String = match row.try_get("endpoint") {
            Ok(e) => e,
            Err(_) => continue,
        };
        let method: String = match row.try_get("method") {
            Ok(m) => m,
            Err(_) => continue,
        };
        let incident_type_str: String = match row.try_get("incident_type") {
            Ok(s) => s,
            Err(_) => continue,
        };
        let severity_str: String = match row.try_get("severity") {
            Ok(s) => s,
            Err(_) => continue,
        };
        let status_str: String = match row.try_get("status") {
            Ok(s) => s,
            Err(_) => continue,
        };
        let detected_at: DateTime<Utc> = match row.try_get("detected_at") {
            Ok(dt) => dt,
            Err(_) => continue,
        };
        let details_json: serde_json::Value = row.try_get("details").unwrap_or_default();
        let created_at: DateTime<Utc> = match row.try_get("created_at") {
            Ok(dt) => dt,
            Err(_) => continue,
        };
        let updated_at: DateTime<Utc> = match row.try_get("updated_at") {
            Ok(dt) => dt,
            Err(_) => continue,
        };

        let incident_type = match incident_type_str.as_str() {
            "breaking_change" => IncidentType::BreakingChange,
            "threshold_exceeded" => IncidentType::ThresholdExceeded,
            _ => continue, // Skip invalid types
        };

        let severity = match severity_str.as_str() {
            "low" => IncidentSeverity::Low,
            "medium" => IncidentSeverity::Medium,
            "high" => IncidentSeverity::High,
            "critical" => IncidentSeverity::Critical,
            _ => continue, // Skip invalid severity
        };

        let status = match status_str.as_str() {
            "open" => IncidentStatus::Open,
            "acknowledged" => IncidentStatus::Acknowledged,
            "resolved" => IncidentStatus::Resolved,
            "closed" => IncidentStatus::Closed,
            _ => continue, // Skip invalid status
        };

        incidents.push(DriftIncident {
            id: id.to_string(),
            budget_id: None,
            workspace_id: workspace_id.map(|u| u.to_string()),
            endpoint,
            method,
            incident_type,
            severity,
            status,
            detected_at: detected_at.timestamp(),
            resolved_at: None,
            details: details_json,
            external_ticket_id: None,
            external_ticket_url: None,
            created_at: created_at.timestamp(),
            updated_at: updated_at.timestamp(),
            sync_cycle_id: None,
            contract_diff_id: None,
            before_sample: None,
            after_sample: None,
            fitness_test_results: Vec::new(),
            affected_consumers: None,
            protocol: None,
        });
    }

    // Generate forecasts from incidents by grouping by endpoint/method
    use mockforge_core::incidents::types::DriftIncident;
    use std::collections::HashMap;
    let mut forecasts_generated = 0;
    let mut endpoint_groups: HashMap<(String, String), Vec<DriftIncident>> = HashMap::new();

    for incident in incidents {
        endpoint_groups
            .entry((incident.endpoint.clone(), incident.method.clone()))
            .or_insert_with(Vec::new)
            .push(incident);
    }

    for ((endpoint, method), group_incidents) in endpoint_groups {
        if let Some(_forecast) = state.forecaster.generate_forecast(
            &group_incidents,
            request.workspace_id.clone(),
            None, // service_id
            None, // service_name
            endpoint,
            method,
            30, // forecast_window_days
        ) {
            forecasts_generated += 1;
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Forecasts refreshed",
        "forecasts_generated": forecasts_generated
    })))
}

/// Refresh forecasts (no database)
///
/// POST /api/v1/forecasts/refresh
#[cfg(not(feature = "database"))]
pub async fn refresh_forecasts(
    State(_state): State<ForecastingState>,
    Json(_request): Json<RefreshForecastsRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "success": false,
        "error": "Database not available"
    })))
}

/// Store a forecast in the database
#[cfg(feature = "database")]
pub async fn store_forecast(
    pool: &sqlx::PgPool,
    forecast: &ChangeForecast,
    workspace_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4();
    let workspace_uuid = workspace_id.and_then(|id| Uuid::parse_str(id).ok());

    sqlx::query(
        r#"
        INSERT INTO api_change_forecasts (
            id, workspace_id, service_id, service_name, endpoint, method,
            forecast_window_days, predicted_change_probability, predicted_break_probability,
            next_expected_change_date, next_expected_break_date, volatility_score, confidence,
            seasonal_patterns, predicted_at, expires_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16
        )
        ON CONFLICT (workspace_id, service_id, endpoint, method, forecast_window_days)
        DO UPDATE SET
            predicted_change_probability = EXCLUDED.predicted_change_probability,
            predicted_break_probability = EXCLUDED.predicted_break_probability,
            next_expected_change_date = EXCLUDED.next_expected_change_date,
            next_expected_break_date = EXCLUDED.next_expected_break_date,
            volatility_score = EXCLUDED.volatility_score,
            confidence = EXCLUDED.confidence,
            seasonal_patterns = EXCLUDED.seasonal_patterns,
            predicted_at = EXCLUDED.predicted_at,
            expires_at = EXCLUDED.expires_at,
            updated_at = NOW()
        "#,
    )
    .bind(id)
    .bind(workspace_uuid)
    .bind(forecast.service_id.as_deref())
    .bind(forecast.service_name.as_deref())
    .bind(&forecast.endpoint)
    .bind(&forecast.method)
    .bind(forecast.forecast_window_days as i32)
    .bind(forecast.predicted_change_probability)
    .bind(forecast.predicted_break_probability)
    .bind(forecast.next_expected_change_date)
    .bind(forecast.next_expected_break_date)
    .bind(forecast.volatility_score)
    .bind(forecast.confidence)
    .bind(serde_json::to_value(&forecast.seasonal_patterns).unwrap_or_default())
    .bind(forecast.predicted_at)
    .bind(forecast.expires_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Create router for forecasting endpoints
pub fn forecasting_router(state: ForecastingState) -> axum::Router {
    use axum::routing::{get, post};
    use axum::Router;

    Router::new()
        .route("/api/v1/forecasts", get(list_forecasts))
        .route("/api/v1/forecasts/service/{service_id}", get(get_service_forecasts))
        .route("/api/v1/forecasts/endpoint/{endpoint}", get(get_endpoint_forecasts))
        .route("/api/v1/forecasts/refresh", post(refresh_forecasts))
        .with_state(state)
}
