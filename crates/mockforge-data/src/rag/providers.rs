//! LLM and embedding provider integrations
//!
//! This module handles integrations with various LLM and embedding providers,
//! providing a unified interface for different AI services.

use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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

/// LLM provider trait
#[async_trait::async_trait]
pub trait LlmProviderTrait: Send + Sync {
    /// Generate text completion
    async fn generate_completion(
        &self,
        prompt: &str,
        max_tokens: Option<usize>,
        temperature: Option<f32>,
        top_p: Option<f32>,
        stop_sequences: Option<Vec<String>>,
    ) -> Result<String>;

    /// Generate chat completion
    async fn generate_chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        max_tokens: Option<usize>,
        temperature: Option<f32>,
        top_p: Option<f32>,
        stop_sequences: Option<Vec<String>>,
    ) -> Result<String>;

    /// Get available models
    async fn get_available_models(&self) -> Result<Vec<String>>;

    /// Check if provider is available
    async fn is_available(&self) -> bool;

    /// Get provider name
    fn name(&self) -> &'static str;

    /// Get maximum context length
    fn max_context_length(&self) -> usize;
}

/// Embedding provider trait
#[async_trait::async_trait]
pub trait EmbeddingProviderTrait: Send + Sync {
    /// Generate embedding for text
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts
    async fn generate_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;

    /// Get embedding dimensions
    fn embedding_dimensions(&self) -> usize;

    /// Get maximum tokens for embedding
    fn max_tokens(&self) -> usize;

    /// Get provider name
    fn name(&self) -> &'static str;

    /// Check if provider is available
    async fn is_available(&self) -> bool;
}

/// Chat message for LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message role
    pub role: ChatRole,
    /// Message content
    pub content: String,
    /// Optional metadata
    pub metadata: Option<HashMap<String, String>>,
}

impl ChatMessage {
    /// Create a new system message
    pub fn system(content: String) -> Self {
        Self {
            role: ChatRole::System,
            content,
            metadata: None,
        }
    }

    /// Create a new user message
    pub fn user(content: String) -> Self {
        Self {
            role: ChatRole::User,
            content,
            metadata: None,
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: String) -> Self {
        Self {
            role: ChatRole::Assistant,
            content,
            metadata: None,
        }
    }

    /// Add metadata to message
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Chat message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// OpenAI provider implementation
pub struct OpenAiProvider {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Create with custom base URL
    pub fn new_with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            base_url,
        }
    }
}

#[async_trait::async_trait]
impl LlmProviderTrait for OpenAiProvider {
    async fn generate_completion(
        &self,
        prompt: &str,
        max_tokens: Option<usize>,
        temperature: Option<f32>,
        top_p: Option<f32>,
        stop_sequences: Option<Vec<String>>,
    ) -> Result<String> {
        let mut request_body = serde_json::json!({
            "model": "gpt-3.5-turbo-instruct",
            "prompt": prompt,
            "max_tokens": max_tokens.unwrap_or(1024),
            "temperature": temperature.unwrap_or(0.7),
        });

        if let Some(top_p) = top_p {
            request_body["top_p"] = serde_json::json!(top_p);
        }

        if let Some(stop) = stop_sequences {
            request_body["stop"] = serde_json::json!(stop);
        }

        let response = self
            .client
            .post(format!("{}/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::Error::generic(format!("OpenAI API error: {}", response.status())));
        }

        let json: Value = response.json().await?;
        let content = json["choices"][0]["text"]
            .as_str()
            .ok_or_else(|| crate::Error::generic("Invalid response format"))?;

        Ok(content.to_string())
    }

    async fn generate_chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        max_tokens: Option<usize>,
        temperature: Option<f32>,
        top_p: Option<f32>,
        stop_sequences: Option<Vec<String>>,
    ) -> Result<String> {
        let openai_messages: Vec<Value> = messages
            .iter()
            .map(|msg| {
                serde_json::json!({
                    "role": format!("{:?}", msg.role).to_lowercase(),
                    "content": msg.content
                })
            })
            .collect();

        let mut request_body = serde_json::json!({
            "model": "gpt-3.5-turbo",
            "messages": openai_messages,
            "max_tokens": max_tokens.unwrap_or(1024),
            "temperature": temperature.unwrap_or(0.7),
        });

        if let Some(top_p) = top_p {
            request_body["top_p"] = serde_json::json!(top_p);
        }

        if let Some(stop) = stop_sequences {
            request_body["stop"] = serde_json::json!(stop);
        }

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
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

    async fn get_available_models(&self) -> Result<Vec<String>> {
        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::Error::generic(format!("OpenAI API error: {}", response.status())));
        }

