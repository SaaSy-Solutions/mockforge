//! Admin API handlers for time travel features

use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::{DateTime, Duration, Utc};
use mockforge_core::{
    time_travel::cron::{CronJob, CronJobAction},
    RepeatConfig, ScheduledResponse, TimeTravelManager, VirtualClock,
};
use mockforge_vbr::{MutationOperation, MutationRule, MutationRuleManager, MutationTrigger};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;

/// Global time travel manager (optional, can be set by the application)
static TIME_TRAVEL_MANAGER: once_cell::sync::OnceCell<Arc<RwLock<Option<Arc<TimeTravelManager>>>>> =
    once_cell::sync::OnceCell::new();

/// Initialize the global time travel manager
pub fn init_time_travel_manager(manager: Arc<TimeTravelManager>) {
    let cell = TIME_TRAVEL_MANAGER.get_or_init(|| Arc::new(RwLock::new(None)));
    let mut guard = cell.write().unwrap();
    *guard = Some(manager);
}

/// Get the global time travel manager
fn get_time_travel_manager() -> Option<Arc<TimeTravelManager>> {
    TIME_TRAVEL_MANAGER.get().and_then(|cell| cell.read().unwrap().clone())
}

/// Global mutation rule manager (optional, can be set by the application)
static MUTATION_RULE_MANAGER: once_cell::sync::OnceCell<
    Arc<RwLock<Option<Arc<MutationRuleManager>>>>,
> = once_cell::sync::OnceCell::new();

/// Initialize the global mutation rule manager
pub fn init_mutation_rule_manager(manager: Arc<MutationRuleManager>) {
    let cell = MUTATION_RULE_MANAGER.get_or_init(|| Arc::new(RwLock::new(None)));
    let mut guard = cell.write().unwrap();
    *guard = Some(manager);
}

/// Get the global mutation rule manager
fn get_mutation_rule_manager() -> Option<Arc<MutationRuleManager>> {
    MUTATION_RULE_MANAGER.get().and_then(|cell| cell.read().unwrap().clone())
}

/// Request to enable time travel at a specific time
#[derive(Debug, Serialize, Deserialize)]
pub struct EnableTimeTravelRequest {
    /// The time to set (ISO 8601 format)
    pub time: Option<DateTime<Utc>>,
    /// Time scale factor (default: 1.0)
    pub scale: Option<f64>,
}

/// Request to advance time
#[derive(Debug, Serialize, Deserialize)]
pub struct AdvanceTimeRequest {
    /// Duration to advance (e.g., "2h", "30m", "10s")
    pub duration: String,
}

/// Request to set time scale
#[derive(Debug, Serialize, Deserialize)]
pub struct SetScaleRequest {
    /// Time scale factor (1.0 = real time, 2.0 = 2x speed)
    pub scale: f64,
}

/// Request to schedule a response
#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleResponseRequest {
    /// When to trigger (ISO 8601 format or relative like "+1h")
    pub trigger_time: String,
    /// Response body (JSON)
    pub body: serde_json::Value,
    /// HTTP status code (default: 200)
    #[serde(default = "default_status")]
    pub status: u16,
    /// Response headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Optional name/label
    pub name: Option<String>,
    /// Repeat configuration
    pub repeat: Option<RepeatConfig>,
}

fn default_status() -> u16 {
    200
}

/// Response with scheduled response ID
#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleResponseResponse {
    pub id: String,
    pub trigger_time: DateTime<Utc>,
}

/// Get time travel status
pub async fn get_time_travel_status() -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            let status = manager.clock().status();
            Json(status).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Enable time travel
