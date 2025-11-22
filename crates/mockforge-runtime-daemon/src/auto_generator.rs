//! Auto-generator for creating mocks from 404 responses
//!
//! This module handles the automatic generation of mocks, types, client stubs,
//! OpenAPI schema entries, and scenarios when a 404 is detected.

use anyhow::{Context, Result};
use serde_json::json;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::config::RuntimeDaemonConfig;

/// Auto-generator for creating mocks from 404s
pub struct AutoGenerator {
    /// Configuration
    config: RuntimeDaemonConfig,
    /// Base URL for the MockForge management API
    management_api_url: String,
}

impl AutoGenerator {
    /// Create a new auto-generator
    pub fn new(config: RuntimeDaemonConfig, management_api_url: String) -> Self {
        Self {
            config,
            management_api_url,
        }
    }

    /// Generate a mock from a 404 response
    ///
    /// This is the main entry point that orchestrates all the generation steps:
    /// 1. Create mock endpoint with intelligent response
    /// 2. Generate type (TypeScript/JSON schema)
    /// 3. Generate client stub code
    /// 4. Add to OpenAPI schema
    /// 5. Add example response
    /// 6. Set up basic scenario
    pub async fn generate_mock_from_404(&self, method: &str, path: &str) -> Result<()> {
        info!("Generating mock for {} {}", method, path);

        // Step 1: Create basic mock endpoint with intelligent response
        let mock_id = self.create_mock_endpoint(method, path).await?;
        debug!("Created mock endpoint with ID: {}", mock_id);

        // Step 2: Generate type (if enabled)
        if self.config.generate_types {
            if let Err(e) = self.generate_type(method, path).await {
                warn!("Failed to generate type: {}", e);
            }
        }

        // Step 3: Generate client stub (if enabled)
        if self.config.generate_client_stubs {
            if let Err(e) = self.generate_client_stub(method, path).await {
                warn!("Failed to generate client stub: {}", e);
            }
        }

        // Step 4: Update OpenAPI schema (if enabled)
        if self.config.update_openapi {
            if let Err(e) = self.update_openapi_schema(method, path).await {
                warn!("Failed to update OpenAPI schema: {}", e);
            }
        }

        // Step 5: Create scenario (if enabled)
        if self.config.create_scenario {
            if let Err(e) = self.create_scenario(method, path, &mock_id).await {
                warn!("Failed to create scenario: {}", e);
            }
        }

        info!("Completed mock generation for {} {}", method, path);
        Ok(())
    }

    /// Create a mock endpoint with a basic intelligent response
    async fn create_mock_endpoint(&self, method: &str, path: &str) -> Result<String> {
        // Generate a basic response based on the path
        let response_body = self.generate_intelligent_response(method, path).await?;

        // Create mock configuration
        let mock_config = json!({
            "method": method,
            "path": path,
            "status_code": 200,
            "body": response_body,
            "name": format!("Auto-generated: {} {}", method, path),
            "enabled": true,
        });

        // Call the management API to create the mock
        let client = reqwest::Client::new();
        let url = format!("{}/__mockforge/api/mocks", self.management_api_url);
        
        let response = client
            .post(&url)
            .json(&mock_config)
            .send()
            .await
            .context("Failed to send request to management API")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Management API returned {}: {}", status, text);
        }

        let created_mock: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse response from management API")?;

        let mock_id = created_mock
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Response missing 'id' field"))?
            .to_string();