        let json: Value = response.json().await?;
        let models = json["data"]
            .as_array()
            .ok_or_else(|| crate::Error::generic("Invalid models response format"))?;

        let model_names = models
            .iter()
            .filter_map(|model| model["id"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(model_names)
    }

    async fn is_available(&self) -> bool {
        (self.get_available_models().await).is_ok()
    }

    fn name(&self) -> &'static str {
        "OpenAI"
    }

    fn max_context_length(&self) -> usize {
        4096 // GPT-3.5 context length
    }
}

/// OpenAI embedding provider implementation
pub struct OpenAiEmbeddingProvider {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OpenAiEmbeddingProvider {
    /// Create a new OpenAI embedding provider
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "text-embedding-ada-002".to_string(),
        }
    }

    /// Create with custom model
    pub fn new_with_model(api_key: String, model: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            model,
        }
    }
}

#[async_trait::async_trait]
impl EmbeddingProviderTrait for OpenAiEmbeddingProvider {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "input": text,
                "model": self.model
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

    async fn generate_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();

        for text in texts {
            let embedding = self.generate_embedding(&text).await?;
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    fn embedding_dimensions(&self) -> usize {
        match self.model.as_str() {
            "text-embedding-ada-002" => 1536,
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            _ => 1536, // Default
        }
    }

    fn max_tokens(&self) -> usize {
        match self.model.as_str() {
            "text-embedding-ada-002" => 8191,
            "text-embedding-3-small" => 8191,
            "text-embedding-3-large" => 8191,
            _ => 8191, // Default
        }
    }

    fn name(&self) -> &'static str {
        "OpenAI"
    }

    async fn is_available(&self) -> bool {
        (self.generate_embedding("test").await).is_ok()
    }
}

/// OpenAI-compatible provider implementation
pub struct OpenAiCompatibleProvider {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OpenAiCompatibleProvider {
    /// Create a new OpenAI-compatible provider
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            base_url,
            model,
        }
    }
}

#[async_trait::async_trait]
impl LlmProviderTrait for OpenAiCompatibleProvider {
    async fn generate_completion(
        &self,
        prompt: &str,
        max_tokens: Option<usize>,
        temperature: Option<f32>,
        top_p: Option<f32>,
        stop_sequences: Option<Vec<String>>,
    ) -> Result<String> {
        let mut request_body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "max_tokens": max_tokens.unwrap_or(1024),
            "temperature": temperature.unwrap_or(0.7),
        });

        if let Some(top_p) = top_p {
            request_body["top_p"] = serde_json::json!(top_p);
        }

        if let Some(stop) = stop_sequences {
            request_body["stop"] = serde_json::json!(stop);
        }

        let response = self
            .client
            .post(format!("{}/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::Error::generic(format!("API error: {}", response.status())));
        }

        let json: Value = response.json().await?;
        let content = json["choices"][0]["text"]
            .as_str()
            .ok_or_else(|| crate::Error::generic("Invalid response format"))?;

        Ok(content.to_string())
    }

    async fn generate_chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        max_tokens: Option<usize>,
        temperature: Option<f32>,
        top_p: Option<f32>,
        stop_sequences: Option<Vec<String>>,
    ) -> Result<String> {
        let openai_messages: Vec<Value> = messages
            .iter()
            .map(|msg| {
                serde_json::json!({
                    "role": format!("{:?}", msg.role).to_lowercase(),
                    "content": msg.content
                })
            })
            .collect();

        let mut request_body = serde_json::json!({
            "model": self.model,
            "messages": openai_messages,
            "max_tokens": max_tokens.unwrap_or(1024),
            "temperature": temperature.unwrap_or(0.7),
        });

        if let Some(top_p) = top_p {
            request_body["top_p"] = serde_json::json!(top_p);
        }

        if let Some(stop) = stop_sequences {
            request_body["stop"] = serde_json::json!(stop);
        }

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
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

    async fn get_available_models(&self) -> Result<Vec<String>> {
        // Try to get models, but fall back gracefully if not available
        match self
            .client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                let json: Value = response.json().await?;
                let models = json["data"]
                    .as_array()
                    .ok_or_else(|| crate::Error::generic("Invalid models response format"))?;
                Ok(models
                    .iter()
                    .filter_map(|model| model["id"].as_str().map(|s| s.to_string()))
                    .collect())
            }
            _ => Ok(vec![self.model.clone()]), // Return configured model as fallback
        }
    }

    async fn is_available(&self) -> bool {
        (self.generate_completion("test", Some(1), None, None, None).await).is_ok()
    }

    fn name(&self) -> &'static str {
        "OpenAI Compatible"
    }

    fn max_context_length(&self) -> usize {
        4096 // Default context length
    }
}

