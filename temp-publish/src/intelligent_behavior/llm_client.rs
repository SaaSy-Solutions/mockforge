//! LLM client wrapper for intelligent behavior
//!
//! This module provides a simplified interface to the RAG engine for
//! intelligent mock behavior generation.

use std::sync::Arc;
use tokio::sync::RwLock;

use super::config::BehaviorModelConfig;
use super::types::LlmGenerationRequest;
use crate::Result;

/// LLM client for generating intelligent responses
pub struct LlmClient {
    /// RAG engine (lazily initialized)
    rag_engine: Arc<RwLock<Option<Box<dyn LlmProvider>>>>,
    /// Configuration
    config: BehaviorModelConfig,
}

impl LlmClient {
    /// Create a new LLM client
    pub fn new(config: BehaviorModelConfig) -> Self {
        Self {
            rag_engine: Arc::new(RwLock::new(None)),
            config,
        }
    }

    /// Initialize the RAG engine (lazy initialization)
    async fn ensure_initialized(&self) -> Result<()> {
        let mut engine = self.rag_engine.write().await;

        if engine.is_none() {
            // Create provider based on configuration
            let provider = self.create_provider()?;
            *engine = Some(provider);
        }

        Ok(())
    }

    /// Create LLM provider based on configuration
    fn create_provider(&self) -> Result<Box<dyn LlmProvider>> {
        match self.config.llm_provider.to_lowercase().as_str() {
            "openai" => Ok(Box::new(OpenAIProvider::new(&self.config)?)),
            "anthropic" => Ok(Box::new(AnthropicProvider::new(&self.config)?)),
            "ollama" => Ok(Box::new(OllamaProvider::new(&self.config)?)),
            "openai-compatible" => Ok(Box::new(OpenAICompatibleProvider::new(&self.config)?)),
            _ => Err(crate::Error::generic(format!(
                "Unsupported LLM provider: {}",
                self.config.llm_provider
            ))),
        }
    }

    /// Generate a response from a prompt
    pub async fn generate(&self, request: &LlmGenerationRequest) -> Result<serde_json::Value> {
        self.ensure_initialized().await?;

        let engine = self.rag_engine.read().await;
        let provider = engine
            .as_ref()
            .ok_or_else(|| crate::Error::generic("LLM provider not initialized"))?;

        // Build messages
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: request.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: request.user_prompt.clone(),
            },
        ];

        // Generate response
        let response_text = provider
            .generate_chat(messages, request.temperature, request.max_tokens)
            .await?;

        // Try to parse as JSON
        match serde_json::from_str::<serde_json::Value>(&response_text) {
            Ok(json) => Ok(json),
            Err(_) => {
                // Try to extract JSON from response
                if let Some(start) = response_text.find('{') {
                    if let Some(end) = response_text.rfind('}') {
                        let json_str = &response_text[start..=end];
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                            return Ok(json);
                        }
                    }
                }

                // Fallback: wrap in object
                Ok(serde_json::json!({
                    "response": response_text,
                    "note": "Response was not valid JSON, wrapped in object"
                }))
            }
        }
    }

    /// Get configuration
    pub fn config(&self) -> &BehaviorModelConfig {
        &self.config
    }
}

/// Chat message for LLM
#[derive(Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

/// LLM provider trait
#[async_trait::async_trait]
trait LlmProvider: Send + Sync {
    /// Generate chat completion
    async fn generate_chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f64,
        max_tokens: usize,
    ) -> Result<String>;
}

/// OpenAI provider implementation
struct OpenAIProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    endpoint: String,
}

impl OpenAIProvider {
    fn new(config: &BehaviorModelConfig) -> Result<Self> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| crate::Error::generic("OpenAI API key not found"))?;

        let endpoint = config
            .api_endpoint
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model: config.model.clone(),
            endpoint,
        })
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAIProvider {
    async fn generate_chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f64,
        max_tokens: usize,
    ) -> Result<String> {
        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            }).collect::<Vec<_>>(),
            "temperature": temperature,
            "max_tokens": max_tokens,
        });

        let response = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| crate::Error::generic(format!("OpenAI API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::Error::generic(format!("OpenAI API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await.map_err(|e| {
            crate::Error::generic(format!("Failed to parse OpenAI response: {}", e))
        })?;

        // Extract content from response
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| crate::Error::generic("Invalid OpenAI response format"))?
            .to_string();

        Ok(content)
    }
}

