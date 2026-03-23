use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};

use super::{ManagementState, MockConfig};

/// Export format for mock configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// YAML format
    Yaml,
}

/// Export mocks in specified format
pub(crate) async fn export_mocks(
    State(state): State<ManagementState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<(StatusCode, String), StatusCode> {
    let mocks = state.mocks.read().await;

    let format = params
        .get("format")
        .map(|f| match f.as_str() {
            "yaml" | "yml" => ExportFormat::Yaml,
            _ => ExportFormat::Json,
        })
        .unwrap_or(ExportFormat::Json);

    match format {
        ExportFormat::Json => serde_json::to_string_pretty(&*mocks)
            .map(|json| (StatusCode::OK, json))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
        ExportFormat::Yaml => serde_yaml::to_string(&*mocks)
            .map(|yaml| (StatusCode::OK, yaml))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Import mocks from JSON/YAML
pub(crate) async fn import_mocks(
    State(state): State<ManagementState>,
    Json(mocks): Json<Vec<MockConfig>>,
) -> impl IntoResponse {
    let mut current_mocks = state.mocks.write().await;
    current_mocks.clear();
    current_mocks.extend(mocks);
    Json(serde_json::json!({ "status": "imported", "count": current_mocks.len() }))
}
