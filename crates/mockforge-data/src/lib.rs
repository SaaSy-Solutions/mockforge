//! # MockForge Data
//!
//! Synthetic data generation engine with faker primitives and RAG (Retrieval-Augmented Generation).

pub mod dataset;
pub mod faker;
pub mod generator;
pub mod provider;
pub mod rag;
pub mod schema;

pub use dataset::{Dataset, DatasetValidationResult};
pub use fake::Faker;
pub use generator::DataGenerator;
pub use rag::{RagEngine, RagConfig, LlmProvider, EmbeddingProvider, SearchResult};
pub use schema::{FieldDefinition, SchemaDefinition};

/// Data generation configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DataConfig {
    /// Number of rows to generate
    pub rows: usize,
    /// Random seed for reproducible generation
    pub seed: Option<u64>,
    /// Enable RAG mode
    pub rag_enabled: bool,
    /// Maximum RAG context length
    pub rag_context_length: usize,
    /// Output format
    pub format: OutputFormat,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            rows: 100,
            seed: None,
            rag_enabled: false,
            rag_context_length: 1000,
            format: OutputFormat::Json,
        }
    }
}

/// Output format for generated data
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// JSON format
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
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.data)
    }

    /// Get data as JSON Lines string
    pub fn to_jsonl_string(&self) -> Result<String, serde_json::Error> {
        let lines: Result<Vec<String>, _> = self.data.iter().map(serde_json::to_string).collect();
        lines.map(|lines| lines.join("\n"))
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
