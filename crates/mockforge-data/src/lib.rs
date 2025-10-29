//! # MockForge Data
//!
//! Synthetic data generation engine with faker primitives and RAG (Retrieval-Augmented Generation).

// Re-export error types from mockforge-core
pub use mockforge_core::{Error, Result};

pub mod dataset;
pub mod domains;
pub mod drift;
pub mod faker;
pub mod generator;
pub mod intelligent_mock;
pub mod mock_generator;
pub mod mock_server;
pub mod provider;
pub mod rag;
pub mod replay_augmentation;
pub mod schema;
pub mod token_resolver;

#[cfg(test)]
mod mock_data_tests;

pub use dataset::{Dataset, DatasetValidationResult};
pub use domains::{Domain, DomainGenerator, ParseDomainError};
pub use drift::{DataDriftConfig, DataDriftEngine, DriftStrategy};
pub use fake::Faker;
pub use generator::DataGenerator;
pub use intelligent_mock::{IntelligentMockConfig, IntelligentMockGenerator, ResponseMode};
pub use mock_generator::{MockDataGenerator, MockDataResult, MockGeneratorConfig, MockResponse};
pub use mock_server::{
    start_mock_server, start_mock_server_with_config, MockServer, MockServerBuilder,
    MockServerConfig,
};
pub use rag::{EmbeddingProvider, LlmProvider, RagConfig, RagEngine, SearchResult};
pub use replay_augmentation::{
    EventStrategy, GeneratedEvent, ReplayAugmentationConfig, ReplayAugmentationEngine, ReplayMode,
};
pub use schema::{FieldDefinition, SchemaDefinition};
pub use token_resolver::{resolve_tokens, resolve_tokens_with_rag, TokenResolver, TokenType};

/// Data generation configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct DataConfig {
    /// Number of rows to generate
    #[serde(default = "default_rows")]
    pub rows: usize,
    /// Random seed for reproducible generation
    pub seed: Option<u64>,
    /// Enable RAG mode
    pub rag_enabled: bool,
    /// Maximum RAG context length
    #[serde(default = "default_rag_context_length")]
    pub rag_context_length: usize,
    /// Output format
    pub format: OutputFormat,
}

fn default_rows() -> usize {
    100
}
fn default_rag_context_length() -> usize {
    1000
}

/// Output format for generated data
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// JSON format
    #[default]
    Json,
    /// JSON Lines format
    JsonLines,
    /// YAML format
    Yaml,
    /// CSV format
    Csv,
}

/// Generation result
#[derive(Debug)]
pub struct GenerationResult {
    /// Generated data
    pub data: Vec<serde_json::Value>,
    /// Number of rows generated
    pub count: usize,
    /// Generation time in milliseconds
    pub generation_time_ms: u128,
    /// Any warnings during generation
    pub warnings: Vec<String>,
}