pub async fn enable_time_travel(Json(req): Json<EnableTimeTravelRequest>) -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            let time = req.time.unwrap_or_else(Utc::now);
            manager.enable_and_set(time);

            if let Some(scale) = req.scale {
                manager.set_scale(scale);
            }

            info!("Time travel enabled at {}", time);

            Json(serde_json::json!({
                "success": true,
                "status": manager.clock().status()
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Disable time travel
pub async fn disable_time_travel() -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            manager.disable();
            info!("Time travel disabled");

            Json(serde_json::json!({
                "success": true,
                "status": manager.clock().status()
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Advance time by a duration
pub async fn advance_time(Json(req): Json<AdvanceTimeRequest>) -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            // Parse duration string (e.g., "2h", "30m", "10s", "1week")
            let duration = parse_duration(&req.duration);

            match duration {
                Ok(dur) => {
                    manager.advance(dur);
                    info!("Time advanced by {}", req.duration);

                    Json(serde_json::json!({
                        "success": true,
                        "status": manager.clock().status()
                    }))
                    .into_response()
                }
                Err(e) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("Invalid duration format: {}", e)
                    })),
                )
                    .into_response(),
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Set time scale
pub async fn set_time_scale(Json(req): Json<SetScaleRequest>) -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            manager.set_scale(req.scale);
            info!("Time scale set to {}x", req.scale);

            Json(serde_json::json!({
                "success": true,
                "status": manager.clock().status()
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Request to set virtual time
#[derive(Debug, Serialize, Deserialize)]
pub struct SetTimeRequest {
    /// The time to set (ISO 8601 format)
    pub time: DateTime<Utc>,
}

/// Set virtual time to a specific point
pub async fn set_time(Json(req): Json<SetTimeRequest>) -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            manager.clock().set_time(req.time);
            info!("Virtual time set to {}", req.time);

            Json(serde_json::json!({
                "success": true,
                "status": manager.clock().status()
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Reset time travel
pub async fn reset_time_travel() -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            manager.clock().reset();
            info!("Time travel reset");

            Json(serde_json::json!({
                "success": true,
                "status": manager.clock().status()
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Schedule a response
pub async fn schedule_response(Json(req): Json<ScheduleResponseRequest>) -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            // Parse trigger time (ISO 8601 or relative like "+1h")
            let trigger_time = parse_trigger_time(&req.trigger_time, manager.clock());

            match trigger_time {
                Ok(time) => {
                    let scheduled_response = ScheduledResponse {
                        id: uuid::Uuid::new_v4().to_string(),
                        trigger_time: time,
                        body: req.body,
                        status: req.status,
                        headers: req.headers,
                        name: req.name,
                        repeat: req.repeat,
                    };

                    match manager.scheduler().schedule(scheduled_response.clone()) {
                        Ok(id) => {
                            info!("Scheduled response {} for {}", id, time);

                            Json(ScheduleResponseResponse {
                                id,
                                trigger_time: time,
                            })
                            .into_response()
                        }
                        Err(e) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({
                                "error": format!("Failed to schedule response: {}", e)
                            })),
                        )
                            .into_response(),
                    }
                }
                Err(e) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": format!("Invalid trigger time: {}", e)
                    })),
                )
                    .into_response(),
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// List scheduled responses
pub async fn list_scheduled_responses() -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            let scheduled = manager.scheduler().list_scheduled();
            Json(scheduled).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Cancel a scheduled response
pub async fn cancel_scheduled_response(Path(id): Path<String>) -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            let cancelled = manager.scheduler().cancel(&id);

            if cancelled {
                info!("Cancelled scheduled response {}", id);
                Json(serde_json::json!({
                    "success": true
                }))
                .into_response()
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": "Scheduled response not found"
                    })),
                )
                    .into_response()
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Clear all scheduled responses
pub async fn clear_scheduled_responses() -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            manager.scheduler().clear_all();
            info!("Cleared all scheduled responses");

            Json(serde_json::json!({
                "success": true
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Request to save a scenario
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveScenarioRequest {
    /// Scenario name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
}

/// Request to load a scenario
#[derive(Debug, Serialize, Deserialize)]
pub struct LoadScenarioRequest {
    /// Scenario name
    pub name: String,
}

/// Save current time travel state as a scenario
pub async fn save_scenario(Json(req): Json<SaveScenarioRequest>) -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            let mut scenario = manager.save_scenario(req.name.clone());
            scenario.description = req.description;

            Json(scenario).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Load a scenario
pub async fn load_scenario(Json(req): Json<LoadScenarioRequest>) -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(_manager) => {
            // For now, scenarios are passed in the request body
            // In a full implementation, scenarios would be stored and loaded from disk
            (
                StatusCode::NOT_IMPLEMENTED,
                Json(serde_json::json!({
                    "error": "Scenario loading from storage not yet implemented. Use save_scenario to get scenario JSON, then POST it back."
                })),
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Request to create a cron job
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCronJobRequest {
    /// Job ID
    pub id: String,
    /// Job name
    pub name: String,
    /// Cron schedule (e.g., "0 3 * * *")
    pub schedule: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Action type: "callback", "response", or "mutation"
    pub action_type: String,
    /// Action metadata (JSON)
    #[serde(default)]
    pub action_metadata: serde_json::Value,
}

/// List all cron jobs
pub async fn list_cron_jobs() -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            let jobs = manager.cron_scheduler().list_jobs().await;
            Json(serde_json::json!({
                "success": true,
                "jobs": jobs
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Get a specific cron job
pub async fn get_cron_job(Path(id): Path<String>) -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => match manager.cron_scheduler().get_job(&id).await {
            Some(job) => Json(serde_json::json!({
                "success": true,
                "job": job
            }))
            .into_response(),
            None => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Cron job '{}' not found", id)
                })),
            )
                .into_response(),
        },
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Create a new cron job
pub async fn create_cron_job(Json(req): Json<CreateCronJobRequest>) -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            // Create the job
            let mut job = CronJob::new(req.id.clone(), req.name.clone(), req.schedule.clone());
            if let Some(desc) = req.description {
                job.description = Some(desc);
            }

            // Create the action based on type
            let action = match req.action_type.as_str() {
                "callback" => {
                    let job_id = req.id.clone();
                    CronJobAction::Callback(Box::new(move |_| {
                        info!("Cron job '{}' executed (callback)", job_id);
                        Ok(())
                    }))
                }
                "response" => {
                    let body =
                        req.action_metadata.get("body").cloned().unwrap_or(serde_json::json!({}));
                    let status = req
                        .action_metadata
                        .get("status")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u16)
                        .unwrap_or(200);
                    let headers = req
                        .action_metadata
                        .get("headers")
                        .and_then(|v| v.as_object())
                        .map(|obj| {
                            obj.iter()
                                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                .collect()
                        })
                        .unwrap_or_default();

                    CronJobAction::ScheduledResponse {
                        body,
                        status,
                        headers,
                    }
                }
                "mutation" => {
                    let entity = req
                        .action_metadata
                        .get("entity")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let operation = req
                        .action_metadata
                        .get("operation")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    CronJobAction::DataMutation { entity, operation }
                }
                _ => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": format!("Invalid action type: {}", req.action_type)
                        })),
                    )
                        .into_response();
                }
            };

            match manager.cron_scheduler().add_job(job, action).await {
                Ok(_) => {
                    info!("Created cron job '{}'", req.id);
                    Json(serde_json::json!({
                        "success": true,
                        "message": format!("Cron job '{}' created", req.id)
                    }))
                    .into_response()
                }
                Err(e) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": e.to_string()
                    })),
                )
                    .into_response(),
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Delete a cron job
pub async fn delete_cron_job(Path(id): Path<String>) -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => {
            let removed = manager.cron_scheduler().remove_job(&id).await;
            if removed {
                info!("Deleted cron job '{}'", id);
                Json(serde_json::json!({
                    "success": true,
                    "message": format!("Cron job '{}' deleted", id)
                }))
                .into_response()
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": format!("Cron job '{}' not found", id)
                    })),
                )
                    .into_response()
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Enable or disable a cron job
#[derive(Debug, Serialize, Deserialize)]
pub struct SetCronJobEnabledRequest {
    /// Whether to enable the job
    pub enabled: bool,
}

