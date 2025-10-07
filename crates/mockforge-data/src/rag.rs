//! RAG (Retrieval-Augmented Generation) for enhanced data synthesis
//!
//! This module has been refactored into sub-modules for better organization:
//! - config: RAG configuration and settings management
//! - engine: Core RAG engine and retrieval logic
//! - providers: LLM and embedding provider integrations
//! - storage: Document storage and vector indexing
//! - utils: Utility functions and helpers for RAG operations

// Re-export sub-modules for backward compatibility
pub mod config;
pub mod engine;
pub mod providers;
pub mod storage;
pub mod utils;

// Re-export commonly used types
pub use config::*;
pub use providers::*;
pub use utils::*;

// Re-export engine and storage types with explicit names to avoid conflicts
pub use engine::StorageStats as EngineStorageStats;
pub use storage::StorageStats as StorageStorageStats;

// Legacy imports for compatibility
use crate::{schema::SchemaDefinition, DataConfig};
use mockforge_core::Result;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Supported LLM providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    /// OpenAI GPT models
    OpenAI,
    /// Anthropic Claude models
    Anthropic,
    /// Generic OpenAI-compatible API
    OpenAICompatible,
    /// Local Ollama instance
    Ollama,
}

/// Supported embedding providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProvider {
    /// OpenAI text-embedding-ada-002
    OpenAI,
    /// Generic OpenAI-compatible embeddings API
    OpenAICompatible,
}

/// RAG configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// LLM provider
    pub provider: LlmProvider,
    /// LLM API endpoint
    pub api_endpoint: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Model name to use
    pub model: String,
    /// Maximum tokens for generation
    pub max_tokens: usize,
    /// Temperature for generation
    pub temperature: f64,
    /// Context window size
    pub context_window: usize,

    /// Whether to use semantic search instead of keyword search
    pub semantic_search_enabled: bool,
    /// Embedding provider for semantic search
    pub embedding_provider: EmbeddingProvider,
    /// Embedding model to use
    pub embedding_model: String,
    /// Embedding API endpoint (if different from LLM endpoint)
    pub embedding_endpoint: Option<String>,
    /// Similarity threshold for semantic search (0.0 to 1.0)
    pub similarity_threshold: f64,
    /// Maximum number of chunks to retrieve for semantic search
    pub max_chunks: usize,

    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    /// Maximum number of retries for failed requests
    pub max_retries: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::OpenAI,
            api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            api_key: None,
            model: "gpt-3.5-turbo".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            context_window: 4000,
            semantic_search_enabled: true,
            embedding_provider: EmbeddingProvider::OpenAI,
            embedding_model: "text-embedding-ada-002".to_string(),
            embedding_endpoint: None,
            similarity_threshold: 0.7,
            max_chunks: 5,
            request_timeout_seconds: 30,
            max_retries: 3,
        }
    }
}

/// Document chunk for RAG indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    /// Chunk ID
    pub id: String,
    /// Content text
    pub content: String,
    /// Metadata
    pub metadata: HashMap<String, Value>,
    /// Embedding vector for semantic search
    pub embedding: Vec<f32>,
}

/// Search result with similarity score
#[derive(Debug)]
pub struct SearchResult<'a> {
    /// The document chunk
    pub chunk: &'a DocumentChunk,
    /// Similarity score (0.0 to 1.0)
    pub score: f64,
}

/// RAG engine for enhanced data generation
#[derive(Debug)]
pub struct RagEngine {
    /// Configuration
    config: RagConfig,
    /// Document chunks for retrieval
    chunks: Vec<DocumentChunk>,
    /// Schema knowledge base
    schema_kb: HashMap<String, Vec<String>>,
    /// HTTP client for LLM API calls
    client: Client,
}

impl RagEngine {
    /// Create a new RAG engine
    pub fn new(config: RagConfig) -> Self {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(config.request_timeout_seconds))
            .build()
            .unwrap_or_else(|e| {
                warn!("Failed to create HTTP client with timeout, using default: {}", e);
                Client::new()
            });

