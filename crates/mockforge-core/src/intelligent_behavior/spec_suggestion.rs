//! AI-powered specification suggestion and generation
//!
//! This module provides intelligent API specification extrapolation using LLMs.
//! Given minimal input (e.g., a single endpoint example or API description),
//! it can generate complete OpenAPI specifications or MockForge configurations.

use super::config::BehaviorModelConfig;
use super::llm_client::LlmClient;
use super::types::LlmGenerationRequest;
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Input type for spec suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SuggestionInput {
    /// Single endpoint example with request/response
    Endpoint {
        /// HTTP method
        method: String,
        /// Path
        path: String,
        /// Request example
        request: Option<Value>,
        /// Response example
        response: Option<Value>,
        /// Optional description
        description: Option<String>,
    },
    /// Text description of the API
    Description {
        /// API description text
        text: String,
    },
    /// Partial OpenAPI specification
    PartialSpec {
        /// Partial OpenAPI spec
        spec: Value,
    },
    /// List of endpoint paths only
    Paths {
        /// List of paths
        paths: Vec<String>,
    },
}

/// Output format for generated specs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// OpenAPI 3.0 specification
    OpenAPI,
    /// MockForge YAML configuration
    MockForge,
    /// Both formats
    Both,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openapi" => Ok(Self::OpenAPI),
            "mockforge" => Ok(Self::MockForge),
            "both" => Ok(Self::Both),
            _ => Err(format!("Invalid output format: {}", s)),
        }
    }
}

/// Configuration for spec suggestion
#[derive(Debug, Clone)]
pub struct SuggestionConfig {
    /// LLM configuration
    pub llm_config: BehaviorModelConfig,
    /// Output format
    pub output_format: OutputFormat,
    /// Number of additional endpoints to suggest
    pub num_suggestions: usize,
    /// Whether to include examples in generated specs
    pub include_examples: bool,
    /// API domain/category hint
    pub domain_hint: Option<String>,
}

impl Default for SuggestionConfig {
    fn default() -> Self {
        Self {
            llm_config: BehaviorModelConfig::default(),
            output_format: OutputFormat::OpenAPI,
            num_suggestions: 5,
            include_examples: true,
            domain_hint: None,
        }
    }
}

/// Result from spec suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionResult {
    /// Generated OpenAPI spec (if requested)
    pub openapi_spec: Option<Value>,
    /// Generated MockForge config (if requested)
    pub mockforge_config: Option<Value>,
    /// Suggestions and reasoning
    pub suggestions: Vec<EndpointSuggestion>,
    /// Metadata about the generation
    pub metadata: SuggestionMetadata,
}

/// Individual endpoint suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointSuggestion {
    /// HTTP method
    pub method: String,
    /// Path
    pub path: String,
    /// Description
    pub description: String,
    /// Suggested parameters
    pub parameters: Vec<ParameterInfo>,
    /// Suggested response schema
    pub response_schema: Option<Value>,
    /// Reasoning for this suggestion
    pub reasoning: String,
}

/// Parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    /// Parameter name
    pub name: String,
    /// Parameter location (path, query, header, body)
    pub location: String,
    /// Data type
    pub data_type: String,
    /// Whether required
    pub required: bool,
    /// Description
    pub description: Option<String>,
}

/// Metadata about the suggestion generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionMetadata {
    /// Number of endpoints generated
    pub endpoint_count: usize,
    /// Detected API domain/category
    pub detected_domain: Option<String>,
    /// Generation timestamp
    pub timestamp: String,
    /// Model used
    pub model: String,
}

/// Engine for AI-powered spec suggestion
pub struct SpecSuggestionEngine {
    /// LLM client
    llm_client: LlmClient,
    /// Configuration
    config: SuggestionConfig,
}

impl SpecSuggestionEngine {
    /// Create a new spec suggestion engine
    pub fn new(config: SuggestionConfig) -> Self {
        let llm_client = LlmClient::new(config.llm_config.clone());
        Self { llm_client, config }
    }

    /// Generate spec suggestions from input
    pub async fn suggest(&self, input: &SuggestionInput) -> Result<SuggestionResult> {
        // Build prompt based on input type
        let (system_prompt, user_prompt) = self.build_prompts(input)?;

        // Generate using LLM
        let request = LlmGenerationRequest {
            system_prompt,
            user_prompt,
            temperature: 0.7,
            max_tokens: 4000,
            schema: None,
        };

        let llm_response = self.llm_client.generate(&request).await?;

        // Parse and structure the response
        self.parse_llm_response(llm_response, input).await
    }

