//! Configuration for the Intelligent Mock Behavior system

use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::types::BehaviorRules;
use super::session::SessionTracking;

/// Configuration for the Intelligent Mock Behavior system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligentBehaviorConfig {
    /// Enable intelligent behavior
    #[serde(default)]
    pub enabled: bool,

    /// Session tracking configuration
    #[serde(default)]
    pub session_tracking: SessionTracking,

    /// Behavior model configuration
    pub behavior_model: BehaviorModelConfig,

    /// Vector store configuration
    #[serde(default)]
    pub vector_store: VectorStoreConfig,

    /// Performance settings
    #[serde(default)]
    pub performance: PerformanceConfig,
}

impl Default for IntelligentBehaviorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            session_tracking: SessionTracking::default(),
            behavior_model: BehaviorModelConfig::default(),
            vector_store: VectorStoreConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

/// Behavior model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorModelConfig {
    /// LLM provider (openai, anthropic, ollama, openai-compatible)
    pub llm_provider: String,

    /// Model name (e.g., gpt-4, claude-3-opus, llama2)
    pub model: String,

    /// API key (optional, can use environment variable)
    pub api_key: Option<String>,

    /// API endpoint (optional, uses provider default)
    pub api_endpoint: Option<String>,

    /// Temperature for LLM generation (0.0 to 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f64,

    /// Maximum tokens for LLM response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,

    /// Behavior rules
    #[serde(default)]
    pub rules: BehaviorRules,
}

impl Default for BehaviorModelConfig {
    fn default() -> Self {
        Self {
            llm_provider: "openai".to_string(),
            model: "gpt-3.5-turbo".to_string(),
            api_key: None,
            api_endpoint: None,
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            rules: BehaviorRules::default(),
        }
    }
}

/// Vector store configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    /// Enable vector store for long-term memory
    #[serde(default)]
    pub enabled: bool,

    /// Embedding provider (openai, openai-compatible)
    #[serde(default = "default_embedding_provider")]
    pub embedding_provider: String,

    /// Embedding model (e.g., text-embedding-ada-002)
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,

    /// Storage path (optional, defaults to in-memory)
    pub storage_path: Option<String>,

    /// Number of top results to retrieve for semantic search
    #[serde(default = "default_search_limit")]
    pub semantic_search_limit: usize,

    /// Similarity threshold for semantic search (0.0 to 1.0)
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
}

impl Default for VectorStoreConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            embedding_provider: default_embedding_provider(),
            embedding_model: default_embedding_model(),
            storage_path: None,
            semantic_search_limit: default_search_limit(),
            similarity_threshold: default_similarity_threshold(),
        }
    }
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,

    /// Maximum number of interactions to keep in session history
    #[serde(default = "default_max_history")]
    pub max_history_length: usize,

    /// Session timeout in seconds (inactive sessions are removed)
    #[serde(default = "default_session_timeout")]
    pub session_timeout_seconds: u64,

    /// Enable response caching for identical requests
    #[serde(default = "default_true")]
    pub enable_response_cache: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            cache_ttl_seconds: default_cache_ttl(),
            max_history_length: default_max_history(),
            session_timeout_seconds: default_session_timeout(),
            enable_response_cache: true,
        }
    }
}

impl PerformanceConfig {
    /// Get cache TTL as Duration
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_seconds)
    }

    /// Get session timeout as Duration
    pub fn session_timeout(&self) -> Duration {
        Duration::from_secs(self.session_timeout_seconds)
    }
}

// Default value functions
fn default_temperature() -> f64 {
    0.7
}

fn default_max_tokens() -> usize {
    1024
}

fn default_embedding_provider() -> String {
    "openai".to_string()
}

fn default_embedding_model() -> String {
    "text-embedding-ada-002".to_string()
}

fn default_search_limit() -> usize {
    10
}

fn default_similarity_threshold() -> f32 {
    0.7
}

fn default_cache_ttl() -> u64 {
    300 // 5 minutes
}

fn default_max_history() -> u64 {
    50
}

fn default_session_timeout() -> u64 {
    3600 // 1 hour
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = IntelligentBehaviorConfig::default();

        assert!(!config.enabled);
        assert!(!config.vector_store.enabled);
        assert_eq!(config.behavior_model.llm_provider, "openai");
        assert_eq!(config.performance.cache_ttl_seconds, 300);
    }

    #[test]
    fn test_performance_config_durations() {
        let config = PerformanceConfig::default();

        assert_eq!(config.cache_ttl(), Duration::from_secs(300));
        assert_eq!(config.session_timeout(), Duration::from_secs(3600));
    }

    #[test]
    fn test_vector_store_config() {
        let config = VectorStoreConfig {
            enabled: true,
            embedding_provider: "openai".to_string(),
            embedding_model: "text-embedding-ada-002".to_string(),
            storage_path: Some("/tmp/vectors".to_string()),
            semantic_search_limit: 5,
            similarity_threshold: 0.8,
        };

        assert!(config.enabled);
        assert_eq!(config.semantic_search_limit, 5);
        assert_eq!(config.similarity_threshold, 0.8);
    }
}
