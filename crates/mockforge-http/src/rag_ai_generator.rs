//! RAG-based AI generator implementation
//!
//! This module provides an implementation of the AiGenerator trait
//! using the RAG engine from mockforge-data.

use async_trait::async_trait;
use mockforge_core::{ai_response::AiResponseConfig, openapi::response::AiGenerator, Result};
use mockforge_data::rag::{LlmProvider, RagConfig, RagEngine};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, warn};

/// RAG-based AI generator that uses the mockforge-data RAG engine
pub struct RagAiGenerator {
    /// The RAG engine instance
    engine: Arc<tokio::sync::RwLock<RagEngine>>,
}

impl RagAiGenerator {
    /// Create a new RAG-based AI generator
    ///
    /// # Arguments
    /// * `rag_config` - Configuration for the RAG engine (provider, model, API key, etc.)
    ///
    /// # Returns
    /// A new RagAiGenerator instance
    pub fn new(rag_config: RagConfig) -> Result<Self> {
        debug!("Creating RAG AI generator with provider: {:?}", rag_config.provider);

        // Create the RAG engine
        let engine = RagEngine::new(rag_config);

        Ok(Self {
            engine: Arc::new(tokio::sync::RwLock::new(engine)),
        })
    }

    /// Create a RAG AI generator from environment variables
    ///
    /// Reads configuration from:
    /// - `MOCKFORGE_AI_PROVIDER`: LLM provider (openai, anthropic, ollama, etc.)
    /// - `MOCKFORGE_AI_API_KEY`: API key for the LLM provider
    /// - `MOCKFORGE_AI_MODEL`: Model name (e.g., gpt-4, claude-3-opus)
    /// - `MOCKFORGE_AI_ENDPOINT`: API endpoint (optional, uses provider default)
    /// - `MOCKFORGE_AI_TEMPERATURE`: Temperature for generation (optional, default: 0.7)
    /// - `MOCKFORGE_AI_MAX_TOKENS`: Max tokens for generation (optional, default: 1024)
    pub fn from_env() -> Result<Self> {
        let provider =
            std::env::var("MOCKFORGE_AI_PROVIDER").unwrap_or_else(|_| "openai".to_string());

        let provider = match provider.to_lowercase().as_str() {
            "openai" => LlmProvider::OpenAI,
            "anthropic" => LlmProvider::Anthropic,
            "ollama" => LlmProvider::Ollama,
            "openai-compatible" => LlmProvider::OpenAICompatible,
            _ => {
                warn!("Unknown AI provider '{}', defaulting to OpenAI", provider);
                LlmProvider::OpenAI
            }
        };

        let api_key = std::env::var("MOCKFORGE_AI_API_KEY").ok();

        let model = std::env::var("MOCKFORGE_AI_MODEL").unwrap_or_else(|_| match provider {
            LlmProvider::OpenAI => "gpt-3.5-turbo".to_string(),
            LlmProvider::Anthropic => "claude-3-haiku-20240307".to_string(),
            LlmProvider::Ollama => "llama2".to_string(),
            LlmProvider::OpenAICompatible => "gpt-3.5-turbo".to_string(),
        });

        let api_endpoint =
            std::env::var("MOCKFORGE_AI_ENDPOINT").unwrap_or_else(|_| match provider {
                LlmProvider::OpenAI => "https://api.openai.com/v1/chat/completions".to_string(),
                LlmProvider::Anthropic => "https://api.anthropic.com/v1/messages".to_string(),
                LlmProvider::Ollama => "http://localhost:11434/api/generate".to_string(),
                LlmProvider::OpenAICompatible => {
                    "http://localhost:8080/v1/chat/completions".to_string()
                }
            });

        let temperature = std::env::var("MOCKFORGE_AI_TEMPERATURE")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.7);

        let max_tokens = std::env::var("MOCKFORGE_AI_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1024);

        let config = RagConfig {
            provider,
            api_key,
            model,
            api_endpoint,
            temperature,
            max_tokens,
            ..Default::default()
        };

        debug!("Creating RAG AI generator from environment variables");
        Self::new(config)
    }
}

