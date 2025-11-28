//! Backend Generator Plugin Interface
//!
//! This module defines the traits and types for plugins that generate
//! framework-specific backend server code from OpenAPI specifications.

use crate::client_generator::{GeneratedFile, OpenApiSpec};
use crate::types::{PluginMetadata, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Backend generator plugin trait for generating framework-specific backend servers
#[async_trait::async_trait]
pub trait BackendGeneratorPlugin: Send + Sync {
    /// Get the backend type/framework name this plugin supports
    fn backend_type(&self) -> &str;

    /// Get a human-readable name for this backend
    fn backend_name(&self) -> &str;

    /// Get the supported OpenAPI spec versions
    fn supported_spec_versions(&self) -> Vec<&str>;

    /// Get the supported file extensions for generated code
    fn supported_extensions(&self) -> Vec<&str>;

    /// Generate backend server code from OpenAPI specification
    async fn generate_backend(
        &self,
        spec: &OpenApiSpec,
        config: &BackendGeneratorConfig,
    ) -> Result<BackendGenerationResult>;

    /// Get plugin metadata
    async fn get_metadata(&self) -> PluginMetadata;

    /// Validate the plugin configuration
    async fn validate_config(&self, _config: &BackendGeneratorConfig) -> Result<()> {
        // Default implementation - plugins can override for custom validation
        Ok(())
    }

    /// Check if this generator supports the given database type
    fn supports_database(&self, db_type: &str) -> bool;

    /// Get default port for this backend type
    fn default_port(&self) -> u16;
}

/// Backend generator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendGeneratorConfig {
    /// Output directory for generated files
    pub output_dir: String,
    /// Server port (default varies by backend)
    pub port: Option<u16>,
    /// Base URL for the API
    pub base_url: Option<String>,
    /// Whether to generate test files
    pub with_tests: bool,
    /// Whether to generate API documentation stubs
    pub with_docs: bool,
    /// Database type for integration hints (postgres, mysql, sqlite, mongo, etc.)
    pub database: Option<String>,
    /// Whether to generate TODO.md file
    pub generate_todo_md: bool,
    /// Additional configuration options
    pub options: HashMap<String, serde_json::Value>,
}

impl Default for BackendGeneratorConfig {
    fn default() -> Self {
        Self {
            output_dir: "./generated-backend".to_string(),
            port: None,
            base_url: None,
            with_tests: false,
            with_docs: false,
            database: None,
            generate_todo_md: true,
            options: HashMap::new(),
        }
    }
}

/// Result of backend generation
#[derive(Debug, Clone)]
pub struct BackendGenerationResult {
    /// Generated files with their content
    pub files: Vec<GeneratedFile>,
    /// Generation warnings
    pub warnings: Vec<String>,
    /// Generation metadata
    pub metadata: BackendGenerationMetadata,
    /// TODO items extracted from generated code
    pub todos: Vec<TodoItem>,
}

/// Generation metadata for backend code
#[derive(Debug, Clone)]
pub struct BackendGenerationMetadata {
    /// Backend framework name
    pub framework: String,
    /// Generated backend name
    pub backend_name: String,
    /// API title from spec
    pub api_title: String,
    /// API version from spec
    pub api_version: String,
    /// Number of operations/endpoints generated
    pub operation_count: usize,
    /// Number of schemas/models generated
    pub schema_count: usize,
    /// Default port for the server
    pub default_port: u16,
}

/// TODO item extracted from generated code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// TODO description
    pub description: String,
    /// File path where TODO was found
    pub file_path: String,
    /// Line number where TODO was found
    pub line_number: usize,
    /// Operation/endpoint this TODO relates to
    pub related_operation: Option<String>,
    /// Category (handler, model, config, test, etc.)
    pub category: TodoCategory,
    /// Definition of Done criteria
    pub definition_of_done: Vec<String>,
    /// Estimated complexity (Low, Medium, High)
    pub complexity: Complexity,
    /// Dependencies on other TODOs
    pub dependencies: Vec<String>,
}

/// TODO category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoCategory {
    /// Handler implementation TODO
    Handler,
    /// Model/schema TODO
    Model,
    /// Configuration TODO
    Config,
    /// Test TODO
    Test,
    /// Documentation TODO
    Docs,
    /// Database/migration TODO
    Database,
    /// Other
    Other,
}

impl std::fmt::Display for TodoCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoCategory::Handler => write!(f, "Handler"),
            TodoCategory::Model => write!(f, "Model"),
            TodoCategory::Config => write!(f, "Configuration"),
            TodoCategory::Test => write!(f, "Testing"),
            TodoCategory::Docs => write!(f, "Documentation"),
            TodoCategory::Database => write!(f, "Database"),
            TodoCategory::Other => write!(f, "Other"),
        }
    }
}

/// Estimated complexity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Complexity {
    /// Low complexity - straightforward implementation
    Low,
    /// Medium complexity - requires some design decisions
    Medium,
    /// High complexity - complex logic or architecture
    High,
}

impl std::fmt::Display for Complexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Complexity::Low => write!(f, "Low"),
            Complexity::Medium => write!(f, "Medium"),
            Complexity::High => write!(f, "High"),
        }
    }
}

/// Backend generator plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendGeneratorPluginConfig {
    /// Plugin name
    pub name: String,
    /// Backend type/framework
    pub backend_type: String,
    /// Plugin-specific options
    pub options: HashMap<String, serde_json::Value>,
}
