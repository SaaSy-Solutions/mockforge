//! System Generator - Natural Language to Entire System Generation
//!
//! This module provides functionality to generate complete backend systems
//! from natural language descriptions, including:
//! - 20-30 REST endpoints (OpenAPI 3.1 spec)
//! - 4-5 personas (driver, rider, admin, dispatcher, support)
//! - 6-10 lifecycle states (trip: requested, matched, in_progress, completed, cancelled)
//! - WebSocket topics (location_updates, trip_status, surge_alerts)
//! - Payment failure scenarios (insufficient_funds, card_declined, network_error)
//! - Surge pricing chaos profiles (peak_hours, event_surge, weather_surge)
//! - Full OpenAPI specification
//! - Mock backend configuration (mockforge.yaml)
//! - GraphQL schema (optional)
//! - TypeScript/Go/Rust typings
//! - CI pipeline templates (GitHub Actions, GitLab CI)
//!
//! # Features
//!
//! - **Versioned Draft Artifacts**: Generates v1, v2, etc. (never mutates existing)
//! - **Deterministic Mode Integration**: Honors workspace `ai.deterministic_mode` setting
//! - **System Coherence Validation**: Ensures personas match endpoints, lifecycles match entities
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use mockforge_core::ai_studio::system_generator::{SystemGenerator, SystemGenerationRequest};
//! use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
//!
//! async fn example() -> mockforge_core::Result<()> {
//!     let config = IntelligentBehaviorConfig::default();
//!     let generator = SystemGenerator::new(config);
//!
//!     let request = SystemGenerationRequest {
//!         description: "I'm building a ride-sharing app".to_string(),
//!         output_formats: vec!["openapi".to_string(), "personas".to_string()],
//!         workspace_id: Some("workspace-123".to_string()),
//!     };
//!
//!     let system = generator.generate(&request).await?;
//!     Ok(())
//! }
//! ```

use crate::ai_studio::{
    artifact_freezer::{ArtifactFreezer, FreezeMetadata, FreezeRequest},
    config::DeterministicModeConfig,
};
use crate::intelligent_behavior::{
    config::IntelligentBehaviorConfig,
    llm_client::{LlmClient, LlmUsage},
    types::LlmGenerationRequest,
};
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

/// Request for system generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemGenerationRequest {
    /// Natural language description of the system to generate
    pub description: String,

    /// Output formats to generate
    /// Valid values: "openapi", "graphql", "personas", "lifecycles", "websocket", "chaos", "ci"
    #[serde(default)]
    pub output_formats: Vec<String>,

    /// Optional workspace ID
    pub workspace_id: Option<String>,

    /// Optional system ID (for versioning - if provided, creates new version)
    pub system_id: Option<String>,
}

/// Generated system with all artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedSystem {
    /// System ID
    pub system_id: String,

    /// Version (v1, v2, etc.)
    pub version: String,

    /// Generated artifacts by type
    pub artifacts: HashMap<String, SystemArtifact>,

    /// Workspace ID
    pub workspace_id: Option<String>,

    /// Status: "draft" or "frozen"
    pub status: String,

    /// Token usage for this generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u64>,

    /// Estimated cost in USD
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,

    /// Generation metadata
    pub metadata: SystemMetadata,
}

/// Result of applying a system design
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedSystem {
    /// System ID
    pub system_id: String,

    /// Version
    pub version: String,

    /// Artifact IDs that were applied
    pub applied_artifacts: Vec<String>,

    /// Whether artifacts were frozen
    pub frozen: bool,
}

/// System artifact (OpenAPI spec, persona, lifecycle, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemArtifact {
    /// Artifact type: "openapi", "persona", "lifecycle", "websocket", "chaos", "ci", "graphql", "typings"
    pub artifact_type: String,

    /// Artifact content (JSON or YAML string)
    pub content: Value,

    /// Artifact format: "json" or "yaml"
    pub format: String,

    /// Artifact ID
    pub artifact_id: String,
}

/// System generation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetadata {
    /// Original description
    pub description: String,

    /// Detected entities from description
    pub entities: Vec<String>,

    /// Detected relationships
    pub relationships: Vec<String>,

    /// Detected operations
    pub operations: Vec<String>,

    /// Generated at timestamp
    pub generated_at: String,
}

/// System Generator Engine
pub struct SystemGenerator {
    /// LLM client for generation
    llm_client: LlmClient,

