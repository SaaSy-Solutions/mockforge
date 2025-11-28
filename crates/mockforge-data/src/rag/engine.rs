//! Core RAG engine and retrieval logic
//!
//! This module contains the main RAG engine implementation,
//! including document processing, query handling, and response generation.

use crate::rag::utils::Cache;
use crate::rag::{
    config::{EmbeddingProvider, RagConfig},
    storage::DocumentStorage,
};
use crate::schema::SchemaDefinition;
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

/// Document chunk for processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    /// Unique chunk ID
    pub id: String,
    /// Chunk content
    pub content: String,
    /// Chunk metadata
    pub metadata: HashMap<String, String>,
    /// Embedding vector
    pub embedding: Vec<f32>,
    /// Source document ID
    pub document_id: String,
    /// Chunk position in document
    pub position: usize,
    /// Chunk length
    pub length: usize,
}

impl DocumentChunk {
    /// Create a new document chunk
    pub fn new(
        id: String,
        content: String,
        metadata: HashMap<String, String>,
        embedding: Vec<f32>,
        document_id: String,
        position: usize,
        length: usize,
    ) -> Self {
        Self {
            id,
            content,
            metadata,
            embedding,
            document_id,
            position,
            length,
        }
    }

    /// Get chunk size
    pub fn size(&self) -> usize {
        self.content.len()
    }

    /// Check if chunk is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Set metadata value
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Calculate similarity with another chunk
    pub fn similarity(&self, other: &DocumentChunk) -> f32 {
        cosine_similarity(&self.embedding, &other.embedding)
    }

    /// Get content preview (first 100 characters)
    pub fn preview(&self) -> String {
        if self.content.len() > 100 {
            format!("{}...", &self.content[..100])
        } else {
            self.content.clone()
        }
    }
}

/// Search result with relevance score
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The document chunk
    pub chunk: DocumentChunk,
    /// Relevance score (0.0 to 1.0)
    pub score: f32,
    /// Rank in results
    pub rank: usize,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(chunk: DocumentChunk, score: f32, rank: usize) -> Self {
        Self { chunk, score, rank }
    }
}

impl PartialEq for SearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for SearchResult {}

impl PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SearchResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// RAG engine for document retrieval and generation
pub struct RagEngine {
    /// RAG configuration
    config: RagConfig,
    /// Document storage backend
    storage: Arc<dyn DocumentStorage>,
    /// HTTP client for API calls
    client: reqwest::Client,
    /// Total response time in milliseconds for calculating average
    total_response_time_ms: f64,
    /// Number of responses for calculating average
    response_count: usize,
    /// Cache for query embeddings
    embedding_cache: Cache<String, Vec<f32>>,
}

impl std::fmt::Debug for RagEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RagEngine")
            .field("config", &self.config)
            .field("storage", &"<DocumentStorage>")
            .field("client", &"<reqwest::Client>")
            .field("total_response_time_ms", &self.total_response_time_ms)
            .field("response_count", &self.response_count)
            .field("embedding_cache", &"<Cache>")
            .finish()
    }
}

impl RagEngine {
    /// Create a new RAG engine
    pub fn new(config: RagConfig, storage: Arc<dyn DocumentStorage>) -> Result<Self> {
        let client = reqwest::ClientBuilder::new().timeout(config.timeout_duration()).build()?;

        let cache_ttl = config.cache_ttl_duration().as_secs();

        Ok(Self {
            config,
            storage,
            client,
            total_response_time_ms: 0.0,
            response_count: 0,
            embedding_cache: Cache::new(cache_ttl, 1000), // Cache up to 1000 embeddings
        })
    }

    /// Record response time for stats
    fn record_response_time(&mut self, duration: Duration) {
        let ms = duration.as_millis() as f64;
        self.total_response_time_ms += ms;
        self.response_count += 1;
    }

    /// Get configuration
    pub fn config(&self) -> &RagConfig {
        &self.config
    }

    /// Get storage backend
    pub fn storage(&self) -> &Arc<dyn DocumentStorage> {
        &self.storage
    }