pub async fn set_cron_job_enabled(
    Path(id): Path<String>,
    Json(req): Json<SetCronJobEnabledRequest>,
) -> impl IntoResponse {
    match get_time_travel_manager() {
        Some(manager) => match manager.cron_scheduler().set_job_enabled(&id, req.enabled).await {
            Ok(_) => {
                info!("Cron job '{}' {}", id, if req.enabled { "enabled" } else { "disabled" });
                Json(serde_json::json!({
                        "success": true,
                        "message": format!("Cron job '{}' {}", id, if req.enabled { "enabled" } else { "disabled" })
                    }))
                    .into_response()
            }
            Err(e) => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": e.to_string()
                })),
            )
                .into_response(),
        },
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Time travel not initialized"
            })),
        )
            .into_response(),
    }
}

/// Request to create a mutation rule
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMutationRuleRequest {
    /// Rule ID
    pub id: String,
    /// Entity name
    pub entity_name: String,
    /// Trigger configuration
    pub trigger: MutationTrigger,
    /// Operation configuration
    pub operation: MutationOperation,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Optional condition (JSONPath)
    #[serde(default)]
    pub condition: Option<String>,
}

/// List all mutation rules
pub async fn list_mutation_rules() -> impl IntoResponse {
    match get_mutation_rule_manager() {
        Some(manager) => {
            let rules = manager.list_rules().await;
            Json(serde_json::json!({
                "success": true,
                "rules": rules
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Mutation rule manager not initialized"
            })),
        )
            .into_response(),
    }
}