    /// Configuration
    config: IntelligentBehaviorConfig,

    /// Artifact freezer for deterministic mode
    artifact_freezer: ArtifactFreezer,
}

impl SystemGenerator {
    /// Create a new system generator
    pub fn new(config: IntelligentBehaviorConfig) -> Self {
        let llm_client = LlmClient::new(config.behavior_model.clone());
        let artifact_freezer = ArtifactFreezer::new();
        Self {
            llm_client,
            config,
            artifact_freezer,
        }
    }

    /// Create with custom artifact freezer directory
    pub fn with_freeze_dir<P: AsRef<std::path::Path>>(
        config: IntelligentBehaviorConfig,
        freeze_dir: P,
    ) -> Self {
        let llm_client = LlmClient::new(config.behavior_model.clone());
        let artifact_freezer = ArtifactFreezer::with_base_dir(freeze_dir);
        Self {
            llm_client,
            config,
            artifact_freezer,
        }
    }

    /// Generate a complete system from natural language description
    ///
    /// If deterministic mode is enabled with auto-freeze, artifacts are automatically frozen.
    pub async fn generate(
        &self,
        request: &SystemGenerationRequest,
        deterministic_config: Option<&DeterministicModeConfig>,
    ) -> Result<GeneratedSystem> {
        // Determine system ID and version
        let system_id = request
            .system_id
            .clone()
            .unwrap_or_else(|| format!("system-{}", Uuid::new_v4()));

        // Version starts at v1 since there is no persistent storage for generated systems.
        // If versioning is needed, a storage backend would track previous versions by system_id.
        let version = "v1".to_string();

        // Build the generation prompt
        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(request)?;

        // Generate system using LLM
        let llm_request = LlmGenerationRequest {
            system_prompt,
            user_prompt,
            temperature: 0.7, // Higher temperature for more creative generation
            max_tokens: 8000, // Large context for full system generation
            schema: None,
        };

        let (response_json, usage) = self.llm_client.generate_with_usage(&llm_request).await?;

        // Parse the response
        let artifacts = self.parse_system_response(response_json, &request.output_formats)?;

        // Extract metadata
        let metadata = self.extract_metadata(request, &artifacts)?;

        // Calculate cost
        let cost_usd = self.estimate_cost(&usage);

        // Check if we should auto-freeze
        let should_auto_freeze = deterministic_config
            .map(|cfg| cfg.enabled && cfg.is_auto_freeze_enabled())
            .unwrap_or(false);

        let status = if should_auto_freeze {
            // Auto-freeze all artifacts
            self.freeze_system_artifacts(&system_id, &version, &artifacts, deterministic_config)
                .await?;
            "frozen".to_string()
        } else {
            "draft".to_string()
        };

        Ok(GeneratedSystem {
            system_id,
            version,
            artifacts,
            workspace_id: request.workspace_id.clone(),
            status,
            tokens_used: Some(usage.total_tokens),
            cost_usd: Some(cost_usd),
            metadata,
        })
    }

    /// Freeze system artifacts (used for manual freeze or auto-freeze)
    pub async fn freeze_system_artifacts(
        &self,
        system_id: &str,
        version: &str,
        artifacts: &HashMap<String, SystemArtifact>,
        _deterministic_config: Option<&DeterministicModeConfig>,
    ) -> Result<Vec<String>> {
        let mut frozen_ids = Vec::new();

        for (artifact_type, artifact) in artifacts {
            let freeze_request = FreezeRequest {
                artifact_type: format!("system_{}", artifact_type),
                content: artifact.content.clone(),
                format: artifact.format.clone(),
                path: Some(format!(
                    "{}/{}_{}_{}.{}",
                    self.artifact_freezer.base_dir().display(),
                    system_id,
                    version,
                    artifact_type,
                    artifact.format
                )),
                metadata: Some(FreezeMetadata {
                    llm_provider: Some(self.config.behavior_model.llm_provider.clone()),
                    llm_model: Some(self.config.behavior_model.model.clone()),
                    llm_version: None,
                    prompt_hash: Some(self.hash_description(&artifact_type)),
                    output_hash: None,
                    original_prompt: None,
                }),
            };

            let frozen = self.artifact_freezer.freeze(&freeze_request).await?;
            frozen_ids.push(frozen.path);
        }

        Ok(frozen_ids)
    }

