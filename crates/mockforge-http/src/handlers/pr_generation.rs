//! PR generation handlers
//!
//! This module provides HTTP handlers for triggering PR generation when contract changes are detected.

use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use mockforge_core::pr_generation::{
    PRFileChange, PRFileChangeType, PRGenerator, PRTemplateContext,
};

/// State for PR generation handlers
#[derive(Clone)]
pub struct PRGenerationState {
    /// PR generator
    pub generator: Option<Arc<PRGenerator>>,
}

/// Request to generate a PR
#[derive(Debug, Deserialize, Serialize)]
pub struct GeneratePRRequest {
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Number of breaking changes
    pub breaking_changes: u32,
    /// Number of non-breaking changes
    pub non_breaking_changes: u32,
    /// Change summary
    pub change_summary: String,
    /// Affected files
    pub affected_files: Vec<String>,
    /// File changes
    pub file_changes: Vec<PRFileChangeRequest>,
    /// Labels to add
    pub labels: Option<Vec<String>>,
    /// Reviewers to request
    pub reviewers: Option<Vec<String>>,
}

/// Request for a file change
#[derive(Debug, Deserialize, Serialize)]
pub struct PRFileChangeRequest {
    /// File path
    pub path: String,
    /// File content
    pub content: String,
    /// Change type
    pub change_type: String,
}

/// Response for PR generation
#[derive(Debug, Serialize)]
pub struct GeneratePRResponse {
    /// PR number
    pub pr_number: Option<u64>,
    /// PR URL
    pub pr_url: Option<String>,
    /// Branch name
    pub branch: Option<String>,
    /// Success status
    pub success: bool,
    /// Error message (if any)
    pub error: Option<String>,
}

/// Generate a PR from contract changes
///
/// POST /api/v1/pr/generate
pub async fn generate_pr(
    State(state): State<PRGenerationState>,
    Json(request): Json<GeneratePRRequest>,
) -> Result<Json<GeneratePRResponse>, StatusCode> {
    let generator = state.generator.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Convert file change requests to PR file changes
    let file_changes: Vec<PRFileChange> = request
        .file_changes
        .into_iter()
        .map(|fc| {
            let change_type = match fc.change_type.as_str() {
                "create" => PRFileChangeType::Create,
                "update" => PRFileChangeType::Update,
                "delete" => PRFileChangeType::Delete,
                _ => PRFileChangeType::Update,
            };

            PRFileChange {
                path: fc.path,
                content: fc.content,
                change_type,
            }
        })
        .collect();

    // Create template context
    let context = PRTemplateContext {
        endpoint: request.endpoint,
        method: request.method,
        breaking_changes: request.breaking_changes,
        non_breaking_changes: request.non_breaking_changes,
        affected_files: request.affected_files,
        change_summary: request.change_summary,
        is_breaking: request.breaking_changes > 0,
        metadata: std::collections::HashMap::new(),
    };

    // Generate PR
    match generator
        .create_pr_from_context(
            context,
            file_changes,
            request.labels.unwrap_or_default(),
            request.reviewers.unwrap_or_default(),
        )
        .await
    {
        Ok(pr_result) => Ok(Json(GeneratePRResponse {
            pr_number: Some(pr_result.number),
            pr_url: Some(pr_result.url),
            branch: Some(pr_result.branch),
            success: true,
            error: None,
        })),
        Err(e) => Ok(Json(GeneratePRResponse {
            pr_number: None,
            pr_url: None,
            branch: None,
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

/// Create PR generation router
pub fn pr_generation_router(state: PRGenerationState) -> axum::Router {
    use axum::routing::post;

    axum::Router::new()
        .route("/api/v1/pr/generate", post(generate_pr))
        .with_state(state)
}

use tracing::warn;