        Self {
            config,
            chunks: Vec::new(),
            schema_kb: HashMap::new(),
            client,
        }
    }

    /// Add a document to the knowledge base
    pub fn add_document(
        &mut self,
        content: String,
        metadata: HashMap<String, Value>,
    ) -> Result<String> {
        let id = format!("chunk_{}", self.chunks.len());
        let chunk = DocumentChunk {
            id: id.clone(),
            content,
            metadata,
            embedding: Vec::new(), // Would compute embedding here
        };

        self.chunks.push(chunk);
        Ok(id)
    }

    /// Add schema information to knowledge base
    pub fn add_schema(&mut self, schema: &SchemaDefinition) -> Result<()> {
        let mut schema_info = Vec::new();

        schema_info.push(format!("Schema: {}", schema.name));

        if let Some(description) = &schema.description {
            schema_info.push(format!("Description: {}", description));
        }

        for field in &schema.fields {
            let mut field_info = format!(
                "Field '{}': type={}, required={}",
                field.name, field.field_type, field.required
            );

            if let Some(description) = &field.description {
                field_info.push_str(&format!(" - {}", description));
            }

            schema_info.push(field_info);
        }

        for (rel_name, relationship) in &schema.relationships {
            schema_info.push(format!(
                "Relationship '{}': {} -> {} ({:?})",
                rel_name, schema.name, relationship.target_schema, relationship.relationship_type
            ));
        }

        self.schema_kb.insert(schema.name.clone(), schema_info);
        Ok(())
    }

    /// Generate data using RAG-augmented prompts
    pub async fn generate_with_rag(
        &self,
        schema: &SchemaDefinition,
        config: &DataConfig,
    ) -> Result<Vec<Value>> {
        if !config.rag_enabled {
            return Err(mockforge_core::Error::generic("RAG is not enabled in config"));
        }

        // Validate RAG configuration before proceeding
        if self.config.api_key.is_none() {
            return Err(mockforge_core::Error::generic(
                "RAG is enabled but no API key is configured. Please set MOCKFORGE_RAG_API_KEY or provide --rag-api-key"
            ));
        }

        let mut results = Vec::new();
        let mut failed_rows = 0;

        // Generate prompts for each row
        for i in 0..config.rows {
            match self.generate_single_row_with_rag(schema, i).await {
                Ok(data) => results.push(data),
                Err(e) => {
                    failed_rows += 1;
                    warn!("Failed to generate RAG data for row {}: {}", i, e);

                    // If too many rows fail, return an error
                    if failed_rows > config.rows / 4 {
                        // Allow up to 25% failure rate
                        return Err(mockforge_core::Error::generic(
                            format!("Too many RAG generation failures ({} out of {} rows failed). Check API configuration and network connectivity.", failed_rows, config.rows)
                        ));
                    }

                    // For failed rows, generate fallback data
                    let fallback_data = self.generate_fallback_data(schema);
                    results.push(fallback_data);
                }
            }
        }

        if failed_rows > 0 {
            warn!(
                "RAG generation completed with {} failed rows out of {}",
                failed_rows, config.rows
            );
        }

        Ok(results)
    }

    /// Generate a single row using RAG
    async fn generate_single_row_with_rag(
        &self,
        schema: &SchemaDefinition,
        row_index: usize,
    ) -> Result<Value> {
        let prompt = self.build_generation_prompt(schema, row_index).await?;
        let generated_data = self.call_llm(&prompt).await?;
        self.parse_llm_response(&generated_data)
    }

    /// Generate fallback data when RAG fails
    fn generate_fallback_data(&self, schema: &SchemaDefinition) -> Value {
        let mut obj = serde_json::Map::new();

        for field in &schema.fields {
            let value = match field.field_type.as_str() {
                "string" => Value::String("sample_data".to_string()),
                "integer" | "number" => Value::Number(42.into()),
                "boolean" => Value::Bool(true),
                _ => Value::String("sample_data".to_string()),
            };
            obj.insert(field.name.clone(), value);
        }

        Value::Object(obj)
    }

    /// Build a generation prompt with retrieved context
    async fn build_generation_prompt(
        &self,
        schema: &SchemaDefinition,
        _row_index: usize,
    ) -> Result<String> {
        let mut prompt =
            format!("Generate a single row of data for the '{}' schema.\n\n", schema.name);

        // Add schema information
        if let Some(schema_info) = self.schema_kb.get(&schema.name) {
            prompt.push_str("Schema Information:\n");
            for info in schema_info {
                prompt.push_str(&format!("- {}\n", info));
            }
            prompt.push('\n');
        }

        // Retrieve relevant context from documents
        let relevant_chunks = self.retrieve_relevant_chunks(&schema.name, 3).await?;
        if !relevant_chunks.is_empty() {
            prompt.push_str("Relevant Context:\n");
            for chunk in relevant_chunks {
                prompt.push_str(&format!("- {}\n", chunk.content));
            }
            prompt.push('\n');
        }

        // Add generation instructions
        prompt.push_str("Instructions:\n");
        prompt.push_str("- Generate realistic data that matches the schema\n");
        prompt.push_str("- Ensure all required fields are present\n");
        prompt.push_str("- Use appropriate data types and formats\n");
        prompt.push_str("- Make relationships consistent if referenced\n");
        prompt.push_str("- Output only valid JSON for a single object\n\n");

        prompt.push_str("Generate the data:");

        Ok(prompt)
    }

    /// Retrieve relevant document chunks using semantic search or keyword search
    async fn retrieve_relevant_chunks(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<&DocumentChunk>> {
        if self.config.semantic_search_enabled {
            // Use semantic search
            let results = self.semantic_search(query, limit).await?;
            Ok(results.into_iter().map(|r| r.chunk).collect())
        } else {
            // Fall back to keyword search
            Ok(self.keyword_search(query, limit))
        }
    }

    /// Perform keyword-based search (fallback)
    pub fn keyword_search(&self, query: &str, limit: usize) -> Vec<&DocumentChunk> {
        self.chunks
            .iter()
            .filter(|chunk| {
                chunk.content.to_lowercase().contains(&query.to_lowercase())
                    || chunk.metadata.values().any(|v| {
                        if let Some(s) = v.as_str() {
                            s.to_lowercase().contains(&query.to_lowercase())
                        } else {
                            false
                        }
                    })
            })
            .take(limit)
            .collect()
    }

    /// Perform semantic search using embeddings
    async fn semantic_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult<'_>>> {
        // Generate embedding for the query
        let query_embedding = self.generate_embedding(query).await?;

        // Calculate similarity scores for all chunks
        let mut results: Vec<SearchResult> = Vec::new();

        for chunk in &self.chunks {
            if chunk.embedding.is_empty() {
                // Skip chunks without embeddings
                continue;
            }

            let score = Self::cosine_similarity(&query_embedding, &chunk.embedding);
            if score >= self.config.similarity_threshold {
                results.push(SearchResult { chunk, score });
            }
        }

        // Sort by similarity score (descending) and take top results
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    /// Generate embedding for text using the configured embedding provider
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        match &self.config.embedding_provider {
            EmbeddingProvider::OpenAI => self.generate_openai_embedding(text).await,
            EmbeddingProvider::OpenAICompatible => {
                self.generate_openai_compatible_embedding(text).await
            }
        }
    }

    /// Generate embedding using OpenAI API
    async fn generate_openai_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| mockforge_core::Error::generic("OpenAI API key not configured"))?;

        let endpoint = self
            .config
            .embedding_endpoint
            .as_ref()
            .unwrap_or(&self.config.api_endpoint)
            .replace("chat/completions", "embeddings");

        let request_body = serde_json::json!({
            "model": self.config.embedding_model,
            "input": text
        });

        debug!("Generating embedding for text with OpenAI API");

        let response = self
            .client
            .post(&endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                mockforge_core::Error::generic(format!("Embedding API request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(mockforge_core::Error::generic(format!(
                "Embedding API error: {}",
                error_text
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to parse embedding response: {}", e))
        })?;

        if let Some(data) = response_json.get("data").and_then(|d| d.as_array()) {
            if let Some(first_item) = data.first() {
                if let Some(embedding) = first_item.get("embedding").and_then(|e| e.as_array()) {
                    let embedding_vec: Vec<f32> =
                        embedding.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect();
                    return Ok(embedding_vec);
                }
            }
        }

        Err(mockforge_core::Error::generic("Invalid embedding response format"))
    }

    /// Generate embedding using OpenAI-compatible API
    async fn generate_openai_compatible_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let endpoint = self
            .config
            .embedding_endpoint
            .as_ref()
            .unwrap_or(&self.config.api_endpoint)
            .replace("chat/completions", "embeddings");

        let request_body = serde_json::json!({
            "model": self.config.embedding_model,
            "input": text
        });

        debug!("Generating embedding for text with OpenAI-compatible API");

        let mut request = self
            .client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .json(&request_body);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await.map_err(|e| {
            mockforge_core::Error::generic(format!("Embedding API request failed: {}", e))
        })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(mockforge_core::Error::generic(format!(
                "Embedding API error: {}",
                error_text
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to parse embedding response: {}", e))
        })?;

        if let Some(data) = response_json.get("data").and_then(|d| d.as_array()) {
            if let Some(first_item) = data.first() {
                if let Some(embedding) = first_item.get("embedding").and_then(|e| e.as_array()) {
                    let embedding_vec: Vec<f32> =
                        embedding.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect();
                    return Ok(embedding_vec);
                }
            }
        }

        Err(mockforge_core::Error::generic("Invalid embedding response format"))
    }

    /// Compute embeddings for all document chunks
    pub async fn compute_embeddings(&mut self) -> Result<()> {
        debug!("Computing embeddings for {} chunks", self.chunks.len());

        // Collect chunks that need embeddings
        let chunks_to_embed: Vec<(usize, String)> = self
            .chunks
            .iter()
            .enumerate()
            .filter(|(_, chunk)| chunk.embedding.is_empty())
            .map(|(idx, chunk)| (idx, chunk.content.clone()))
            .collect();

        // Generate embeddings for chunks that need them
        for (idx, content) in chunks_to_embed {
            let embedding = self.generate_embedding(&content).await?;
            self.chunks[idx].embedding = embedding;
            debug!("Computed embedding for chunk {}", self.chunks[idx].id);
        }

        Ok(())
    }

    /// Calculate cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let mut dot_product = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        for i in 0..a.len() {
            dot_product += a[i] as f64 * b[i] as f64;
            norm_a += (a[i] as f64).powi(2);
            norm_b += (b[i] as f64).powi(2);
        }

        norm_a = norm_a.sqrt();
        norm_b = norm_b.sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    /// Call LLM API with provider-specific implementation and retry logic
    async fn call_llm(&self, prompt: &str) -> Result<String> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match self.call_llm_single_attempt(prompt).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        let delay = Duration::from_millis(500 * (attempt + 1) as u64);
                        warn!(
                            "LLM API call failed (attempt {}), retrying in {:?}: {:?}",
                            attempt + 1,
                            delay,
                            last_error
                        );
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| mockforge_core::Error::generic("All LLM API retry attempts failed")))
    }

    /// Single attempt to call LLM API with provider-specific implementation
    async fn call_llm_single_attempt(&self, prompt: &str) -> Result<String> {
        match &self.config.provider {
            LlmProvider::OpenAI => self.call_openai(prompt).await,
            LlmProvider::Anthropic => self.call_anthropic(prompt).await,
            LlmProvider::OpenAICompatible => self.call_openai_compatible(prompt).await,
            LlmProvider::Ollama => self.call_ollama(prompt).await,
        }
    }

    /// Call OpenAI API
    async fn call_openai(&self, prompt: &str) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| mockforge_core::Error::generic("OpenAI API key not configured"))?;

        let request_body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature
        });

        debug!("Calling OpenAI API with model: {}", self.config.model);

        let response = self
            .client
            .post(&self.config.api_endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                mockforge_core::Error::generic(format!("OpenAI API request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(mockforge_core::Error::generic(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to parse OpenAI response: {}", e))
        })?;

        if let Some(choices) = response_json.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                if let Some(message) = choice.get("message").and_then(|m| m.get("content")) {
                    if let Some(content) = message.as_str() {
                        return Ok(content.to_string());
                    }
                }
            }
        }

        Err(mockforge_core::Error::generic("Invalid OpenAI response format"))
    }

    /// Call Anthropic API
    async fn call_anthropic(&self, prompt: &str) -> Result<String> {
        let api_key =
            self.config.api_key.as_ref().ok_or_else(|| {
                mockforge_core::Error::generic("Anthropic API key not configured")
            })?;

        let request_body = serde_json::json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        debug!("Calling Anthropic API with model: {}", self.config.model);

        let response = self
            .client
            .post(&self.config.api_endpoint)
            .header("x-api-key", api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                mockforge_core::Error::generic(format!("Anthropic API request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(mockforge_core::Error::generic(format!(
                "Anthropic API error: {}",
                error_text
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to parse Anthropic response: {}", e))
        })?;

        if let Some(content) = response_json.get("content") {
            if let Some(content_array) = content.as_array() {
                if let Some(first_content) = content_array.first() {
                    if let Some(text) = first_content.get("text").and_then(|t| t.as_str()) {
                        return Ok(text.to_string());
                    }
                }
            }
        }

        Err(mockforge_core::Error::generic("Invalid Anthropic response format"))
    }

    /// Call OpenAI-compatible API
    async fn call_openai_compatible(&self, prompt: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature
        });

        debug!("Calling OpenAI-compatible API with model: {}", self.config.model);

        let mut request = self
            .client
            .post(&self.config.api_endpoint)
            .header("Content-Type", "application/json")
            .json(&request_body);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await.map_err(|e| {
            mockforge_core::Error::generic(format!("OpenAI-compatible API request failed: {}", e))
        })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(mockforge_core::Error::generic(format!(
                "OpenAI-compatible API error: {}",
                error_text
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            mockforge_core::Error::generic(format!(
                "Failed to parse OpenAI-compatible response: {}",
                e
            ))
        })?;

        if let Some(choices) = response_json.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                if let Some(message) = choice.get("message").and_then(|m| m.get("content")) {
                    if let Some(content) = message.as_str() {
                        return Ok(content.to_string());
                    }
                }
            }
        }

        Err(mockforge_core::Error::generic("Invalid OpenAI-compatible response format"))
    }

    /// Call Ollama API
    async fn call_ollama(&self, prompt: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "model": self.config.model,
            "prompt": prompt,
            "stream": false
        });

        debug!("Calling Ollama API with model: {}", self.config.model);

        let response = self
            .client
            .post(&self.config.api_endpoint)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                mockforge_core::Error::generic(format!("Ollama API request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(mockforge_core::Error::generic(format!(
                "Ollama API error: {}",
                error_text
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to parse Ollama response: {}", e))
        })?;

        if let Some(response_text) = response_json.get("response").and_then(|r| r.as_str()) {
            return Ok(response_text.to_string());
        }

        Err(mockforge_core::Error::generic("Invalid Ollama response format"))
    }

    /// Parse LLM response into structured data
    fn parse_llm_response(&self, response: &str) -> Result<Value> {
        // Try to parse as JSON
        match serde_json::from_str(response) {
            Ok(value) => Ok(value),
            Err(e) => {
                // If direct parsing fails, try to extract JSON from the response
                if let Some(start) = response.find('{') {
                    if let Some(end) = response.rfind('}') {
                        let json_str = &response[start..=end];
                        match serde_json::from_str(json_str) {
                            Ok(value) => Ok(value),
                            Err(_) => Err(mockforge_core::Error::generic(format!(
                                "Failed to parse LLM response: {}",
                                e
                            ))),
                        }
                    } else {
                        Err(mockforge_core::Error::generic(format!(
                            "No closing brace found in response: {}",
                            e
                        )))
                    }
                } else {
                    Err(mockforge_core::Error::generic(format!("No JSON found in response: {}", e)))
                }
            }
        }
    }

    /// Update RAG configuration
    pub fn update_config(&mut self, config: RagConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &RagConfig {
        &self.config
    }

    /// Get number of indexed chunks
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Get number of indexed schemas
    pub fn schema_count(&self) -> usize {
        self.schema_kb.len()
    }

    /// Get chunk by index
    pub fn get_chunk(&self, index: usize) -> Option<&DocumentChunk> {
        self.chunks.get(index)
    }

    /// Check if schema exists in knowledge base
    pub fn has_schema(&self, name: &str) -> bool {
        self.schema_kb.contains_key(name)
    }

    /// Generate text using LLM (for intelligent mock generation)
    pub async fn generate_text(&self, prompt: &str) -> Result<String> {
        self.call_llm(prompt).await
    }
}

