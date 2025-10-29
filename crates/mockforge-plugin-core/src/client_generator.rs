//! Client Generator Plugin Interface
//!
//! This module defines the traits and types for plugins that generate
//! framework-specific mock clients from OpenAPI specifications.

use crate::types::{PluginMetadata, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Client generator plugin trait for generating framework-specific mock clients
#[async_trait::async_trait]
pub trait ClientGeneratorPlugin {
    /// Get the framework name this plugin supports
    fn framework_name(&self) -> &str;

    /// Get the supported file extensions for this framework
    fn supported_extensions(&self) -> Vec<&str>;

    /// Generate mock client code from OpenAPI specification
    async fn generate_client(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<ClientGenerationResult>;

    /// Get plugin metadata
    async fn get_metadata(&self) -> PluginMetadata;

    /// Validate the plugin configuration
    async fn validate_config(&self, _config: &ClientGeneratorConfig) -> Result<()> {
        // Default implementation - plugins can override for custom validation
        Ok(())
    }
}

/// OpenAPI specification data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    /// OpenAPI version (e.g., "3.0.0")
    pub openapi: String,
    /// API information
    pub info: ApiInfo,
    /// Server URLs
    pub servers: Option<Vec<Server>>,
    /// API paths and operations
    pub paths: HashMap<String, PathItem>,
    /// Component schemas
    pub components: Option<Components>,
}

/// API information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    /// API title
    pub title: String,
    /// API version
    pub version: String,
    /// API description
    pub description: Option<String>,
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    /// Server URL
    pub url: String,
    /// Server description
    pub description: Option<String>,
}

/// Path item containing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
    /// HTTP operations
    #[serde(flatten)]
    pub operations: HashMap<String, Operation>,
}

/// API operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Operation summary
    pub summary: Option<String>,
    /// Operation description
    pub description: Option<String>,
    /// Operation ID
    pub operation_id: Option<String>,
    /// Request parameters
    pub parameters: Option<Vec<Parameter>>,
    /// Request body
    pub request_body: Option<RequestBody>,
    /// Responses
    pub responses: HashMap<String, Response>,
    /// Operation tags
    pub tags: Option<Vec<String>>,
}

/// Request parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Parameter location (query, path, header, cookie)
    pub r#in: String,
    /// Parameter description
    pub description: Option<String>,
    /// Whether parameter is required
    pub required: Option<bool>,
    /// Parameter schema
    pub schema: Option<Schema>,
}

/// Request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    /// Request body description
    pub description: Option<String>,
    /// Request body content
    pub content: HashMap<String, MediaType>,
    /// Whether request body is required
    pub required: Option<bool>,
}

/// Media type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    /// Media type schema
    pub schema: Option<Schema>,
}

/// API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Response description
    pub description: String,
    /// Response content
    pub content: Option<HashMap<String, MediaType>>,
    /// Response headers
    pub headers: Option<HashMap<String, Header>>,
}

/// Response header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    /// Header description
    pub description: Option<String>,
    /// Header schema
    pub schema: Option<Schema>,
}

/// Components (schemas, responses, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    /// Schema definitions
    pub schemas: Option<HashMap<String, Schema>>,
    /// Response definitions
    pub responses: Option<HashMap<String, Response>>,
    /// Parameter definitions
    pub parameters: Option<HashMap<String, Parameter>>,
}

/// JSON Schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Schema type
    pub r#type: Option<String>,
    /// Schema format
    pub format: Option<String>,
    /// Schema properties
    pub properties: Option<HashMap<String, Schema>>,
    /// Required properties
    pub required: Option<Vec<String>>,
    /// Schema items (for arrays)
    pub items: Option<Box<Schema>>,
    /// Schema description
    pub description: Option<String>,
    /// Schema example
    pub example: Option<serde_json::Value>,
    /// Schema enum values
    pub r#enum: Option<Vec<serde_json::Value>>,
    /// Reference to another schema
    #[serde(rename = "$ref")]
    pub ref_path: Option<String>,
}

/// Client generator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientGeneratorConfig {
    /// Output directory for generated files
    pub output_dir: String,
    /// Base URL for the API
    pub base_url: Option<String>,
    /// Whether to include TypeScript types
    pub include_types: bool,
    /// Whether to include mock data generation
    pub include_mocks: bool,
    /// Custom template directory
    pub template_dir: Option<String>,
    /// Additional configuration options
    pub options: HashMap<String, serde_json::Value>,
}

/// Result of client generation
#[derive(Debug, Clone)]
pub struct ClientGenerationResult {
    /// Generated files with their content
    pub files: Vec<GeneratedFile>,
    /// Generation warnings
    pub warnings: Vec<String>,
    /// Generation metadata
    pub metadata: GenerationMetadata,
}

/// Generated file
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// File path relative to output directory
    pub path: String,
    /// File content
    pub content: String,
    /// File type (e.g., "typescript", "javascript", "vue", "react")
    pub file_type: String,
}

/// Generation metadata
#[derive(Debug, Clone)]
pub struct GenerationMetadata {
    /// Framework name
    pub framework: String,
    /// Generated client name
    pub client_name: String,
    /// API title from spec
    pub api_title: String,
    /// API version from spec
    pub api_version: String,
    /// Number of operations generated
    pub operation_count: usize,
    /// Number of schemas generated
    pub schema_count: usize,
}

/// Client generator plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientGeneratorPluginConfig {
    /// Plugin name
    pub name: String,
    /// Framework name
    pub framework: String,
    /// Plugin-specific options
    pub options: HashMap<String, serde_json::Value>,
}