/// Anthropic provider implementation
pub struct AnthropicProvider {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            base_url,
            model,
        }
    }
}

#[async_trait::async_trait]
impl LlmProviderTrait for AnthropicProvider {
    async fn generate_completion(
        &self,
        prompt: &str,
        max_tokens: Option<usize>,
        temperature: Option<f32>,
        top_p: Option<f32>,
        stop_sequences: Option<Vec<String>>,
    ) -> Result<String> {
        let mut request_body = serde_json::json!({
            "model": self.model,
            "max_tokens": max_tokens.unwrap_or(1024),
            "messages": [
                {
                    "role": "user",
                    "content": prompt,
                }
            ],
        });

        if let Some(temp) = temperature {
            request_body["temperature"] = serde_json::json!(temp);
        }
        if let Some(p) = top_p {
            request_body["top_p"] = serde_json::json!(p);
        }
        if let Some(stop) = stop_sequences {
            request_body["stop_sequences"] = serde_json::json!(stop);
        }

        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::Error::generic(format!(
                "Anthropic API error: {}",
                response.status()
            )));
        }

        let json: Value = response.json().await?;
        let content = json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| crate::Error::generic("Invalid response format"))?;

        Ok(content.to_string())
    }

    async fn generate_chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        max_tokens: Option<usize>,
        temperature: Option<f32>,
        top_p: Option<f32>,
        stop_sequences: Option<Vec<String>>,
    ) -> Result<String> {
        let mut anthropic_messages = Vec::new();
        let mut system_parts = Vec::new();

        for message in messages {
            match message.role {
                ChatRole::System => system_parts.push(message.content),
                ChatRole::User => anthropic_messages.push(serde_json::json!({
                    "role": "user",
                    "content": message.content,
                })),
                ChatRole::Assistant => anthropic_messages.push(serde_json::json!({
                    "role": "assistant",
                    "content": message.content,
                })),
            }
        }

        if anthropic_messages.is_empty() {
            anthropic_messages.push(serde_json::json!({
                "role": "user",
                "content": "",
            }));
        }

        let mut request_body = serde_json::json!({
            "model": self.model,
            "max_tokens": max_tokens.unwrap_or(1024),
            "messages": anthropic_messages,
        });

        if !system_parts.is_empty() {
            request_body["system"] = serde_json::json!(system_parts.join("\n"));
        }
        if let Some(temp) = temperature {
            request_body["temperature"] = serde_json::json!(temp);
        }
        if let Some(p) = top_p {
            request_body["top_p"] = serde_json::json!(p);
        }
        if let Some(stop) = stop_sequences {
            request_body["stop_sequences"] = serde_json::json!(stop);
        }

        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::Error::generic(format!(
                "Anthropic API error: {}",
                response.status()
            )));
        }

        let json: Value = response.json().await?;
        let content = json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| crate::Error::generic("Invalid response format"))?;

        Ok(content.to_string())
    }

    async fn get_available_models(&self) -> Result<Vec<String>> {
        Ok(vec![
            "claude-3-5-sonnet-latest".to_string(),
            "claude-3-5-haiku-latest".to_string(),
        ])
    }

    fn name(&self) -> &'static str {
        "Anthropic"
    }

    fn max_context_length(&self) -> usize {
        200_000
    }

    async fn is_available(&self) -> bool {
        (self.get_available_models().await).is_ok()
    }
}