    /// Build prompts based on input type
    fn build_prompts(&self, input: &SuggestionInput) -> Result<(String, String)> {
        let system_prompt = self.build_system_prompt();
        let user_prompt = match input {
            SuggestionInput::Endpoint {
                method,
                path,
                request,
                response,
                description,
            } => self.build_endpoint_prompt(method, path, request, response, description),
            SuggestionInput::Description { text } => self.build_description_prompt(text),
            SuggestionInput::PartialSpec { spec } => self.build_partial_spec_prompt(spec),
            SuggestionInput::Paths { paths } => self.build_paths_prompt(paths),
        };

        Ok((system_prompt, user_prompt))
    }

    /// Build system prompt for spec generation
    fn build_system_prompt(&self) -> String {
        let format_desc = match self.config.output_format {
            OutputFormat::OpenAPI => "OpenAPI 3.0 specification",
            OutputFormat::MockForge => "MockForge YAML configuration",
            OutputFormat::Both => "both OpenAPI 3.0 specification and MockForge YAML configuration",
        };

        format!(
            r#"You are an expert API architect and specification designer. Your role is to analyze API examples or descriptions and generate comprehensive, production-ready API specifications.

Your task is to generate {}. When generating specifications, follow these principles:

1. **RESTful Best Practices**: Use appropriate HTTP methods, status codes, and follow REST conventions
2. **Consistency**: Maintain consistent naming conventions, response structures, and error handling
3. **Completeness**: Include request/response schemas, parameters, error responses, and examples
4. **Realistic**: Generate realistic and practical API designs that solve real problems
5. **Security**: Include authentication/authorization considerations where appropriate
6. **Documentation**: Provide clear descriptions for all endpoints, parameters, and responses

When suggesting additional endpoints, consider:
- CRUD operations for identified resources
- Common utility endpoints (health, status, metrics)
- Related resources and their relationships
- Filtering, pagination, and search capabilities
- Batch operations where appropriate

Respond with valid JSON in the following structure:
{{
  "detected_domain": "string (e.g., 'e-commerce', 'social-media', 'fintech')",
  "endpoints": [
    {{
      "method": "GET|POST|PUT|DELETE|PATCH",
      "path": "/api/resource",
      "description": "What this endpoint does",
      "parameters": [
        {{
          "name": "param_name",
          "location": "path|query|header|body",
          "data_type": "string|integer|boolean|object",
          "required": true|false,
          "description": "Parameter description"
        }}
      ],
      "response_schema": {{ /* JSON schema */ }},
      "reasoning": "Why this endpoint is suggested"
    }}
  ],
  "openapi_spec": {{ /* Complete OpenAPI 3.0 spec if requested */ }},
  "mockforge_config": {{ /* Complete MockForge config if requested */ }}
}}

Generate {} additional endpoint suggestions beyond what was provided in the input."#,
            format_desc, self.config.num_suggestions
        )
    }

    /// Build prompt for single endpoint input
    fn build_endpoint_prompt(
        &self,
        method: &str,
        path: &str,
        request: &Option<Value>,
        response: &Option<Value>,
        description: &Option<String>,
    ) -> String {
        let domain_hint = self.config.domain_hint.as_deref().unwrap_or("general");

        let desc_text = description
            .as_ref()
            .map(|d| format!("Description: {}\n", d))
            .unwrap_or_default();

        let request_text = request
            .as_ref()
            .map(|r| {
                format!(
                    "Request:\n```json\n{}\n```\n",
                    serde_json::to_string_pretty(r).unwrap_or_default()
                )
            })
            .unwrap_or_default();

        let response_text = response
            .as_ref()
            .map(|r| {
                format!(
                    "Response:\n```json\n{}\n```\n",
                    serde_json::to_string_pretty(r).unwrap_or_default()
                )
            })
            .unwrap_or_default();

        format!(
            r#"I have the following API endpoint example:

Method: {}
Path: {}
{}{}{}
API Domain/Category: {}

Based on this single endpoint, please:
1. Analyze the API's purpose and domain
2. Suggest additional endpoints that would typically exist in such an API
3. Generate a complete specification with realistic request/response schemas
4. Include appropriate error handling and status codes
5. Add pagination, filtering, or search capabilities where relevant

Focus on creating a cohesive and practical API design that follows industry best practices."#,
            method, path, desc_text, request_text, response_text, domain_hint
        )
    }

    /// Build prompt for description input
    fn build_description_prompt(&self, description: &str) -> String {
        let domain_hint = self.config.domain_hint.as_deref().unwrap_or("general");

        format!(
            r#"I need to create an API with the following description:

{}

API Domain/Category: {}

Based on this description, please:
1. Design a comprehensive REST API with all necessary endpoints
2. Define resource models and their relationships
3. Include CRUD operations for main resources
4. Add supporting endpoints (search, filters, pagination)
5. Generate complete request/response schemas with realistic examples
6. Consider authentication, authorization, and error handling
7. Generate a complete specification ready for implementation

Create a production-ready API design that follows REST best practices and industry standards."#,
            description, domain_hint
        )
    }

