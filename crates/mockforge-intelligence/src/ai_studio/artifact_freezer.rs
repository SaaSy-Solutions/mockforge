//! Artifact freezer for converting AI outputs to deterministic formats
//!
//! This module provides functionality to freeze AI-generated artifacts into
//! deterministic YAML/JSON files for version control and reproducible testing.

use chrono::Utc;
use mockforge_foundation::Result;
// Data types re-exported from foundation.
pub use mockforge_foundation::ai_studio_types::{FreezeMetadata, FreezeRequest, FrozenArtifact};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;

mod bundles;
mod discovery;
mod loading;
mod provenance;
mod publication;
mod rendering;

/// Artifact freezer for deterministic output
pub struct ArtifactFreezer {
    /// Base directory for frozen artifacts
    base_dir: PathBuf,
}

impl ArtifactFreezer {
    /// Create a new artifact freezer with default directory
    pub fn new() -> Self {
        Self {
            base_dir: PathBuf::from(".mockforge/frozen"),
        }
    }

    /// Create a new artifact freezer with custom base directory
    pub fn with_base_dir<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Get the base directory for frozen artifacts
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Freeze an AI-generated artifact to deterministic format
    ///
    /// This method converts AI-generated content (mocks, personas, scenarios, etc.)
    /// into deterministic YAML/JSON files that can be version controlled and used
    /// for reproducible testing.
    pub async fn freeze(&self, request: &FreezeRequest) -> Result<FrozenArtifact> {
        validate_artifact_type(&request.artifact_type)?;
        if !matches!(request.format.as_str(), "json" | "yaml" | "yml") {
            return Err(mockforge_foundation::Error::config(
                "Frozen artifact format must be json or yaml",
            ));
        }
        if request.content.get("_frozen_metadata").is_some() {
            return Err(mockforge_foundation::Error::config(
                "_frozen_metadata is reserved for artifact provenance",
            ));
        }
        let path = request.path.as_ref().map(PathBuf::from).unwrap_or_else(|| {
            let extension = if request.format == "json" {
                "json"
            } else {
                "yaml"
            };
            self.base_dir.join(format!(
                "{}_{}_{}.{}",
                request.artifact_type,
                Utc::now().format("%Y%m%d_%H%M%S"),
                uuid::Uuid::new_v4(),
                extension
            ))
        });

        // Ensure parent directory exists
        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            fs::create_dir_all(parent).await.map_err(|e| {
                mockforge_foundation::Error::io_with_context(
                    "create frozen artifacts directory",
                    e.to_string(),
                )
            })?;
        }

        // Hash the payload separately from optional provenance.
        let output_hash = Some(content_hash(&request.content)?);

        let (frozen_content, content_str) = rendering::render(request, output_hash.as_deref())?;

        // Non-object JSON values cannot carry embedded provenance without changing type.
        if !frozen_content.is_object() {
            provenance::write(&path, &rendering::provenance(request, output_hash.as_deref())?)
                .await?;
        }

        // Write to file
        fs::write(&path, content_str).await.map_err(|e| {
            mockforge_foundation::Error::io_with_context("write frozen artifact", e.to_string())
        })?;

        Ok(FrozenArtifact {
            artifact_type: request.artifact_type.clone(),
            content: frozen_content,
            format: request.format.clone(),
            path: path.to_string_lossy().to_string(),
            metadata: request.metadata.clone(),
            output_hash,
        })
    }

    /// Auto-freeze an artifact if auto-freeze is enabled
    ///
    /// This method checks the deterministic mode config and automatically freezes
    /// the artifact if auto-freeze is enabled.
    pub async fn auto_freeze_if_enabled(
        &self,
        request: &FreezeRequest,
        deterministic_config: &crate::ai_studio::config::DeterministicModeConfig,
    ) -> Result<Option<FrozenArtifact>> {
        if deterministic_config.enabled && deterministic_config.is_auto_freeze_enabled() {
            Ok(Some(self.freeze(request).await?))
        } else {
            Ok(None)
        }
    }

    /// Verify the integrity of a frozen artifact
    ///
    /// Checks that the output hash matches the current content.
    pub async fn verify_frozen_artifact(&self, artifact: &FrozenArtifact) -> Result<bool> {
        let mut content = artifact.content.clone();
        if let Some(object) = content.as_object_mut() {
            object.remove("_frozen_metadata");
        }
        let current_hash = content_hash(&content)?;
        Ok(artifact.output_hash.as_ref().is_some_and(|stored| *stored == current_hash))
    }

    /// Freeze multiple artifacts at once
    pub async fn freeze_batch(&self, requests: &[FreezeRequest]) -> Result<Vec<FrozenArtifact>> {
        let mut results = Vec::new();
        for request in requests {
            results.push(self.freeze(request).await?);
        }
        Ok(results)
    }
}

impl Default for ArtifactFreezer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;

fn content_hash(content: &Value) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(content)?)))
}

fn validate_artifact_type(artifact_type: &str) -> Result<()> {
    if artifact_type.is_empty()
        || !artifact_type
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err(mockforge_foundation::Error::config("Invalid frozen artifact type"));
    }
    Ok(())
}