/// Get a specific mutation rule
pub async fn get_mutation_rule(Path(id): Path<String>) -> impl IntoResponse {
    match get_mutation_rule_manager() {
        Some(manager) => match manager.get_rule(&id).await {
            Some(rule) => Json(serde_json::json!({
                "success": true,
                "rule": rule
            }))
            .into_response(),
            None => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Mutation rule '{}' not found", id)
                })),
            )
                .into_response(),
        },
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Mutation rule manager not initialized"
            })),
        )
            .into_response(),
    }
}

/// Create a new mutation rule
pub async fn create_mutation_rule(Json(req): Json<CreateMutationRuleRequest>) -> impl IntoResponse {
    match get_mutation_rule_manager() {
        Some(manager) => {
            let mut rule = MutationRule::new(
                req.id.clone(),
                req.entity_name.clone(),
                req.trigger.clone(),
                req.operation.clone(),
            );
            if let Some(desc) = req.description {
                rule.description = Some(desc);
            }
            if let Some(cond) = req.condition {
                rule.condition = Some(cond);
            }

            match manager.add_rule(rule).await {
                Ok(_) => {
                    info!("Created mutation rule '{}'", req.id);
                    Json(serde_json::json!({
                        "success": true,
                        "message": format!("Mutation rule '{}' created", req.id)
                    }))
                    .into_response()
                }
                Err(e) => (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": e.to_string().to_string()
                    })),
                )
                    .into_response(),
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Mutation rule manager not initialized"
            })),
        )
            .into_response(),
    }
}

/// Delete a mutation rule
pub async fn delete_mutation_rule(Path(id): Path<String>) -> impl IntoResponse {
    match get_mutation_rule_manager() {
        Some(manager) => {
            let removed = manager.remove_rule(&id).await;
            if removed {
                info!("Deleted mutation rule '{}'", id);
                Json(serde_json::json!({
                    "success": true,
                    "message": format!("Mutation rule '{}' deleted", id)
                }))
                .into_response()
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": format!("Mutation rule '{}' not found", id)
                    })),
                )
                    .into_response()
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Mutation rule manager not initialized"
            })),
        )
            .into_response(),
    }
}

/// Enable or disable a mutation rule
#[derive(Debug, Serialize, Deserialize)]
pub struct SetMutationRuleEnabledRequest {
    /// Whether to enable the rule
    pub enabled: bool,
}

pub async fn set_mutation_rule_enabled(
    Path(id): Path<String>,
    Json(req): Json<SetMutationRuleEnabledRequest>,
) -> impl IntoResponse {
    match get_mutation_rule_manager() {
        Some(manager) => match manager.set_rule_enabled(&id, req.enabled).await {
            Ok(_) => {
                info!(
                    "Mutation rule '{}' {}",
                    id,
                    if req.enabled { "enabled" } else { "disabled" }
                );
                Json(serde_json::json!({
                        "success": true,
                        "message": format!("Mutation rule '{}' {}", id, if req.enabled { "enabled" } else { "disabled" })
                    }))
                    .into_response()
            }
            Err(e) => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": e.to_string()
                })),
            )
                .into_response(),
        },
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Mutation rule manager not initialized"
            })),
        )
            .into_response(),
    }
}

