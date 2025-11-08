//! OpenAPI specification generator from recorded traffic
//!
//! This module analyzes recorded API traffic and generates OpenAPI 3.0 specifications
//! using pattern detection and LLM inference.

use super::config::BehaviorModelConfig;
use super::llm_client::LlmClient;
use super::types::LlmGenerationRequest;
use crate::openapi::spec::OpenApiSpec;
use crate::Result;
use chrono::{DateTime, Utc};
use openapiv3::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// HTTP exchange data for OpenAPI generation
///
/// This is a simplified representation of recorded HTTP exchanges
/// that can be passed to the OpenAPI generator without requiring
/// the full recorder crate dependency.
#[derive(Debug, Clone)]
pub struct HttpExchange {
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Query parameters
    pub query_params: Option<String>,
    /// Request headers (JSON string)
    pub headers: String,
    /// Request body (optional)
    pub body: Option<String>,
    /// Request body encoding
    pub body_encoding: String,
    /// Response status code
    pub status_code: Option<i32>,
    /// Response headers (JSON string)
    pub response_headers: Option<String>,
    /// Response body (optional)
    pub response_body: Option<String>,
    /// Response body encoding
    pub response_body_encoding: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Configuration for OpenAPI spec generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiGenerationConfig {
    /// Minimum confidence score for including inferred paths (0.0 to 1.0)
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,

    /// Behavior model config for LLM inference
    pub behavior_model: Option<BehaviorModelConfig>,
}

fn default_min_confidence() -> f64 {
    0.7
}

impl Default for OpenApiGenerationConfig {
    fn default() -> Self {
        Self {
            min_confidence: default_min_confidence(),
            behavior_model: None,
        }
    }
}

/// Confidence score for an inferred OpenAPI element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    /// Confidence value (0.0 to 1.0)
    pub value: f64,
    /// Reason for the confidence score
    pub reason: String,
}

/// Metadata about the generated OpenAPI spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiGenerationMetadata {
    /// Number of requests analyzed
    pub requests_analyzed: usize,
    /// Number of paths inferred
    pub paths_inferred: usize,
    /// Confidence scores per path
    pub path_confidence: HashMap<String, ConfidenceScore>,
    /// Timestamp of generation
    pub generated_at: DateTime<Utc>,
    /// Generation duration in milliseconds
    pub duration_ms: u64,
}

/// Result of OpenAPI spec generation
#[derive(Debug, Clone)]
pub struct OpenApiGenerationResult {
    /// Generated OpenAPI specification
    pub spec: OpenApiSpec,
    /// Generation metadata
    pub metadata: OpenApiGenerationMetadata,
}

/// OpenAPI specification generator from recorded traffic
pub struct OpenApiSpecGenerator {
    /// LLM client for AI-assisted generation
    llm_client: Option<LlmClient>,
    /// Configuration
    config: OpenApiGenerationConfig,
}

impl OpenApiSpecGenerator {
    /// Create a new OpenAPI spec generator
    pub fn new(config: OpenApiGenerationConfig) -> Self {
        let llm_client = config.behavior_model.as_ref().map(|bm| LlmClient::new(bm.clone()));

        Self { llm_client, config }
    }

    /// Generate OpenAPI spec from HTTP exchanges
    ///
    /// This method:
    /// 1. Groups requests by path patterns (normalize paths with parameters)
    /// 2. Analyzes request/response schemas using JSON schema inference
    /// 3. Uses LLM to infer OpenAPI spec structure from patterns
    /// 4. Generates paths, operations, schemas, and examples
    pub async fn generate_from_exchanges(
        &self,
        exchanges: Vec<HttpExchange>,
    ) -> Result<OpenApiGenerationResult> {
        let start_time = Utc::now();

        if exchanges.is_empty() {
            return Err(crate::Error::generic("No HTTP exchanges provided for OpenAPI generation"));
        }

        tracing::info!("Analyzing {} HTTP exchanges for OpenAPI generation", exchanges.len());

        // 1. Group requests by path patterns
        let path_groups = self.group_by_path_pattern(&exchanges);

        // 2. Infer path parameters
        let normalized_paths = self.infer_path_parameters(&path_groups);

        // 3. Extract schemas from request/response bodies
        let schemas = self.infer_schemas(&exchanges).await?;

        // 4. Generate OpenAPI spec structure
        let spec = if let Some(ref llm_client) = self.llm_client {
            // Use LLM for AI-assisted generation
            self.generate_with_llm(&normalized_paths, &schemas, &exchanges, llm_client)
                .await?
        } else {
            // Fallback to pattern-based generation
            self.generate_pattern_based(&normalized_paths, &schemas, &exchanges).await?
        };

        let duration_ms = (Utc::now() - start_time).num_milliseconds() as u64;

        // 5. Calculate confidence scores
        let path_confidence = self.calculate_confidence_scores(&normalized_paths, &exchanges);

        let metadata = OpenApiGenerationMetadata {
            requests_analyzed: exchanges.len(),
            paths_inferred: normalized_paths.len(),
            path_confidence,
            generated_at: start_time,
            duration_ms,
        };

        Ok(OpenApiGenerationResult { spec, metadata })
    }