    /// Update configuration
    pub fn update_config(&mut self, config: RagConfig) -> Result<()> {
        config.validate()?;
        self.config = config;
        Ok(())
    }

    /// Add document to the knowledge base
    pub async fn add_document(
        &self,
        document_id: String,
        content: String,
        metadata: HashMap<String, String>,
    ) -> Result<()> {
        debug!("Adding document: {}", document_id);

        // Split document into chunks
        let chunks = self.create_chunks(document_id.clone(), content, metadata).await?;

        // Generate embeddings for chunks
        let chunks_with_embeddings = self.generate_embeddings(chunks).await?;

        // Store chunks
        self.storage.store_chunks(chunks_with_embeddings).await?;

        debug!("Successfully added document: {}", document_id);
        Ok(())
    }

    /// Search for relevant documents
    pub async fn search(&mut self, query: &str, top_k: Option<usize>) -> Result<Vec<SearchResult>> {
        let start = tokio::time::Instant::now();
        let top_k = top_k.unwrap_or(self.config.top_k);
        debug!("Searching for: {} (top_k: {})", query, top_k);

        // Generate embedding for query
        let query_embedding = self.generate_query_embedding(query).await?;

        // Search for similar chunks
        let candidates = self.storage.search_similar(&query_embedding, top_k * 2).await?; // Get more candidates for reranking

        // Rerank results if needed
        let results = if self.config.hybrid_search {
            self.hybrid_search(query, &query_embedding, candidates).await?
        } else {
            self.semantic_search(&query_embedding, candidates).await?
        };

        debug!("Found {} relevant chunks", results.len());
        let duration = start.elapsed();
        self.record_response_time(duration);
        Ok(results)
    }

    /// Generate response using RAG
    pub async fn generate(&mut self, query: &str, context: Option<&str>) -> Result<String> {
        let start = tokio::time::Instant::now();
        debug!("Generating response for query: {}", query);

        // Search for relevant context
        let search_results = self.search(query, None).await?;

        // Build context from search results
        let rag_context = self.build_context(&search_results, context);

        // Generate response using LLM
        let response = self.generate_with_llm(query, &rag_context).await?;

        debug!("Generated response ({} chars)", response.len());
        let duration = start.elapsed();
        self.record_response_time(duration);
        Ok(response)
    }

    /// Generate enhanced dataset using RAG
    pub async fn generate_dataset(
        &mut self,
        schema: &SchemaDefinition,
        count: usize,
        context: Option<&str>,
    ) -> Result<Vec<HashMap<String, Value>>> {
        let start = tokio::time::Instant::now();
        debug!("Generating dataset with {} rows using schema: {}", count, schema.name);

        // Create generation prompt
        let prompt = self.create_generation_prompt(schema, count, context);

        // Generate response
        let response = self.generate(&prompt, None).await?;

        // Parse response into structured data
        let dataset = self.parse_dataset_response(&response, schema)?;

        debug!("Generated dataset with {} rows", dataset.len());
        let duration = start.elapsed();
        self.record_response_time(duration);
        Ok(dataset)
    }

    /// Get engine statistics
    pub async fn get_stats(&self) -> Result<RagStats> {
        let storage_stats = self.storage.get_stats().await?;

        let average_response_time_ms = if self.response_count > 0 {
            (self.total_response_time_ms / self.response_count as f64) as f32
        } else {
            0.0
        };

        Ok(RagStats {
            total_documents: storage_stats.total_documents,
            total_chunks: storage_stats.total_chunks,
            index_size_bytes: storage_stats.index_size_bytes,
            last_updated: storage_stats.last_updated,
            cache_hit_rate: self.embedding_cache.hit_rate(),
            average_response_time_ms,
        })
    }

