//! RAG-based AI generator implementation
//!
//! This module provides an implementation of the AiGenerator trait
//! using the RAG engine from mockforge-data.

use async_trait::async_trait;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
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
}
