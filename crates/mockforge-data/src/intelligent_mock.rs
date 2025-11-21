//! Intelligent mock generation using LLMs
//!
//! This module provides AI-driven mock data generation that goes beyond static templates,
//! allowing users to define intent instead of explicit examples.

use crate::rag::{RagConfig, RagEngine};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Response generation mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResponseMode {
    /// Static response with templates
    Static,
    /// Intelligent response using LLM
    Intelligent,
    /// Hybrid mode - use templates with LLM enhancement
    Hybrid,
}

impl Default for ResponseMode {
    fn default() -> Self {
        Self::Static
    }
}

/// Intelligent mock configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligentMockConfig {
    /// Response generation mode
    pub mode: ResponseMode,
    /// Intent/prompt for LLM-based generation
    pub prompt: Option<String>,
    /// Context for generation (e.g., schema, domain knowledge)
    pub context: Option<String>,
    /// Number of examples to generate
    pub count: usize,
    /// Schema to conform to (JSON Schema format)
    pub schema: Option<Value>,
    /// Additional constraints
    pub constraints: HashMap<String, Value>,
    /// Temperature for LLM (0.0 to 2.0)
    pub temperature: Option<f32>,
    /// Enable caching for repeated requests
    pub cache_enabled: bool,
    /// RAG configuration
    pub rag_config: Option<RagConfig>,
}

impl Default for IntelligentMockConfig {
    fn default() -> Self {
        Self {
            mode: ResponseMode::Static,
            prompt: None,
            context: None,
            count: 1,
            schema: None,
            constraints: HashMap::new(),
            temperature: Some(0.7),
            cache_enabled: true,
            rag_config: None,
        }
    }
}

impl IntelligentMockConfig {
    /// Create a new intelligent mock configuration
    pub fn new(mode: ResponseMode) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }

    /// Set the intent prompt
    pub fn with_prompt(mut self, prompt: String) -> Self {
        self.prompt = Some(prompt);
        self
    }

    /// Set the context
    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    /// Set the schema
    pub fn with_schema(mut self, schema: Value) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Set the count
    pub fn with_count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    /// Add a constraint
    pub fn with_constraint(mut self, key: String, value: Value) -> Self {
        self.constraints.insert(key, value);
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set RAG configuration
    pub fn with_rag_config(mut self, config: RagConfig) -> Self {
        self.rag_config = Some(config);
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if (self.mode == ResponseMode::Intelligent || self.mode == ResponseMode::Hybrid)
            && self.prompt.is_none()
        {
            return Err(Error::generic("Prompt is required for intelligent/hybrid response mode"));
        }

        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(Error::generic("Temperature must be between 0.0 and 2.0"));
            }
        }

        Ok(())
    }
}

/// Intelligent mock generator
pub struct IntelligentMockGenerator {
    /// Configuration
    config: IntelligentMockConfig,
    /// RAG engine for LLM-based generation
    rag_engine: Option<RagEngine>,
    /// Response cache
    cache: HashMap<String, Value>,
}

impl IntelligentMockGenerator {
    /// Create a new intelligent mock generator
    pub fn new(config: IntelligentMockConfig) -> Result<Self> {
        config.validate()?;

        let rag_engine = if config.mode != ResponseMode::Static {
            let rag_config = config.rag_config.clone().unwrap_or_default();
            Some(RagEngine::new(rag_config))
        } else {
            None
        };

        Ok(Self {
            config,
            rag_engine,
            cache: HashMap::new(),
        })
    }

    /// Generate a mock response based on the configuration
    pub async fn generate(&mut self) -> Result<Value> {
        match self.config.mode {
            ResponseMode::Static => self.generate_static(),
            ResponseMode::Intelligent => self.generate_intelligent().await,
            ResponseMode::Hybrid => self.generate_hybrid().await,
        }
    }

    /// Generate a batch of mock responses
    pub async fn generate_batch(&mut self, count: usize) -> Result<Vec<Value>> {
        let mut results = Vec::with_capacity(count);
        for _ in 0..count {
            let response = self.generate().await?;
            results.push(response);
        }
        Ok(results)
    }

    /// Generate static response (fallback)
    fn generate_static(&self) -> Result<Value> {
        if let Some(schema) = &self.config.schema {
            Ok(schema.clone())
        } else {
            Ok(serde_json::json!({}))
        }
    }