    /// Create chunks from document
    async fn create_chunks(
        &self,
        document_id: String,
        content: String,
        metadata: HashMap<String, String>,
    ) -> Result<Vec<DocumentChunk>> {
        let mut chunks = Vec::new();
        let words: Vec<&str> = content.split_whitespace().collect();
        let chunk_size = self.config.chunk_size;
        let overlap = self.config.chunk_overlap;

        for (i, chunk_start) in (0..words.len()).step_by(chunk_size - overlap).enumerate() {
            let chunk_end = (chunk_start + chunk_size).min(words.len());
            let chunk_words: Vec<&str> = words[chunk_start..chunk_end].to_vec();
            let chunk_content = chunk_words.join(" ");

            if !chunk_content.is_empty() {
                let chunk_id = format!("{}_chunk_{}", document_id, i);

                chunks.push(DocumentChunk::new(
                    chunk_id,
                    chunk_content,
                    metadata.clone(),
                    Vec::new(), // Embedding will be generated separately
                    document_id.clone(),
                    i,
                    chunk_words.len(),
                ));
            }
        }

        Ok(chunks)
    }

    /// Generate embeddings for chunks
    async fn generate_embeddings(&self, chunks: Vec<DocumentChunk>) -> Result<Vec<DocumentChunk>> {
        let mut chunks_with_embeddings = Vec::new();

        for chunk in chunks {
            let embedding = self.generate_embedding(&chunk.content).await?;
            let mut chunk_with_embedding = chunk;
            chunk_with_embedding.embedding = embedding;
            chunks_with_embeddings.push(chunk_with_embedding);
        }

        Ok(chunks_with_embeddings)
    }

    /// Generate embedding for text
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let provider = &self.config.embedding_provider;
        let model = &self.config.embedding_model;