impl Default for RagEngine {
    fn default() -> Self {
        Self::new(RagConfig::default())
    }
}

/// RAG-enhanced data generation utilities
pub mod rag_utils {
    use super::*;

    /// Create a RAG engine with common business domain knowledge
    pub fn create_business_rag_engine() -> Result<RagEngine> {
        let mut engine = RagEngine::default();

        // Add common business knowledge
        engine.add_document(
            "Customer data typically includes personal information like name, email, phone, and address. Customers usually have unique identifiers and account creation dates.".to_string(),
            HashMap::from([
                ("domain".to_string(), Value::String("customer".to_string())),
                ("type".to_string(), Value::String("general".to_string())),
            ]),
        )?;

        engine.add_document(
            "Product information includes name, description, price, category, and stock status. Products should have unique SKUs or IDs.".to_string(),
            HashMap::from([
                ("domain".to_string(), Value::String("product".to_string())),
                ("type".to_string(), Value::String("general".to_string())),
            ]),
        )?;

        engine.add_document(
            "Order data contains customer references, product lists, total amounts, status, and timestamps. Orders should maintain referential integrity with customers and products.".to_string(),
            HashMap::from([
                ("domain".to_string(), Value::String("order".to_string())),
                ("type".to_string(), Value::String("general".to_string())),
            ]),
        )?;

        Ok(engine)
    }

