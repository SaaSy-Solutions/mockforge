//! Artifact freezer for converting AI outputs to deterministic formats
//!
//! This module provides functionality to freeze AI-generated artifacts into
//! deterministic YAML/JSON files for version control and reproducible testing.

use crate::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
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

        // Calculate output hash if metadata tracking is enabled
        let output_hash = if request.metadata.is_some() {
            let content_str = serde_json::to_string(&request.content)?;
            let mut hasher = Sha256::new();
            hasher.update(content_str.as_bytes());
            Some(format!("{:x}", hasher.finalize()))
        } else {
            None
        };

        // Add metadata to the artifact
        let mut frozen_content = request.content.clone();
        if let Some(obj) = frozen_content.as_object_mut() {
            let mut metadata_json = serde_json::json!({
                "frozen_at": Utc::now().to_rfc3339(),
                "artifact_type": request.artifact_type,
                "source": "ai_generated",
                "format": request.format,
            });

            // Add detailed metadata if provided
            if let Some(ref metadata) = request.metadata {
                if let Some(ref provider) = metadata.llm_provider {
                    metadata_json["llm_provider"] = Value::String(provider.clone());
                }
                if let Some(ref model) = metadata.llm_model {
                    metadata_json["llm_model"] = Value::String(model.clone());
                }
                if let Some(ref version) = metadata.llm_version {
                    metadata_json["llm_version"] = Value::String(version.clone());
                }
                if let Some(ref prompt_hash) = metadata.prompt_hash {
                    metadata_json["prompt_hash"] = Value::String(prompt_hash.clone());
                }
                if let Some(ref output_hash) = output_hash {
                    metadata_json["output_hash"] = Value::String(output_hash.clone());
                }
                if let Some(ref prompt) = metadata.original_prompt {
                    metadata_json["original_prompt"] = Value::String(prompt.clone());
                }
            }

            obj.insert("_frozen_metadata".to_string(), metadata_json);
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
        // Calculate current hash
        let content_str = serde_json::to_string(&artifact.content)?;
        let mut hasher = Sha256::new();
        hasher.update(content_str.as_bytes());
        let current_hash = format!("{:x}", hasher.finalize());

        // Compare with stored hash
        if let Some(ref stored_hash) = artifact.output_hash {
            Ok(current_hash == *stored_hash)
        } else {
            // No hash stored, assume valid
            Ok(true)
        }
    }

    /// Freeze multiple artifacts at once
    pub async fn freeze_batch(&self, requests: &[FreezeRequest]) -> Result<Vec<FrozenArtifact>> {
        let mut results = Vec::new();
        for request in requests {
            results.push(self.freeze(request).await?);
        }
        Ok(results)
    }

    /// Load a frozen artifact by type and identifier
    ///
    /// In deterministic mode, this method searches for frozen artifacts matching
    /// the given type and identifier (e.g., description hash, persona ID).
    pub async fn load_frozen(
        &self,
        artifact_type: &str,
        identifier: Option<&str>,
    ) -> Result<Option<FrozenArtifact>> {
        // Build search pattern
        let _search_pattern = if let Some(id) = identifier {
            format!("{}_*_{}", artifact_type, id)
        } else {
            format!("{}_*", artifact_type)
        };

        // Search for matching files
        let mut entries = fs::read_dir(&self.base_dir).await.map_err(|e| {
            crate::Error::generic(format!("Failed to read frozen artifacts directory: {}", e))
        })?;

        let mut latest_match: Option<FrozenArtifact> = None;
        let mut latest_time = chrono::DateTime::<Utc>::MIN_UTC;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| crate::Error::generic(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Check if file matches pattern
                let matches = if identifier.is_some() {
                    file_name.contains(artifact_type) && file_name.contains(identifier.unwrap())
                } else {
                    file_name.starts_with(&format!("{}_", artifact_type))
                };

                if matches {
                    // Try to load the file
                    let content = fs::read_to_string(&path).await.map_err(|e| {
                        crate::Error::generic(format!("Failed to read frozen artifact: {}", e))
                    })?;

                    let content_value: Value = if path.extension().and_then(|e| e.to_str())
                        == Some("yaml")
                        || path.extension().and_then(|e| e.to_str()) == Some("yml")
                    {
                        serde_yaml::from_str(&content).map_err(|e| {
                            crate::Error::generic(format!("Failed to parse YAML: {}", e))
                        })?
                    } else {
                        serde_json::from_str(&content).map_err(|e| {
                            crate::Error::generic(format!("Failed to parse JSON: {}", e))
                        })?
                    };

                    // Extract frozen_at timestamp if available
                    let frozen_time = content_value
                        .get("_frozen_metadata")
                        .and_then(|m| m.get("frozen_at"))
                        .and_then(|t| t.as_str())
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|| {
                            // Fallback: use file metadata if available, otherwise use current time
                            // Note: We can't use await in unwrap_or_else, so we'll use current time as fallback
                            // The file metadata would need to be retrieved before this point if needed
                            Utc::now()
                        });

                    // Keep the latest match
                    if frozen_time > latest_time {
                        latest_time = frozen_time;
                        latest_match = Some(FrozenArtifact {
                            artifact_type: artifact_type.to_string(),
                            content: content_value,
                            format: if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                                || path.extension().and_then(|e| e.to_str()) == Some("yml")
                            {
                                "yaml".to_string()
                            } else {
                                "json".to_string()
                            },
                            path: path.to_string_lossy().to_string(),
                            metadata: None,
                            output_hash: None,
                        });
                    }
                }
            }
        }

        Ok(latest_match)
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
    pub content: Value,

    /// Output format (yaml or json)
    pub format: String,

    /// Output path
    pub path: Option<String>,

    /// Optional metadata for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<FreezeMetadata>,
}

/// Metadata for frozen artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeMetadata {
    /// LLM provider used (e.g., "openai", "anthropic", "ollama")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,

    /// LLM model used (e.g., "gpt-4", "claude-3-opus")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// LLM version (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_version: Option<String>,

    /// Hash of the input prompt/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_hash: Option<String>,

    /// Hash of the output content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_hash: Option<String>,

    /// Original prompt/description (optional, for reference)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_prompt: Option<String>,
}

/// Frozen artifact result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrozenArtifact {
    /// Type of artifact
    pub artifact_type: String,

    /// Frozen content
    pub content: Value,

    /// Output format
    pub format: String,

    /// File path where artifact was saved
    pub path: String,

    /// Metadata used for freezing (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<FreezeMetadata>,

    /// Output hash for integrity verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_hash: Option<String>,
}