#[async_trait]
impl AiGenerator for RagAiGenerator {
    async fn generate(&self, prompt: &str, config: &AiResponseConfig) -> Result<Value> {
        debug!("Generating AI response with RAG engine");

        // Lock the engine for generation
        let mut engine = self.engine.write().await;

        // Update engine config with request-specific settings if needed
        let mut engine_config = engine.config().clone();
        engine_config.temperature = config.temperature as f64;
        engine_config.max_tokens = config.max_tokens;

        // Temporarily update the engine config
        engine.update_config(engine_config);

        // Generate the response using the RAG engine
        match engine.generate_text(prompt).await {
            Ok(response_text) => {
                debug!("RAG engine generated response ({} chars)", response_text.len());

                // Try to parse the response as JSON
                match serde_json::from_str::<Value>(&response_text) {
                    Ok(json_value) => Ok(json_value),
                    Err(_) => {
                        // If not valid JSON, try to extract JSON from the response
                        if let Some(start) = response_text.find('{') {
                            if let Some(end) = response_text.rfind('}') {
                                let json_str = &response_text[start..=end];
                                match serde_json::from_str::<Value>(json_str) {
                                    Ok(json_value) => Ok(json_value),
                                    Err(_) => {
                                        // If still not valid JSON, wrap in an object
                                        Ok(serde_json::json!({
                                            "response": response_text,
                                            "note": "Response was not valid JSON, wrapped in object"
                                        }))
                                    }
                                }
                            } else {
                                Ok(serde_json::json!({
                                    "response": response_text,
                                    "note": "Response was not valid JSON, wrapped in object"
                                }))
                            }
                        } else {
                            Ok(serde_json::json!({
                                "response": response_text,
                                "note": "Response was not valid JSON, wrapped in object"
                            }))
                        }
                    }
                }
            }
            Err(e) => {
                warn!("RAG engine generation failed: {}", e);
                Err(mockforge_core::Error::Config {
                    message: format!("RAG engine generation failed: {}", e),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== RagAiGenerator Creation Tests ====================

    #[test]
    fn test_rag_generator_creation() {
        let config = RagConfig {
            provider: LlmProvider::Ollama,
            api_key: None,
            model: "llama2".to_string(),
            api_endpoint: "http://localhost:11434/api/generate".to_string(),
            ..Default::default()
        };

        let result = RagAiGenerator::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rag_generator_creation_openai() {
        let config = RagConfig {
            provider: LlmProvider::OpenAI,
            api_key: Some("test-api-key".to_string()),
            model: "gpt-4".to_string(),
            api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            ..Default::default()
        };

        let result = RagAiGenerator::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rag_generator_creation_anthropic() {
        let config = RagConfig {
            provider: LlmProvider::Anthropic,
            api_key: Some("test-api-key".to_string()),
            model: "claude-3-opus".to_string(),
            api_endpoint: "https://api.anthropic.com/v1/messages".to_string(),
            ..Default::default()
        };

        let result = RagAiGenerator::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rag_generator_creation_openai_compatible() {
        let config = RagConfig {
            provider: LlmProvider::OpenAICompatible,
            api_key: None,
            model: "local-model".to_string(),
            api_endpoint: "http://localhost:8080/v1/chat/completions".to_string(),
            ..Default::default()
        };

        let result = RagAiGenerator::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rag_generator_creation_with_custom_settings() {
        let config = RagConfig {
            provider: LlmProvider::Ollama,
            api_key: None,
            model: "codellama".to_string(),
            api_endpoint: "http://localhost:11434/api/generate".to_string(),
            temperature: 0.5,
            max_tokens: 2048,
            ..Default::default()
        };

        let result = RagAiGenerator::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rag_generator_creation_with_low_temperature() {
        let config = RagConfig {
            provider: LlmProvider::OpenAI,
            api_key: Some("test-key".to_string()),
            model: "gpt-3.5-turbo".to_string(),
            api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            temperature: 0.0,
            max_tokens: 512,
            ..Default::default()
        };

        let result = RagAiGenerator::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rag_generator_creation_with_high_temperature() {
        let config = RagConfig {
            provider: LlmProvider::OpenAI,
            api_key: Some("test-key".to_string()),
            model: "gpt-4".to_string(),
            api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            temperature: 1.0,
            max_tokens: 4096,
            ..Default::default()
        };

        let result = RagAiGenerator::new(config);
        assert!(result.is_ok());
    }

    // ==================== RagConfig Tests ====================

    #[test]
    fn test_rag_config_default() {
        let config = RagConfig::default();
        // Default config should have reasonable defaults
        assert!(config.temperature >= 0.0);
        assert!(config.max_tokens > 0);
    }

    #[test]
    fn test_rag_config_clone() {
        let config = RagConfig {
            provider: LlmProvider::Ollama,
            api_key: Some("secret".to_string()),
            model: "llama2".to_string(),
            api_endpoint: "http://localhost:11434/api/generate".to_string(),
            temperature: 0.7,
            max_tokens: 1024,
            ..Default::default()
        };

        let cloned = config.clone();
        assert_eq!(cloned.model, config.model);
        assert_eq!(cloned.api_key, config.api_key);
    }

    // ==================== LlmProvider Tests ====================

    #[test]
    fn test_llm_provider_openai() {
        let provider = LlmProvider::OpenAI;
        let config = RagConfig {
            provider,
            ..Default::default()
        };
        assert!(matches!(config.provider, LlmProvider::OpenAI));
    }

    #[test]
    fn test_llm_provider_anthropic() {
        let provider = LlmProvider::Anthropic;
        let config = RagConfig {
            provider,
            ..Default::default()
        };
        assert!(matches!(config.provider, LlmProvider::Anthropic));
    }

    #[test]
    fn test_llm_provider_ollama() {
        let provider = LlmProvider::Ollama;
        let config = RagConfig {
            provider,
            ..Default::default()
        };
        assert!(matches!(config.provider, LlmProvider::Ollama));
    }

    #[test]
    fn test_llm_provider_openai_compatible() {
        let provider = LlmProvider::OpenAICompatible;
        let config = RagConfig {
            provider,
            ..Default::default()
        };
        assert!(matches!(config.provider, LlmProvider::OpenAICompatible));
    }

    // ==================== Generator Async Tests ====================

    #[tokio::test]
    async fn test_generate_fallback_to_json() {
        // This test verifies that non-JSON responses are wrapped properly
        // In a real scenario, this would require mocking the RAG engine

        let config = RagConfig {
            provider: LlmProvider::Ollama,
            api_key: None,
            model: "test-model".to_string(),
            api_endpoint: "http://localhost:11434/api/generate".to_string(),
            ..Default::default()
        };

        // We can't easily test the actual generation without a real LLM,
        // but we can verify the generator was created successfully
        let generator = RagAiGenerator::new(config);
        assert!(generator.is_ok());
    }

    #[tokio::test]
    async fn test_generator_engine_access() {
        let config = RagConfig {
            provider: LlmProvider::Ollama,
            api_key: None,
            model: "llama2".to_string(),
            api_endpoint: "http://localhost:11434/api/generate".to_string(),
            temperature: 0.8,
            max_tokens: 512,
            ..Default::default()
        };

        let generator = RagAiGenerator::new(config).unwrap();
        // The engine is wrapped in Arc<RwLock>, verify we can access it
        let engine = generator.engine.read().await;
        let engine_config = engine.config();
        assert_eq!(engine_config.model, "llama2");
    }

    #[tokio::test]
    async fn test_generator_can_be_cloned_via_arc() {
        let config = RagConfig {
            provider: LlmProvider::OpenAI,
            api_key: Some("test".to_string()),
            model: "gpt-3.5-turbo".to_string(),
            api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            ..Default::default()
        };

        let generator = RagAiGenerator::new(config).unwrap();
        // Engine is Arc-wrapped, so cloning should work
        let engine_clone = generator.engine.clone();
        assert!(Arc::strong_count(&engine_clone) >= 2);
    }

    // ==================== AiResponseConfig Tests ====================

    #[test]
    fn test_ai_response_config_with_generator() {
        // Test that we can create AiResponseConfig compatible with the generator
        let ai_config = AiResponseConfig {
            temperature: 0.7,
            max_tokens: 1024,
            ..Default::default()
        };

        assert!((ai_config.temperature - 0.7).abs() < 0.001);
        assert_eq!(ai_config.max_tokens, 1024);
    }

    #[test]
    fn test_ai_response_config_low_temp() {
        let ai_config = AiResponseConfig {
            temperature: 0.0,
            max_tokens: 256,
            ..Default::default()
        };

        assert!((ai_config.temperature - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_ai_response_config_high_tokens() {
        let ai_config = AiResponseConfig {
            temperature: 0.5,
            max_tokens: 8192,
            ..Default::default()
        };

        assert_eq!(ai_config.max_tokens, 8192);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_generator_with_empty_model_name() {
        let config = RagConfig {
            provider: LlmProvider::Ollama,
            api_key: None,
            model: String::new(), // Empty model name
            api_endpoint: "http://localhost:11434/api/generate".to_string(),
            ..Default::default()
        };

        // Should still create successfully (validation happens later)
        let result = RagAiGenerator::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generator_with_empty_endpoint() {
        let config = RagConfig {
            provider: LlmProvider::Ollama,
            api_key: None,
            model: "llama2".to_string(),
            api_endpoint: String::new(), // Empty endpoint
            ..Default::default()
        };

        // Should still create successfully (validation happens at request time)
        let result = RagAiGenerator::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generator_with_no_api_key_openai() {
        let config = RagConfig {
            provider: LlmProvider::OpenAI,
            api_key: None, // No API key (will fail at request time)
            model: "gpt-4".to_string(),
            api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            ..Default::default()
        };

        // Creation should succeed, failure happens when making requests
        let result = RagAiGenerator::new(config);
        assert!(result.is_ok());
    }

    // ==================== Integration-style Tests ====================

    #[tokio::test]
    async fn test_multiple_generators_different_providers() {
        let openai_config = RagConfig {
            provider: LlmProvider::OpenAI,
            api_key: Some("test-key".to_string()),
            model: "gpt-4".to_string(),
            api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            ..Default::default()
        };

        let ollama_config = RagConfig {
            provider: LlmProvider::Ollama,
            api_key: None,
            model: "llama2".to_string(),
            api_endpoint: "http://localhost:11434/api/generate".to_string(),
            ..Default::default()
        };

        let anthropic_config = RagConfig {
            provider: LlmProvider::Anthropic,
            api_key: Some("test-key".to_string()),
            model: "claude-3-haiku-20240307".to_string(),
            api_endpoint: "https://api.anthropic.com/v1/messages".to_string(),
            ..Default::default()
        };

        // All three should create successfully
        assert!(RagAiGenerator::new(openai_config).is_ok());
        assert!(RagAiGenerator::new(ollama_config).is_ok());
        assert!(RagAiGenerator::new(anthropic_config).is_ok());
    }

    #[tokio::test]
    async fn test_generator_engine_update() {
        let config = RagConfig {
            provider: LlmProvider::Ollama,
            api_key: None,
            model: "llama2".to_string(),
            api_endpoint: "http://localhost:11434/api/generate".to_string(),
            temperature: 0.7,
            max_tokens: 1024,
            ..Default::default()
        };

        let generator = RagAiGenerator::new(config).unwrap();

        // Test that we can read and the engine has correct config
        {
            let engine = generator.engine.read().await;
            let engine_config = engine.config();
            assert!((engine_config.temperature - 0.7).abs() < 0.001);
            assert_eq!(engine_config.max_tokens, 1024);
        }

        // Test that we can write to update the config
        {
            let mut engine = generator.engine.write().await;
            let mut new_config = engine.config().clone();
            new_config.temperature = 0.5;
            new_config.max_tokens = 2048;
            engine.update_config(new_config);
        }

        // Verify the update took effect
        {
            let engine = generator.engine.read().await;
            let engine_config = engine.config();
            assert!((engine_config.temperature - 0.5).abs() < 0.001);
            assert_eq!(engine_config.max_tokens, 2048);
        }
    }
}
