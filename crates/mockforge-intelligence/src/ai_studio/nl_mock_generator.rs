//! Natural language mock generator
//!
//! This module provides functionality to generate mocks from natural language descriptions.
//! It integrates with the existing VoiceCommandParser and VoiceSpecGenerator to leverage
//! the proven mock generation infrastructure.

use crate::ai_studio::artifact_freezer::FreezeMetadata;
use crate::ai_studio::config::DeterministicModeConfig;
use crate::ai_studio::frozen_generation::{self, FrozenGeneration};
use crate::intelligent_behavior::IntelligentBehaviorConfig;
use crate::voice::{command_parser::VoiceCommandParser, spec_generator::VoiceSpecGenerator};
use mockforge_foundation::Result;
use mockforge_openapi::OpenApiSpec;
use serde::{Deserialize, Serialize};

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
        let snapshot = FrozenGeneration::new(
            serde_json::json!({"description": description, "workspace_id": _workspace_id}),
            &self.config,
            ai_mode,
            deterministic_config,
        )?;
        if let Some(frozen) = snapshot.load("mock").await? {
            let spec = frozen_generation::payload(&frozen);
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

        // Parse the natural language command
        let parsed = self.parser.parse_command(description).await?;

        // Generate OpenAPI spec from parsed command
        let spec = self.spec_generator.generate_spec(&parsed).await?;

        // Convert spec to JSON for response
        let spec_json = serde_json::to_value(&spec.spec)?;

        let frozen_artifact = snapshot
            .freeze(
                "mock",
                spec_json.clone(),
                FreezeMetadata {
                    llm_provider: Some(self.config.behavior_model.llm_provider.clone()),
                    llm_model: Some(self.config.behavior_model.model.clone()),
                    llm_version: None,
                    prompt_hash: None,
                    output_hash: None,
                    original_prompt: Some(description.to_string()),
                },
            )
            .await?;
        let spec_json =
            frozen_artifact.as_ref().map(frozen_generation::payload).unwrap_or(spec_json);

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

        // Record AI pillar usage. Merge counts as ai_refinement; new generation as ai_generation.
        let metric_name = if existing_spec.is_some() {
            "ai_refinement"
        } else {
            "ai_generation"
        };
        mockforge_foundation::pillar_tracking::track_ai_pillar_telemetry(
            None,
            None,
            metric_name,
            serde_json::json!({
                "type": "mock",
                "endpoints": parsed.endpoints.len(),
                "models": parsed.models.len(),
            }),
        )
        .await;

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
