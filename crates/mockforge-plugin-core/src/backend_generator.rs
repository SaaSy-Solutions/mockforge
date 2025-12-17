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

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== BackendGeneratorConfig Tests ====================

    #[test]
    fn test_backend_generator_config_default() {
        let config = BackendGeneratorConfig::default();

        assert_eq!(config.output_dir, "./generated-backend");
        assert!(config.port.is_none());
        assert!(config.base_url.is_none());
        assert!(!config.with_tests);
        assert!(!config.with_docs);
        assert!(config.database.is_none());
        assert!(config.generate_todo_md);
        assert!(config.options.is_empty());
    }

    #[test]
    fn test_backend_generator_config_custom() {
        let config = BackendGeneratorConfig {
            output_dir: "/custom/output".to_string(),
            port: Some(8080),
            base_url: Some("http://localhost".to_string()),
            with_tests: true,
            with_docs: true,
            database: Some("postgres".to_string()),
            generate_todo_md: false,
            options: HashMap::new(),
        };

        assert_eq!(config.output_dir, "/custom/output");
        assert_eq!(config.port, Some(8080));
        assert_eq!(config.base_url, Some("http://localhost".to_string()));
        assert!(config.with_tests);
        assert!(config.with_docs);
        assert_eq!(config.database, Some("postgres".to_string()));
        assert!(!config.generate_todo_md);
    }

    #[test]
    fn test_backend_generator_config_clone() {
        let config = BackendGeneratorConfig {
            output_dir: "./test".to_string(),
            port: Some(3000),
            ..Default::default()
        };

        let cloned = config.clone();
        assert_eq!(cloned.output_dir, config.output_dir);
        assert_eq!(cloned.port, config.port);
    }

    #[test]
    fn test_backend_generator_config_serialization() {
        let config = BackendGeneratorConfig::default();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: BackendGeneratorConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.output_dir, config.output_dir);
        assert_eq!(deserialized.with_tests, config.with_tests);
    }

    #[test]
    fn test_backend_generator_config_with_options() {
        let mut options = HashMap::new();
        options.insert("key1".to_string(), serde_json::json!("value1"));
        options.insert("count".to_string(), serde_json::json!(42));

        let config = BackendGeneratorConfig {
            options,
            ..Default::default()
        };

        assert_eq!(config.options.len(), 2);
        assert_eq!(config.options.get("key1"), Some(&serde_json::json!("value1")));
    }

    // ==================== BackendGenerationMetadata Tests ====================

    #[test]
    fn test_backend_generation_metadata() {
        let metadata = BackendGenerationMetadata {
            framework: "axum".to_string(),
            backend_name: "MyApi".to_string(),
            api_title: "My API".to_string(),
            api_version: "1.0.0".to_string(),
            operation_count: 10,
            schema_count: 5,
            default_port: 8080,
        };

        assert_eq!(metadata.framework, "axum");
        assert_eq!(metadata.backend_name, "MyApi");
        assert_eq!(metadata.api_title, "My API");
        assert_eq!(metadata.api_version, "1.0.0");
        assert_eq!(metadata.operation_count, 10);
        assert_eq!(metadata.schema_count, 5);
        assert_eq!(metadata.default_port, 8080);
    }

    #[test]
    fn test_backend_generation_metadata_clone() {
        let metadata = BackendGenerationMetadata {
            framework: "actix-web".to_string(),
            backend_name: "TestBackend".to_string(),
            api_title: "Test".to_string(),
            api_version: "2.0.0".to_string(),
            operation_count: 20,
            schema_count: 10,
            default_port: 3000,
        };

        let cloned = metadata.clone();
        assert_eq!(cloned.framework, metadata.framework);
        assert_eq!(cloned.operation_count, metadata.operation_count);
    }

    // ==================== TodoCategory Tests ====================

    #[test]
    fn test_todo_category_display_handler() {
        assert_eq!(format!("{}", TodoCategory::Handler), "Handler");
    }

    #[test]
    fn test_todo_category_display_model() {
        assert_eq!(format!("{}", TodoCategory::Model), "Model");
    }

    #[test]
    fn test_todo_category_display_config() {
        assert_eq!(format!("{}", TodoCategory::Config), "Configuration");
    }

    #[test]
    fn test_todo_category_display_test() {
        assert_eq!(format!("{}", TodoCategory::Test), "Testing");
    }

    #[test]
    fn test_todo_category_display_docs() {
        assert_eq!(format!("{}", TodoCategory::Docs), "Documentation");
    }

    #[test]
    fn test_todo_category_display_database() {
        assert_eq!(format!("{}", TodoCategory::Database), "Database");
    }

    #[test]
    fn test_todo_category_display_other() {
        assert_eq!(format!("{}", TodoCategory::Other), "Other");
    }

    #[test]
    fn test_todo_category_equality() {
        assert_eq!(TodoCategory::Handler, TodoCategory::Handler);
        assert_ne!(TodoCategory::Handler, TodoCategory::Model);
    }

    #[test]
    fn test_todo_category_serialization() {
        let category = TodoCategory::Handler;
        let json = serde_json::to_string(&category).unwrap();
        let deserialized: TodoCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, category);
    }

    // ==================== Complexity Tests ====================

    #[test]
    fn test_complexity_display_low() {
        assert_eq!(format!("{}", Complexity::Low), "Low");
    }

    #[test]
    fn test_complexity_display_medium() {
        assert_eq!(format!("{}", Complexity::Medium), "Medium");
    }

    #[test]
    fn test_complexity_display_high() {
        assert_eq!(format!("{}", Complexity::High), "High");
    }

    #[test]
    fn test_complexity_equality() {
        assert_eq!(Complexity::Low, Complexity::Low);
        assert_ne!(Complexity::Low, Complexity::High);
    }

    #[test]
    fn test_complexity_serialization() {
        let complexity = Complexity::Medium;
        let json = serde_json::to_string(&complexity).unwrap();
        let deserialized: Complexity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, complexity);
    }

    // ==================== TodoItem Tests ====================

    #[test]
    fn test_todo_item_creation() {
        let todo = TodoItem {
            description: "Implement user authentication".to_string(),
            file_path: "src/handlers/auth.rs".to_string(),
            line_number: 42,
            related_operation: Some("login".to_string()),
            category: TodoCategory::Handler,
            definition_of_done: vec!["Add JWT validation".to_string()],
            complexity: Complexity::Medium,
            dependencies: vec![],
        };

        assert_eq!(todo.description, "Implement user authentication");
        assert_eq!(todo.file_path, "src/handlers/auth.rs");
        assert_eq!(todo.line_number, 42);
        assert_eq!(todo.related_operation, Some("login".to_string()));
        assert_eq!(todo.category, TodoCategory::Handler);
        assert_eq!(todo.complexity, Complexity::Medium);
    }

    #[test]
    fn test_todo_item_without_operation() {
        let todo = TodoItem {
            description: "Configure logging".to_string(),
            file_path: "src/config.rs".to_string(),
            line_number: 10,
            related_operation: None,
            category: TodoCategory::Config,
            definition_of_done: vec![],
            complexity: Complexity::Low,
            dependencies: vec![],
        };

        assert!(todo.related_operation.is_none());
        assert!(todo.definition_of_done.is_empty());
    }

    #[test]
    fn test_todo_item_with_dependencies() {
        let todo = TodoItem {
            description: "Implement order processing".to_string(),
            file_path: "src/handlers/orders.rs".to_string(),
            line_number: 100,
            related_operation: Some("processOrder".to_string()),
            category: TodoCategory::Handler,
            definition_of_done: vec!["Handle edge cases".to_string()],
            complexity: Complexity::High,
            dependencies: vec!["auth_todo".to_string(), "db_todo".to_string()],
        };

        assert_eq!(todo.dependencies.len(), 2);
        assert!(todo.dependencies.contains(&"auth_todo".to_string()));
    }

    #[test]
    fn test_todo_item_clone() {
        let todo = TodoItem {
            description: "Test item".to_string(),
            file_path: "test.rs".to_string(),
            line_number: 1,
            related_operation: None,
            category: TodoCategory::Test,
            definition_of_done: vec![],
            complexity: Complexity::Low,
            dependencies: vec![],
        };

        let cloned = todo.clone();
        assert_eq!(cloned.description, todo.description);
        assert_eq!(cloned.category, todo.category);
    }

    #[test]
    fn test_todo_item_serialization() {
        let todo = TodoItem {
            description: "Serialize test".to_string(),
            file_path: "test.rs".to_string(),
            line_number: 1,
            related_operation: Some("test".to_string()),
            category: TodoCategory::Test,
            definition_of_done: vec!["Done".to_string()],
            complexity: Complexity::Low,
            dependencies: vec![],
        };

        let json = serde_json::to_string(&todo).unwrap();
        let deserialized: TodoItem = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.description, todo.description);
        assert_eq!(deserialized.category, todo.category);
    }

    // ==================== BackendGeneratorPluginConfig Tests ====================

    #[test]
    fn test_backend_generator_plugin_config() {
        let config = BackendGeneratorPluginConfig {
            name: "axum-generator".to_string(),
            backend_type: "axum".to_string(),
            options: HashMap::new(),
        };

        assert_eq!(config.name, "axum-generator");
        assert_eq!(config.backend_type, "axum");
        assert!(config.options.is_empty());
    }

    #[test]
    fn test_backend_generator_plugin_config_with_options() {
        let mut options = HashMap::new();
        options.insert("feature_flags".to_string(), serde_json::json!(["auth", "logging"]));

        let config = BackendGeneratorPluginConfig {
            name: "custom-generator".to_string(),
            backend_type: "custom".to_string(),
            options,
        };

        assert_eq!(config.options.len(), 1);
    }

    #[test]
    fn test_backend_generator_plugin_config_clone() {
        let config = BackendGeneratorPluginConfig {
            name: "test".to_string(),
            backend_type: "test".to_string(),
            options: HashMap::new(),
        };

        let cloned = config.clone();
        assert_eq!(cloned.name, config.name);
    }

    #[test]
    fn test_backend_generator_plugin_config_serialization() {
        let config = BackendGeneratorPluginConfig {
            name: "test-plugin".to_string(),
            backend_type: "actix".to_string(),
            options: HashMap::new(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: BackendGeneratorPluginConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, config.name);
        assert_eq!(deserialized.backend_type, config.backend_type);
    }

    // ==================== BackendGenerationResult Tests ====================

    #[test]
    fn test_backend_generation_result() {
        use crate::client_generator::GeneratedFile;

        let result = BackendGenerationResult {
            files: vec![GeneratedFile {
                path: "src/main.rs".to_string(),
                content: "fn main() {}".to_string(),
                file_type: "rust".to_string(),
            }],
            warnings: vec!["Warning 1".to_string()],
            metadata: BackendGenerationMetadata {
                framework: "axum".to_string(),
                backend_name: "Test".to_string(),
                api_title: "Test API".to_string(),
                api_version: "1.0.0".to_string(),
                operation_count: 5,
                schema_count: 3,
                default_port: 8080,
            },
            todos: vec![],
        };

        assert_eq!(result.files.len(), 1);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.todos.is_empty());
        assert_eq!(result.metadata.framework, "axum");
    }

    #[test]
    fn test_backend_generation_result_clone() {
        use crate::client_generator::GeneratedFile;

        let result = BackendGenerationResult {
            files: vec![GeneratedFile {
                path: "test.rs".to_string(),
                content: "test".to_string(),
                file_type: "rust".to_string(),
            }],
            warnings: vec![],
            metadata: BackendGenerationMetadata {
                framework: "test".to_string(),
                backend_name: "test".to_string(),
                api_title: "Test".to_string(),
                api_version: "1.0.0".to_string(),
                operation_count: 0,
                schema_count: 0,
                default_port: 8080,
            },
            todos: vec![],
        };

        let cloned = result.clone();
        assert_eq!(cloned.files.len(), result.files.len());
        assert_eq!(cloned.metadata.framework, result.metadata.framework);
    }
}
