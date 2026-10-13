//! Snapshot storage, filtering and typed provider routes.

use super::{comparison::compare_snapshot_objects, models::*, SnapshotStateProvider};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use std::{collections::HashMap, sync::Arc};

/// In-memory storage for snapshots
type SnapshotStorage = Arc<tokio::sync::RwLock<HashMap<String, MockSnapshot>>>;

/// Global snapshot storage
fn snapshot_store() -> &'static SnapshotStorage {
    static STORE: std::sync::OnceLock<SnapshotStorage> = std::sync::OnceLock::new();
    STORE.get_or_init(|| Arc::new(tokio::sync::RwLock::new(HashMap::new())))
}

/// Create a snapshot of current mock server state
async fn create_snapshot<S: SnapshotStateProvider>(
    State(state): State<S>,
    Json(request): Json<CreateSnapshotRequest>,
) -> Result<Json<MockSnapshot>, StatusCode> {
    let snapshot_items = state.snapshot_items().await;

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

    // Store snapshot in memory
    let mut store = snapshot_store().write().await;
    store.insert(snapshot.id.clone(), snapshot.clone());

    Ok(Json(snapshot))
}

/// Get a snapshot by ID
async fn get_snapshot<S: SnapshotStateProvider>(
    Path(snapshot_id): Path<String>,
    State(_state): State<S>,
) -> Result<Json<MockSnapshot>, StatusCode> {
    let store = snapshot_store().read().await;
    store.get(&snapshot_id).cloned().map(Json).ok_or(StatusCode::NOT_FOUND)
}

/// List all snapshots
async fn list_snapshots<S: SnapshotStateProvider>(
    Query(params): Query<HashMap<String, String>>,
    State(_state): State<S>,
) -> Result<Json<Vec<MockSnapshot>>, StatusCode> {
    let store = snapshot_store().read().await;
    let mut snapshots: Vec<MockSnapshot> = store
        .values()
        .filter(|s| {
            // Filter by environment_id if provided
            if let Some(env_id) = params.get("environment_id") {
                if s.environment_id.as_deref() != Some(env_id.as_str()) {
                    return false;
                }
            }
            // Filter by persona_id if provided
            if let Some(persona_id) = params.get("persona_id") {
                if s.persona_id.as_deref() != Some(persona_id.as_str()) {
                    return false;
                }
            }
            // Filter by scenario_id if provided
            if let Some(scenario_id) = params.get("scenario_id") {
                if s.scenario_id.as_deref() != Some(scenario_id.as_str()) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    // Sort by timestamp descending (most recent first)
    snapshots.sort_by_key(|b| std::cmp::Reverse(b.timestamp));

    Ok(Json(snapshots))
}

/// Compare two snapshots
async fn compare_snapshots<S: SnapshotStateProvider>(
    State(state): State<S>,
    Json(request): Json<CompareSnapshotsRequest>,
) -> Result<Json<SnapshotDiff>, StatusCode> {
    let store = snapshot_store().read().await;

    // Load left snapshot: by ID if provided, otherwise create from current mocks
    let left_snapshot = if let Some(ref id) = request.left_snapshot_id {
        store.get(id).cloned().ok_or(StatusCode::NOT_FOUND)?
    } else {
        let current_mocks = state.snapshot_items().await;
        create_snapshot_from_items(
            current_mocks,
            request.left_environment_id.clone(),
            request.left_persona_id.clone(),
            request.left_scenario_id.clone(),
            request.left_reality_level,
        )
    };

    // Load right snapshot: by ID if provided, otherwise create from current mocks
    let right_snapshot = if let Some(ref id) = request.right_snapshot_id {
        store.get(id).cloned().ok_or(StatusCode::NOT_FOUND)?
    } else {
        let current_mocks = state.snapshot_items().await;
        create_snapshot_from_items(
            current_mocks,
            request.right_environment_id.clone(),
            request.right_persona_id.clone(),
            request.right_scenario_id.clone(),
            request.right_reality_level,
        )
    };

    // Compare snapshots
    let diff = compare_snapshot_objects(&left_snapshot, &right_snapshot);

    Ok(Json(diff))
}

/// Helper to create a snapshot from mocks
fn create_snapshot_from_items(
    snapshot_items: Vec<MockSnapshotItem>,
    environment_id: Option<String>,
    persona_id: Option<String>,
    scenario_id: Option<String>,
    reality_level: Option<f64>,
) -> MockSnapshot {
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

/// Build the snapshot diff router
pub fn snapshot_diff_router<S: SnapshotStateProvider>(state: S) -> Router<S> {
    Router::new()
        .route("/snapshots", post(create_snapshot::<S>))
        .route("/snapshots", get(list_snapshots::<S>))
        .route("/snapshots/{id}", get(get_snapshot::<S>))
        .route("/snapshots/compare", post(compare_snapshots::<S>))
        .with_state(state)
}
