//! RAG configuration and settings management
//!
//! This module handles all configuration aspects of the RAG system,
//! including provider settings, model configurations, and operational parameters.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

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
    /// Local Ollama instance
    Ollama,
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
    /// Temperature for generation (0.0 to 2.0)
    pub temperature: f32,
    /// Top-p sampling (0.0 to 1.0)
    pub top_p: f32,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Embedding provider
    pub embedding_provider: EmbeddingProvider,
    /// Embedding model name
    pub embedding_model: String,
    /// Embedding dimensions
    pub embedding_dimensions: usize,
    /// Chunk size for document splitting
    pub chunk_size: usize,
    /// Chunk overlap for document splitting
    pub chunk_overlap: usize,
    /// Top-k similar chunks to retrieve
    pub top_k: usize,
    /// Similarity threshold (0.0 to 1.0)
    pub similarity_threshold: f32,
    /// Enable hybrid search (combines semantic and keyword search)
    pub hybrid_search: bool,
    /// Weight for semantic search in hybrid mode (0.0 to 1.0)
    pub semantic_weight: f32,
    /// Weight for keyword search in hybrid mode (0.0 to 1.0)
    pub keyword_weight: f32,
    /// Enable query expansion
    pub query_expansion: bool,
    /// Enable response filtering
    pub response_filtering: bool,
    /// Enable caching
    pub caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
    /// Retry configuration
    pub retry_config: RetryConfig,
    /// Custom headers for API requests
    pub custom_headers: HashMap<String, String>,
    /// Enable debug mode
    pub debug_mode: bool,
    /// Maximum context length
    pub max_context_length: usize,
    /// Response format preferences
    pub response_format: ResponseFormat,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Performance monitoring
    pub monitoring: MonitoringConfig,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::OpenAI,
            api_endpoint: "https://api.openai.com/v1".to_string(),
            api_key: None,
            model: "gpt-3.5-turbo".to_string(),
            max_tokens: 1024,
            temperature: 0.7,
            top_p: 0.9,
            timeout_secs: 30,
            max_retries: 3,
            embedding_provider: EmbeddingProvider::OpenAI,
            embedding_model: "text-embedding-ada-002".to_string(),
            embedding_dimensions: 1536,
            chunk_size: 1000,
            chunk_overlap: 200,
            top_k: 5,
            similarity_threshold: 0.7,
            hybrid_search: true,
            semantic_weight: 0.7,
            keyword_weight: 0.3,
            query_expansion: false,
            response_filtering: true,
            caching: true,
            cache_ttl_secs: 3600,
            rate_limiting: RateLimitConfig::default(),
            retry_config: RetryConfig::default(),
            custom_headers: HashMap::new(),
            debug_mode: false,
            max_context_length: 4096,
            response_format: ResponseFormat::Json,
            logging: LoggingConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl RagConfig {
    /// Create a new RAG configuration
    pub fn new(provider: LlmProvider, model: String) -> Self {
        Self {
            provider,
            model,
            ..Default::default()
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    /// Set API endpoint
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.api_endpoint = endpoint;
        self
    }

    /// Set model parameters
    pub fn with_model_params(mut self, max_tokens: usize, temperature: f32, top_p: f32) -> Self {
        self.max_tokens = max_tokens;
        self.temperature = temperature;
        self.top_p = top_p;
        self
    }

    /// Set embedding configuration
    pub fn with_embedding(
        mut self,
        provider: EmbeddingProvider,
        model: String,
        dimensions: usize,
    ) -> Self {
        self.embedding_provider = provider;
        self.embedding_model = model;
        self.embedding_dimensions = dimensions;
        self
    }

    /// Set chunking parameters
    pub fn with_chunking(mut self, chunk_size: usize, chunk_overlap: usize) -> Self {
        self.chunk_size = chunk_size;
        self.chunk_overlap = chunk_overlap;
        self
    }

    /// Set retrieval parameters
    pub fn with_retrieval(mut self, top_k: usize, similarity_threshold: f32) -> Self {
        self.top_k = top_k;
        self.similarity_threshold = similarity_threshold;
        self
    }

    /// Enable hybrid search
    pub fn with_hybrid_search(mut self, semantic_weight: f32, keyword_weight: f32) -> Self {
        self.hybrid_search = true;
        self.semantic_weight = semantic_weight;
        self.keyword_weight = keyword_weight;
        self
    }

    /// Set caching configuration
    pub fn with_caching(mut self, enabled: bool, ttl_secs: u64) -> Self {
        self.caching = enabled;
        self.cache_ttl_secs = ttl_secs;
        self
    }

    /// Set rate limiting
    pub fn with_rate_limit(mut self, requests_per_minute: u32, burst_size: u32) -> Self {
        self.rate_limiting = RateLimitConfig {
            requests_per_minute,
            burst_size,
            enabled: true,
        };
        self
    }

    /// Set retry configuration
    pub fn with_retry(mut self, max_attempts: u32, backoff_secs: u64) -> Self {
        self.retry_config = RetryConfig {
            max_attempts,
            backoff_secs,
            exponential_backoff: true,
        };
        self
    }

    /// Add custom header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.custom_headers.insert(key, value);
        self
    }

    /// Enable debug mode
    pub fn with_debug_mode(mut self, debug: bool) -> Self {
        self.debug_mode = debug;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.api_endpoint.is_empty() {
            return Err(crate::Error::generic("API endpoint cannot be empty"));
        }

        if self.model.is_empty() {
            return Err(crate::Error::generic("Model name cannot be empty"));
        }

        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(crate::Error::generic("Temperature must be between 0.0 and 2.0"));
        }

        if !(0.0..=1.0).contains(&self.top_p) {
            return Err(crate::Error::generic("Top-p must be between 0.0 and 1.0"));
        }

        if self.chunk_size == 0 {
            return Err(crate::Error::generic("Chunk size must be greater than 0"));
        }

        if self.chunk_overlap >= self.chunk_size {
            return Err(crate::Error::generic("Chunk overlap must be less than chunk size"));
        }

        if !(0.0..=1.0).contains(&self.similarity_threshold) {
            return Err(crate::Error::generic("Similarity threshold must be between 0.0 and 1.0"));
        }

        if self.hybrid_search {
            let total_weight = self.semantic_weight + self.keyword_weight;
            if (total_weight - 1.0).abs() > f32::EPSILON {
                return Err(crate::Error::generic("Hybrid search weights must sum to 1.0"));
            }
        }

        Ok(())
    }

    /// Get timeout duration
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    /// Get cache TTL duration
    pub fn cache_ttl_duration(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_secs)
    }

    /// Check if caching is enabled
    pub fn is_caching_enabled(&self) -> bool {
        self.caching
    }

    /// Check if rate limiting is enabled
    pub fn is_rate_limited(&self) -> bool {
        self.rate_limiting.enabled
    }

    /// Get requests per minute limit
    pub fn requests_per_minute(&self) -> u32 {
        self.rate_limiting.requests_per_minute
    }

    /// Get burst size for rate limiting
    pub fn burst_size(&self) -> u32 {
        self.rate_limiting.burst_size
    }

    /// Get maximum retry attempts
    pub fn max_retry_attempts(&self) -> u32 {
        self.retry_config.max_attempts
    }

    /// Get backoff duration for retries
    pub fn backoff_duration(&self) -> Duration {
        Duration::from_secs(self.retry_config.backoff_secs)
    }

    /// Check if exponential backoff is enabled
    pub fn is_exponential_backoff(&self) -> bool {
        self.retry_config.exponential_backoff
    }

    /// Get response format
    pub fn response_format(&self) -> &ResponseFormat {
        &self.response_format
    }

    /// Get logging configuration
    pub fn logging_config(&self) -> &LoggingConfig {
        &self.logging
    }

    /// Get monitoring configuration
    pub fn monitoring_config(&self) -> &MonitoringConfig {
        &self.monitoring
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Number of requests allowed per minute
    pub requests_per_minute: u32,
    /// Burst size for rate limiting
    pub burst_size: u32,
    /// Whether rate limiting is enabled
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
            enabled: true,
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Base backoff time in seconds
    pub backoff_secs: u64,
    /// Whether to use exponential backoff
    pub exponential_backoff: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_secs: 1,
            exponential_backoff: true,
        }
    }
}

