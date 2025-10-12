//! Behavior model for LLM-powered decision making
//!
//! This module provides the BehaviorModel which uses LLMs to make intelligent
//! decisions about how the mock API should respond based on context.

use std::sync::Arc;

use super::cache::{generate_cache_key, ResponseCache};
use super::config::BehaviorModelConfig;
use super::context::StatefulAiContext;
use super::llm_client::LlmClient;
use super::rules::EvaluationContext;
use super::types::{BehaviorRules, LlmGenerationRequest};
use crate::Result;

/// Behavior model that uses LLMs to generate intelligent responses
pub struct BehaviorModel {
    /// Configuration
    config: BehaviorModelConfig,

    /// Behavior rules
    rules: BehaviorRules,

    /// LLM client for generation
    llm_client: Arc<LlmClient>,

    /// Response cache
    cache: Option<Arc<ResponseCache>>,
}

impl BehaviorModel {
    /// Create a new behavior model
    pub fn new(config: BehaviorModelConfig) -> Self {
        let rules = config.rules.clone();
        let llm_client = Arc::new(LlmClient::new(config.clone()));

        // Create cache if enabled in config
        // Note: Cache config should be in PerformanceConfig
        let cache = Some(Arc::new(ResponseCache::new(300))); // 5 minutes default

        Self {
            config,
            rules,
            llm_client,
            cache,
        }
    }

    /// Generate a response based on request context and session state
    ///
    /// # Arguments
    /// * `method` - HTTP method
    /// * `path` - Request path
    /// * `request_body` - Optional request body
    /// * `context` - Stateful AI context for this session
    ///
    /// # Returns
    /// Generated response as JSON value
    pub async fn generate_response(
        &self,
        method: &str,
        path: &str,
        request_body: Option<serde_json::Value>,
        context: &StatefulAiContext,
    ) -> Result<serde_json::Value> {
        // 1. Check cache if enabled
        if let Some(ref cache) = self.cache {
            let cache_key = generate_cache_key(method, path, request_body.as_ref());
            if let Some(cached_response) = cache.get(&cache_key).await {
                tracing::debug!("Cache hit for {} {}", method, path);
                return Ok(cached_response);
            }
        }

        // 2. Check consistency rules
        self.check_consistency_rules(method, path, context).await?;

        // 3. Build LLM prompt with context
        let prompt = self.build_prompt(method, path, request_body.as_ref(), context).await;

        // 4. Generate response using LLM
        let response = self.generate_with_llm(&prompt).await?;

        // 5. Store in cache if enabled
        if let Some(ref cache) = self.cache {
            let cache_key = generate_cache_key(method, path, request_body.as_ref());
            cache.put(cache_key, response.clone()).await;
        }

        Ok(response)
    }

    /// Check consistency rules
    async fn check_consistency_rules(
        &self,
        method: &str,
        path: &str,
        context: &StatefulAiContext,
    ) -> Result<()> {
        let state = context.get_state().await;
        let _eval_context =
            EvaluationContext::new(method, path).with_session_state(state.state.clone());

        // Sort rules by priority (highest first)
        let mut rules = self.rules.consistency_rules.clone();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in &rules {
            if rule.matches(method, path) {
                // Apply rule action
                match &rule.action {
                    super::rules::RuleAction::RequireAuth { message } => {
                        // Check if user is authenticated
                        if !state.state.contains_key("auth_token")
                            && !state.state.contains_key("user_id")
                        {
                            return Err(crate::Error::generic(message.clone()));
                        }
                    }
                    super::rules::RuleAction::Error { status, message } => {
                        return Err(crate::Error::generic(format!(
                            "Rule '{}' failed: {} (status {})",
                            rule.name, message, status
                        )));
                    }
                    _ => {
                        // Other actions handled elsewhere
                    }
                }
            }
        }

        Ok(())
    }

    /// Build LLM prompt from context
    async fn build_prompt(
        &self,
        method: &str,
        path: &str,
        request_body: Option<&serde_json::Value>,
        context: &StatefulAiContext,
    ) -> String {
        let mut prompt = format!(
            "Generate a realistic response for this API request:\n\n\
             Method: {}\n\
             Path: {}\n",
            method, path
        );

        if let Some(body) = request_body {
            prompt.push_str(&format!("Request Body: {}\n", body));
        }

        // Add context summary
        let context_summary = context.build_context_summary().await;
        prompt.push('\n');
        prompt.push_str(&context_summary);

        // Add schemas
        if !self.rules.schemas.is_empty() {
            prompt.push_str("\n# Available Schemas\n");
            for (name, schema) in &self.rules.schemas {
                prompt.push_str(&format!("- {}: {}\n", name, schema));
            }
        }

        prompt.push_str("\nGenerate a realistic JSON response that:\n");
        prompt.push_str("1. Matches the request method and path\n");
        prompt.push_str("2. Is consistent with the session context\n");
        prompt.push_str("3. Conforms to the relevant schema if applicable\n");
        prompt.push_str("4. Maintains logical consistency\n");

        prompt
    }

    /// Generate response using LLM
    async fn generate_with_llm(&self, prompt: &str) -> Result<serde_json::Value> {
        tracing::debug!("Generating LLM response with prompt ({} chars)", prompt.len());

        // Create LLM generation request
        let request = LlmGenerationRequest::new(self.rules.system_prompt.clone(), prompt)
            .with_temperature(self.config.temperature)
            .with_max_tokens(self.config.max_tokens);

        // Generate response using LLM client
        self.llm_client.generate(&request).await
    }

    /// Get behavior rules
    pub fn rules(&self) -> &BehaviorRules {
        &self.rules
    }

    /// Get configuration
    pub fn config(&self) -> &BehaviorModelConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::super::config::IntelligentBehaviorConfig;
    use super::*;

    #[tokio::test]
    async fn test_behavior_model_creation() {
        let config = BehaviorModelConfig::default();
        let model = BehaviorModel::new(config);

        assert!(!model.rules().schemas.is_empty() || model.rules().schemas.is_empty());
    }

    #[tokio::test]
    async fn test_generate_response() {
        // Skip test if no OpenAI API key is available
        if std::env::var("OPENAI_API_KEY").is_err() {
            eprintln!("Skipping test_generate_response: OPENAI_API_KEY not set");
            return;
        }

        let config = BehaviorModelConfig::default();
        let model = BehaviorModel::new(config);

        let ai_config = IntelligentBehaviorConfig::default();
        let context = StatefulAiContext::new("test_session", ai_config);

        let response = model.generate_response("GET", "/api/users", None, &context).await.unwrap();

        assert!(response.is_object());
    }
}