        Ok(mock_id)
    }

    /// Generate an intelligent response based on the method and path
    async fn generate_intelligent_response(&self, method: &str, path: &str) -> Result<serde_json::Value> {
        // Use AI generation if enabled
        #[cfg(feature = "ai")]
        if self.config.ai_generation {
            return self.generate_ai_response(method, path).await;
        }
        
        // Fallback to pattern-based generation

        // Infer entity type from path
        let entity_type = self.infer_entity_type(path);
        
        // Generate a basic response structure
        let response = match method.to_uppercase().as_str() {
            "GET" => {
                // For GET, return a single object or array based on path
                if path.ends_with('/') || !path.split('/').last().unwrap_or("").parse::<u64>().is_ok() {
                    // Looks like a collection endpoint
                    json!([{
                        "id": "{{uuid}}",
                        "name": format!("Sample {}", entity_type),
                        "created_at": "{{now}}",
                    }])
                } else {
                    // Looks like a single resource endpoint
                    json!({
                        "id": path.split('/').last().unwrap_or("123"),
                        "name": format!("Sample {}", entity_type),
                        "created_at": "{{now}}",
                    })
                }
            }
            "POST" => {
                // For POST, return the created resource
                json!({
                    "id": "{{uuid}}",
                    "name": format!("New {}", entity_type),
                    "created_at": "{{now}}",
                    "status": "created",
                })
            }
            "PUT" | "PATCH" => {
                // For PUT/PATCH, return the updated resource
                json!({
                    "id": path.split('/').last().unwrap_or("123"),
                    "name": format!("Updated {}", entity_type),
                    "updated_at": "{{now}}",
                })
            }
            "DELETE" => {
                // For DELETE, return success status
                json!({
                    "success": true,
                    "message": "Resource deleted",
                })
            }
            _ => {
                // Default response
                json!({
                    "message": "Auto-generated response",
                    "method": method,
                    "path": path,
                })
            }
        };

        Ok(response)
    }

    /// Generate AI-powered response using IntelligentMockGenerator
    #[cfg(feature = "ai")]
    async fn generate_ai_response(&self, method: &str, path: &str) -> Result<serde_json::Value> {
        use mockforge_data::{IntelligentMockConfig, IntelligentMockGenerator, ResponseMode};
        use std::collections::HashMap;

        // Infer entity type and build a prompt
        let entity_type = self.infer_entity_type(path);
        let prompt = format!(
            "Generate a realistic {} API response for {} {} endpoint. \
            The response should be appropriate for a {} operation and include realistic data \
            for a {} entity.",
            entity_type, method, path, method, entity_type
        );

        // Build schema based on inferred entity type
        let schema = self.build_schema_for_entity(&entity_type, method);

        // Create intelligent mock config
        let mut ai_config = IntelligentMockConfig::new(ResponseMode::Intelligent)
            .with_prompt(prompt)
            .with_schema(schema)
            .with_count(1);

        // Try to load RAG config from environment
        if let Ok(rag_config) = self.load_rag_config_from_env() {
            ai_config = ai_config.with_rag_config(rag_config);
        }

        // Create generator and generate response
        let mut generator = IntelligentMockGenerator::new(ai_config)
            .context("Failed to create intelligent mock generator")?;

        let response = generator.generate().await
            .context("Failed to generate AI response")?;

        info!("Generated AI-powered response for {} {}", method, path);
        Ok(response)
    }

    /// Load RAG configuration from environment variables
    #[cfg(feature = "ai")]
    fn load_rag_config_from_env(&self) -> Result<mockforge_data::RagConfig> {
        use mockforge_data::{EmbeddingProvider, LlmProvider, RagConfig};

        // Try to determine provider from environment
        let provider = std::env::var("MOCKFORGE_RAG_PROVIDER")
            .unwrap_or_else(|_| "openai".to_string())
            .to_lowercase();

        let provider = match provider.as_str() {
            "openai" => LlmProvider::OpenAI,
            "anthropic" => LlmProvider::Anthropic,
            "ollama" => LlmProvider::Ollama,
            _ => LlmProvider::OpenAI,
        };

        let model = std::env::var("MOCKFORGE_RAG_MODEL")
            .unwrap_or_else(|_| "gpt-3.5-turbo".to_string());

        let api_key = std::env::var("MOCKFORGE_RAG_API_KEY").ok();

        let api_endpoint = std::env::var("MOCKFORGE_RAG_API_ENDPOINT")
            .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string());

        let mut config = RagConfig::default();
        config.provider = provider;
        config.model = model;
        config.api_key = api_key;
        config.api_endpoint = api_endpoint;

        // Set embedding provider to match LLM provider
        config.embedding_provider = match config.provider {
            LlmProvider::OpenAI => EmbeddingProvider::OpenAI,
            LlmProvider::Anthropic => EmbeddingProvider::OpenAI, // Anthropic doesn't have embeddings, use OpenAI
            LlmProvider::Ollama => EmbeddingProvider::Ollama,
        };

        Ok(config)
    }

    /// Build a basic JSON schema for an entity type
    fn build_schema_for_entity(&self, entity_type: &str, method: &str) -> serde_json::Value {
        let base_schema = json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "format": "uuid"
                },
                "name": {
                    "type": "string"
                },
                "created_at": {
                    "type": "string",
                    "format": "date-time"
                }
            },
            "required": ["id", "name"]
        });

        // Adjust schema based on HTTP method
        match method.to_uppercase().as_str() {
            "GET" => {
                // For GET, might be array or single object
                if entity_type.ends_with('s') {
                    json!({
                        "type": "array",
                        "items": base_schema
                    })
                } else {
                    base_schema
                }
            }
            "POST" => {
                // For POST, add status field
                let mut schema = base_schema.clone();
                if let Some(props) = schema.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    props.insert("status".to_string(), json!({
                        "type": "string",
                        "enum": ["created", "pending", "active"]
                    }));
                }
                schema
            }
            "PUT" | "PATCH" => {
                // For PUT/PATCH, add updated_at
                let mut schema = base_schema.clone();
                if let Some(props) = schema.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    props.insert("updated_at".to_string(), json!({
                        "type": "string",
                        "format": "date-time"
                    }));
                }
                schema
            }
            _ => base_schema,
        }
    }

    /// Infer entity type from path
    fn infer_entity_type(&self, path: &str) -> String {
        // Extract entity type from path (e.g., /api/users -> "user")
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        if let Some(last_part) = parts.last() {
            // Remove common prefixes and pluralization
            let entity = last_part
                .trim_end_matches('s') // Remove plural 's'
                .to_lowercase();
            
            if !entity.is_empty() {
                return entity;
            }
        }
        
        "resource".to_string()
    }

    /// Generate a type (TypeScript/JSON schema) for the endpoint
    async fn generate_type(&self, method: &str, path: &str) -> Result<()> {
        use std::path::PathBuf;

        // Determine output directory (use workspace_dir if configured, otherwise current dir)
        let output_dir = if let Some(ref workspace_dir) = self.config.workspace_dir {
            PathBuf::from(workspace_dir)
        } else {
            PathBuf::from(".")
        };

        // Create types directory if it doesn't exist
        let types_dir = output_dir.join("types");
        if !types_dir.exists() {
            std::fs::create_dir_all(&types_dir)
                .context("Failed to create types directory")?;
        }

        // Generate TypeScript type from the response schema
        let entity_type = self.infer_entity_type(path);
        let type_name = self.sanitize_type_name(&entity_type);
        
        // Get the response schema we built earlier
        let schema = self.build_schema_for_entity(&entity_type, method);
        
        // Generate TypeScript interface
        let ts_type = self.generate_typescript_interface(&type_name, &schema, method)?;
        
        // Write TypeScript type file
        let ts_file = types_dir.join(format!("{}.ts", type_name.to_lowercase()));
        std::fs::write(&ts_file, ts_type)
            .context("Failed to write TypeScript type file")?;

        // Also generate JSON schema
        let json_schema = self.generate_json_schema(&type_name, &schema)?;
        let json_file = types_dir.join(format!("{}.schema.json", type_name.to_lowercase()));
        std::fs::write(&json_file, serde_json::to_string_pretty(&json_schema)?)
            .context("Failed to write JSON schema file")?;

        info!("Generated types for {} {}: {} and {}.schema.json", 
              method, path, ts_file.display(), json_file.display());
        
        Ok(())
    }

    /// Generate TypeScript interface from schema
    fn generate_typescript_interface(
        &self,
        type_name: &str,
        schema: &serde_json::Value,
        method: &str,
    ) -> Result<String> {
        let mut code = String::new();
        code.push_str(&format!("// Generated TypeScript type for {} {}\n", method, type_name));
        code.push_str("// Auto-generated by MockForge Runtime Daemon\n\n");

        // Determine if it's an array or object
        let schema_type = schema.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("object");

        if schema_type == "array" {
            // Generate array type
            if let Some(items) = schema.get("items") {
                let item_type_name = format!("{}Item", type_name);
                code.push_str(&self.generate_typescript_interface(&item_type_name, items, method)?);
                code.push_str(&format!("export type {} = {}[];\n", type_name, item_type_name));
            } else {
                code.push_str(&format!("export type {} = any[];\n", type_name));
            }
        } else {
            // Generate interface
            code.push_str(&format!("export interface {} {{\n", type_name));
            
            if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
                let required = schema.get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                    .unwrap_or_default();

                for (prop_name, prop_schema) in properties {
                    let prop_type = self.schema_value_to_typescript_type(prop_schema)?;
                    let is_optional = !required.contains(&prop_name.as_str());
                    let optional_marker = if is_optional { "?" } else { "" };
                    
                    code.push_str(&format!("  {}{}: {};\n", prop_name, optional_marker, prop_type));
                }
            }
            
            code.push_str("}\n");
        }

        Ok(code)
    }

    /// Convert a JSON schema value to TypeScript type string
    fn schema_value_to_typescript_type(&self, schema: &serde_json::Value) -> Result<String> {
        let schema_type = schema.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("any");

        match schema_type {
            "string" => {
                // Check format
                if let Some(format) = schema.get("format").and_then(|f| f.as_str()) {
                    match format {
                        "date-time" | "date" => Ok("string".to_string()),
                        "uuid" => Ok("string".to_string()),
                        _ => Ok("string".to_string()),
                    }
                } else {
                    Ok("string".to_string())
                }
            }
            "integer" | "number" => Ok("number".to_string()),
            "boolean" => Ok("boolean".to_string()),
            "array" => {
                if let Some(items) = schema.get("items") {
                    let item_type = self.schema_value_to_typescript_type(items)?;
                    Ok(format!("{}[]", item_type))
                } else {
                    Ok("any[]".to_string())
                }
            }
            "object" => {
                if schema.get("properties").is_some() {
                    // Inline object type
                    Ok("Record<string, any>".to_string())
                } else {
                    Ok("Record<string, any>".to_string())
                }
            }
            _ => Ok("any".to_string()),
        }
    }

    /// Generate JSON schema from the type
    fn generate_json_schema(
        &self,
        type_name: &str,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let mut json_schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": type_name,
            "type": schema.get("type").unwrap_or(&json!("object")),
        });

        if let Some(properties) = schema.get("properties") {
            json_schema["properties"] = properties.clone();
        }

        if let Some(required) = schema.get("required") {
            json_schema["required"] = required.clone();
        }

        Ok(json_schema)
    }

    /// Sanitize a name to be a valid TypeScript type name
    fn sanitize_type_name(&self, name: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = true;

        for ch in name.chars() {
            match ch {
                '-' | '_' | ' ' => capitalize_next = true,
                ch if ch.is_alphanumeric() => {
                    if capitalize_next {
                        result.push(ch.to_uppercase().next().unwrap_or(ch));
                        capitalize_next = false;
                    } else {
                        result.push(ch);
                    }
                }
                _ => {}
            }
        }

        if result.is_empty() {
            "Resource".to_string()
        } else {
            // Ensure first character is uppercase
            let mut chars = result.chars();
            if let Some(first) = chars.next() {
                format!("{}{}", first.to_uppercase(), chars.as_str())
            } else {
                "Resource".to_string()
            }
        }
    }

    /// Generate a client stub for the endpoint
    async fn generate_client_stub(&self, method: &str, path: &str) -> Result<()> {
        use std::path::PathBuf;
        use tokio::fs;

        // Determine output directory
        let output_dir = if let Some(ref workspace_dir) = self.config.workspace_dir {
            PathBuf::from(workspace_dir)
        } else {
            PathBuf::from(".")
        };

        // Create client-stubs directory if it doesn't exist
        let stubs_dir = output_dir.join("client-stubs");
        if !stubs_dir.exists() {
            fs::create_dir_all(&stubs_dir).await
                .context("Failed to create client-stubs directory")?;
        }

        // Generate client stub code
        let entity_type = self.infer_entity_type(path);
        let function_name = self.generate_function_name(method, path);
        let stub_code = self.generate_client_stub_code(method, path, &function_name, &entity_type)?;

        // Write TypeScript client stub file
        let stub_file = stubs_dir.join(format!("{}.ts", function_name.to_lowercase()));
        fs::write(&stub_file, stub_code).await
            .context("Failed to write client stub file")?;

        info!("Generated client stub for {} {}: {}", method, path, stub_file.display());
        Ok(())
    }

    /// Generate function name from method and path
    fn generate_function_name(&self, method: &str, path: &str) -> String {
        let entity_type = self.infer_entity_type(path);
        let method_prefix = match method.to_uppercase().as_str() {
            "GET" => "get",
            "POST" => "create",
            "PUT" => "update",
            "PATCH" => "patch",
            "DELETE" => "delete",
            _ => "call",
        };

        // Check if path has an ID parameter (single resource)
        let has_id = path.split('/').any(|segment| {
            segment.starts_with('{') && segment.ends_with('}')
        });

        if has_id && method.to_uppercase() == "GET" {
            format!("{}{}", method_prefix, self.sanitize_type_name(&entity_type))
        } else if method.to_uppercase() == "GET" {
            format!("list{}s", self.sanitize_type_name(&entity_type))
        } else {
            format!("{}{}", method_prefix, self.sanitize_type_name(&entity_type))
        }
    }

    /// Generate client stub TypeScript code
    fn generate_client_stub_code(
        &self,
        method: &str,
        path: &str,
        function_name: &str,
        entity_type: &str,
    ) -> Result<String> {
        let method_upper = method.to_uppercase();
        let type_name = self.sanitize_type_name(entity_type);
        
        // Extract path parameters
        let path_params: Vec<String> = path
            .split('/')
            .filter_map(|segment| {
                if segment.starts_with('{') && segment.ends_with('}') {
                    Some(segment.trim_matches(|c| c == '{' || c == '}').to_string())
                } else {
                    None
                }
            })
            .collect();

        // Build function parameters
        let mut params = String::new();
        if !path_params.is_empty() {
            for param in &path_params {
                params.push_str(&format!("{}: string", param));
                params.push_str(", ");
            }
        }
        
        // Add request body parameter for POST/PUT/PATCH
        if matches!(method_upper.as_str(), "POST" | "PUT" | "PATCH") {
            params.push_str(&format!("data?: Partial<{}>", type_name));
        }

        // Add query parameters for GET
        if method_upper == "GET" {
            if !params.is_empty() {
                params.push_str(", ");
            }
            params.push_str("queryParams?: Record<string, any>");
        }

        // Build endpoint path with template literals
        let mut endpoint_path = path.to_string();
        for param in &path_params {
            endpoint_path = endpoint_path.replace(
                &format!("{{{}}}", param),
                &format!("${{{}}}", param)
            );
        }

        // Generate the stub code
        let stub = format!(
            r#"// Auto-generated client stub for {} {}
// Generated by MockForge Runtime Daemon

import type {{ {} }} from '../types/{}';

/**
 * {} {} endpoint
 * 
 * @param {} - Request parameters
 * @returns Promise resolving to {} response
 */
export async function {}({}): Promise<{}> {{
  const endpoint = `{}`;
  const url = `${{baseUrl}}${{endpoint}}`;
  
  const response = await fetch(url, {{
    method: '{}',
    headers: {{
      'Content-Type': 'application/json',
      ...(headers || {{}}),
    }},
    {}{}
  }});
  
  if (!response.ok) {{
    throw new Error(`Request failed: ${{response.status}} ${{response.statusText}}`);
  }}
  
  return response.json();
}}

/**
 * Base URL configuration
 * Override this to point to your API server
 */
export let baseUrl = 'http://localhost:3000';
"#,
            method, path,
            type_name, entity_type.to_lowercase(),
            method, path,
            if params.is_empty() { "headers?: Record<string, string>" } else { &params },
            type_name,
            function_name,
            if params.is_empty() { "headers?: Record<string, string>" } else { &params },
            type_name,
            endpoint_path,
            method_upper,
            if matches!(method_upper.as_str(), "POST" | "PUT" | "PATCH") {
                "body: JSON.stringify(data || {}),\n    ".to_string()
            } else if method_upper == "GET" && !path_params.is_empty() {
                format!("{}const queryString = queryParams ? '?' + new URLSearchParams(queryParams).toString() : '';\n  const urlWithQuery = url + queryString;\n  ", 
                    if !path_params.is_empty() { "" } else { "" })
            } else {
                String::new()
            },
            if method_upper == "GET" && !path_params.is_empty() {
                "url: urlWithQuery,\n    ".to_string()
            } else {
                String::new()
            }
        );

        Ok(stub)
    }

    /// Update the OpenAPI schema with the new endpoint
    async fn update_openapi_schema(&self, method: &str, path: &str) -> Result<()> {
        use mockforge_core::openapi::OpenApiSpec;
        use std::path::PathBuf;

        // Determine OpenAPI spec file path
        let spec_path = self.find_or_create_openapi_spec_path().await?;
        
        // Load existing spec or create new one
        let mut spec = if spec_path.exists() {
            OpenApiSpec::from_file(&spec_path).await
                .context("Failed to load existing OpenAPI spec")?
        } else {
            // Create a new OpenAPI spec
            self.create_new_openapi_spec().await?
        };

        // Add the new endpoint to the spec
        self.add_endpoint_to_spec(&mut spec, method, path).await?;

        // Save the updated spec
        self.save_openapi_spec(&spec, &spec_path).await?;

        info!("Updated OpenAPI schema at {} with {} {}", spec_path.display(), method, path);
        Ok(())
    }

    /// Find existing OpenAPI spec file or determine where to create one
    async fn find_or_create_openapi_spec_path(&self) -> Result<PathBuf> {
        use std::path::PathBuf;

        // Check common locations
        let possible_paths = vec![
            PathBuf::from("openapi.yaml"),
            PathBuf::from("openapi.yml"),
            PathBuf::from("openapi.json"),
            PathBuf::from("api.yaml"),
            PathBuf::from("api.yml"),
            PathBuf::from("api.json"),
        ];

        // Also check in workspace directory if configured
        let mut all_paths = possible_paths.clone();
        if let Some(ref workspace_dir) = self.config.workspace_dir {
            for path in possible_paths {
                all_paths.push(PathBuf::from(workspace_dir).join(path));
            }
        }

        // Find first existing spec file
        for path in &all_paths {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        // If none found, use default location (workspace_dir or current dir)
        let default_path = if let Some(ref workspace_dir) = self.config.workspace_dir {
            PathBuf::from(workspace_dir).join("openapi.yaml")
        } else {
            PathBuf::from("openapi.yaml")
        };

        Ok(default_path)
    }

    /// Create a new OpenAPI spec
    async fn create_new_openapi_spec(&self) -> Result<mockforge_core::openapi::OpenApiSpec> {
        use mockforge_core::openapi::OpenApiSpec;
        use serde_json::json;

        let spec_json = json!({
            "openapi": "3.0.3",
            "info": {
                "title": "Auto-generated API",
                "version": "1.0.0",
                "description": "API specification auto-generated by MockForge Runtime Daemon"
            },
            "paths": {},
            "components": {
                "schemas": {}
            }
        });

        OpenApiSpec::from_json(spec_json)
            .context("Failed to create new OpenAPI spec")
    }

    /// Add an endpoint to the OpenAPI spec
    async fn add_endpoint_to_spec(
        &self,
        spec: &mut mockforge_core::openapi::OpenApiSpec,
        method: &str,
        path: &str,
    ) -> Result<()> {
        // Get the raw document to modify
        let mut spec_json = spec.raw_document.clone()
            .ok_or_else(|| anyhow::anyhow!("OpenAPI spec missing raw document"))?;

        // Ensure paths object exists
        if !spec_json.get("paths").is_some() {
            spec_json["paths"] = json!({});
        }

        let paths = spec_json.get_mut("paths")
            .and_then(|p| p.as_object_mut())
            .ok_or_else(|| anyhow::anyhow!("Failed to get paths object"))?;

        // Get or create path item
        let path_entry = paths.entry(path.to_string())
            .or_insert_with(|| json!({}));

        // Convert method to lowercase for OpenAPI
        let method_lower = method.to_lowercase();

        // Create operation
        let operation = json!({
            "summary": format!("Auto-generated {} endpoint", method),
            "description": format!("Endpoint auto-generated by MockForge Runtime Daemon for {} {}", method, path),
            "operationId": self.generate_operation_id(method, path),
            "responses": {
                "200": {
                    "description": "Successful response",
                    "content": {
                        "application/json": {
                            "schema": self.build_schema_for_entity(&self.infer_entity_type(path), method)
                        }
                    }
                }
            }
        });

        // Add the operation to the path
        path_entry[method_lower] = operation;

        // Reload the spec from the updated JSON
        *spec = mockforge_core::openapi::OpenApiSpec::from_json(spec_json)
            .context("Failed to reload OpenAPI spec after update")?;

        Ok(())
    }

    /// Generate an operation ID from method and path
    fn generate_operation_id(&self, method: &str, path: &str) -> String {
        let entity_type = self.infer_entity_type(path);
        let method_lower = method.to_lowercase();
        
        // Convert path segments to camelCase
        let path_parts: Vec<&str> = path.split('/')
            .filter(|s| !s.is_empty() && !s.starts_with('{'))
            .collect();
        
        if path_parts.is_empty() {
            format!("{}_{}", method_lower, entity_type)
        } else {
            let mut op_id = String::new();
            op_id.push_str(&method_lower);
            for part in path_parts {
                let mut chars = part.chars();
                if let Some(first) = chars.next() {
                    op_id.push(first.to_uppercase().next().unwrap_or(first));
                    op_id.push_str(&chars.as_str());
                }
            }
            op_id
        }
    }

    /// Save OpenAPI spec to file
    async fn save_openapi_spec(
        &self,
        spec: &mockforge_core::openapi::OpenApiSpec,
        path: &PathBuf,
    ) -> Result<()> {
        use tokio::fs;

        let spec_json = spec.raw_document.clone()
            .ok_or_else(|| anyhow::anyhow!("OpenAPI spec missing raw document"))?;

        // Determine format based on file extension
        let is_yaml = path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s == "yaml" || s == "yml")
            .unwrap_or(false);

        let content = if is_yaml {
            serde_yaml::to_string(&spec_json)
                .context("Failed to serialize OpenAPI spec to YAML")?
        } else {
            serde_json::to_string_pretty(&spec_json)
                .context("Failed to serialize OpenAPI spec to JSON")?
        };

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await
                .context("Failed to create OpenAPI spec directory")?;
        }

        fs::write(path, content).await
            .context("Failed to write OpenAPI spec file")?;

        Ok(())
    }

    /// Create a basic scenario for the endpoint
    async fn create_scenario(&self, method: &str, path: &str, mock_id: &str) -> Result<()> {
        use std::path::PathBuf;
        use tokio::fs;
        use chrono::Utc;

        // Determine output directory
        let output_dir = if let Some(ref workspace_dir) = self.config.workspace_dir {
            PathBuf::from(workspace_dir)
        } else {
            PathBuf::from(".")
        };

        // Create scenarios directory if it doesn't exist
        let scenarios_dir = output_dir.join("scenarios");
        if !scenarios_dir.exists() {
            fs::create_dir_all(&scenarios_dir).await
                .context("Failed to create scenarios directory")?;
        }

        // Generate scenario name from endpoint
        let entity_type = self.infer_entity_type(path);
        let scenario_name = format!("auto-{}-{}", entity_type, method.to_lowercase());
        let scenario_dir = scenarios_dir.join(&scenario_name);

        // Create scenario directory
        if !scenario_dir.exists() {
            fs::create_dir_all(&scenario_dir).await
                .context("Failed to create scenario directory")?;
        }

        // Generate scenario manifest
        let manifest = self.generate_scenario_manifest(&scenario_name, method, path, mock_id)?;

        // Write scenario.yaml
        let manifest_path = scenario_dir.join("scenario.yaml");
        let manifest_yaml = serde_yaml::to_string(&manifest)
            .context("Failed to serialize scenario manifest")?;
        fs::write(&manifest_path, manifest_yaml).await
            .context("Failed to write scenario manifest")?;

        // Create a basic config.yaml for the scenario
        let config = self.generate_scenario_config(method, path, mock_id)?;
        let config_path = scenario_dir.join("config.yaml");
        let config_yaml = serde_yaml::to_string(&config)
            .context("Failed to serialize scenario config")?;
        fs::write(&config_path, config_yaml).await
            .context("Failed to write scenario config")?;

        info!("Created scenario '{}' at {}", scenario_name, scenario_dir.display());
        Ok(())
    }

    /// Generate scenario manifest YAML structure
    fn generate_scenario_manifest(
        &self,
        scenario_name: &str,
        method: &str,
        path: &str,
        _mock_id: &str,
    ) -> Result<serde_json::Value> {
        use chrono::Utc;

        let entity_type = self.infer_entity_type(path);
        let title = format!("Auto-generated {} {} Scenario", method, entity_type);

        let manifest = json!({
            "manifest_version": "1.0",
            "name": scenario_name,
            "version": "1.0.0",
            "title": title,
            "description": format!(
                "Auto-generated scenario for {} {} endpoint. Created by MockForge Runtime Daemon.",
                method, path
            ),
            "author": "MockForge Runtime Daemon",
            "author_email": None::<String>,
            "category": "other",
            "tags": ["auto-generated", "runtime-daemon", entity_type],
            "compatibility": {
                "min_version": "0.3.0",
                "max_version": null,
                "required_features": [],
                "protocols": ["http"]
            },
            "files": [
                "scenario.yaml",
                "config.yaml"
            ],
            "readme": None::<String>,
            "example_usage": format!(
                "# Use this scenario\nmockforge scenario use {}\n\n# Start server\nmockforge serve --config config.yaml",
                scenario_name
            ),
            "required_features": [],
            "plugin_dependencies": [],
            "metadata": {
                "auto_generated": true,
                "endpoint": path,
                "method": method,
                "entity_type": entity_type
            },
            "created_at": Utc::now().to_rfc3339(),
            "updated_at": Utc::now().to_rfc3339()
        });

        Ok(manifest)
    }

    /// Generate scenario config YAML structure
    fn generate_scenario_config(
        &self,
        method: &str,
        path: &str,
        mock_id: &str,
    ) -> Result<serde_json::Value> {
        let entity_type = self.infer_entity_type(path);
        let response_body = serde_json::to_value(self.build_schema_for_entity(&entity_type, method))?;

        let config = json!({
            "http": {
                "enabled": true,
                "port": 3000,
                "mocks": [
                    {
                        "id": mock_id,
                        "method": method,
                        "path": path,
                        "status_code": 200,
                        "body": response_body,
                        "name": format!("Auto-generated: {} {}", method, path),
                        "enabled": true
                    }
                ]
            }
        });

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_entity_type() {
        let config = RuntimeDaemonConfig::default();
        let generator = AutoGenerator::new(config, "http://localhost:3000".to_string());

        assert_eq!(generator.infer_entity_type("/api/users"), "user");
        assert_eq!(generator.infer_entity_type("/api/products"), "product");
        assert_eq!(generator.infer_entity_type("/api/orders/123"), "order");
        assert_eq!(generator.infer_entity_type("/api"), "resource");
    }
}