        match provider {
            EmbeddingProvider::OpenAI => self.generate_openai_embedding(text, model).await,
            EmbeddingProvider::OpenAICompatible => {
                self.generate_openai_compatible_embedding(text, model).await
            }
            EmbeddingProvider::Ollama => {
                // Ollama uses OpenAI-compatible API for embeddings
                self.generate_openai_compatible_embedding(text, model).await
            }
        }
    }

    /// Generate query embedding
    async fn generate_query_embedding(&mut self, query: &str) -> Result<Vec<f32>> {
        // Check cache first
        if let Some(embedding) = self.embedding_cache.get(&query.to_string()) {
            return Ok(embedding);
        }

        // Generate new embedding
        let embedding = self.generate_embedding(query).await?;

        // Cache the result
        self.embedding_cache.put(query.to_string(), embedding.clone());

        Ok(embedding)
    }

    /// Perform semantic search
    async fn semantic_search(
        &self,
        query_embedding: &[f32],
        candidates: Vec<DocumentChunk>,
    ) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        // Calculate similarity scores
        for (rank, chunk) in candidates.iter().enumerate() {
            let score = cosine_similarity(query_embedding, &chunk.embedding);

            results.push(SearchResult::new(chunk.clone(), score, rank));
        }

        // Sort by score and filter by threshold
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        results.retain(|r| r.score >= self.config.similarity_threshold);

        // Take top-k results
        results.truncate(self.config.top_k);

        Ok(results)
    }

    /// Perform hybrid search (semantic + keyword)
    async fn hybrid_search(
        &self,
        query: &str,
        query_embedding: &[f32],
        candidates: Vec<DocumentChunk>,
    ) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        // Perform semantic search
        let semantic_results = self.semantic_search(query_embedding, candidates.clone()).await?;

        // Perform keyword search (placeholder)
        let keyword_results = self.keyword_search(query, &candidates).await?;

        // Combine results using weights
        let semantic_weight = self.config.semantic_weight;
        let keyword_weight = self.config.keyword_weight;

        for (rank, chunk) in candidates.iter().enumerate() {
            let semantic_score = semantic_results
                .iter()
                .find(|r| r.chunk.id == chunk.id)
                .map(|r| r.score)
                .unwrap_or(0.0);

            let keyword_score = keyword_results
                .iter()
                .find(|r| r.chunk.id == chunk.id)
                .map(|r| r.score)
                .unwrap_or(0.0);

            let combined_score = semantic_score * semantic_weight + keyword_score * keyword_weight;

            results.push(SearchResult::new(chunk.clone(), combined_score, rank));
        }

        // Sort and filter results
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        results.retain(|r| r.score >= self.config.similarity_threshold);
        results.truncate(self.config.top_k);

        Ok(results)
    }

    /// Perform keyword search (placeholder)
    async fn keyword_search(
        &self,
        _query: &str,
        _candidates: &[DocumentChunk],
    ) -> Result<Vec<SearchResult>> {
        // Placeholder implementation
        Ok(Vec::new())
    }

    /// Build context from search results
    fn build_context(
        &self,
        search_results: &[SearchResult],
        additional_context: Option<&str>,
    ) -> String {
        let mut context_parts = Vec::new();

        // Add search results
        for result in search_results {
            context_parts
                .push(format!("Content: {}\nRelevance: {:.2}", result.chunk.content, result.score));
        }

        // Add additional context if provided
        if let Some(context) = additional_context {
            context_parts.push(format!("Additional Context: {}", context));
        }

        context_parts.join("\n\n")
    }

    /// Generate response using LLM
    async fn generate_with_llm(&self, query: &str, context: &str) -> Result<String> {
        let provider = &self.config.provider;
        let model = &self.config.model;

        match provider {
            crate::rag::config::LlmProvider::OpenAI => {
                self.generate_openai_response(query, context, model).await
            }
            crate::rag::config::LlmProvider::Anthropic => {
                self.generate_anthropic_response(query, context, model).await
            }
            crate::rag::config::LlmProvider::OpenAICompatible => {
                self.generate_openai_compatible_response(query, context, model).await
            }
            crate::rag::config::LlmProvider::Ollama => {
                self.generate_ollama_response(query, context, model).await
            }
        }
    }

    /// Create generation prompt for dataset creation
    fn create_generation_prompt(
        &self,
        schema: &SchemaDefinition,
        count: usize,
        context: Option<&str>,
    ) -> String {
        let mut prompt = format!(
            "Generate {} rows of sample data following this schema:\n\n{:?}\n\n",
            count, schema
        );

        if let Some(context) = context {
            prompt.push_str(&format!("Additional context: {}\n\n", context));
        }

        prompt.push_str("Please generate the data in JSON format as an array of objects.");
        prompt
    }

    /// Parse dataset response from LLM
    fn parse_dataset_response(
        &self,
        response: &str,
        _schema: &SchemaDefinition,
    ) -> Result<Vec<HashMap<String, Value>>> {
        // Try to parse as JSON array
        match serde_json::from_str::<Vec<HashMap<String, Value>>>(response) {
            Ok(data) => Ok(data),
            Err(_) => {
                // Try to extract JSON from response text
                if let Some(json_start) = response.find('[') {
                    if let Some(json_end) = response.rfind(']') {
                        let json_part = &response[json_start..=json_end];
                        serde_json::from_str(json_part).map_err(|e| {
                            crate::Error::generic(format!("Failed to parse JSON: {}", e))
                        })
                    } else {
                        Err(crate::Error::generic("No closing bracket found in response"))
                    }
                } else {
                    Err(crate::Error::generic("No JSON array found in response"))
                }
            }
        }
    }

    /// Generate OpenAI embedding
    async fn generate_openai_embedding(&self, text: &str, model: &str) -> Result<Vec<f32>> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| crate::Error::generic("OpenAI API key not configured"))?;

        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "input": text,
                "model": model
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::Error::generic(format!("OpenAI API error: {}", response.status())));
        }

        let json: Value = response.json().await?;
        let embedding = json["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| crate::Error::generic("Invalid embedding response format"))?;

        Ok(embedding.iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect())
    }

    /// Generate OpenAI compatible embedding
    async fn generate_openai_compatible_embedding(
        &self,
        text: &str,
        model: &str,
    ) -> Result<Vec<f32>> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| crate::Error::generic("API key not configured"))?;

        let response = self
            .client
            .post(format!("{}/embeddings", self.config.api_endpoint))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "input": text,
                "model": model
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::Error::generic(format!("API error: {}", response.status())));
        }

        let json: Value = response.json().await?;
        let embedding = json["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| crate::Error::generic("Invalid embedding response format"))?;

        Ok(embedding.iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect())
    }

    /// Generate OpenAI response
    async fn generate_openai_response(
        &self,
        query: &str,
        context: &str,
        model: &str,
    ) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| crate::Error::generic("OpenAI API key not configured"))?;

        let messages = vec![
            serde_json::json!({
                "role": "system",
                "content": "You are a helpful assistant. Use the provided context to answer questions accurately."
            }),
            serde_json::json!({
                "role": "user",
                "content": format!("Context: {}\n\nQuestion: {}", context, query)
            }),
        ];

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "messages": messages,
                "max_tokens": self.config.max_tokens,
                "temperature": self.config.temperature,
                "top_p": self.config.top_p
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::Error::generic(format!("OpenAI API error: {}", response.status())));
        }

        let json: Value = response.json().await?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| crate::Error::generic("Invalid response format"))?;

        Ok(content.to_string())
    }

    /// Generate Anthropic response
    async fn generate_anthropic_response(
        &self,
        _query: &str,
        _context: &str,
        _model: &str,
    ) -> Result<String> {
        // Placeholder implementation
        Ok("Anthropic response placeholder".to_string())
    }

    /// Generate OpenAI compatible response
    async fn generate_openai_compatible_response(
        &self,
        query: &str,
        context: &str,
        model: &str,
    ) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| crate::Error::generic("API key not configured"))?;

        let messages = vec![
            serde_json::json!({
                "role": "system",
                "content": "You are a helpful assistant. Use the provided context to answer questions accurately."
            }),
            serde_json::json!({
                "role": "user",
                "content": format!("Context: {}\n\nQuestion: {}", context, query)
            }),
        ];

        let response = self
            .client
            .post(format!("{}/chat/completions", self.config.api_endpoint))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "messages": messages,
                "max_tokens": self.config.max_tokens,
                "temperature": self.config.temperature,
                "top_p": self.config.top_p
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::Error::generic(format!("API error: {}", response.status())));
        }

        let json: Value = response.json().await?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| crate::Error::generic("Invalid response format"))?;

        Ok(content.to_string())
    }

    /// Generate Ollama response
    async fn generate_ollama_response(
        &self,
        _query: &str,
        _context: &str,
        _model: &str,
    ) -> Result<String> {
        // Placeholder implementation
        Ok("Ollama response placeholder".to_string())
    }
}

