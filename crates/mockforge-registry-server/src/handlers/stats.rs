//! Statistics handlers

use axum::{extract::State, Json};
use serde_json::Value;

use crate::{error::ApiResult, AppState};

pub async fn get_stats(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let total_plugins = state.db.get_total_plugins().await?;
    let total_downloads = state.db.get_total_downloads().await?;
    let total_users = state.db.get_total_users().await?;

    Ok(Json(serde_json::json!({
        "total_plugins": total_plugins,
        "total_downloads": total_downloads,
        "total_users": total_users
    })))
}
