//! Snapshot Diff API Handlers
//!
//! Provides endpoints for comparing mock server snapshots across different
//! environments, personas, scenarios, or "realities" (reality continuum levels).
//!
//! This enables side-by-side visualization for demos and debugging.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::management::{ManagementState, MockConfig};

/// Snapshot of mock server state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSnapshot {
    /// Snapshot ID
    pub id: String,
    /// Timestamp when snapshot was taken
    pub timestamp: i64,
    /// Environment ID (if applicable)
    pub environment_id: Option<String>,
    /// Persona ID (if applicable)
    pub persona_id: Option<String>,
    /// Scenario ID (if applicable)
    pub scenario_id: Option<String>,
    /// Reality level (0.0-1.0, if applicable)
    pub reality_level: Option<f64>,
    /// All mocks in this snapshot
    pub mocks: Vec<MockSnapshotItem>,
    /// Metadata about the snapshot
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Individual mock item in a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSnapshotItem {
    /// Mock ID
    pub id: String,
    /// HTTP method
    pub method: String,
    /// Path pattern
    pub path: String,
    /// Response status code
    pub status_code: u16,
    /// Response body
    pub response_body: serde_json::Value,
    /// Response headers
    pub response_headers: Option<HashMap<String, String>>,
    /// Mock configuration
    pub config: serde_json::Value,
}

/// Comparison between two snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    /// Left snapshot (baseline)
    pub left: MockSnapshot,
    /// Right snapshot (comparison)
    pub right: MockSnapshot,
    /// Differences found
    pub differences: Vec<Difference>,
    /// Summary statistics
    pub summary: DiffSummary,
}

/// Individual difference between snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Difference {
    /// Type of difference
    pub diff_type: DifferenceType,
    /// Mock ID (if applicable)
    pub mock_id: Option<String>,
    /// Path of the mock
    pub path: String,
    /// Method of the mock
    pub method: String,
    /// Description of the difference
    pub description: String,
    /// Left value (if applicable)
    pub left_value: Option<serde_json::Value>,
    /// Right value (if applicable)
    pub right_value: Option<serde_json::Value>,
    /// Field path where difference occurs (for nested differences)
    pub field_path: Option<String>,
}

/// Type of difference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DifferenceType {
    /// Mock exists in left but not right
    MissingInRight,
    /// Mock exists in right but not left
    MissingInLeft,
    /// Status code differs
    StatusCodeMismatch,
    /// Response body differs
    BodyMismatch,
    /// Response headers differ
    HeadersMismatch,
    /// Configuration differs
    ConfigMismatch,
}

/// Summary statistics for a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Total mocks in left snapshot
    pub left_total: usize,
    /// Total mocks in right snapshot
    pub right_total: usize,
    /// Number of differences found
    pub differences_count: usize,
    /// Number of mocks only in left
    pub only_in_left: usize,
    /// Number of mocks only in right
    pub only_in_right: usize,
    /// Number of mocks with differences
    pub mocks_with_differences: usize,
}

