//! Artifact freezer for converting AI outputs to deterministic formats
//!
//! This module provides functionality to freeze AI-generated artifacts into
//! deterministic YAML/JSON files for version control and reproducible testing.

use crate::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::fs;

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

    /// Freeze an AI-generated artifact to deterministic format
    ///
    /// This method converts AI-generated content (mocks, personas, scenarios, etc.)
    /// into deterministic YAML/JSON files that can be version controlled and used
    /// for reproducible testing.
    pub async fn freeze(&self, request: &FreezeRequest) -> Result<FrozenArtifact> {
        // Determine output path
        let path = if let Some(custom_path) = &request.path {
            PathBuf::from(custom_path)
        } else {
            // Generate default path based on artifact type and timestamp
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let extension = if request.format == "yaml" || request.format == "yml" {
                "yaml"
            } else {
                "json"
            };
            self.base_dir
                .join(format!("{}_{}.{}", request.artifact_type, timestamp, extension))
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                crate::Error::generic(format!("Failed to create frozen artifacts directory: {}", e))
            })?;
        }

        // Add metadata to the artifact
        let mut frozen_content = request.content.clone();
        if let Some(obj) = frozen_content.as_object_mut() {
            obj.insert(
                "_frozen_metadata".to_string(),
                serde_json::json!({
                    "frozen_at": Utc::now().to_rfc3339(),
                    "artifact_type": request.artifact_type,
                    "source": "ai_generated",
                    "format": request.format,
                }),
            );
        }

        // Serialize to the requested format
        let content_str = if request.format == "yaml" || request.format == "yml" {
            serde_yaml::to_string(&frozen_content)
                .map_err(|e| crate::Error::generic(format!("Failed to serialize to YAML: {}", e)))?
        } else {
            serde_json::to_string_pretty(&frozen_content)
                .map_err(|e| crate::Error::generic(format!("Failed to serialize to JSON: {}", e)))?
        };

        // Write to file
        fs::write(&path, content_str).await.map_err(|e| {
            crate::Error::generic(format!("Failed to write frozen artifact: {}", e))
        })?;

        Ok(FrozenArtifact {
            artifact_type: request.artifact_type.clone(),
            content: frozen_content,
            format: request.format.clone(),
            path: path.to_string_lossy().to_string(),
        })
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

/// Request to freeze an artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeRequest {
    /// Type of artifact (mock, persona, scenario, etc.)
    pub artifact_type: String,

    /// Artifact content
    pub content: serde_json::Value,

    /// Output format (yaml or json)
    pub format: String,

    /// Output path
    pub path: Option<String>,
}

/// Frozen artifact result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrozenArtifact {
    /// Type of artifact
    pub artifact_type: String,

    /// Frozen content
    pub content: serde_json::Value,

    /// Output format
    pub format: String,

    /// File path where artifact was saved
    pub path: String,
}
