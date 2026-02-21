//! Natural language mock generator
//!
//! This module provides functionality to generate mocks from natural language descriptions.
//! It integrates with the existing VoiceCommandParser and VoiceSpecGenerator to leverage
//! the proven mock generation infrastructure.

use crate::ai_studio::artifact_freezer::{ArtifactFreezer, FreezeMetadata};
use crate::ai_studio::config::DeterministicModeConfig;
use crate::intelligent_behavior::IntelligentBehaviorConfig;
use crate::voice::{command_parser::VoiceCommandParser, spec_generator::VoiceSpecGenerator};
use crate::{OpenApiSpec, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Mock generator for creating mocks from natural language
pub struct MockGenerator {
    /// Voice command parser for parsing NL descriptions
    parser: VoiceCommandParser,
    /// Spec generator for creating OpenAPI specs
    spec_generator: VoiceSpecGenerator,
    /// Configuration (needed for accessing LLM provider/model info)
    config: IntelligentBehaviorConfig,
}

impl MockGenerator {
    /// Create a new mock generator with default configuration
    pub fn new() -> Self {
        let config = IntelligentBehaviorConfig::default();
        Self {
            parser: VoiceCommandParser::new(config.clone()),
            spec_generator: VoiceSpecGenerator::new(),
            config,
        }
    }

    /// Create a new mock generator with custom configuration
    pub fn with_config(config: IntelligentBehaviorConfig) -> Self {
        Self {
            parser: VoiceCommandParser::new(config.clone()),
            spec_generator: VoiceSpecGenerator::new(),
            config,
        }
    }

    /// Generate a mock from natural language description
    ///
    /// This method parses the natural language description and generates a complete
    /// OpenAPI specification ready for use with MockForge.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use mockforge_core::ai_studio::nl_mock_generator::MockGenerator;
    ///
    /// async fn example() -> mockforge_core::Result<()> {
    ///     let generator = MockGenerator::new();
    ///     let result = generator.generate(
    ///         "Create a user API with CRUD operations for managing users",
    ///         None,
    ///         None,
    ///         None,
    ///     ).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn generate(
        &self,
        description: &str,
        _workspace_id: Option<&str>,
        ai_mode: Option<crate::ai_studio::config::AiMode>,
        deterministic_config: Option<&DeterministicModeConfig>,
    ) -> Result<MockGenerationResult> {
        // In deterministic mode, check for frozen artifacts first
        if ai_mode == Some(crate::ai_studio::config::AiMode::GenerateOnceFreeze) {
            let freezer = ArtifactFreezer::new();

            // Create identifier from description hash
            let mut hasher = DefaultHasher::new();
            description.hash(&mut hasher);
            let description_hash = format!("{:x}", hasher.finish());

            // Try to load frozen artifact
            if let Some(frozen) = freezer.load_frozen("mock", Some(&description_hash)).await? {
                // Extract spec from frozen content (remove metadata)
                let mut spec = frozen.content.clone();
                if let Some(obj) = spec.as_object_mut() {
                    obj.remove("_frozen_metadata");
                }

                return Ok(MockGenerationResult {
                    spec: Some(spec),
                    message: format!(
                        "Loaded frozen mock artifact from {} (deterministic mode)",
                        frozen.path
                    ),
                    parsed_command: None,
                    frozen_artifact: Some(frozen),
                });
            }
        }

        // Parse the natural language command
        let parsed = self.parser.parse_command(description).await?;

        // Generate OpenAPI spec from parsed command
        let spec = self.spec_generator.generate_spec(&parsed).await?;

        // Convert spec to JSON for response
        let spec_json = serde_json::to_value(&spec.spec)?;

        // Auto-freeze if enabled
        let frozen_artifact = if let Some(config) = deterministic_config {
            if config.enabled && config.is_auto_freeze_enabled() {
                let freezer = ArtifactFreezer::new();

                // Calculate prompt hash
                let mut hasher = Sha256::new();
                hasher.update(description.as_bytes());
                let prompt_hash = format!("{:x}", hasher.finalize());

                // Create metadata
                let metadata = if config.track_metadata {
                    Some(FreezeMetadata {
                        llm_provider: Some(self.config.behavior_model.llm_provider.clone()),
                        llm_model: Some(self.config.behavior_model.model.clone()),
                        llm_version: None, // Would need to be passed in or retrieved
                        prompt_hash: Some(prompt_hash),
                        output_hash: None, // Will be calculated by freezer
                        original_prompt: Some(description.to_string()),
                    })
                } else {
                    None
                };

                let freeze_request = crate::ai_studio::artifact_freezer::FreezeRequest {
                    artifact_type: "mock".to_string(),
                    content: spec_json.clone(),
                    format: config.freeze_format.clone(),
                    path: None,
                    metadata,
                };

                freezer.auto_freeze_if_enabled(&freeze_request, config).await?
            } else {
                None
            }
        } else {
            None
        };

        Ok(MockGenerationResult {
            spec: Some(spec_json),
            message: format!(
                "Successfully generated API '{}' with {} endpoints and {} models{}",
                parsed.title,
                parsed.endpoints.len(),
                parsed.models.len(),
                if frozen_artifact.is_some() {
                    " (auto-frozen)"
                } else {
                    ""
                }
            ),
            parsed_command: Some(parsed),
            frozen_artifact,
        })
    }

    /// Generate a mock with additional context (for conversational mode)
    ///
    /// This method allows generating mocks that extend or modify existing specifications.
    pub async fn generate_with_context(
        &self,
        description: &str,
        existing_spec: Option<&OpenApiSpec>,
    ) -> Result<MockGenerationResult> {
        // Parse the natural language command
        let parsed = self.parser.parse_command(description).await?;

        // Generate or merge spec
        let spec = if let Some(existing) = existing_spec {
            // Merge with existing spec
            self.spec_generator.merge_spec(existing, &parsed).await?
        } else {
            // Generate new spec
            self.spec_generator.generate_spec(&parsed).await?
        };

        // Convert spec to JSON for response
        let spec_json = serde_json::to_value(&spec.spec)?;

        Ok(MockGenerationResult {
            spec: Some(spec_json),
            message: format!(
                "Successfully {} API '{}' with {} endpoints and {} models",
                if existing_spec.is_some() {
                    "updated"
                } else {
                    "generated"
                },
                parsed.title,
                parsed.endpoints.len(),
                parsed.models.len()
            ),
            parsed_command: Some(parsed),
            frozen_artifact: None,
        })
    }
}

impl Default for MockGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of mock generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockGenerationResult {
    /// Generated OpenAPI spec (if any)
    pub spec: Option<serde_json::Value>,

    /// Status message
    pub message: String,

    /// Parsed command details (for debugging/preview)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed_command: Option<crate::voice::command_parser::ParsedCommand>,

    /// Frozen artifact (if auto-freeze was enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frozen_artifact: Option<crate::ai_studio::artifact_freezer::FrozenArtifact>,
}