    /// Build prompt for partial spec input
    fn build_partial_spec_prompt(&self, spec: &Value) -> String {
        format!(
            r#"I have a partial API specification:

```json
{}
```

Please:
1. Analyze the existing specification structure
2. Complete missing sections (schemas, responses, parameters)
3. Suggest additional endpoints that would complement the existing ones
4. Ensure consistency across all endpoints
5. Add realistic examples and descriptions
6. Fill in any gaps in the specification
7. Generate a complete, production-ready specification

Maintain the style and conventions of the original specification while expanding it."#,
            serde_json::to_string_pretty(spec).unwrap_or_default()
        )
    }

    /// Build prompt for paths-only input
    fn build_paths_prompt(&self, paths: &[String]) -> String {
        let paths_list = paths.join("\n- ");
        let domain_hint = self.config.domain_hint.as_deref().unwrap_or("general");

        format!(
            r#"I have a list of API endpoint paths:

- {}

API Domain/Category: {}

Based on these paths, please:
1. Infer the API's purpose and resource model
2. Design appropriate HTTP methods for each path
3. Generate complete request/response schemas
4. Add query parameters for filtering, pagination, and sorting where appropriate
5. Include proper error responses
6. Suggest additional related endpoints that are missing
7. Generate a complete specification

Create a cohesive API design that makes sense for these endpoints and follows REST conventions."#,
            paths_list, domain_hint
        )
    }

    /// Parse LLM response into structured result
    async fn parse_llm_response(
        &self,
        response: Value,
        _input: &SuggestionInput,
    ) -> Result<SuggestionResult> {
        // Extract endpoints
        let endpoints = response
            .get("endpoints")
            .and_then(|e| e.as_array())
            .ok_or_else(|| crate::Error::generic("No endpoints in LLM response"))?;

        let suggestions: Vec<EndpointSuggestion> =
            endpoints.iter().filter_map(|e| self.parse_endpoint_suggestion(e)).collect();

        // Extract specs based on format
        let openapi_spec =
            if matches!(self.config.output_format, OutputFormat::OpenAPI | OutputFormat::Both) {
                response.get("openapi_spec").cloned()
            } else {
                None
            };

        let mockforge_config =
            if matches!(self.config.output_format, OutputFormat::MockForge | OutputFormat::Both) {
                response.get("mockforge_config").cloned()
            } else {
                None
            };

        // Extract metadata
        let detected_domain =
            response.get("detected_domain").and_then(|d| d.as_str()).map(String::from);

        let metadata = SuggestionMetadata {
            endpoint_count: suggestions.len(),
            detected_domain,
            timestamp: chrono::Utc::now().to_rfc3339(),
            model: self.config.llm_config.model.clone(),
        };

        Ok(SuggestionResult {
            openapi_spec,
            mockforge_config,
            suggestions,
            metadata,
        })
    }

    /// Parse individual endpoint suggestion
    fn parse_endpoint_suggestion(&self, endpoint: &Value) -> Option<EndpointSuggestion> {
        let method = endpoint.get("method")?.as_str()?.to_string();
        let path = endpoint.get("path")?.as_str()?.to_string();
        let description = endpoint.get("description")?.as_str()?.to_string();
        let reasoning = endpoint
            .get("reasoning")
            .and_then(|r| r.as_str())
            .unwrap_or("Suggested by AI")
            .to_string();

        let parameters = endpoint
            .get("parameters")
            .and_then(|p| p.as_array())
            .map(|params| params.iter().filter_map(|p| self.parse_parameter(p)).collect())
            .unwrap_or_default();

        let response_schema = endpoint.get("response_schema").cloned();

        Some(EndpointSuggestion {
            method,
            path,
            description,
            parameters,
            response_schema,
            reasoning,
        })
    }

    /// Parse parameter information
    fn parse_parameter(&self, param: &Value) -> Option<ParameterInfo> {
        Some(ParameterInfo {
            name: param.get("name")?.as_str()?.to_string(),
            location: param.get("location")?.as_str()?.to_string(),
            data_type: param.get("data_type")?.as_str()?.to_string(),
            required: param.get("required")?.as_bool()?,
            description: param.get("description").and_then(|d| d.as_str()).map(String::from),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!("openapi".parse::<OutputFormat>().unwrap(), OutputFormat::OpenAPI);
        assert_eq!("mockforge".parse::<OutputFormat>().unwrap(), OutputFormat::MockForge);
        assert_eq!("both".parse::<OutputFormat>().unwrap(), OutputFormat::Both);
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_suggestion_config_default() {
        let config = SuggestionConfig::default();
        assert_eq!(config.output_format, OutputFormat::OpenAPI);
        assert_eq!(config.num_suggestions, 5);
        assert!(config.include_examples);
    }
}