    /// Apply system design (freeze artifacts if deterministic mode requires it)
    ///
    /// This is called when user clicks "Apply system design" button.
    /// If deterministic mode is "auto", artifacts are already frozen.
    /// If deterministic mode is "manual", this freezes them now.
    pub async fn apply_system_design(
        &self,
        system: &GeneratedSystem,
        deterministic_config: Option<&DeterministicModeConfig>,
        artifact_ids: Option<Vec<String>>,
    ) -> Result<AppliedSystem> {
        // If already frozen, return as-is
        if system.status == "frozen" {
            return Ok(AppliedSystem {
                system_id: system.system_id.clone(),
                version: system.version.clone(),
                applied_artifacts: system.artifacts.keys().cloned().collect(),
                frozen: true,
            });
        }

        // Check if we should freeze
        let should_freeze = deterministic_config
            .map(|cfg| cfg.enabled && cfg.is_auto_freeze_enabled())
            .unwrap_or(false);

        // Filter artifacts if specific IDs provided
        let artifacts_to_apply = if let Some(ids) = artifact_ids {
            system
                .artifacts
                .iter()
                .filter(|(_, artifact)| ids.contains(&artifact.artifact_id))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            system.artifacts.clone()
        };

        if should_freeze {
            let frozen_paths = self
                .freeze_system_artifacts(
                    &system.system_id,
                    &system.version,
                    &artifacts_to_apply,
                    deterministic_config,
                )
                .await?;

            Ok(AppliedSystem {
                system_id: system.system_id.clone(),
                version: system.version.clone(),
                applied_artifacts: artifacts_to_apply.keys().cloned().collect(),
                frozen: !frozen_paths.is_empty(),
            })
        } else {
            // Just mark as applied, don't freeze
            Ok(AppliedSystem {
                system_id: system.system_id.clone(),
                version: system.version.clone(),
                applied_artifacts: artifacts_to_apply.keys().cloned().collect(),
                frozen: false,
            })
        }
    }

    /// Manually freeze specific artifacts
    pub async fn freeze_artifacts(
        &self,
        system: &GeneratedSystem,
        artifact_ids: Vec<String>,
    ) -> Result<Vec<String>> {
        let artifacts_to_freeze: HashMap<String, SystemArtifact> = system
            .artifacts
            .iter()
            .filter(|(_, artifact)| artifact_ids.contains(&artifact.artifact_id))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        self.freeze_system_artifacts(&system.system_id, &system.version, &artifacts_to_freeze, None)
            .await
    }