/// Parse a duration string like "2h", "30m", "10s", "1d", "+1h", "+1 week", "1month", "1year"
///
/// Supports:
/// - Standard formats: "1h", "30m", "2d", etc.
/// - With + prefix: "+1h", "+1 week", "+2d"
/// - Week units: "1week", "1 week", "2weeks", "+1week"
/// - Month/year: "1month", "1year"
fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty duration string".to_string());
    }

    // Strip leading + or - (for relative time notation)
    let s = s.strip_prefix('+').unwrap_or(s);
    let s = s.strip_prefix('-').unwrap_or(s);

    // Handle weeks (with or without space)
    if s.ends_with("week") || s.ends_with("weeks") || s.ends_with(" week") || s.ends_with(" weeks")
    {
        let num_str = s
            .trim_end_matches("week")
            .trim_end_matches("weeks")
            .trim_end_matches(" week")
            .trim_end_matches(" weeks")
            .trim();
        let amount: i64 =
            num_str.parse().map_err(|e| format!("Invalid number for weeks: {}", e))?;
        // 1 week = 7 days
        return Ok(Duration::days(amount * 7));
    }

    // Handle months and years (approximate)
    if s.ends_with("month")
        || s.ends_with("months")
        || s.ends_with(" month")
        || s.ends_with(" months")
    {
        let num_str = s
            .trim_end_matches("month")
            .trim_end_matches("months")
            .trim_end_matches(" month")
            .trim_end_matches(" months")
            .trim();
        let amount: i64 =
            num_str.parse().map_err(|e| format!("Invalid number for months: {}", e))?;
        // Approximate: 1 month = 30 days
        return Ok(Duration::days(amount * 30));
    }
    if s.ends_with('y')
        || s.ends_with("year")
        || s.ends_with("years")
        || s.ends_with(" year")
        || s.ends_with(" years")
    {
        let num_str = s
            .trim_end_matches('y')
            .trim_end_matches("year")
            .trim_end_matches("years")
            .trim_end_matches(" year")
            .trim_end_matches(" years")
            .trim();
        let amount: i64 =
            num_str.parse().map_err(|e| format!("Invalid number for years: {}", e))?;
        // Approximate: 1 year = 365 days
        return Ok(Duration::days(amount * 365));
    }

    // Extract number and unit for standard durations
    let (num_str, unit) = if let Some(pos) = s.chars().position(|c| !c.is_numeric() && c != '-') {
        (&s[..pos], &s[pos..].trim())
    } else {
        return Err("No unit specified (use s, m, h, d, week, month, or year)".to_string());
    };

    let amount: i64 = num_str.parse().map_err(|e| format!("Invalid number: {}", e))?;

    match *unit {
        "s" | "sec" | "secs" | "second" | "seconds" => Ok(Duration::seconds(amount)),
        "m" | "min" | "mins" | "minute" | "minutes" => Ok(Duration::minutes(amount)),
        "h" | "hr" | "hrs" | "hour" | "hours" => Ok(Duration::hours(amount)),
        "d" | "day" | "days" => Ok(Duration::days(amount)),
        "w" | "week" | "weeks" => Ok(Duration::days(amount * 7)),
        _ => Err(format!("Unknown unit: {}. Use s, m, h, d, week, month, or year", unit)),
    }
}

/// Parse a trigger time (ISO 8601 or relative like "+1h")
fn parse_trigger_time(s: &str, clock: Arc<VirtualClock>) -> Result<DateTime<Utc>, String> {
    let s = s.trim();

    // Check if it's a relative time (starts with + or -)
    if s.starts_with('+') || s.starts_with('-') {
        let duration = parse_duration(&s[1..])?;
        let current = clock.now();

        if s.starts_with('+') {
            Ok(current + duration)
        } else {
            Ok(current - duration)
        }
    } else {
        // Parse as ISO 8601
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| format!("Invalid ISO 8601 date: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("10s").unwrap(), Duration::seconds(10));
        assert_eq!(parse_duration("30m").unwrap(), Duration::minutes(30));
        assert_eq!(parse_duration("2h").unwrap(), Duration::hours(2));
        assert_eq!(parse_duration("1d").unwrap(), Duration::days(1));

        assert!(parse_duration("").is_err());
        assert!(parse_duration("10").is_err());
        assert!(parse_duration("10x").is_err());
    }

    #[test]
    fn test_parse_trigger_time_relative() {
        let clock = Arc::new(VirtualClock::new());
        let now = Utc::now();
        clock.enable_and_set(now);

        let future = parse_trigger_time("+1h", clock.clone()).unwrap();
        assert!((future - now - Duration::hours(1)).num_seconds().abs() < 1);

        let past = parse_trigger_time("-30m", clock.clone()).unwrap();
        assert!((past - now + Duration::minutes(30)).num_seconds().abs() < 1);
    }
}
