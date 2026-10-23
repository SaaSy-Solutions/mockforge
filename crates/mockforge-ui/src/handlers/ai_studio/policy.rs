//! Resolve persisted local authoring settings once for all generation endpoints.
use super::{AdminState, IntelligentBehaviorConfig};
use axum::http::StatusCode;
use mockforge_core::ai_studio::config::{AiMode, DeterministicModeConfig};

pub(super) struct GenerationPolicy {
    pub behavior: IntelligentBehaviorConfig,
    pub mode: Option<AiMode>,
    pub deterministic: Option<DeterministicModeConfig>,
}

pub(super) async fn resolve(
    state: &AdminState,
    workspace_id: Option<&str>,
) -> Result<GenerationPolicy, StatusCode> {
    let mut behavior = match &state.mockai {
        Some(mockai) => mockai.read().await.get_config().clone(),
        None => IntelligentBehaviorConfig::default(),
    };
    let mut mode = state.ai_mode_default().await;
    let mut deterministic = None;
    if let Some(id) = workspace_id {
        let workspace = load_workspace_config(state, id).await?;
        mode = workspace.ai_mode.or(mode);
        if let Some(policy) = workspace.ai_generation {
            if policy.seed.is_some() {
                behavior.behavior_model.seed = policy.seed;
            }
            deterministic = Some(policy.deterministic_mode);
        }
    }
    Ok(GenerationPolicy {
        behavior,
        mode,
        deterministic,
    })
}

async fn load_workspace_config(
    state: &AdminState,
    id: &str,
) -> Result<mockforge_core::WorkspaceConfig, StatusCode> {
    if let Some(registry) = &state.workspace_registry {
        match registry.read().await.get_workspace(id) {
            Ok(workspace) if workspace.enabled => return Ok(workspace.workspace.config),
            Ok(_) => return Err(StatusCode::FORBIDDEN),
            Err(mockforge_core::Error::NotFound { .. }) => {}
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
    // A live registry ID is not necessarily a UUID or a safe filename. Only
    // disk fallback accepts filename-safe IDs; never interpolate other IDs.
    if id.is_empty() || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_') {
        return Err(StatusCode::BAD_REQUEST);
    }
    state
        .workspace_persistence
        .load_workspace(id)
        .await
        .map(|workspace| workspace.config)
        .map_err(|error| match error {
            mockforge_core::Error::NotFound { .. } => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })
}
