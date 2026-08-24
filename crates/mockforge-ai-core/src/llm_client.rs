//! LLM client wrapper for intelligent behavior
//!
//! This module provides a simplified interface to the RAG engine for
//! intelligent mock behavior generation.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::BehaviorModelConfig;
use crate::types::LlmGenerationRequest;
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
            _ => Err(crate::Error::internal(format!(
                "Unsupported LLM provider: {}",
                self.config.llm_provider
            ))),
        }
    }

    /// Resolve the effective sampling seed (#852): per-request override,
    /// then the behavior-model config, then the `MOCKFORGE_AI_SEED` env var.
    fn resolve_seed(&self, request: &LlmGenerationRequest) -> Option<i64> {
        request
            .seed
            .or(self.config.seed)
            .or_else(|| {
                std::env::var("MOCKFORGE_AI_SEED")
                    .ok()
                    .and_then(|s| s.trim().parse().ok())
            })
    }

    /// Generate a response from a prompt
    pub async fn generate(&self, request: &LlmGenerationRequest) -> Result<serde_json::Value> {
        self.ensure_initialized().await?;

        let engine = self.rag_engine.read().await;
        let provider = engine
            .as_ref()
            .ok_or_else(|| crate::Error::internal("LLM provider not initialized"))?;

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
            .generate_chat(messages, request.temperature, request.max_tokens, self.resolve_seed(request))
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

    /// Generate a response and return usage information
    ///
    /// NOTE (#869): usage returned here is informational only. No caller
    /// persists it and this crate is NOT part of the billing path —
    /// platform-token accounting for billable AI spend lives in
    /// `mockforge-registry-server` (`ai::quota`). If a billable caller
    /// ever appears, wire `LlmUsage` into that counter explicitly.
    pub async fn generate_with_usage(
        &self,
        request: &LlmGenerationRequest,
    ) -> Result<(serde_json::Value, LlmUsage)> {
        self.ensure_initialized().await?;

        let engine = self.rag_engine.read().await;
        let provider = engine
            .as_ref()
            .ok_or_else(|| crate::Error::internal("LLM provider not initialized"))?;

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

        // Generate response with usage tracking
        let (response_text, usage) = provider
            .generate_chat_with_usage(messages, request.temperature, request.max_tokens, self.resolve_seed(request))
            .await?;

        // Try to parse as JSON
        let json_value = match serde_json::from_str::<serde_json::Value>(&response_text) {
            Ok(json) => json,
            Err(_) => {
                // Try to extract JSON from response
                if let Some(start) = response_text.find('{') {
                    if let Some(end) = response_text.rfind('}') {
                        let json_str = &response_text[start..=end];
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                            json
                        } else {
                            serde_json::json!({
                                "response": response_text,
                                "note": "Response was not valid JSON, wrapped in object"
                            })
                        }
                    } else {
                        serde_json::json!({
                            "response": response_text,
                            "note": "Response was not valid JSON, wrapped in object"
                        })
                    }
                } else {
                    serde_json::json!({
                        "response": response_text,
                        "note": "Response was not valid JSON, wrapped in object"
                    })
                }
            }
        };

        Ok((json_value, usage))
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

/// LLM usage information
#[derive(Debug, Clone, Default)]
pub struct LlmUsage {
    /// Prompt tokens used
    pub prompt_tokens: u64,
    /// Completion tokens used
    pub completion_tokens: u64,
    /// Total tokens used
    pub total_tokens: u64,
}

impl LlmUsage {
    /// Create new usage info
    pub fn new(prompt_tokens: u64, completion_tokens: u64) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

/// LLM provider trait
#[async_trait::async_trait]
trait LlmProvider: Send + Sync {
    /// Generate chat completion.
    ///
    /// `seed` (#852): provider-native sampling seed for deterministic
    /// generation. Providers without native seed support ignore it —
    /// callers wanting determinism there should also pin temperature 0.
    async fn generate_chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f64,
        max_tokens: usize,
        seed: Option<i64>,
    ) -> Result<String>;

    /// Generate chat completion with usage tracking
    async fn generate_chat_with_usage(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f64,
        max_tokens: usize,
        seed: Option<i64>,
    ) -> Result<(String, LlmUsage)> {
        // Default implementation: call generate_chat and estimate tokens
        let response = self.generate_chat(messages, temperature, max_tokens, seed).await?;
        // Rough estimation: ~4 characters per token
        let estimated_tokens = (response.len() as f64 / 4.0) as u64;
        Ok((response, LlmUsage::new(estimated_tokens, estimated_tokens)))
    }
}

/// `OpenAI` provider implementation
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
            .ok_or_else(|| crate::Error::internal("OpenAI API key not found"))?;

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
        seed: Option<i64>,
    ) -> Result<String> {
        let mut request_body = serde_json::json!({
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
        if let Some(seed) = seed {
            request_body["seed"] = serde_json::json!(seed);
        }

        let response = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| crate::Error::internal(format!("OpenAI API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::Error::internal(format!("OpenAI API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await.map_err(|e| {
            crate::Error::internal(format!("Failed to parse OpenAI response: {}", e))
        })?;

        // Extract content from response
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| crate::Error::internal("Invalid OpenAI response format"))?
            .to_string();

        Ok(content)
    }

    async fn generate_chat_with_usage(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f64,
        max_tokens: usize,
        seed: Option<i64>,
    ) -> Result<(String, LlmUsage)> {
        let mut request_body = serde_json::json!({
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
        if let Some(seed) = seed {
            request_body["seed"] = serde_json::json!(seed);
        }

        let response = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| crate::Error::internal(format!("OpenAI API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::Error::internal(format!("OpenAI API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await.map_err(|e| {
            crate::Error::internal(format!("Failed to parse OpenAI response: {}", e))
        })?;

        // Extract content from response
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| crate::Error::internal("Invalid OpenAI response format"))?
            .to_string();

        // Extract usage information
        let usage = if let Some(usage_obj) = response_json.get("usage") {
            LlmUsage::new(
                usage_obj["prompt_tokens"].as_u64().unwrap_or(0),
                usage_obj["completion_tokens"].as_u64().unwrap_or(0),
            )
        } else {
            // Fallback: estimate tokens
            let estimated = (content.len() as f64 / 4.0) as u64;
            LlmUsage::new(estimated, estimated)
        };

        Ok((content, usage))
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
        seed: Option<i64>,
    ) -> Result<String> {
        let mut request_body = serde_json::json!({
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
        // Ollama takes the seed inside `options` (#852).
        if let Some(seed) = seed {
            request_body["options"]["seed"] = serde_json::json!(seed);
        }

        let response = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| crate::Error::internal(format!("Ollama API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::Error::internal(format!("Ollama API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await.map_err(|e| {
            crate::Error::internal(format!("Failed to parse Ollama response: {}", e))
        })?;

        // Extract content from response
        let content = response_json["message"]["content"]
            .as_str()
            .ok_or_else(|| crate::Error::internal("Invalid Ollama response format"))?
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
            .ok_or_else(|| crate::Error::internal("Anthropic API key not found"))?;

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
        // Anthropic's Messages API has no sampling-seed parameter; ignored.
        // Determinism here comes from pinning temperature (see #852 docs).
        _seed: Option<i64>,
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
            .map_err(|e| crate::Error::internal(format!("Anthropic API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::Error::internal(format!("Anthropic API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response.json().await.map_err(|e| {
            crate::Error::internal(format!("Failed to parse Anthropic response: {}", e))
        })?;

        // Extract content from response
        let content = response_json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| crate::Error::internal("Invalid Anthropic response format"))?
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
            crate::Error::internal("API endpoint required for OpenAI-compatible provider")
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
        seed: Option<i64>,
    ) -> Result<String> {
        let mut request_body = serde_json::json!({
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
        // Most OpenAI-compatible servers (vLLM, llama.cpp server, LM Studio)
        // honour the OpenAI `seed` field (#852); those that don't ignore it.
        if let Some(seed) = seed {
            request_body["seed"] = serde_json::json!(seed);
        }

        let mut request =
            self.client.post(&self.endpoint).header("Content-Type", "application/json");

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .json(&request_body)
            .send()
            .await
            .map_err(|e| crate::Error::internal(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::Error::internal(format!("API error: {}", error_text)));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| crate::Error::internal(format!("Failed to parse API response: {}", e)))?;

        // Extract content (try both OpenAI and Ollama formats)
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .or_else(|| response_json["message"]["content"].as_str())
            .ok_or_else(|| crate::Error::internal("Invalid API response format"))?
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

    #[test]
    fn test_seed_resolution_precedence() {
        // request > config > env; here: no env, no config seed -> None
        std::env::remove_var("MOCKFORGE_AI_SEED");
        let client = LlmClient::new(BehaviorModelConfig {
            seed: Some(7),
            ..BehaviorModelConfig::default()
        });
        let req = LlmGenerationRequest::new("s", "u");
        assert_eq!(client.resolve_seed(&req), Some(7), "config seed used");

        let req = LlmGenerationRequest::new("s", "u").with_seed(42);
        assert_eq!(client.resolve_seed(&req), Some(42), "request seed wins");

        let client = LlmClient::new(BehaviorModelConfig::default());
        std::env::set_var("MOCKFORGE_AI_SEED", "1234");
        let req = LlmGenerationRequest::new("s", "u");
        assert_eq!(client.resolve_seed(&req), Some(1234), "env fallback");
        std::env::remove_var("MOCKFORGE_AI_SEED");
    }

    #[test]
    fn test_seed_serializes_roundtrip() {
        let req = LlmGenerationRequest::new("s", "u").with_seed(99);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"seed\":99"));
        let back: LlmGenerationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.seed, Some(99));
        // absent seed stays None (back-compat with old payloads)
        let old: LlmGenerationRequest =
            serde_json::from_str(r#"{"system_prompt":"a","user_prompt":"b"}"#).unwrap();
        assert_eq!(old.seed, None);
    }
}