    /// Create a RAG engine with technical domain knowledge
    pub fn create_technical_rag_engine() -> Result<RagEngine> {
        let mut engine = RagEngine::default();

        // Add technical knowledge
        engine.add_document(
            "API endpoints should follow RESTful conventions with proper HTTP methods. GET for retrieval, POST for creation, PUT for updates, DELETE for removal.".to_string(),
            HashMap::from([
                ("domain".to_string(), Value::String("api".to_string())),
                ("type".to_string(), Value::String("technical".to_string())),
            ]),
        )?;

        engine.add_document(
            "Database records typically have auto-incrementing primary keys, created_at and updated_at timestamps, and foreign key relationships.".to_string(),
            HashMap::from([
                ("domain".to_string(), Value::String("database".to_string())),
                ("type".to_string(), Value::String("technical".to_string())),
            ]),
        )?;

        Ok(engine)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_provider_variants() {
        let openai = LlmProvider::OpenAI;
        let anthropic = LlmProvider::Anthropic;
        let compatible = LlmProvider::OpenAICompatible;
        let ollama = LlmProvider::Ollama;

        assert!(matches!(openai, LlmProvider::OpenAI));
        assert!(matches!(anthropic, LlmProvider::Anthropic));
        assert!(matches!(compatible, LlmProvider::OpenAICompatible));
        assert!(matches!(ollama, LlmProvider::Ollama));
    }

    #[test]
    fn test_embedding_provider_variants() {
        let openai = EmbeddingProvider::OpenAI;
        let compatible = EmbeddingProvider::OpenAICompatible;

        assert!(matches!(openai, EmbeddingProvider::OpenAI));
        assert!(matches!(compatible, EmbeddingProvider::OpenAICompatible));
    }

    #[test]
    fn test_rag_config_default() {
        let config = RagConfig::default();

        assert!(config.max_tokens > 0);
        assert!(config.temperature >= 0.0 && config.temperature <= 1.0);
        assert!(config.context_window > 0);
    }
}
