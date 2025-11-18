//! Natural language mock generator
//!
//! This module provides functionality to generate mocks from natural language descriptions.
//! It integrates with the existing VoiceCommandParser and VoiceSpecGenerator to leverage
//! the proven mock generation infrastructure.

use crate::intelligent_behavior::IntelligentBehaviorConfig;
use crate::voice::{command_parser::VoiceCommandParser, spec_generator::VoiceSpecGenerator};
use crate::{OpenApiSpec, Result};
use serde::{Deserialize, Serialize};

/// Mock generator for creating mocks from natural language
pub struct MockGenerator {
    /// Voice command parser for parsing NL descriptions
    parser: VoiceCommandParser,
    /// Spec generator for creating OpenAPI specs
    spec_generator: VoiceSpecGenerator,
}

impl MockGenerator {
    /// Create a new mock generator with default configuration
    pub fn new() -> Self {
        let config = IntelligentBehaviorConfig::default();
        Self {
            parser: VoiceCommandParser::new(config),
            spec_generator: VoiceSpecGenerator::new(),
        }
    }

    /// Create a new mock generator with custom configuration
    pub fn with_config(config: IntelligentBehaviorConfig) -> Self {
        Self {
            parser: VoiceCommandParser::new(config),
            spec_generator: VoiceSpecGenerator::new(),
        }
    }

    /// Generate a mock from natural language description
    ///
    /// This method parses the natural language description and generates a complete
    /// OpenAPI specification ready for use with MockForge.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example() -> mockforge_core::Result<()> {
    /// let generator = MockGenerator::new();
    /// let result = generator.generate(
    ///     "Create a user API with CRUD operations for managing users"
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate(&self, description: &str) -> Result<MockGenerationResult> {
        // Parse the natural language command
        let parsed = self.parser.parse_command(description).await?;

        // Generate OpenAPI spec from parsed command
        let spec = self.spec_generator.generate_spec(&parsed).await?;

        // Convert spec to JSON for response
        let spec_json = serde_json::to_value(&spec.spec)?;

        Ok(MockGenerationResult {
            spec: Some(spec_json),
            message: format!(
                "Successfully generated API '{}' with {} endpoints and {} models",
                parsed.title,
                parsed.endpoints.len(),
                parsed.models.len()
            ),
            parsed_command: Some(parsed),
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
}