/// Response format preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseFormat {
    /// Plain text response
    Text,
    /// JSON structured response
    Json,
    /// Markdown formatted response
    Markdown,
    /// Custom format with template
    Custom(String),
}

impl Default for ResponseFormat {
    fn default() -> Self {
        Self::Json
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level for RAG operations
    pub log_level: String,
    /// Enable request/response logging
    pub log_requests: bool,
    /// Enable performance logging
    pub log_performance: bool,
    /// Log file path (if any)
    pub log_file: Option<String>,
    /// Maximum log file size in MB
    pub max_log_size_mb: u64,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            log_requests: false,
            log_performance: true,
            log_file: None,
            max_log_size_mb: 100,
        }
    }
}

/// Performance monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Metrics collection interval in seconds
    pub metrics_interval_secs: u64,
    /// Enable tracing
    pub enable_tracing: bool,
    /// Tracing sample rate (0.0 to 1.0)
    pub trace_sample_rate: f32,
    /// Performance thresholds
    pub thresholds: PerformanceThresholds,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            metrics_interval_secs: 60,
            enable_tracing: false,
            trace_sample_rate: 0.1,
            thresholds: PerformanceThresholds::default(),
        }
    }
}

/// Performance thresholds for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// Maximum response time in seconds
    pub max_response_time_secs: f64,
    /// Minimum similarity score threshold
    pub min_similarity_score: f32,
    /// Maximum memory usage in MB
    pub max_memory_usage_mb: u64,
    /// Maximum CPU usage percentage
    pub max_cpu_usage_percent: f32,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_response_time_secs: 30.0,
            min_similarity_score: 0.7,
            max_memory_usage_mb: 1024,
            max_cpu_usage_percent: 80.0,
        }
    }
}