/// Request to create a snapshot
#[derive(Debug, Deserialize)]
pub struct CreateSnapshotRequest {
    /// Environment ID (optional)
    pub environment_id: Option<String>,
    /// Persona ID (optional)
    pub persona_id: Option<String>,
    /// Scenario ID (optional)
    pub scenario_id: Option<String>,
    /// Reality level (optional, 0.0-1.0)
    pub reality_level: Option<f64>,
    /// Metadata to attach
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Request to compare snapshots
#[derive(Debug, Deserialize)]
pub struct CompareSnapshotsRequest {
    /// Left snapshot ID (or use environment/persona/scenario)
    pub left_snapshot_id: Option<String>,
    /// Right snapshot ID (or use environment/persona/scenario)
    pub right_snapshot_id: Option<String>,
    /// Left environment ID (alternative to snapshot_id)
    pub left_environment_id: Option<String>,
    /// Right environment ID (alternative to snapshot_id)
    pub right_environment_id: Option<String>,
    /// Left persona ID (alternative to snapshot_id)
    pub left_persona_id: Option<String>,
    /// Right persona ID (alternative to snapshot_id)
    pub right_persona_id: Option<String>,
    /// Left scenario ID (alternative to snapshot_id)
    pub left_scenario_id: Option<String>,
    /// Right scenario ID (alternative to snapshot_id)
    pub right_scenario_id: Option<String>,
    /// Left reality level (alternative to snapshot_id)
    pub left_reality_level: Option<f64>,
    /// Right reality level (alternative to snapshot_id)
    pub right_reality_level: Option<f64>,
}

/// In-memory storage for snapshots (in production, use a database)
type SnapshotStorage = Arc<tokio::sync::RwLock<HashMap<String, MockSnapshot>>>;

/// Create a snapshot of current mock server state
async fn create_snapshot(
    State(state): State<ManagementState>,
    Json(request): Json<CreateSnapshotRequest>,
) -> Result<Json<MockSnapshot>, StatusCode> {
    // Get current mocks from the state
    let mocks = state.mocks.read().await.clone();

    // Convert mocks to snapshot items
    let snapshot_items: Vec<MockSnapshotItem> = mocks
        .iter()
        .map(|mock| MockSnapshotItem {
            id: mock.id.clone(),
            method: mock.method.clone(),
            path: mock.path.clone(),
            status_code: mock.status_code.unwrap_or(200),
            response_body: mock.response.body.clone(),
            response_headers: mock.response.headers.clone(),
            config: serde_json::to_value(mock).unwrap_or_default(),
        })
        .collect();

    // Create snapshot
    let snapshot = MockSnapshot {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        environment_id: request.environment_id,
        persona_id: request.persona_id,
        scenario_id: request.scenario_id,
        reality_level: request.reality_level,
        mocks: snapshot_items,
        metadata: request.metadata,
    };

    // Store snapshot (in production, persist to database)
    // For now, we'll use a simple in-memory storage
    // In a real implementation, this would be stored in the ManagementState

    Ok(Json(snapshot))
}

/// Get a snapshot by ID
async fn get_snapshot(
    Path(_snapshot_id): Path<String>,
    State(_state): State<ManagementState>,
) -> Result<Json<MockSnapshot>, StatusCode> {
    // In production, retrieve from database
    // For now, return error (snapshots need to be stored)
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// List all snapshots
async fn list_snapshots(
    Query(params): Query<HashMap<String, String>>,
    State(_state): State<ManagementState>,
) -> Result<Json<Vec<MockSnapshot>>, StatusCode> {
    // In production, retrieve from database with filters
    // For now, return empty list
    Ok(Json(vec![]))
}

/// Compare two snapshots
async fn compare_snapshots(
    State(state): State<ManagementState>,
    Json(request): Json<CompareSnapshotsRequest>,
) -> Result<Json<SnapshotDiff>, StatusCode> {
    // Get current mocks as baseline
    let current_mocks = state.mocks.read().await.clone();

    // For now, we'll create snapshots on-the-fly for comparison
    // In production, you'd load from stored snapshots

    let left_snapshot = create_snapshot_from_mocks(
        &current_mocks,
        request.left_environment_id.clone(),
        request.left_persona_id.clone(),
        request.left_scenario_id.clone(),
        request.left_reality_level,
    );

    let right_snapshot = create_snapshot_from_mocks(
        &current_mocks,
        request.right_environment_id.clone(),
        request.right_persona_id.clone(),
        request.right_scenario_id.clone(),
        request.right_reality_level,
    );

    // Compare snapshots
    let diff = compare_snapshot_objects(&left_snapshot, &right_snapshot);

    Ok(Json(diff))
}

/// Helper to create a snapshot from mocks
fn create_snapshot_from_mocks(
    mocks: &[MockConfig],
    environment_id: Option<String>,
    persona_id: Option<String>,
    scenario_id: Option<String>,
    reality_level: Option<f64>,
) -> MockSnapshot {
    let snapshot_items: Vec<MockSnapshotItem> = mocks
        .iter()
        .map(|mock| MockSnapshotItem {
            id: mock.id.clone(),
            method: mock.method.clone(),
            path: mock.path.clone(),
            status_code: mock.status_code.unwrap_or(200),
            response_body: mock.response.body.clone(),
            response_headers: mock.response.headers.clone(),
            config: serde_json::to_value(mock).unwrap_or_default(),
        })
        .collect();

    MockSnapshot {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        environment_id,
        persona_id,
        scenario_id,
        reality_level,
        mocks: snapshot_items,
        metadata: HashMap::new(),
    }
}

/// Compare two snapshot objects
fn compare_snapshot_objects(left: &MockSnapshot, right: &MockSnapshot) -> SnapshotDiff {
    let mut differences = Vec::new();

    // Create maps for quick lookup
    let left_map: HashMap<String, &MockSnapshotItem> =
        left.mocks.iter().map(|m| (format!("{}:{}", m.method, m.path), m)).collect();

    let right_map: HashMap<String, &MockSnapshotItem> =
        right.mocks.iter().map(|m| (format!("{}:{}", m.method, m.path), m)).collect();

    // Find mocks only in left
    for (key, left_mock) in &left_map {
        if !right_map.contains_key(key) {
            differences.push(Difference {
                diff_type: DifferenceType::MissingInRight,
                mock_id: Some(left_mock.id.clone()),
                path: left_mock.path.clone(),
                method: left_mock.method.clone(),
                description: format!(
                    "Mock {} {} exists in left but not in right",
                    left_mock.method, left_mock.path
                ),
                left_value: Some(serde_json::to_value(left_mock).unwrap_or_default()),
                right_value: None,
                field_path: None,
            });
        }
    }

    // Find mocks only in right
    for (key, right_mock) in &right_map {
        if !left_map.contains_key(key) {
            differences.push(Difference {
                diff_type: DifferenceType::MissingInLeft,
                mock_id: Some(right_mock.id.clone()),
                path: right_mock.path.clone(),
                method: right_mock.method.clone(),
                description: format!(
                    "Mock {} {} exists in right but not in left",
                    right_mock.method, right_mock.path
                ),
                left_value: None,
                right_value: Some(serde_json::to_value(right_mock).unwrap_or_default()),
                field_path: None,
            });
        }
    }

    // Compare common mocks
    for (key, left_mock) in &left_map {
        if let Some(right_mock) = right_map.get(key) {
            // Compare status codes
            if left_mock.status_code != right_mock.status_code {
                differences.push(Difference {
                    diff_type: DifferenceType::StatusCodeMismatch,
                    mock_id: Some(left_mock.id.clone()),
                    path: left_mock.path.clone(),
                    method: left_mock.method.clone(),
                    description: format!(
                        "Status code differs: {} vs {}",
                        left_mock.status_code, right_mock.status_code
                    ),
                    left_value: Some(serde_json::json!(left_mock.status_code)),
                    right_value: Some(serde_json::json!(right_mock.status_code)),
                    field_path: Some("status_code".to_string()),
                });
            }

            // Compare response bodies (deep comparison)
            if left_mock.response_body != right_mock.response_body {
                differences.push(Difference {
                    diff_type: DifferenceType::BodyMismatch,
                    mock_id: Some(left_mock.id.clone()),
                    path: left_mock.path.clone(),
                    method: left_mock.method.clone(),
                    description: format!(
                        "Response body differs for {} {}",
                        left_mock.method, left_mock.path
                    ),
                    left_value: Some(left_mock.response_body.clone()),
                    right_value: Some(right_mock.response_body.clone()),
                    field_path: Some("response_body".to_string()),
                });
            }

            // Compare headers
            if left_mock.response_headers != right_mock.response_headers {
                differences.push(Difference {
                    diff_type: DifferenceType::HeadersMismatch,
                    mock_id: Some(left_mock.id.clone()),
                    path: left_mock.path.clone(),
                    method: left_mock.method.clone(),
                    description: format!(
                        "Response headers differ for {} {}",
                        left_mock.method, left_mock.path
                    ),
                    left_value: left_mock
                        .response_headers
                        .as_ref()
                        .map(|h| serde_json::to_value(h).unwrap_or_default()),
                    right_value: right_mock
                        .response_headers
                        .as_ref()
                        .map(|h| serde_json::to_value(h).unwrap_or_default()),
                    field_path: Some("response_headers".to_string()),
                });
            }
        }
    }

    // Calculate summary
    let only_in_left = differences
        .iter()
        .filter(|d| matches!(d.diff_type, DifferenceType::MissingInRight))
        .count();
    let only_in_right = differences
        .iter()
        .filter(|d| matches!(d.diff_type, DifferenceType::MissingInLeft))
        .count();
    let mocks_with_differences: std::collections::HashSet<String> =
        differences.iter().filter_map(|d| d.mock_id.clone()).collect();

    let summary = DiffSummary {
        left_total: left.mocks.len(),
        right_total: right.mocks.len(),
        differences_count: differences.len(),
        only_in_left,
        only_in_right,
        mocks_with_differences: mocks_with_differences.len(),
    };

    SnapshotDiff {
        left: left.clone(),
        right: right.clone(),
        differences,
        summary,
    }
}

/// Build the snapshot diff router
pub fn snapshot_diff_router(state: ManagementState) -> Router<ManagementState> {
    Router::new()
        .route("/snapshots", post(create_snapshot))
        .route("/snapshots", get(list_snapshots))
        .route("/snapshots/{id}", get(get_snapshot))
        .route("/snapshots/compare", post(compare_snapshots))
        .with_state(state)
}