impl GenerationResult {
    /// Create a new generation result
    pub fn new(data: Vec<serde_json::Value>, generation_time_ms: u128) -> Self {
        Self {
            count: data.len(),
            data,
            generation_time_ms,
            warnings: Vec::new(),
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }

    /// Get data as JSON string
    pub fn to_json_string(&self) -> mockforge_core::Result<String> {
        Ok(serde_json::to_string_pretty(&self.data)?)
    }

    /// Get data as JSON Lines string
    pub fn to_jsonl_string(&self) -> mockforge_core::Result<String> {
        let lines: Vec<String> = self
            .data
            .iter()
            .map(serde_json::to_string)
            .collect::<std::result::Result<_, _>>()?;
        Ok(lines.join("\n"))
    }
}

/// Quick data generation function
pub async fn generate_data(
    schema: SchemaDefinition,
    config: DataConfig,
) -> mockforge_core::Result<GenerationResult> {
    let mut generator = DataGenerator::new(schema, config)?;
    generator.generate().await
}

/// Generate sample data from a JSON schema
pub async fn generate_from_json_schema(
    json_schema: &serde_json::Value,
    rows: usize,
) -> mockforge_core::Result<GenerationResult> {
    let schema = SchemaDefinition::from_json_schema(json_schema)?;
    let config = DataConfig {
        rows,
        ..Default::default()
    };
    generate_data(schema, config).await
}

/// Generate sample data from an OpenAPI schema
pub async fn generate_from_openapi(
    openapi_spec: &serde_json::Value,
    rows: usize,
) -> mockforge_core::Result<GenerationResult> {
    let schema = SchemaDefinition::from_openapi_spec(openapi_spec)?;
    let config = DataConfig {
        rows,
        ..Default::default()
    };
    generate_data(schema, config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_data_config_default() {
        let config = DataConfig::default();
        assert_eq!(config.rows, 0); // Default for usize is 0
        assert_eq!(config.seed, None);
        assert!(!config.rag_enabled);
        assert_eq!(config.rag_context_length, 0); // Default for usize is 0
        assert!(matches!(config.format, OutputFormat::Json));
    }

    #[test]
    fn test_data_config_custom() {
        let config = DataConfig {
            rows: 50,
            seed: Some(42),
            rag_enabled: true,
            rag_context_length: 2000,
            format: OutputFormat::Csv,
        };

        assert_eq!(config.rows, 50);
        assert_eq!(config.seed, Some(42));
        assert!(config.rag_enabled);
        assert_eq!(config.rag_context_length, 2000);
        assert!(matches!(config.format, OutputFormat::Csv));
    }

    #[test]
    fn test_output_format_variants() {
        let json = OutputFormat::Json;
        let jsonlines = OutputFormat::JsonLines;
        let yaml = OutputFormat::Yaml;
        let csv = OutputFormat::Csv;

        assert!(matches!(json, OutputFormat::Json));
        assert!(matches!(jsonlines, OutputFormat::JsonLines));
        assert!(matches!(yaml, OutputFormat::Yaml));
        assert!(matches!(csv, OutputFormat::Csv));
    }

    #[test]
    fn test_generation_result_new() {
        let data = vec![json!({"id": 1, "name": "test"})];
        let result = GenerationResult::new(data.clone(), 100);

        assert_eq!(result.count, 1);
        assert_eq!(result.data.len(), 1);
        assert_eq!(result.generation_time_ms, 100);
        assert_eq!(result.warnings.len(), 0);
    }

    #[test]
    fn test_generation_result_with_warning() {
        let data = vec![json!({"id": 1})];
        let result = GenerationResult::new(data, 50).with_warning("Test warning".to_string());

        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0], "Test warning");
    }

    #[test]
    fn test_generation_result_to_json_string() {
        let data = vec![json!({"id": 1, "name": "test"})];
        let result = GenerationResult::new(data, 10);

        let json_string = result.to_json_string();
        assert!(json_string.is_ok());
        let json_str = json_string.unwrap();
        assert!(json_str.contains("\"id\""));
        assert!(json_str.contains("\"name\""));
    }

    #[test]
    fn test_generation_result_to_jsonl_string() {
        let data = vec![json!({"id": 1}), json!({"id": 2})];
        let result = GenerationResult::new(data, 10);

        let jsonl_string = result.to_jsonl_string();
        assert!(jsonl_string.is_ok());
        let jsonl_str = jsonl_string.unwrap();
        assert!(jsonl_str.contains("{\"id\":1}"));
        assert!(jsonl_str.contains("{\"id\":2}"));
        assert!(jsonl_str.contains("\n"));
    }

    #[test]
    fn test_generation_result_multiple_warnings() {
        let data = vec![json!({"id": 1})];
        let result = GenerationResult::new(data, 10)
            .with_warning("Warning 1".to_string())
            .with_warning("Warning 2".to_string());

        assert_eq!(result.warnings.len(), 2);
        assert_eq!(result.warnings[0], "Warning 1");
        assert_eq!(result.warnings[1], "Warning 2");
    }

    #[test]
    fn test_default_rows() {
        assert_eq!(default_rows(), 100);
    }

    #[test]
    fn test_default_rag_context_length() {
        assert_eq!(default_rag_context_length(), 1000);
    }
}