/// Configuration builder for RAG
#[derive(Debug)]
pub struct RagConfigBuilder {
    config: RagConfig,
}

impl RagConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            config: RagConfig::default(),
        }
    }

    /// Build the configuration
    pub fn build(self) -> Result<RagConfig> {
        self.config.validate()?;
        Ok(self.config)
    }

    /// Set the LLM provider
    pub fn provider(mut self, provider: LlmProvider) -> Self {
        self.config.provider = provider;
        self
    }

    /// Set the model name
    pub fn model(mut self, model: String) -> Self {
        self.config.model = model;
        self
    }

    /// Set the API key
    pub fn api_key(mut self, api_key: String) -> Self {
        self.config.api_key = Some(api_key);
        self
    }

    /// Set the API endpoint
    pub fn endpoint(mut self, endpoint: String) -> Self {
        self.config.api_endpoint = endpoint;
        self
    }

    /// Set model parameters
    pub fn model_params(mut self, max_tokens: usize, temperature: f32) -> Self {
        self.config.max_tokens = max_tokens;
        self.config.temperature = temperature;
        self
    }

    /// Set embedding configuration
    pub fn embedding(mut self, model: String, dimensions: usize) -> Self {
        self.config.embedding_model = model;
        self.config.embedding_dimensions = dimensions;
        self
    }

    /// Set chunking parameters
    pub fn chunking(mut self, size: usize, overlap: usize) -> Self {
        self.config.chunk_size = size;
        self.config.chunk_overlap = overlap;
        self
    }

    /// Set retrieval parameters
    pub fn retrieval(mut self, top_k: usize, threshold: f32) -> Self {
        self.config.top_k = top_k;
        self.config.similarity_threshold = threshold;
        self
    }

    /// Enable hybrid search
    pub fn hybrid_search(mut self, semantic_weight: f32) -> Self {
        self.config.hybrid_search = true;
        self.config.semantic_weight = semantic_weight;
        self.config.keyword_weight = 1.0 - semantic_weight;
        self
    }

    /// Enable caching
    pub fn caching(mut self, enabled: bool) -> Self {
        self.config.caching = enabled;
        self
    }

    /// Set rate limiting
    pub fn rate_limit(mut self, requests_per_minute: u32) -> Self {
        self.config.rate_limiting = RateLimitConfig {
            requests_per_minute,
            burst_size: requests_per_minute / 6, // 10-second burst
            enabled: true,
        };
        self
    }

    /// Enable debug mode
    pub fn debug(mut self, debug: bool) -> Self {
        self.config.debug_mode = debug;
        self
    }
}

impl Default for RagConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
    }
}
