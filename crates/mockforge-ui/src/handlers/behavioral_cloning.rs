//! Behavioral cloning handlers for Admin UI
//!
//! This module provides API endpoints for managing flows and scenarios
//! in the Admin UI.

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use mockforge_recorder::behavioral_cloning::{
    flow_recorder::{FlowRecorder, FlowRecordingConfig},
    FlowCompiler, ScenarioStorage,
};
use mockforge_recorder::RecorderDatabase;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::handlers::AdminState;
use crate::models::ApiResponse;

/// Get list of flows
pub async fn get_flows(
    State(_state): State<AdminState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Value>> {
    // Get database path from config or use default
    let db_path = params
        .get("db_path")
        .cloned()
        .unwrap_or_else(|| "./mockforge-recordings.db".to_string());

    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(50);

    match RecorderDatabase::new(&db_path).await {
        Ok(db) => {
            let recorder = FlowRecorder::new(db.clone(), FlowRecordingConfig::default());
            match recorder.list_flows(Some(limit)).await {
                Ok(flows) => {
                    let flows_json: Vec<Value> = flows
                        .into_iter()
                        .map(|flow| {
                            json!({
                                "id": flow.id,
                                "name": flow.name,
                                "description": flow.description,
                                "created_at": flow.created_at,
                                "tags": flow.tags,
                                "step_count": flow.steps.len(),
                            })
                        })
                        .collect();

                    Json(ApiResponse {
                        success: true,
                        data: Some(json!({
                            "flows": flows_json,
                            "total": flows_json.len()
                        })),
                        error: None,
                        timestamp: chrono::Utc::now(),
                    })
                }
                Err(e) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to list flows: {}", e)),
                    timestamp: chrono::Utc::now(),
                }),
            }
        }
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Failed to connect to database: {}", e)),
            timestamp: chrono::Utc::now(),
        }),
    }
}

/// Get flow details with timeline
pub async fn get_flow(
    State(_state): State<AdminState>,
    Path(flow_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Value>> {
    let db_path = params
        .get("db_path")
        .cloned()
        .unwrap_or_else(|| "./mockforge-recordings.db".to_string());

    match RecorderDatabase::new(&db_path).await {
        Ok(db) => {
            let recorder = FlowRecorder::new(db.clone(), FlowRecordingConfig::default());
            match recorder.get_flow(&flow_id).await {
                Ok(Some(flow)) => {
                    // Build timeline data
                    let steps: Vec<Value> = flow
                        .steps
                        .iter()
                        .enumerate()
                        .map(|(idx, step)| {
                            json!({
                                "index": idx,
                                "request_id": step.request_id,
                                "step_label": step.step_label,
                                "timing_ms": step.timing_ms,
                            })
                        })
                        .collect();

                    Json(ApiResponse {
                        success: true,
                        data: Some(json!({
                            "id": flow.id,
                            "name": flow.name,
                            "description": flow.description,
                            "created_at": flow.created_at,
                            "tags": flow.tags,
                            "steps": steps,
                            "step_count": steps.len(),
                        })),
                        error: None,
                        timestamp: chrono::Utc::now(),
                    })
                }
                Ok(None) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Flow not found: {}", flow_id)),
                    timestamp: chrono::Utc::now(),
                }),
                Err(e) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to get flow: {}", e)),
                    timestamp: chrono::Utc::now(),
                }),
            }
        }
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Failed to connect to database: {}", e)),
            timestamp: chrono::Utc::now(),
        }),
    }
}

/// Tag a flow
#[derive(Deserialize)]
pub struct TagFlowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

pub async fn tag_flow(
    State(_state): State<AdminState>,
    Path(flow_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(payload): Json<TagFlowRequest>,
) -> Json<ApiResponse<Value>> {
    let db_path = params
        .get("db_path")
        .cloned()
        .unwrap_or_else(|| "./mockforge-recordings.db".to_string());

    match RecorderDatabase::new(&db_path).await {
        Ok(db) => {
            let recorder = FlowRecorder::new(db.clone(), FlowRecordingConfig::default());
            match db
                .update_flow_metadata(
                    &flow_id,
                    payload.name.as_deref(),
                    payload.description.as_deref(),
                    Some(&payload.tags.unwrap_or_default()),
                )
                .await
            {
                Ok(_) => Json(ApiResponse {
                    success: true,
                    data: Some(json!({
                        "message": "Flow tagged successfully",
                        "flow_id": flow_id
                    })),
                    error: None,
                    timestamp: chrono::Utc::now(),
                }),
                Err(e) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to tag flow: {}", e)),
                    timestamp: chrono::Utc::now(),
                }),
            }
        }
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Failed to connect to database: {}", e)),
            timestamp: chrono::Utc::now(),
        }),
    }
}

/// Compile flow to scenario
#[derive(Deserialize)]
pub struct CompileFlowRequest {
    pub scenario_name: String,
    pub flex_mode: Option<bool>,
}

