//! Statistics handlers

use axum::{extract::State, Json};
use serde_json::Value;

use crate::{error::ApiResult, AppState};

pub async fn get_stats(
    State(_state): State<AppState>,
) -> ApiResult<Json<Value>> {
    // TODO: Implement statistics
    Ok(Json(serde_json::json!({
        "total_plugins": 0,
        "total_downloads": 0,
        "total_users": 0
    })))
}
