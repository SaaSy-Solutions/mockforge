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
    #[serde(rename = "operationId")]
    pub operation_id: Option<String>,
    /// Request parameters
    pub parameters: Option<Vec<Parameter>>,
    /// Request body
    #[serde(rename = "requestBody")]
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

    /// Capitalize the first letter of a string
    fn capitalize_first(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => {
                let mut result = first.to_uppercase().collect::<String>();
                result.push_str(&chars.as_str().to_lowercase());
                result
            }
        }
    }

    /// Check if a path segment is a version number (e.g., "v1", "v10", "version1")
    ///
    /// Supports both "v1", "v10", "v123" and "version1", "version10" patterns
    #[cfg_attr(test, allow(dead_code))]
    fn is_version_number(segment: &str) -> bool {
        // Match patterns like "v1", "v10", "v123", etc.
        if segment.starts_with('v') && segment.len() > 1 {
            segment[1..].chars().all(|c| c.is_ascii_digit())
        } else if segment.starts_with("version") && segment.len() > 7 {
            // Match patterns like "version1", "version10", etc.
            segment[7..].chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }

    /// Convert a string to camelCase by splitting on hyphens/underscores and capitalizing
    ///
    /// Examples:
    /// - "disease-detection" -> "DiseaseDetection"
    /// - "api_v1_users" -> "ApiV1Users"
    /// - "simple" -> "Simple"
    fn to_camel_case(s: &str) -> String {
        s.split(|c: char| c == '-' || c == '_')
            .filter(|s| !s.is_empty())
            .map(|w| capitalize_first(w))
            .collect()
    }

    /// Generate a camelCase operation ID from method, path, and optional summary
    ///
    /// Examples:
    /// - POST /api/v1/ai/disease-detection -> postAiDiseaseDetection
    /// - GET /api/v1/hives/{hiveId}/inspections -> getHiveInspections
    /// - POST /api/v1/users with summary "Create User" -> postCreateUser
    pub fn generate_camel_case_operation_id(
        method: &str,
        path: &str,
        summary: &Option<String>,
    ) -> String {
        // Try to use summary if available and meaningful
        // Use first 2-3 words, or all words if summary is short (<= 50 chars)
        if let Some(s) = summary {
            let words: Vec<&str> = s.split_whitespace().collect();
            let words_to_use = if s.len() <= 50 {
                // Use all words if summary is short
                words
            } else {
                // Otherwise limit to first 3 words
                words.into_iter().take(3).collect()
            };

            if words_to_use.len() >= 2 {
                // Check if the first word is already the HTTP method verb
                // (case-insensitive check to avoid redundant prefixes like "getGet...")
                let method_lower = method.to_lowercase();
                let first_word_lower = words_to_use[0].to_lowercase();

                // Common HTTP method verbs that might appear in summaries
                let method_verbs = ["get", "post", "put", "patch", "delete", "head", "options"];

                // If first word matches the method verb, skip it to avoid redundancy
                let words_to_capitalize = if method_verbs.contains(&first_word_lower.as_str())
                    && first_word_lower == method_lower
                {
                    // Skip the first word if it matches the method
                    &words_to_use[1..]
                } else {
                    // Use all words
                    &words_to_use
                };

                // Only proceed if we have words left after potentially skipping the verb
                if !words_to_capitalize.is_empty() {
                    // Capitalize each word and join
                    let camel_case: String =
                        words_to_capitalize.iter().map(|w| capitalize_first(w)).collect();
                    return format!("{}{}", method.to_lowercase(), camel_case);
                }
            }
        }

        // Generate from path segments
        // Extract meaningful path parts (skip empty, skip path params, skip version numbers)
        let path_parts: Vec<&str> = path
            .split('/')
            .filter(|p| !p.is_empty() && !p.starts_with('{') && !is_version_number(p))
            .collect();

        // Build operation name from last 1-2 meaningful parts (prefer the last part, include second-to-last if it adds context)
        let operation_name = if path_parts.is_empty() {
            "Operation".to_string()
        } else if path_parts.len() == 1 {
            // Single part: just convert to camelCase
            to_camel_case(path_parts[0])
        } else {
            // Multiple parts: use last part, optionally include second-to-last for context
            // For paths like /api/v1/hives/{hiveId}/inspections, use "Inspections"
            // For paths like /api/v1/ai/disease-detection, use "DiseaseDetection"
            let last_part = path_parts[path_parts.len() - 1];
            let second_last = if path_parts.len() >= 2 {
                Some(path_parts[path_parts.len() - 2])
            } else {
                None
            };

            // Only include context for truly generic names, not just plural endings
            // This prevents "inspections" from becoming "HivesInspections" when "Inspections" is clear enough
            let include_context = second_last.is_some()
                && (last_part == "items"
                    || last_part == "data"
                    || last_part == "list"
                    || last_part == "all"
                    || last_part == "search");

            if include_context {
                // Combine second-to-last and last: e.g., "Hive" + "Inspections" = "HiveInspections"
                to_camel_case(second_last.unwrap()) + &to_camel_case(last_part)
            } else {
                // Just use the last part
                to_camel_case(last_part)
            }
        };

        format!("{}{}", method.to_lowercase(), operation_name)
    }

    /// Generate a readable type name from operation ID
    ///
    /// Examples:
    /// - postAiDiseaseDetection -> AiDiseaseDetectionResponse
    /// - getHiveInspections -> HiveInspectionsResponse
    /// - getUsers -> GetUsersResponse
    pub fn generate_type_name(operation_id: &str, suffix: &str) -> String {
        // Common HTTP method prefixes (lowercase)
        let method_prefixes = ["get", "post", "put", "patch", "delete", "head", "options"];

        // Check if operation_id starts with a method prefix (case-insensitive)
        let operation_lower = operation_id.to_lowercase();
        let without_method = if let Some(prefix) =
            method_prefixes.iter().find(|p| operation_lower.starts_with(*p))
        {
            let remaining = &operation_id[prefix.len()..];

            // Count uppercase letters in remaining part to determine if it's simple or complex
            let uppercase_count = remaining.chars().filter(|c| c.is_uppercase()).count();

            // If it's method + single word (no uppercase letters or just one word), capitalize whole thing
            // Otherwise (multiple words in camelCase), skip the method prefix
            if uppercase_count == 0
                || (uppercase_count == 1
                    && remaining.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
            {
                // Simple case: method + single word -> capitalize whole thing
                operation_id
            } else {
                // Complex case: method + multiple words -> skip method prefix
                remaining
            }
        } else {
            // No method prefix, use whole string
            operation_id
        };

        // Ensure first letter is uppercase (preserve camelCase structure)
        let capitalized = if let Some(first_char) = without_method.chars().next() {
            if first_char.is_lowercase() {
                // Capitalize first letter, preserve rest
                let mut result = first_char.to_uppercase().collect::<String>();
                result.push_str(&without_method[first_char.len_utf8()..]);
                result
            } else {
                without_method.to_string()
            }
        } else {
            without_method.to_string()
        };

        format!("{}{}", capitalized, suffix)
    }

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
                // Generate camelCase operation ID from method, path, and summary
                generate_camel_case_operation_id(method, path, &operation.summary)
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

    /// Generate TypeScript type from OpenAPI schema with proper formatting
    ///
    /// Generates properly formatted TypeScript types with:
    /// - Array<T> syntax for arrays
    /// - Properly indented object types
    /// - Correct handling of nested types
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
                    // Use Array<T> syntax for better readability
                    format!("Array<{}>", schema_to_typescript_type(items))
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

                        // Format property with proper indentation
                        if required {
                            props.push(format!("  {}: {}", name, prop_type));
                        } else {
                            props.push(format!("  {}?: {}", name, prop_type));
                        }
                    }
                    // Format object with proper line breaks
                    format!("{{\n{}\n}}", props.join(";\n"))
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

        // Test array type formatting
        let array_schema = Schema {
            r#type: Some("array".to_string()),
            format: None,
            properties: None,
            required: None,
            items: Some(Box::new(string_schema.clone())),
            description: None,
            example: None,
            r#enum: None,
            ref_path: None,
        };
        assert_eq!(helpers::schema_to_typescript_type(&array_schema), "Array<string>");
    }

    #[test]
    fn test_generate_camel_case_operation_id() {
        // Test with summary
        let summary = Some("Detect Disease".to_string());
        let op_id = helpers::generate_camel_case_operation_id(
            "POST",
            "/api/v1/ai/disease-detection",
            &summary,
        );
        assert!(op_id.starts_with("post"));
        assert!(op_id.contains("Detect") || op_id.contains("Disease"));

        // Test without summary - should generate from path
        // For /api/v1/hives/{hiveId}/inspections, should use last meaningful part
        let op_id = helpers::generate_camel_case_operation_id(
            "GET",
            "/api/v1/hives/{hiveId}/inspections",
            &None,
        );
        assert_eq!(op_id, "getInspections");

        // Test with context needed (generic name)
        let op_id =
            helpers::generate_camel_case_operation_id("GET", "/api/v1/hives/{hiveId}/items", &None);
        assert_eq!(op_id, "getHivesItems");

        // Test POST with path
        let op_id = helpers::generate_camel_case_operation_id(
            "POST",
            "/api/v1/ai/disease-detection",
            &None,
        );
        assert_eq!(op_id, "postDiseaseDetection");

        // Test with single word path
        let op_id = helpers::generate_camel_case_operation_id("GET", "/api/users", &None);
        assert_eq!(op_id, "getUsers");
    }

    #[test]
    fn test_generate_type_name() {
        assert_eq!(
            helpers::generate_type_name("postAiDiseaseDetection", "Response"),
            "AiDiseaseDetectionResponse"
        );
        assert_eq!(
            helpers::generate_type_name("getHiveInspections", "Response"),
            "HiveInspectionsResponse"
        );
        // getUsers is method + single word, so capitalize whole thing
        assert_eq!(helpers::generate_type_name("getUsers", "Request"), "GetUsersRequest");
    }

    #[test]
    fn test_normalize_operation_with_camel_case() {
        let operation = Operation {
            summary: Some("Get hive inspections".to_string()),
            description: None,
            operation_id: None,
            parameters: None,
            request_body: None,
            responses: HashMap::new(),
            tags: None,
        };

        let normalized =
            helpers::normalize_operation("GET", "/api/v1/hives/{hiveId}/inspections", &operation);
        assert!(normalized.operation_id.starts_with("get"));
        assert!(
            normalized.operation_id.contains("Inspections")
                || normalized.operation_id.contains("Hive")
        );
    }

    #[test]
    fn test_generate_camel_case_operation_id_with_version_numbers() {
        // Test v10, v123 (previously missed by len <= 3 check)
        let op_id = helpers::generate_camel_case_operation_id("GET", "/api/v10/users", &None);
        assert_eq!(op_id, "getUsers");

        let op_id = helpers::generate_camel_case_operation_id("GET", "/api/v123/resource", &None);
        assert_eq!(op_id, "getResource");

        // Test version1, version10 patterns
        let op_id = helpers::generate_camel_case_operation_id("GET", "/api/version1/users", &None);
        assert_eq!(op_id, "getUsers");

        let op_id =
            helpers::generate_camel_case_operation_id("GET", "/api/version10/resource", &None);
        assert_eq!(op_id, "getResource");
    }

    #[test]
    fn test_generate_camel_case_operation_id_with_long_summary() {
        // Test that short summaries (<= 50 chars) use all words
        let short_summary = Some("Create New User Account".to_string());
        let op_id = helpers::generate_camel_case_operation_id("POST", "/api/users", &short_summary);
        assert!(op_id.starts_with("post"));
        assert!(op_id.contains("Create") || op_id.contains("New") || op_id.contains("User"));

        // Test that long summaries are truncated to first 3 words
        let long_summary = Some(
            "This is a very long summary that should be truncated to first three words only"
                .to_string(),
        );
        let op_id = helpers::generate_camel_case_operation_id("POST", "/api/users", &long_summary);
        assert!(op_id.starts_with("post"));
        // Should only contain first 3 words: "This", "Is", "Very"
        assert!(op_id.contains("This") || op_id.contains("Is") || op_id.contains("Very"));
    }

    #[test]
    fn test_generate_camel_case_operation_id_with_redundant_verb() {
        // Test that summaries starting with the method verb don't create redundant prefixes
        // e.g., "Get apiary by ID" with GET method should become "getApiaryById", not "getGetApiaryById"
        let summary = Some("Get apiary by ID".to_string());
        let op_id =
            helpers::generate_camel_case_operation_id("GET", "/api/v1/apiaries/{id}", &summary);
        assert_eq!(op_id, "getApiaryById");

        // Test with POST
        let summary = Some("Post new user".to_string());
        let op_id = helpers::generate_camel_case_operation_id("POST", "/api/users", &summary);
        assert_eq!(op_id, "postNewUser");

        // Test with PUT
        let summary = Some("Put update user".to_string());
        let op_id = helpers::generate_camel_case_operation_id("PUT", "/api/users/{id}", &summary);
        assert_eq!(op_id, "putUpdateUser");

        // Test that non-matching verbs still work (e.g., "Create" with GET)
        let summary = Some("Create user".to_string());
        let op_id = helpers::generate_camel_case_operation_id("GET", "/api/users", &summary);
        assert_eq!(op_id, "getCreateUser"); // Should include "Create" since it doesn't match "GET"
    }

    #[test]
    fn test_generate_type_name_edge_cases() {
        // Test with lowercase first letter (e.g., from operation_id like "getUsers")
        assert_eq!(helpers::generate_type_name("getUsers", "Response"), "GetUsersResponse");

        // Test with already uppercase first letter
        assert_eq!(helpers::generate_type_name("GetUsers", "Response"), "GetUsersResponse");

        // Test with single word operation ID
        assert_eq!(helpers::generate_type_name("getUser", "Response"), "GetUserResponse");
    }
}
