use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use tracing::*;

use super::ManagementState;
use super::MockConfig;

/// List all mocks
pub(crate) async fn list_mocks(State(state): State<ManagementState>) -> Json<serde_json::Value> {
    let mocks = state.mocks.read().await;
    Json(serde_json::json!({
        "mocks": *mocks,
        "total": mocks.len(),
        "enabled": mocks.iter().filter(|m| m.enabled).count()
    }))
}

/// Get a specific mock by ID
pub(crate) async fn get_mock(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> Result<Json<MockConfig>, StatusCode> {
    let mocks = state.mocks.read().await;
    mocks
        .iter()
        .find(|m| m.id == id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Create a new mock
pub(crate) async fn create_mock(
    State(state): State<ManagementState>,
    Json(mut mock): Json<MockConfig>,
) -> Result<(StatusCode, Json<MockConfig>), StatusCode> {
    let mut mocks = state.mocks.write().await;

    // Generate ID if not provided
    if mock.id.is_empty() {
        mock.id = uuid::Uuid::new_v4().to_string();
    }

    // Check for duplicate ID
    if mocks.iter().any(|m| m.id == mock.id) {
        return Err(StatusCode::CONFLICT);
    }

    info!("Creating mock: {} {} {}", mock.method, mock.path, mock.id);

    // Invoke lifecycle hooks
    if let Some(hooks) = &state.lifecycle_hooks {
        let event = mockforge_core::lifecycle::MockLifecycleEvent::Created {
            id: mock.id.clone(),
            name: mock.name.clone(),
            config: serde_json::to_value(&mock).unwrap_or_default(),
        };
        hooks.invoke_mock_created(&event).await;
    }

    mocks.push(mock.clone());

    // Broadcast WebSocket event
    if let Some(tx) = &state.ws_broadcast {
        let _ = tx.send(crate::management_ws::MockEvent::mock_created(mock.clone()));
    }

    Ok((StatusCode::CREATED, Json(mock)))
}

/// Update an existing mock
pub(crate) async fn update_mock(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    Json(updated_mock): Json<MockConfig>,
) -> Result<Json<MockConfig>, StatusCode> {
    let mut mocks = state.mocks.write().await;

    let position = mocks.iter().position(|m| m.id == id).ok_or(StatusCode::NOT_FOUND)?;

    // Get old mock for comparison
    let old_mock = mocks[position].clone();

    info!("Updating mock: {}", id);
    mocks[position] = updated_mock.clone();

    // Invoke lifecycle hooks
    if let Some(hooks) = &state.lifecycle_hooks {
        let event = mockforge_core::lifecycle::MockLifecycleEvent::Updated {
            id: updated_mock.id.clone(),
            name: updated_mock.name.clone(),
            config: serde_json::to_value(&updated_mock).unwrap_or_default(),
        };
        hooks.invoke_mock_updated(&event).await;

        // Check if enabled state changed
        if old_mock.enabled != updated_mock.enabled {
            let state_event = if updated_mock.enabled {
                mockforge_core::lifecycle::MockLifecycleEvent::Enabled {
                    id: updated_mock.id.clone(),
                }
            } else {
                mockforge_core::lifecycle::MockLifecycleEvent::Disabled {
                    id: updated_mock.id.clone(),
                }
            };
            hooks.invoke_mock_state_changed(&state_event).await;
        }
    }

    // Broadcast WebSocket event
    if let Some(tx) = &state.ws_broadcast {
        let _ = tx.send(crate::management_ws::MockEvent::mock_updated(updated_mock.clone()));
    }

    Ok(Json(updated_mock))
}

/// Delete a mock
pub(crate) async fn delete_mock(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let mut mocks = state.mocks.write().await;

    let position = mocks.iter().position(|m| m.id == id).ok_or(StatusCode::NOT_FOUND)?;

    // Get mock info before deletion for lifecycle hooks
    let deleted_mock = mocks[position].clone();

    info!("Deleting mock: {}", id);
    mocks.remove(position);

    // Invoke lifecycle hooks
    if let Some(hooks) = &state.lifecycle_hooks {
        let event = mockforge_core::lifecycle::MockLifecycleEvent::Deleted {
            id: deleted_mock.id.clone(),
            name: deleted_mock.name.clone(),
        };
        hooks.invoke_mock_deleted(&event).await;
    }

    // Broadcast WebSocket event
    if let Some(tx) = &state.ws_broadcast {
        let _ = tx.send(crate::management_ws::MockEvent::mock_deleted(id.clone()));
    }

    Ok(StatusCode::NO_CONTENT)
}