/// OpenAI-compatible embedding provider implementation
pub struct OpenAiCompatibleEmbeddingProvider {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OpenAiCompatibleEmbeddingProvider {
    /// Create a new OpenAI-compatible embedding provider
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            base_url,
            model,
        }
    }
}

#[async_trait::async_trait]
impl EmbeddingProviderTrait for OpenAiCompatibleEmbeddingProvider {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "input": text,
                "model": self.model
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

    async fn generate_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();

        for text in texts {
            let embedding = self.generate_embedding(&text).await?;
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    fn embedding_dimensions(&self) -> usize {
        1536 // Default OpenAI embedding dimensions
    }

    fn max_tokens(&self) -> usize {
        8191 // Default OpenAI token limit
    }

    fn name(&self) -> &'static str {
        "OpenAI Compatible"
    }

    async fn is_available(&self) -> bool {
        (self.generate_embedding("test").await).is_ok()
    }
}

/// Provider factory for creating LLM and embedding providers
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create LLM provider from configuration
    pub fn create_llm_provider(
        provider_type: LlmProvider,
        api_key: String,
        base_url: Option<String>,
        model: String,
    ) -> Result<Box<dyn LlmProviderTrait>> {
        match provider_type {
            LlmProvider::OpenAI => {
                let base_url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                Ok(Box::new(OpenAiProvider::new_with_base_url(api_key, base_url)))
            }
            LlmProvider::Anthropic => {
                let base_url =
                    base_url.unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());
                Ok(Box::new(AnthropicProvider::new(api_key, base_url, model)))
            }
            LlmProvider::Ollama => {
                let base_url = base_url.unwrap_or_else(|| "http://localhost:11434/v1".to_string());
                Ok(Box::new(OpenAiCompatibleProvider::new(api_key, base_url, model)))
            }
            LlmProvider::OpenAICompatible => {
                let base_url = base_url.ok_or_else(|| {
                    crate::Error::generic("Base URL required for OpenAI compatible provider")
                })?;
                Ok(Box::new(OpenAiCompatibleProvider::new(api_key, base_url, model)))
            }
        }
    }

    /// Create embedding provider from configuration
    pub fn create_embedding_provider(
        provider_type: EmbeddingProvider,
        api_key: String,
        base_url: Option<String>,
        model: String,
    ) -> Result<Box<dyn EmbeddingProviderTrait>> {
        match provider_type {
            EmbeddingProvider::OpenAI => {
                Ok(Box::new(OpenAiEmbeddingProvider::new_with_model(api_key, model)))
            }
            EmbeddingProvider::OpenAICompatible => {
                let base_url = base_url.ok_or_else(|| {
                    crate::Error::generic(
                        "Base URL required for OpenAI compatible embedding provider",
                    )
                })?;
                Ok(Box::new(OpenAiCompatibleEmbeddingProvider::new(api_key, base_url, model)))
            }
            EmbeddingProvider::Ollama => {
                // Ollama embeddings use OpenAI-compatible API
                let base_url = base_url.ok_or_else(|| {
                    crate::Error::generic("Base URL required for Ollama embedding provider")
                })?;
                // Ollama doesn't require API key, use empty string
                Ok(Box::new(OpenAiCompatibleEmbeddingProvider::new(String::new(), base_url, model)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LlmProvider, ProviderFactory};

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
    }

    #[test]
    fn test_create_anthropic_provider() {
        let provider = ProviderFactory::create_llm_provider(
            LlmProvider::Anthropic,
            "key".to_string(),
            None,
            "claude-3-5-sonnet-latest".to_string(),
        )
        .expect("provider");
        assert_eq!(provider.name(), "Anthropic");
    }

    #[test]
    fn test_create_ollama_provider() {
        let provider = ProviderFactory::create_llm_provider(
            LlmProvider::Ollama,
            String::new(),
            None,
            "llama3.1".to_string(),
        )
        .expect("provider");
        assert_eq!(provider.name(), "OpenAI Compatible");
    }
}