    /// Group exchanges by path pattern
    pub fn group_by_path_pattern<'a>(
        &self,
        exchanges: &'a [HttpExchange],
    ) -> HashMap<String, Vec<&'a HttpExchange>> {
        let mut groups: HashMap<String, Vec<&HttpExchange>> = HashMap::new();

        for exchange in exchanges {
            let path = &exchange.path;
            groups.entry(path.clone()).or_insert_with(Vec::new).push(exchange);
        }

        groups
    }

    /// Infer path parameters from path patterns
    ///
    /// Detects patterns like `/api/users/123` and `/api/users/456` and normalizes
    /// them to `/api/users/{id}`.
    pub fn infer_path_parameters<'a>(
        &self,
        path_groups: &HashMap<String, Vec<&'a HttpExchange>>,
    ) -> HashMap<String, Vec<&'a HttpExchange>> {
        let mut normalized: HashMap<String, Vec<&HttpExchange>> = HashMap::new();

        // Group paths by their base pattern
        let mut path_segments: Vec<Vec<String>> = path_groups
            .keys()
            .map(|path| path.split('/').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect())
            .collect();

        // Find common patterns
        for (original_path, exchanges) in path_groups {
            let segments: Vec<&str> = original_path.split('/').filter(|s| !s.is_empty()).collect();

            // Try to find similar paths
            let mut normalized_path = original_path.clone();
            let mut found_match = false;

            for (other_path, _) in path_groups {
                if other_path == original_path {
                    continue;
                }

                let other_segments: Vec<&str> =
                    other_path.split('/').filter(|s| !s.is_empty()).collect();

                if segments.len() == other_segments.len() {
                    // Check if paths differ only in the last segment (likely an ID)
                    let mut normalized_segments: Vec<String> = Vec::new();
                    let mut is_parameter = false;

                    for (i, (seg, other_seg)) in
                        segments.iter().zip(other_segments.iter()).enumerate()
                    {
                        if seg == other_seg {
                            normalized_segments.push(seg.to_string());
                        } else if i == segments.len() - 1 {
                            // Last segment differs - likely a parameter
                            normalized_segments
                                .push(format!("{{{}}}", self.infer_parameter_name(seg)));
                            is_parameter = true;
                        } else {
                            // Different in middle - not a match
                            break;
                        }
                    }

                    if is_parameter {
                        normalized_path = format!("/{}", normalized_segments.join("/"));
                        found_match = true;
                        break;
                    }
                }
            }

            normalized.entry(normalized_path).or_insert_with(Vec::new).extend(exchanges);
        }

        normalized
    }

    /// Infer parameter name from path segment
    fn infer_parameter_name(&self, segment: &str) -> String {
        // Try to detect common patterns
        if segment.chars().all(|c| c.is_ascii_digit()) {
            "id".to_string()
        } else if segment.starts_with("user") || segment.contains("user") {
            "userId".to_string()
        } else if segment.starts_with("order") || segment.contains("order") {
            "orderId".to_string()
        } else {
            // Default: use singular form or generic name
            "id".to_string()
        }
    }

    /// Infer JSON schemas from request/response bodies
    pub async fn infer_schemas(
        &self,
        exchanges: &[HttpExchange],
    ) -> Result<HashMap<String, Value>> {
        let mut schemas: HashMap<String, Value> = HashMap::new();

        for exchange in exchanges {
            // Parse request body if present
            if let Some(ref body) = exchange.body {
                if exchange.body_encoding == "utf8" {
                    if let Ok(json_value) = serde_json::from_str::<Value>(body) {
                        let schema = self.json_to_schema(&json_value);
                        schemas.insert("RequestBody".to_string(), schema);
                    }
                }
            }

            // Parse response body if present
            if let Some(ref body) = exchange.response_body {
                if exchange.response_body_encoding.as_deref() == Some("utf8") {
                    if let Ok(json_value) = serde_json::from_str::<Value>(body) {
                        let schema = self.json_to_schema(&json_value);
                        schemas.insert("ResponseBody".to_string(), schema);
                    }
                }
            }
        }

        Ok(schemas)
    }

    /// Convert JSON value to JSON Schema
    pub fn json_to_schema(&self, value: &Value) -> Value {
        match value {
            Value::Null => json!({"type": "null"}),
            Value::Bool(_) => json!({"type": "boolean"}),
            Value::Number(n) => {
                if n.is_i64() {
                    json!({"type": "integer"})
                } else {
                    json!({"type": "number"})
                }
            }
            Value::String(_) => json!({"type": "string"}),
            Value::Array(arr) => {
                if let Some(first) = arr.first() {
                    json!({
                        "type": "array",
                        "items": self.json_to_schema(first)
                    })
                } else {
                    json!({"type": "array"})
                }
            }
            Value::Object(obj) => {
                let mut properties = serde_json::Map::new();
                let mut required = Vec::new();

                for (key, val) in obj {
                    properties.insert(key.clone(), self.json_to_schema(val));
                    // Assume all fields are required for now
                    required.push(key.clone());
                }

                json!({
                    "type": "object",
                    "properties": properties,
                    "required": required
                })
            }
        }
    }

    /// Generate OpenAPI spec using LLM inference
    async fn generate_with_llm(
        &self,
        normalized_paths: &HashMap<String, Vec<&HttpExchange>>,
        schemas: &HashMap<String, Value>,
        exchanges: &[HttpExchange],
        llm_client: &LlmClient,
    ) -> Result<OpenApiSpec> {
        // Build prompt for LLM
        let prompt = self.build_llm_prompt(normalized_paths, schemas, exchanges);

        let request = LlmGenerationRequest {
            system_prompt: "You are an expert at generating OpenAPI 3.0 specifications from API traffic patterns. Generate valid, well-structured OpenAPI specs.".to_string(),
            user_prompt: prompt,
            temperature: 0.3, // Lower temperature for more consistent output
            max_tokens: 4000,
            schema: None, // No schema constraint for OpenAPI generation
        };

        // Generate spec using LLM
        let response = llm_client.generate(&request).await?;

        // Parse response as OpenAPI spec
        // The LLM should return a JSON object that can be converted to OpenAPI
        let spec = OpenApiSpec::from_json(response)?;

        Ok(spec)
    }

    /// Build LLM prompt from traffic patterns
    fn build_llm_prompt(
        &self,
        normalized_paths: &HashMap<String, Vec<&HttpExchange>>,
        schemas: &HashMap<String, Value>,
        exchanges: &[HttpExchange],
    ) -> String {
        let mut prompt = String::from(
            "Generate an OpenAPI 3.0 specification from the following API traffic patterns:\n\n",
        );

        // Add path patterns
        prompt.push_str("## Paths and Methods:\n");
        for (path, path_exchanges) in normalized_paths {
            let methods: Vec<String> = path_exchanges
                .iter()
                .map(|e| e.method.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            prompt.push_str(&format!("- {}: {}\n", path, methods.join(", ")));
        }

        // Add sample request/response examples
        prompt.push_str("\n## Sample Exchanges:\n");
        for (i, exchange) in exchanges.iter().take(10).enumerate() {
            prompt.push_str(&format!("\n### Exchange {}\n", i + 1));
            prompt.push_str(&format!("Method: {}\n", exchange.method));
            prompt.push_str(&format!("Path: {}\n", exchange.path));
            if let Some(ref body) = exchange.body {
                if exchange.body_encoding == "utf8" {
                    prompt.push_str(&format!("Request Body: {}\n", body));
                }
            }
            if let Some(status) = exchange.status_code {
                prompt.push_str(&format!("Status: {}\n", status));
                if let Some(ref body) = exchange.response_body {
                    if exchange.response_body_encoding.as_deref() == Some("utf8") {
                        prompt.push_str(&format!("Response Body: {}\n", body));
                    }
                }
            }
        }

        // Add inferred schemas
        if !schemas.is_empty() {
            prompt.push_str("\n## Inferred Schemas:\n");
            prompt.push_str(&serde_json::to_string_pretty(schemas).unwrap_or_default());
        }

        prompt.push_str("\n\nGenerate a complete OpenAPI 3.0 specification in JSON format with:");
        prompt.push_str("\n- info section with title and version");
        prompt.push_str("\n- paths section with all detected endpoints");
        prompt.push_str("\n- components/schemas section with request/response schemas");
        prompt.push_str("\n- proper HTTP methods, status codes, and content types");

        prompt
    }

    /// Generate OpenAPI spec using pattern-based inference (fallback)
    async fn generate_pattern_based(
        &self,
        normalized_paths: &HashMap<String, Vec<&HttpExchange>>,
        schemas: &HashMap<String, Value>,
        exchanges: &[HttpExchange],
    ) -> Result<OpenApiSpec> {
        // Create a basic OpenAPI 3.0 spec structure
        let mut spec = OpenAPI {
            openapi: "3.0.3".to_string(),
            info: Info {
                title: "Generated API".to_string(),
                version: "1.0.0".to_string(),
                description: Some(
                    "OpenAPI specification generated from recorded traffic".to_string(),
                ),
                ..Default::default()
            },
            paths: Paths {
                paths: indexmap::IndexMap::new(),
                ..Default::default()
            },
            components: Some(Components {
                schemas: indexmap::IndexMap::new(),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Add paths
        for (path, path_exchanges) in normalized_paths {
            let mut path_item = PathItem::default();

            // Group by method
            let mut method_groups: HashMap<String, Vec<&HttpExchange>> = HashMap::new();
            for exchange in path_exchanges {
                method_groups
                    .entry(exchange.method.clone())
                    .or_insert_with(Vec::new)
                    .push(exchange);
            }

            // Add operations for each method
            for (method, method_exchanges) in method_groups {
                let operation = self.create_operation_from_exchanges(&method_exchanges)?;

                match method.as_str() {
                    "GET" => path_item.get = Some(operation),
                    "POST" => path_item.post = Some(operation),
                    "PUT" => path_item.put = Some(operation),
                    "DELETE" => path_item.delete = Some(operation),
                    "PATCH" => path_item.patch = Some(operation),
                    _ => {} // Other methods not yet supported
                }
            }

            spec.paths.paths.insert(path.clone(), ReferenceOr::Item(path_item));
        }

        // Add schemas to components
        if let Some(ref mut components) = spec.components {
            for (name, schema_value) in schemas {
                // Convert JSON Schema to OpenAPI Schema
                // This is a simplified conversion
                if let Ok(schema) = serde_json::from_value::<Schema>(schema_value.clone()) {
                    components.schemas.insert(name.clone(), ReferenceOr::Item(schema));
                }
            }
        }

        // Create raw document for serialization
        let raw_document = serde_json::to_value(&spec)?;

        Ok(OpenApiSpec {
            spec,
            file_path: None,
            raw_document: Some(raw_document),
        })
    }

    /// Create OpenAPI operation from exchanges
    fn create_operation_from_exchanges(&self, exchanges: &[&HttpExchange]) -> Result<Operation> {
        // Use the first exchange as a template
        let first = exchanges
            .first()
            .ok_or_else(|| crate::Error::generic("No exchanges provided"))?;

        let mut operation = Operation {
            summary: Some(format!("{} {}", first.method, first.path)),
            ..Default::default()
        };

        // Add responses
        let mut responses = Responses::default();
        for exchange in exchanges {
            if let Some(status_code) = exchange.status_code {
                let status = StatusCode::Code(status_code as u16);
                let mut response_obj = Response::default();

                // Add content if response has body
                if let Some(ref body) = exchange.response_body {
                    if exchange.response_body_encoding.as_deref() == Some("utf8") {
                        if let Ok(json_value) = serde_json::from_str::<Value>(body) {
                            let mut content = indexmap::IndexMap::new();
                            let mut media_type = MediaType::default();

                            // Convert JSON Schema to OpenAPI Schema
                            // For now, create a basic object schema
                            // A full conversion would require parsing the JSON Schema structure
                            let schema = match json_value {
                                Value::Object(_) => Schema {
                                    schema_data: SchemaData::default(),
                                    schema_kind: SchemaKind::Type(openapiv3::Type::Object(
                                        openapiv3::ObjectType {
                                            properties: indexmap::IndexMap::new(),
                                            required: vec![],
                                            additional_properties: None,
                                            ..Default::default()
                                        },
                                    )),
                                },
                                Value::Array(_) => Schema {
                                    schema_data: SchemaData::default(),
                                    schema_kind: SchemaKind::Type(openapiv3::Type::Array(
                                        openapiv3::ArrayType {
                                            items: None,
                                            min_items: None,
                                            max_items: None,
                                            unique_items: false,
                                        },
                                    )),
                                },
                                Value::String(_) => Schema {
                                    schema_data: SchemaData::default(),
                                    schema_kind: SchemaKind::Type(openapiv3::Type::String(
                                        openapiv3::StringType {
                                            enumeration: vec![],
                                            min_length: None,
                                            max_length: None,
                                            pattern: None,
                                            format: openapiv3::VariantOrUnknownOrEmpty::Empty,
                                        },
                                    )),
                                },
                                Value::Number(n) => {
                                    if n.is_f64() {
                                        Schema {
                                            schema_data: SchemaData::default(),
                                            schema_kind: SchemaKind::Type(openapiv3::Type::Number(
                                                openapiv3::NumberType {
                                                    minimum: None,
                                                    maximum: None,
                                                    exclusive_minimum: false,
                                                    exclusive_maximum: false,
                                                    multiple_of: None,
                                                    enumeration: vec![],
                                                    format:
                                                        openapiv3::VariantOrUnknownOrEmpty::Empty,
                                                },
                                            )),
                                        }
                                    } else {
                                        Schema {
                                            schema_data: SchemaData::default(),
                                            schema_kind: SchemaKind::Type(
                                                openapiv3::Type::Integer(openapiv3::IntegerType {
                                                    minimum: None,
                                                    maximum: None,
                                                    exclusive_minimum: false,
                                                    exclusive_maximum: false,
                                                    multiple_of: None,
                                                    enumeration: vec![],
                                                    format:
                                                        openapiv3::VariantOrUnknownOrEmpty::Item(
                                                            openapiv3::IntegerFormat::Int64,
                                                        ),
                                                }),
                                            ),
                                        }
                                    }
                                }
                                Value::Bool(_) => Schema {
                                    schema_data: SchemaData::default(),
                                    schema_kind: SchemaKind::Type(openapiv3::Type::Boolean(
                                        openapiv3::BooleanType {
                                            enumeration: vec![],
                                        },
                                    )),
                                },
                                Value::Null => Schema {
                                    schema_data: SchemaData::default(),
                                    schema_kind: SchemaKind::Type(openapiv3::Type::Object(
                                        openapiv3::ObjectType {
                                            properties: indexmap::IndexMap::new(),
                                            required: vec![],
                                            additional_properties: None,
                                            ..Default::default()
                                        },
                                    )),
                                },
                            };

                            media_type.schema = Some(ReferenceOr::Item(schema));
                            content.insert("application/json".to_string(), media_type);
                            response_obj.content = content;
                        }
                    }
                }

                responses.responses.insert(status, ReferenceOr::Item(response_obj));
            }
        }

        operation.responses = responses;

        Ok(operation)
    }

    /// Calculate confidence scores for inferred paths
    pub fn calculate_confidence_scores(
        &self,
        normalized_paths: &HashMap<String, Vec<&HttpExchange>>,
        exchanges: &[HttpExchange],
    ) -> HashMap<String, ConfidenceScore> {
        let mut scores = HashMap::new();

        for (path, path_exchanges) in normalized_paths {
            // Confidence based on:
            // 1. Number of examples (more = higher confidence)
            // 2. Consistency of status codes
            // 3. Presence of request/response bodies

            let example_count = path_exchanges.len();
            let example_ratio = (example_count as f64) / (exchanges.len() as f64);

            // Check status code consistency
            let status_codes: Vec<i32> =
                path_exchanges.iter().filter_map(|e| e.status_code).collect();
            let unique_statuses =
                status_codes.iter().collect::<std::collections::HashSet<_>>().len();
            let consistency = if unique_statuses <= 2 { 1.0 } else { 0.7 };

            // Check for request/response bodies
            let has_bodies =
                path_exchanges.iter().any(|e| e.body.is_some() || e.response_body.is_some());
            let body_score = if has_bodies { 1.0 } else { 0.5 };

            // Calculate overall confidence
            let confidence = (example_ratio * 0.4 + consistency * 0.3 + body_score * 0.3).min(1.0);

            let reason = format!(
                "Based on {} examples ({}% of total), {} unique status codes, {}",
                example_count,
                (example_ratio * 100.0) as u32,
                unique_statuses,
                if has_bodies {
                    "with request/response bodies"
                } else {
                    "without bodies"
                }
            );

            scores.insert(
                path.clone(),
                ConfidenceScore {
                    value: confidence,
                    reason,
                },
            );
        }

        scores
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_parameter_name() {
        let generator = OpenApiSpecGenerator::new(OpenApiGenerationConfig::default());
        assert_eq!(generator.infer_parameter_name("123"), "id");
        assert_eq!(generator.infer_parameter_name("user123"), "userId");
    }

    #[test]
    fn test_json_to_schema() {
        let generator = OpenApiSpecGenerator::new(OpenApiGenerationConfig::default());
        let json = json!({"name": "test", "age": 25});
        let schema = generator.json_to_schema(&json);
        assert!(schema.get("type").is_some());
        assert_eq!(schema["type"], "object");
    }
}