/// Ollama provider implementation
struct OllamaProvider {
    client: reqwest::Client,
    model: String,
    endpoint: String,
}

impl OllamaProvider {
    fn new(config: &BehaviorModelConfig) -> Result<Self> {
        let endpoint = config
            .api_endpoint
            .clone()
            .unwrap_or_else(|| "http://localhost:11434/api/chat".to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            model: config.model.clone(),
            endpoint,
        })
    }
}

#[async_trait::async_trait]
impl LlmProvider for OllamaProvider {
    async fn generate_chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f64,
        max_tokens: usize,
    ) -> Result<String> {
        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            }).collect::<Vec<_>>(),
            "options": {
                "temperature": temperature,
                "num_predict": max_tokens,
            },
            "stream": false,
        });

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| crate::Error::generic(format!("Ollama API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::Error::generic(format!("Ollama API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await.map_err(|e| {
            crate::Error::generic(format!("Failed to parse Ollama response: {}", e))
        })?;

        // Extract content from response
        let content = response_json["message"]["content"]
            .as_str()
            .ok_or_else(|| crate::Error::generic("Invalid Ollama response format"))?
            .to_string();

        Ok(content)
    }
}

/// Anthropic provider implementation
struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    endpoint: String,
}

impl AnthropicProvider {
    fn new(config: &BehaviorModelConfig) -> Result<Self> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| crate::Error::generic("Anthropic API key not found"))?;

        let endpoint = config
            .api_endpoint
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model: config.model.clone(),
            endpoint,
        })
    }
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicProvider {
    async fn generate_chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f64,
        max_tokens: usize,
    ) -> Result<String> {
        // Separate system message from other messages
        let system_message =
            messages.iter().find(|m| m.role == "system").map(|m| m.content.clone());

        let chat_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect();

        let mut request_body = serde_json::json!({
            "model": self.model,
            "messages": chat_messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        });

        if let Some(system) = system_message {
            request_body["system"] = serde_json::Value::String(system);
        }

        let response = self
            .client
            .post(&self.endpoint)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| crate::Error::generic(format!("Anthropic API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::Error::generic(format!("Anthropic API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await.map_err(|e| {
            crate::Error::generic(format!("Failed to parse Anthropic response: {}", e))
        })?;

        // Extract content from response
        let content = response_json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| crate::Error::generic("Invalid Anthropic response format"))?
            .to_string();

        Ok(content)
    }
}

/// OpenAI-compatible provider (generic)
struct OpenAICompatibleProvider {
    client: reqwest::Client,
    api_key: Option<String>,
    model: String,
    endpoint: String,
}

impl OpenAICompatibleProvider {
    fn new(config: &BehaviorModelConfig) -> Result<Self> {
        let endpoint = config.api_endpoint.clone().ok_or_else(|| {
            crate::Error::generic("API endpoint required for OpenAI-compatible provider")
        })?;

        Ok(Self {
            client: reqwest::Client::new(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            endpoint,
        })
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAICompatibleProvider {
    async fn generate_chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f64,
        max_tokens: usize,
    ) -> Result<String> {
        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            }).collect::<Vec<_>>(),
            "temperature": temperature,
            "max_tokens": max_tokens,
        });

        let mut request =
            self.client.post(&self.endpoint).header("Content-Type", "application/json");

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .json(&request_body)
            .send()
            .await
            .map_err(|e| crate::Error::generic(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::Error::generic(format!("API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| crate::Error::generic(format!("Failed to parse API response: {}", e)))?;

        // Extract content (try both OpenAI and Ollama formats)
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .or_else(|| response_json["message"]["content"].as_str())
            .ok_or_else(|| crate::Error::generic("Invalid API response format"))?
            .to_string();

        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_client_creation() {
        let config = BehaviorModelConfig::default();
        let client = LlmClient::new(config);
        assert_eq!(client.config().llm_provider, "openai");
    }
}