/// Helper functions for client generation
pub mod helpers {
    use super::*;

    /// Convert OpenAPI operation to a more convenient format
    pub fn normalize_operation(
        method: &str,
        path: &str,
        operation: &Operation,
    ) -> NormalizedOperation {
        NormalizedOperation {
            method: method.to_uppercase(),
            path: path.to_string(),
            operation_id: operation.operation_id.clone().unwrap_or_else(|| {
                // Generate operation ID from method and path
                format!(
                    "{}_{}",
                    method.to_lowercase(),
                    path.replace('/', "_").replace('{', "").replace('}', "")
                )
            }),
            summary: operation.summary.clone(),
            description: operation.description.clone(),
            parameters: operation.parameters.clone().unwrap_or_default(),
            request_body: operation.request_body.clone(),
            responses: operation.responses.clone(),
            tags: operation.tags.clone().unwrap_or_default(),
        }
    }

    /// Normalized operation structure
    #[derive(Debug, Clone)]
    pub struct NormalizedOperation {
        /// HTTP method (GET, POST, etc.)
        pub method: String,
        /// API path
        pub path: String,
        /// Operation identifier
        pub operation_id: String,
        /// Operation summary
        pub summary: Option<String>,
        /// Operation description
        pub description: Option<String>,
        /// Request parameters
        pub parameters: Vec<Parameter>,
        /// Request body specification
        pub request_body: Option<RequestBody>,
        /// Response specifications
        pub responses: HashMap<String, Response>,
        /// Operation tags
        pub tags: Vec<String>,
    }

    /// Generate TypeScript type from OpenAPI schema
    pub fn schema_to_typescript_type(schema: &Schema) -> String {
        match schema.r#type.as_deref() {
            Some("string") => match schema.format.as_deref() {
                Some("date") => "string".to_string(),
                Some("date-time") => "string".to_string(),
                Some("email") => "string".to_string(),
                Some("uri") => "string".to_string(),
                _ => "string".to_string(),
            },
            Some("integer") | Some("number") => "number".to_string(),
            Some("boolean") => "boolean".to_string(),
            Some("array") => {
                if let Some(items) = &schema.items {
                    format!("{}[]", schema_to_typescript_type(items))
                } else {
                    "any[]".to_string()
                }
            }
            Some("object") => {
                if let Some(properties) = &schema.properties {
                    let mut props = Vec::new();
                    for (name, prop_schema) in properties {
                        let prop_type = schema_to_typescript_type(prop_schema);
                        let required =
                            schema.required.as_ref().map(|req| req.contains(name)).unwrap_or(false);

                        if required {
                            props.push(format!("  {}: {}", name, prop_type));
                        } else {
                            props.push(format!("  {}?: {}", name, prop_type));
                        }
                    }
                    format!("{{\n{}\n}}", props.join(",\n"))
                } else {
                    "Record<string, any>".to_string()
                }
            }
            _ => "any".to_string(),
        }
    }

    /// Extract path parameters from OpenAPI path
    pub fn extract_path_parameters(path: &str) -> Vec<String> {
        let mut params = Vec::new();
        let mut chars = path.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                let mut param = String::new();
                while let Some(ch) = chars.next() {
                    if ch == '}' {
                        break;
                    }
                    param.push(ch);
                }
                if !param.is_empty() {
                    params.push(param);
                }
            }
        }

        params
    }

    /// Convert OpenAPI path to framework-specific path format
    pub fn convert_path_format(path: &str, framework: &str) -> String {
        match framework {
            "react" | "vue" | "angular" => {
                // Convert {param} to :param for most frontend frameworks
                path.replace('{', ":").replace('}', "")
            }
            "svelte" => {
                // Svelte uses [param] format
                let result = path.to_string();
                let mut chars = result.chars().collect::<Vec<_>>();
                let mut i = 0;
                while i < chars.len() {
                    if chars[i] == '{' {
                        chars[i] = '[';
                        i += 1;
                        while i < chars.len() && chars[i] != '}' {
                            i += 1;
                        }
                        if i < chars.len() {
                            chars[i] = ']';
                        }
                    }
                    i += 1;
                }
                chars.into_iter().collect()
            }
            _ => path.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_path_parameters() {
        assert_eq!(helpers::extract_path_parameters("/api/users/{id}"), vec!["id"]);
        assert_eq!(
            helpers::extract_path_parameters("/api/users/{id}/posts/{postId}"),
            vec!["id", "postId"]
        );
        assert_eq!(helpers::extract_path_parameters("/api/users"), Vec::<String>::new());
    }

    #[test]
    fn test_convert_path_format() {
        assert_eq!(helpers::convert_path_format("/api/users/{id}", "react"), "/api/users/:id");
        assert_eq!(helpers::convert_path_format("/api/users/{id}", "svelte"), "/api/users/[id]");
    }

    #[test]
    fn test_schema_to_typescript_type() {
        let string_schema = Schema {
            r#type: Some("string".to_string()),
            format: None,
            properties: None,
            required: None,
            items: None,
            description: None,
            example: None,
            r#enum: None,
            ref_path: None,
        };
        assert_eq!(helpers::schema_to_typescript_type(&string_schema), "string");

        let number_schema = Schema {
            r#type: Some("number".to_string()),
            format: None,
            properties: None,
            required: None,
            items: None,
            description: None,
            example: None,
            r#enum: None,
            ref_path: None,
        };
        assert_eq!(helpers::schema_to_typescript_type(&number_schema), "number");
    }
}
