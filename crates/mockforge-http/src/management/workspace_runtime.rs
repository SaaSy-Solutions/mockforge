//! Atomic publication of a workspace's namespaced dynamic HTTP mocks.
use super::{ManagementState, MockConfig};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

/// ID marker identifying mocks owned by workspace activation.
pub const WORKSPACE_MOCK_PREFIX: &str = "__workspace_live:";

pub(crate) async fn activate_workspace_mocks(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
    Json(mocks): Json<Vec<MockConfig>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let prefix = format!("{WORKSPACE_MOCK_PREFIX}{}:", urlencoding::encode(&id));
    let path_prefix = format!("/workspace/{}/", urlencoding::encode(&id));
    let mut ids = std::collections::HashSet::new();
    for mock in &mocks {
        if !mock.id.starts_with(&prefix)
            || !mock.path.starts_with(&path_prefix)
            || !ids.insert(&mock.id)
            || mock.status_code.is_some_and(|status| !(100..=599).contains(&status))
        {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let count = mocks.len();
    let mut current = state.mocks.write().await;
    current.retain(|mock| !mock.id.starts_with(&prefix));
    current.extend(mocks);
    Ok(Json(
        serde_json::json!({"workspace_id":id,"active":true,"routes":count,"path_prefix":path_prefix}),
    ))
}