pub async fn compile_flow(
    State(_state): State<AdminState>,
    Path(flow_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(payload): Json<CompileFlowRequest>,
) -> Json<ApiResponse<Value>> {
    let db_path = params
        .get("db_path")
        .cloned()
        .unwrap_or_else(|| "./mockforge-recordings.db".to_string());

    match RecorderDatabase::new(&db_path).await {
        Ok(db) => {
            let recorder = FlowRecorder::new(db.clone(), FlowRecordingConfig::default());
            match recorder.get_flow(&flow_id).await {
                Ok(Some(flow)) => {
                    let compiler = FlowCompiler::new(db.clone());
                    let strict_mode = !payload.flex_mode.unwrap_or(false);
                    match compiler
                        .compile_flow(&flow, payload.scenario_name.clone(), strict_mode)
                        .await
                    {
                        Ok(scenario) => {
                            // Store the scenario
                            let storage = ScenarioStorage::new(db);
                            match storage.store_scenario_auto_version(&scenario).await {
                                Ok(version) => Json(ApiResponse {
                                    success: true,
                                    data: Some(json!({
                                        "scenario_id": scenario.id,
                                        "scenario_name": scenario.name,
                                        "version": version,
                                        "message": "Flow compiled successfully"
                                    })),
                                    error: None,
                                    timestamp: chrono::Utc::now(),
                                }),
                                Err(e) => Json(ApiResponse {
                                    success: false,
                                    data: None,
                                    error: Some(format!("Failed to store scenario: {}", e)),
                                    timestamp: chrono::Utc::now(),
                                }),
                            }
                        }
                        Err(e) => Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some(format!("Failed to compile flow: {}", e)),
                            timestamp: chrono::Utc::now(),
                        }),
                    }
                }
                Ok(None) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Flow not found: {}", flow_id)),
                    timestamp: chrono::Utc::now(),
                }),
                Err(e) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to get flow: {}", e)),
                    timestamp: chrono::Utc::now(),
                }),
            }
        }
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Failed to connect to database: {}", e)),
            timestamp: chrono::Utc::now(),
        }),
    }
}

/// Get list of scenarios
pub async fn get_scenarios(
    State(_state): State<AdminState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Value>> {
    let db_path = params
        .get("db_path")
        .cloned()
        .unwrap_or_else(|| "./mockforge-recordings.db".to_string());

    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(50);

    match RecorderDatabase::new(&db_path).await {
        Ok(db) => {
            let storage = ScenarioStorage::new(db);
            match storage.list_scenarios(Some(limit)).await {
                Ok(scenarios) => {
                    let scenarios_json: Vec<Value> = scenarios
                        .into_iter()
                        .map(|s| {
                            json!({
                                "id": s.id,
                                "name": s.name,
                                "version": s.version,
                                "description": s.description,
                                "created_at": s.created_at,
                                "updated_at": s.updated_at,
                                "tags": s.tags,
                            })
                        })
                        .collect();

                    Json(ApiResponse {
                        success: true,
                        data: Some(json!({
                            "scenarios": scenarios_json,
                            "total": scenarios_json.len()
                        })),
                        error: None,
                        timestamp: chrono::Utc::now(),
                    })
                }
                Err(e) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to list scenarios: {}", e)),
                    timestamp: chrono::Utc::now(),
                }),
            }
        }
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Failed to connect to database: {}", e)),
            timestamp: chrono::Utc::now(),
        }),
    }
}

/// Get scenario details
pub async fn get_scenario(
    State(_state): State<AdminState>,
    Path(scenario_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Value>> {
    let db_path = params
        .get("db_path")
        .cloned()
        .unwrap_or_else(|| "./mockforge-recordings.db".to_string());

    match RecorderDatabase::new(&db_path).await {
        Ok(db) => {
            let storage = ScenarioStorage::new(db);
            match storage.get_scenario(&scenario_id).await {
                Ok(Some(scenario)) => {
                    let steps: Vec<Value> = scenario
                        .steps
                        .iter()
                        .map(|step| {
                            json!({
                                "step_id": step.step_id,
                                "label": step.label,
                                "method": step.request.method,
                                "path": step.request.path,
                                "status_code": step.response.status_code,
                                "timing_ms": step.timing_ms,
                            })
                        })
                        .collect();

                    Json(ApiResponse {
                        success: true,
                        data: Some(json!({
                            "id": scenario.id,
                            "name": scenario.name,
                            "description": scenario.description,
                            "strict_mode": scenario.strict_mode,
                            "steps": steps,
                            "step_count": steps.len(),
                            "state_variables": scenario.state_variables.len(),
                        })),
                        error: None,
                        timestamp: chrono::Utc::now(),
                    })
                }
                Ok(None) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Scenario not found: {}", scenario_id)),
                    timestamp: chrono::Utc::now(),
                }),
                Err(e) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to get scenario: {}", e)),
                    timestamp: chrono::Utc::now(),
                }),
            }
        }
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Failed to connect to database: {}", e)),
            timestamp: chrono::Utc::now(),
        }),
    }
}

/// Export scenario
pub async fn export_scenario(
    State(_state): State<AdminState>,
    Path(scenario_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Value>> {
    let db_path = params
        .get("db_path")
        .cloned()
        .unwrap_or_else(|| "./mockforge-recordings.db".to_string());

    let format = params.get("format").cloned().unwrap_or_else(|| "yaml".to_string());

    match RecorderDatabase::new(&db_path).await {
        Ok(db) => {
            let storage = ScenarioStorage::new(db);
            match storage.export_scenario(&scenario_id, &format).await {
                Ok(content) => Json(ApiResponse {
                    success: true,
                    data: Some(json!({
                        "scenario_id": scenario_id,
                        "format": format,
                        "content": content,
                    })),
                    error: None,
                    timestamp: chrono::Utc::now(),
                }),
                Err(e) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to export scenario: {}", e)),
                    timestamp: chrono::Utc::now(),
                }),
            }
        }
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Failed to connect to database: {}", e)),
            timestamp: chrono::Utc::now(),
        }),
    }
}
