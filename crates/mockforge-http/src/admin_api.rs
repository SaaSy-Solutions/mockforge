//! Admin API (Axum): /__admin/api/state and /__admin/api/replay
use axum::{routing::get, Router, extract::State, Json};
use serde_json::{json, Value};
use crate::replay_listing;

#[derive(Clone)]
pub struct AppState {
    pub started_at: std::time::Instant,
    pub profiles_count_op: usize,
    pub profiles_count_tag: usize,
    pub fixtures_root: String,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/state", get(state_handler))
        .route("/replay", get(replay_handler))
        .with_state(state)
}

async fn state_handler(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "profiles": { "operations": state.profiles_count_op, "tags": state.profiles_count_tag },
        "uptime_sec": state.started_at.elapsed().as_secs()
    }))
}

async fn replay_handler(State(state): State<AppState>) -> Json<Value> {
    let items = replay_listing::list_all(&state.fixtures_root).unwrap_or_default();
    Json(json!({ "items": items }))
}