impl Default for RagEngine {
    fn default() -> Self {
        use crate::rag::storage::InMemoryStorage;

        // Create a default RAG engine with in-memory storage
        // This is primarily for testing and compatibility purposes
        let config = crate::rag::config::RagConfig::default();
        let storage = Arc::new(InMemoryStorage::default());

        // We can unwrap here since default config should be valid
        Self::new(config, storage).expect("Failed to create default RagEngine")
    }
}

/// Cosine similarity calculation
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

/// RAG engine statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagStats {
    /// Total number of documents in the knowledge base
    pub total_documents: usize,
    /// Total number of chunks
    pub total_chunks: usize,
    /// Index size in bytes
    pub index_size_bytes: u64,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f32,
    /// Average response time in milliseconds
    pub average_response_time_ms: f32,
}

impl Default for RagStats {
    fn default() -> Self {
        Self {
            total_documents: 0,
            total_chunks: 0,
            index_size_bytes: 0,
            last_updated: chrono::Utc::now(),
            cache_hit_rate: 0.0,
            average_response_time_ms: 0.0,
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Total number of documents
    pub total_documents: usize,
    /// Total number of chunks
    pub total_chunks: usize,
    /// Index size in bytes
    pub index_size_bytes: u64,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for StorageStats {
    fn default() -> Self {
        Self {
            total_documents: 0,
            total_chunks: 0,
            index_size_bytes: 0,
            last_updated: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
    }
}
