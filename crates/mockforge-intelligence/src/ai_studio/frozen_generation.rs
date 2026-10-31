//! Shared immutable snapshot identity for mock and persona authoring.
use super::{
    artifact_freezer::ArtifactFreezer,
    config::{AiMode, DeterministicModeConfig},
};
use crate::intelligent_behavior::IntelligentBehaviorConfig;
use mockforge_foundation::{
    ai_studio_types::{FreezeMetadata, FreezeRequest, FrozenArtifact},
    Result,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(super) struct FrozenGeneration {
    freezer: ArtifactFreezer,
    identifier: String,
    config: DeterministicModeConfig,
    replay: bool,
}

impl FrozenGeneration {
    pub(super) fn new(
        inputs: Value,
        behavior: &IntelligentBehaviorConfig,
        mode: Option<AiMode>,
        config: Option<&DeterministicModeConfig>,
    ) -> Result<Self> {
        let config = config.cloned().unwrap_or_default();
        let mut settings = serde_json::to_value(behavior)?;
        if let Some(model) = settings.get_mut("behavior_model").and_then(Value::as_object_mut) {
            model.remove("api_key");
        }
        let seed = behavior.behavior_model.seed.or_else(|| {
            std::env::var("MOCKFORGE_AI_SEED")
                .ok()
                .and_then(|seed| seed.trim().parse::<i64>().ok())
        });
        let mut identity = serde_json::json!({"version":1,"inputs":inputs,"settings":settings,"effective_seed":seed});
        identity.sort_all_objects();
        Ok(Self {
            freezer: ArtifactFreezer::with_base_dir(&config.freeze_directory),
            identifier: format!("{:x}", Sha256::digest(serde_json::to_vec(&identity)?)),
            config,
            replay: mode == Some(AiMode::GenerateOnceFreeze),
        })
    }

    pub(super) async fn load(&self, artifact_type: &str) -> Result<Option<FrozenArtifact>> {
        if self.replay {
            self.freezer
                .load_snapshot(artifact_type, &self.identifier, &self.config.freeze_format)
                .await
        } else {
            Ok(None)
        }
    }

    pub(super) async fn freeze(
        &self,
        artifact_type: &str,
        content: Value,
        mut metadata: FreezeMetadata,
    ) -> Result<Option<FrozenArtifact>> {
        if !(self.replay || self.config.enabled && self.config.is_auto_freeze_enabled()) {
            return Ok(None);
        }
        if !self.config.track_metadata {
            metadata = FreezeMetadata {
                llm_provider: None,
                llm_model: None,
                llm_version: None,
                prompt_hash: None,
                output_hash: None,
                original_prompt: None,
            };
        }
        // Snapshot identity and content integrity are required even when optional
        // prompt/provider provenance is disabled.
        metadata.prompt_hash = Some(self.identifier.clone());
        let request = FreezeRequest {
            artifact_type: artifact_type.into(),
            content,
            format: self.config.freeze_format.clone(),
            path: None,
            metadata: Some(metadata),
        };
        let frozen = if self.replay {
            self.freezer.freeze_once(&request, &self.identifier).await?
        } else {
            self.freezer.freeze(&request).await?
        };
        Ok(Some(frozen))
    }
}

pub(super) fn payload(artifact: &FrozenArtifact) -> Value {
    let mut value = artifact.content.clone();
    if let Some(object) = value.as_object_mut() {
        object.remove("_frozen_metadata");
    }
    value
}

#[cfg(test)]
mod tests;