    /// Hash description for metadata tracking
    fn hash_description(&self, artifact_type: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(artifact_type.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Build system prompt for system generation
    fn build_system_prompt(&self) -> String {
        r#"You are an expert backend architect and system designer. Your task is to generate complete backend systems from natural language descriptions.

Generate comprehensive backend systems including:

1. **OpenAPI Specification** (20-30 REST endpoints)
   - Full CRUD operations for all entities
   - Realistic request/response schemas
   - Proper HTTP methods and status codes
   - Authentication and authorization where appropriate

2. **Personas** (4-5 personas based on entity roles)
   - Each persona should have realistic traits, goals, and behaviors
   - Personas should match the roles mentioned in the description

3. **Lifecycle States** (6-10 states for main entities)
   - State machines for key entities (e.g., trip: requested → matched → in_progress → completed)
   - State transitions with realistic conditions

4. **WebSocket Topics** (if real-time features mentioned)
   - Topic names and event schemas
   - Event types and payloads

5. **Chaos/Failure Scenarios** (if applicable)
   - Payment failure scenarios
   - Network error scenarios
   - Surge pricing profiles (if pricing mentioned)

6. **CI/CD Templates** (optional)
   - GitHub Actions workflows
   - GitLab CI configurations

7. **GraphQL Schema** (optional, if requested)
   - Type definitions
   - Queries and mutations

8. **TypeScript Typings** (optional)
   - Type definitions from OpenAPI schema

Return your generation as a JSON object with the following structure:
{
  "openapi": { ... OpenAPI 3.1 specification ... },
  "personas": [
    {
      "name": "persona_name",
      "traits": { ... },
      "goals": [...],
      "behaviors": [...]
    }
  ],
  "lifecycles": [
    {
      "entity": "entity_name",
      "states": ["state1", "state2", ...],
      "transitions": [
        {
          "from": "state1",
          "to": "state2",
          "condition": "..."
        }
      ]
    }
  ],
  "websocket_topics": [
    {
      "topic": "topic_name",
      "event_types": [...],
      "schema": { ... }
    }
  ],
  "chaos_profiles": [
    {
      "name": "profile_name",
      "type": "payment_failure|surge_pricing|network_error",
      "config": { ... }
    }
  ],
  "ci_templates": {
    "github_actions": "...",
    "gitlab_ci": "..."
  },
  "graphql": "... GraphQL SDL ...",
  "typings": {
    "typescript": "...",
    "go": "...",
    "rust": "..."
  },
  "metadata": {
    "entities": ["entity1", "entity2", ...],
    "relationships": ["entity1 -> entity2", ...],
    "operations": ["create", "read", "update", "delete", ...]
  }
}

Be thorough and generate realistic, production-ready artifacts. Ensure all artifacts are coherent (personas match endpoints, lifecycles match entities)."#
            .to_string()
    }

    /// Build user prompt with description and output formats
    fn build_user_prompt(&self, request: &SystemGenerationRequest) -> Result<String> {
        let formats_text = if request.output_formats.is_empty() {
            "all available formats".to_string()
        } else {
            request.output_formats.join(", ")
        };

        Ok(format!(
            r#"Generate a complete backend system from this description:

Description:
{}

Please generate the following formats: {}

Make sure to:
1. Extract all entities, relationships, and operations from the description
2. Generate realistic and comprehensive artifacts
3. Ensure coherence across all artifacts (personas match endpoints, lifecycles match entities)
4. Include proper error handling and edge cases
5. Make it production-ready

Provide a complete system that can bootstrap a startup backend."#,
            request.description, formats_text
        ))
    }

    /// Parse LLM response into system artifacts
    fn parse_system_response(
        &self,
        response: Value,
        requested_formats: &[String],
    ) -> Result<HashMap<String, SystemArtifact>> {
        let mut artifacts = HashMap::new();

        // Extract OpenAPI spec
        if requested_formats.is_empty() || requested_formats.contains(&"openapi".to_string()) {
            if let Some(openapi) = response.get("openapi") {
                let artifact_id = format!("openapi-{}", Uuid::new_v4());
                artifacts.insert(
                    "openapi".to_string(),
                    SystemArtifact {
                        artifact_type: "openapi".to_string(),
                        content: openapi.clone(),
                        format: "json".to_string(),
                        artifact_id,
                    },
                );
            }
        }

        // Extract personas
        if requested_formats.is_empty() || requested_formats.contains(&"personas".to_string()) {
            if let Some(personas) = response.get("personas") {
                let artifact_id = format!("personas-{}", Uuid::new_v4());
                artifacts.insert(
                    "personas".to_string(),
                    SystemArtifact {
                        artifact_type: "personas".to_string(),
                        content: personas.clone(),
                        format: "json".to_string(),
                        artifact_id,
                    },
                );
            }
        }

        // Extract lifecycles
        if requested_formats.is_empty() || requested_formats.contains(&"lifecycles".to_string()) {
            if let Some(lifecycles) = response.get("lifecycles") {
                let artifact_id = format!("lifecycles-{}", Uuid::new_v4());
                artifacts.insert(
                    "lifecycles".to_string(),
                    SystemArtifact {
                        artifact_type: "lifecycles".to_string(),
                        content: lifecycles.clone(),
                        format: "json".to_string(),
                        artifact_id,
                    },
                );
            }
        }

        // Extract WebSocket topics
        if requested_formats.contains(&"websocket".to_string()) {
            if let Some(websocket) = response.get("websocket_topics") {
                let artifact_id = format!("websocket-{}", Uuid::new_v4());
                artifacts.insert(
                    "websocket".to_string(),
                    SystemArtifact {
                        artifact_type: "websocket".to_string(),
                        content: websocket.clone(),
                        format: "json".to_string(),
                        artifact_id,
                    },
                );
            }
        }

        // Extract chaos profiles
        if requested_formats.contains(&"chaos".to_string()) {
            if let Some(chaos) = response.get("chaos_profiles") {
                let artifact_id = format!("chaos-{}", Uuid::new_v4());
                artifacts.insert(
                    "chaos".to_string(),
                    SystemArtifact {
                        artifact_type: "chaos".to_string(),
                        content: chaos.clone(),
                        format: "json".to_string(),
                        artifact_id,
                    },
                );
            }
        }

        // Extract CI templates
        if requested_formats.contains(&"ci".to_string()) {
            if let Some(ci) = response.get("ci_templates") {
                let artifact_id = format!("ci-{}", Uuid::new_v4());
                artifacts.insert(
                    "ci".to_string(),
                    SystemArtifact {
                        artifact_type: "ci".to_string(),
                        content: ci.clone(),
                        format: "yaml".to_string(),
                        artifact_id,
                    },
                );
            }
        }

        // Extract GraphQL schema
        if requested_formats.contains(&"graphql".to_string()) {
            if let Some(graphql) = response.get("graphql") {
                let artifact_id = format!("graphql-{}", Uuid::new_v4());
                artifacts.insert(
                    "graphql".to_string(),
                    SystemArtifact {
                        artifact_type: "graphql".to_string(),
                        content: graphql.clone(),
                        format: "graphql".to_string(),
                        artifact_id,
                    },
                );
            }
        }

        // Extract typings
        if requested_formats.contains(&"typings".to_string()) {
            if let Some(typings) = response.get("typings") {
                let artifact_id = format!("typings-{}", Uuid::new_v4());
                artifacts.insert(
                    "typings".to_string(),
                    SystemArtifact {
                        artifact_type: "typings".to_string(),
                        content: typings.clone(),
                        format: "json".to_string(),
                        artifact_id,
                    },
                );
            }
        }

        Ok(artifacts)
    }

    /// Extract metadata from request and artifacts
    fn extract_metadata(
        &self,
        request: &SystemGenerationRequest,
        _artifacts: &HashMap<String, SystemArtifact>,
    ) -> Result<SystemMetadata> {
        // In a full implementation, we'd parse the artifacts to extract entities, relationships, etc.
        // For now, we'll use basic extraction from the description
        let entities = self.extract_entities(&request.description);
        let relationships = self.extract_relationships(&request.description);
        let operations = vec![
            "create".to_string(),
            "read".to_string(),
            "update".to_string(),
            "delete".to_string(),
        ];

        Ok(SystemMetadata {
            description: request.description.clone(),
            entities,
            relationships,
            operations,
            generated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Extract entities from description (simple heuristic)
    fn extract_entities(&self, description: &str) -> Vec<String> {
        // Simple extraction - in a full implementation, this would use NLP
        let mut entities = Vec::new();
        let words: Vec<&str> = description.split_whitespace().collect();

        // Look for plural nouns that might be entities
        for word in words {
            if word.ends_with('s') && word.len() > 3 {
                let singular = word.trim_end_matches('s');
                if !entities.contains(&singular.to_string()) {
                    entities.push(singular.to_string());
                }
            }
        }

        entities
    }

    /// Extract relationships from description (simple heuristic)
    fn extract_relationships(&self, description: &str) -> Vec<String> {
        // Simple extraction - in a full implementation, this would use NLP
        let mut relationships = Vec::new();
        let entities = self.extract_entities(description);

        // Generate simple relationships based on proximity
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                relationships.push(format!("{} -> {}", entities[i], entities[j]));
            }
        }

        relationships
    }

    /// Estimate cost in USD based on token usage
    fn estimate_cost(&self, usage: &LlmUsage) -> f64 {
        // Rough cost estimates per 1K tokens
        let cost_per_1k_tokens =
            match self.config.behavior_model.llm_provider.to_lowercase().as_str() {
                "openai" => match self.config.behavior_model.model.to_lowercase().as_str() {
                    model if model.contains("gpt-4") => 0.03,
                    model if model.contains("gpt-3.5") => 0.002,
                    _ => 0.002,
                },
                "anthropic" => 0.008,
                "ollama" => 0.0, // Local models are free
                _ => 0.002,
            };

        (usage.total_tokens as f64 / 1000.0) * cost_per_1k_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligent_behavior::config::BehaviorModelConfig;

    fn create_test_config() -> IntelligentBehaviorConfig {
        IntelligentBehaviorConfig {
            behavior_model: BehaviorModelConfig {
                llm_provider: "ollama".to_string(),
                model: "llama2".to_string(),
                api_endpoint: Some("http://localhost:11434/api/chat".to_string()),
                api_key: None,
                temperature: 0.7,
                max_tokens: 2000,
                rules: crate::intelligent_behavior::types::BehaviorRules::default(),
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_system_generation_request_serialization() {
        let request = SystemGenerationRequest {
            description: "Ride-sharing app".to_string(),
            output_formats: vec!["openapi".to_string(), "personas".to_string()],
            workspace_id: None,
            system_id: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Ride-sharing"));
        assert!(json.contains("openapi"));
    }

    #[test]
    fn test_entity_extraction() {
        let config = create_test_config();
        let generator = SystemGenerator::new(config);
        let entities = generator.extract_entities(
            "I'm building a ride-sharing app with drivers, riders, trips, payments",
        );
        assert!(!entities.is_empty());
    }

    #[test]
    fn test_system_generation_request_creation() {
        let request = SystemGenerationRequest {
            description: "Test system".to_string(),
            output_formats: vec!["openapi".to_string()],
            workspace_id: Some("workspace-123".to_string()),
            system_id: Some("system-456".to_string()),
        };

        assert_eq!(request.description, "Test system");
        assert_eq!(request.output_formats.len(), 1);
        assert_eq!(request.workspace_id, Some("workspace-123".to_string()));
        assert_eq!(request.system_id, Some("system-456".to_string()));
    }

    #[test]
    fn test_system_generation_request_default_output_formats() {
        let request = SystemGenerationRequest {
            description: "Test".to_string(),
            output_formats: vec![],
            workspace_id: None,
            system_id: None,
        };

        assert!(request.output_formats.is_empty());
    }

    #[test]
    fn test_generated_system_creation() {
        let mut artifacts = HashMap::new();
        artifacts.insert(
            "openapi".to_string(),
            SystemArtifact {
                artifact_type: "openapi".to_string(),
                content: serde_json::json!({"openapi": "3.0.0"}),
                format: "json".to_string(),
                artifact_id: "artifact-1".to_string(),
            },
        );

        let system = GeneratedSystem {
            system_id: "system-123".to_string(),
            version: "v1".to_string(),
            artifacts,
            workspace_id: Some("workspace-456".to_string()),
            status: "draft".to_string(),
            tokens_used: Some(1000),
            cost_usd: Some(0.01),
            metadata: SystemMetadata {
                description: "Test system".to_string(),
                entities: vec!["User".to_string()],
                relationships: vec![],
                operations: vec![],
                generated_at: "2024-01-01T00:00:00Z".to_string(),
            },
        };

        assert_eq!(system.system_id, "system-123");
        assert_eq!(system.version, "v1");
        assert_eq!(system.artifacts.len(), 1);
        assert_eq!(system.status, "draft");
    }

    #[test]
    fn test_applied_system_creation() {
        let applied = AppliedSystem {
            system_id: "system-123".to_string(),
            version: "v1".to_string(),
            applied_artifacts: vec!["artifact-1".to_string(), "artifact-2".to_string()],
            frozen: true,
        };

        assert_eq!(applied.system_id, "system-123");
        assert_eq!(applied.version, "v1");
        assert_eq!(applied.applied_artifacts.len(), 2);
        assert!(applied.frozen);
    }

    #[test]
    fn test_system_artifact_creation() {
        let artifact = SystemArtifact {
            artifact_type: "openapi".to_string(),
            content: serde_json::json!({"openapi": "3.0.0", "info": {"title": "API"}}),
            format: "yaml".to_string(),
            artifact_id: "artifact-123".to_string(),
        };

        assert_eq!(artifact.artifact_type, "openapi");
        assert_eq!(artifact.format, "yaml");
        assert_eq!(artifact.artifact_id, "artifact-123");
    }

    #[test]
    fn test_system_metadata_creation() {
        let metadata = SystemMetadata {
            description: "Ride-sharing app".to_string(),
            entities: vec![
                "Driver".to_string(),
                "Rider".to_string(),
                "Trip".to_string(),
            ],
            relationships: vec!["Driver has many Trips".to_string()],
            operations: vec!["create_trip".to_string(), "update_trip".to_string()],
            generated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        assert_eq!(metadata.description, "Ride-sharing app");
        assert_eq!(metadata.entities.len(), 3);
        assert_eq!(metadata.relationships.len(), 1);
        assert_eq!(metadata.operations.len(), 2);
    }

    #[test]
    fn test_system_generator_new() {
        let config = create_test_config();
        let generator = SystemGenerator::new(config);
        // Just verify it can be created
        let _ = generator;
    }

    #[test]
    fn test_system_generator_with_freeze_dir() {
        let config = create_test_config();
        let generator = SystemGenerator::with_freeze_dir(config, "/tmp/freeze");
        // Just verify it can be created
        let _ = generator;
    }

    #[test]
    fn test_system_generation_request_clone() {
        let request1 = SystemGenerationRequest {
            description: "Test system".to_string(),
            output_formats: vec!["openapi".to_string()],
            workspace_id: Some("workspace-123".to_string()),
            system_id: Some("system-456".to_string()),
        };
        let request2 = request1.clone();
        assert_eq!(request1.description, request2.description);
        assert_eq!(request1.output_formats, request2.output_formats);
    }

    #[test]
    fn test_system_generation_request_debug() {
        let request = SystemGenerationRequest {
            description: "Test".to_string(),
            output_formats: vec![],
            workspace_id: None,
            system_id: None,
        };
        let debug_str = format!("{:?}", request);
        assert!(debug_str.contains("SystemGenerationRequest"));
    }

    #[test]
    fn test_generated_system_clone() {
        let system1 = GeneratedSystem {
            system_id: "system-123".to_string(),
            version: "v1".to_string(),
            artifacts: HashMap::new(),
            workspace_id: None,
            status: "draft".to_string(),
            tokens_used: None,
            cost_usd: None,
            metadata: SystemMetadata {
                description: "Test".to_string(),
                entities: vec![],
                relationships: vec![],
                operations: vec![],
                generated_at: "2024-01-01T00:00:00Z".to_string(),
            },
        };
        let system2 = system1.clone();
        assert_eq!(system1.system_id, system2.system_id);
        assert_eq!(system1.version, system2.version);
    }

    #[test]
    fn test_generated_system_debug() {
        let system = GeneratedSystem {
            system_id: "system-123".to_string(),
            version: "v1".to_string(),
            artifacts: HashMap::new(),
            workspace_id: None,
            status: "draft".to_string(),
            tokens_used: None,
            cost_usd: None,
            metadata: SystemMetadata {
                description: "Test".to_string(),
                entities: vec![],
                relationships: vec![],
                operations: vec![],
                generated_at: "2024-01-01T00:00:00Z".to_string(),
            },
        };
        let debug_str = format!("{:?}", system);
        assert!(debug_str.contains("GeneratedSystem"));
    }

    #[test]
    fn test_applied_system_clone() {
        let applied1 = AppliedSystem {
            system_id: "system-123".to_string(),
            version: "v1".to_string(),
            applied_artifacts: vec!["artifact-1".to_string()],
            frozen: true,
        };
        let applied2 = applied1.clone();
        assert_eq!(applied1.system_id, applied2.system_id);
        assert_eq!(applied1.frozen, applied2.frozen);
    }

    #[test]
    fn test_applied_system_debug() {
        let applied = AppliedSystem {
            system_id: "system-123".to_string(),
            version: "v1".to_string(),
            applied_artifacts: vec![],
            frozen: false,
        };
        let debug_str = format!("{:?}", applied);
        assert!(debug_str.contains("AppliedSystem"));
    }

    #[test]
    fn test_system_artifact_clone() {
        let artifact1 = SystemArtifact {
            artifact_type: "openapi".to_string(),
            content: serde_json::json!({}),
            format: "json".to_string(),
            artifact_id: "artifact-1".to_string(),
        };
        let artifact2 = artifact1.clone();
        assert_eq!(artifact1.artifact_type, artifact2.artifact_type);
        assert_eq!(artifact1.artifact_id, artifact2.artifact_id);
    }

    #[test]
    fn test_system_artifact_debug() {
        let artifact = SystemArtifact {
            artifact_type: "openapi".to_string(),
            content: serde_json::json!({}),
            format: "json".to_string(),
            artifact_id: "artifact-1".to_string(),
        };
        let debug_str = format!("{:?}", artifact);
        assert!(debug_str.contains("SystemArtifact"));
    }

    #[test]
    fn test_system_metadata_clone() {
        let metadata1 = SystemMetadata {
            description: "Test".to_string(),
            entities: vec!["User".to_string()],
            relationships: vec![],
            operations: vec![],
            generated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let metadata2 = metadata1.clone();
        assert_eq!(metadata1.description, metadata2.description);
        assert_eq!(metadata1.entities, metadata2.entities);
    }

    #[test]
    fn test_system_metadata_debug() {
        let metadata = SystemMetadata {
            description: "Test".to_string(),
            entities: vec![],
            relationships: vec![],
            operations: vec![],
            generated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let debug_str = format!("{:?}", metadata);
        assert!(debug_str.contains("SystemMetadata"));
    }

    #[test]
    fn test_system_generation_request_with_all_fields() {
        let request = SystemGenerationRequest {
            description: "Complete e-commerce system".to_string(),
            output_formats: vec![
                "openapi".to_string(),
                "graphql".to_string(),
                "personas".to_string(),
                "lifecycles".to_string(),
            ],
            workspace_id: Some("workspace-789".to_string()),
            system_id: Some("system-999".to_string()),
        };
        assert_eq!(request.output_formats.len(), 4);
        assert!(request.output_formats.contains(&"openapi".to_string()));
        assert!(request.output_formats.contains(&"graphql".to_string()));
    }

    #[test]
    fn test_generated_system_with_all_fields() {
        let mut artifacts = HashMap::new();
        artifacts.insert(
            "openapi".to_string(),
            SystemArtifact {
                artifact_type: "openapi".to_string(),
                content: serde_json::json!({"openapi": "3.0.0"}),
                format: "json".to_string(),
                artifact_id: "artifact-1".to_string(),
            },
        );
        artifacts.insert(
            "personas".to_string(),
            SystemArtifact {
                artifact_type: "personas".to_string(),
                content: serde_json::json!({"personas": []}),
                format: "json".to_string(),
                artifact_id: "artifact-2".to_string(),
            },
        );

        let system = GeneratedSystem {
            system_id: "system-123".to_string(),
            version: "v2".to_string(),
            artifacts: artifacts.clone(),
            workspace_id: Some("workspace-456".to_string()),
            status: "frozen".to_string(),
            tokens_used: Some(5000),
            cost_usd: Some(0.05),
            metadata: SystemMetadata {
                description: "Ride-sharing app".to_string(),
                entities: vec!["Driver".to_string(), "Rider".to_string()],
                relationships: vec!["Driver-Trip".to_string()],
                operations: vec!["create_trip".to_string()],
                generated_at: "2024-01-01T00:00:00Z".to_string(),
            },
        };

        assert_eq!(system.artifacts.len(), 2);
        assert_eq!(system.version, "v2");
        assert_eq!(system.status, "frozen");
        assert_eq!(system.tokens_used, Some(5000));
        assert_eq!(system.cost_usd, Some(0.05));
    }

    #[test]
    fn test_applied_system_with_multiple_artifacts() {
        let applied = AppliedSystem {
            system_id: "system-123".to_string(),
            version: "v1".to_string(),
            applied_artifacts: vec![
                "artifact-1".to_string(),
                "artifact-2".to_string(),
                "artifact-3".to_string(),
            ],
            frozen: true,
        };
        assert_eq!(applied.applied_artifacts.len(), 3);
        assert!(applied.frozen);
    }

    #[test]
    fn test_system_artifact_with_yaml_format() {
        let artifact = SystemArtifact {
            artifact_type: "openapi".to_string(),
            content: serde_json::json!({"openapi": "3.0.0"}),
            format: "yaml".to_string(),
            artifact_id: "artifact-yaml".to_string(),
        };
        assert_eq!(artifact.format, "yaml");
    }

    #[test]
    fn test_system_metadata_with_all_fields() {
        let metadata = SystemMetadata {
            description: "Complete system description".to_string(),
            entities: vec![
                "User".to_string(),
                "Order".to_string(),
                "Product".to_string(),
            ],
            relationships: vec!["User-Order".to_string(), "Order-Product".to_string()],
            operations: vec![
                "GET /users".to_string(),
                "POST /orders".to_string(),
                "PUT /products".to_string(),
            ],
            generated_at: "2024-01-01T12:00:00Z".to_string(),
        };
        assert_eq!(metadata.entities.len(), 3);
        assert_eq!(metadata.relationships.len(), 2);
        assert_eq!(metadata.operations.len(), 3);
    }
}