    /// Generate intelligent response using LLM
    async fn generate_intelligent(&mut self) -> Result<Value> {
        let prompt = self.config.prompt.as_ref().ok_or_else(|| {
            Error::generic("Prompt is required for intelligent response generation")
        })?;

        // Check cache first
        if self.config.cache_enabled {
            let cache_key = format!("{:?}:{}", self.config.mode, prompt);
            if let Some(cached) = self.cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let rag_engine = self
            .rag_engine
            .as_mut()
            .ok_or_else(|| Error::generic("RAG engine not initialized for intelligent mode"))?;

        // Build the generation prompt
        let mut full_prompt =
            format!("Generate realistic mock data based on the following intent:\n\n{}\n", prompt);

        if let Some(context) = &self.config.context {
            full_prompt.push_str(&format!("\nContext: {}\n", context));
        }

        if let Some(schema) = &self.config.schema {
            full_prompt.push_str(&format!(
                "\nConform to this schema:\n{}\n",
                serde_json::to_string_pretty(schema).unwrap_or_default()
            ));
        }

        if !self.config.constraints.is_empty() {
            full_prompt.push_str("\nAdditional constraints:\n");
            for (key, value) in &self.config.constraints {
                full_prompt.push_str(&format!("- {}: {}\n", key, value));
            }
        }

        full_prompt.push_str("\nReturn valid JSON only, no additional text.");

        // Generate using RAG engine
        let response = rag_engine.generate_text(&full_prompt).await?;

        // Parse the response as JSON
        let json_response = self.extract_json(&response)?;

        // Cache the result
        if self.config.cache_enabled {
            let cache_key = format!("{:?}:{}", self.config.mode, prompt);
            self.cache.insert(cache_key, json_response.clone());
        }

        Ok(json_response)
    }

    /// Generate hybrid response (template + LLM enhancement)
    async fn generate_hybrid(&mut self) -> Result<Value> {
        // First generate static response
        let mut base_response = self.generate_static()?;

        // Then enhance with LLM
        let prompt =
            self.config.prompt.as_ref().ok_or_else(|| {
                Error::generic("Prompt is required for hybrid response generation")
            })?;

        let rag_engine = self
            .rag_engine
            .as_mut()
            .ok_or_else(|| Error::generic("RAG engine not initialized for hybrid mode"))?;

        let enhancement_prompt = format!(
            "Enhance this mock data based on the intent: {}\n\nCurrent data:\n{}\n\nReturn the enhanced JSON only.",
            prompt,
            serde_json::to_string_pretty(&base_response).unwrap_or_default()
        );

        let response = rag_engine.generate_text(&enhancement_prompt).await?;
        let enhanced_response = self.extract_json(&response)?;

        // Merge the enhanced response with the base
        if let (Some(base_obj), Some(enhanced_obj)) =
            (base_response.as_object_mut(), enhanced_response.as_object())
        {
            for (key, value) in enhanced_obj {
                base_obj.insert(key.clone(), value.clone());
            }
        } else {
            base_response = enhanced_response;
        }

        Ok(base_response)
    }

    /// Extract JSON from LLM response (handles markdown code blocks)
    fn extract_json(&self, response: &str) -> Result<Value> {
        let trimmed = response.trim();

        // Try to extract from markdown code blocks
        let json_str = if trimmed.starts_with("```json") {
            trimmed
                .strip_prefix("```json")
                .and_then(|s| s.strip_suffix("```"))
                .unwrap_or(trimmed)
                .trim()
        } else if trimmed.starts_with("```") {
            trimmed
                .strip_prefix("```")
                .and_then(|s| s.strip_suffix("```"))
                .unwrap_or(trimmed)
                .trim()
        } else {
            trimmed
        };

        // Parse JSON
        serde_json::from_str(json_str)
            .map_err(|e| Error::generic(format!("Failed to parse LLM response as JSON: {}", e)))
    }

    /// Update configuration
    pub fn update_config(&mut self, config: IntelligentMockConfig) -> Result<()> {
        config.validate()?;
        self.config = config;
        Ok(())
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Get current configuration
    pub fn config(&self) -> &IntelligentMockConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_mode_default() {
        assert_eq!(ResponseMode::default(), ResponseMode::Static);
    }

    #[test]
    fn test_intelligent_mock_config_default() {
        let config = IntelligentMockConfig::default();
        assert_eq!(config.mode, ResponseMode::Static);
        assert_eq!(config.count, 1);
        assert!(config.cache_enabled);
    }

    #[test]
    fn test_intelligent_mock_config_builder() {
        let config = IntelligentMockConfig::new(ResponseMode::Intelligent)
            .with_prompt("Generate customer data".to_string())
            .with_count(10)
            .with_temperature(0.8);

        assert_eq!(config.mode, ResponseMode::Intelligent);
        assert_eq!(config.prompt, Some("Generate customer data".to_string()));
        assert_eq!(config.count, 10);
        assert_eq!(config.temperature, Some(0.8));
    }

    #[test]
    fn test_intelligent_mock_config_validate_missing_prompt() {
        let config = IntelligentMockConfig::new(ResponseMode::Intelligent);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_intelligent_mock_config_validate_invalid_temperature() {
        let config = IntelligentMockConfig::new(ResponseMode::Static).with_temperature(3.0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_intelligent_mock_config_validate_valid() {
        let config = IntelligentMockConfig::new(ResponseMode::Intelligent)
            .with_prompt("Test prompt".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_extract_json_plain() {
        let generator =
            IntelligentMockGenerator::new(IntelligentMockConfig::new(ResponseMode::Static))
                .unwrap();

        let json_str = r#"{"key": "value"}"#;
        let result = generator.extract_json(json_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_json_markdown() {
        let generator =
            IntelligentMockGenerator::new(IntelligentMockConfig::new(ResponseMode::Static))
                .unwrap();

        let json_str = "```json\n{\"key\": \"value\"}\n```";
        let result = generator.extract_json(json_str);
        assert!(result.is_ok());
    }
}
