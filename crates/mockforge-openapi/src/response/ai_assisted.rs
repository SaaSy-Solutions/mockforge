//! AI-assisted response generation for MockForge
//!
//! Contains the `generate_ai_response` method and related helpers.

use super::*;

impl ResponseGenerator {
    /// Generate an AI-assisted response using LLM
    ///
    /// This method generates a dynamic response based on request context
    /// using the configured LLM provider (OpenAI, Anthropic, etc.)
    ///
    /// # Arguments
    /// * `ai_config` - The AI response configuration
    /// * `context` - The request context for prompt expansion
    /// * `generator` - Optional AI generator implementation (if None, returns placeholder)
    ///
    /// # Returns
    /// A JSON value containing the generated response
    pub async fn generate_ai_response(
        ai_config: &AiResponseConfig,
        context: &RequestContext,
        generator: Option<&dyn AiGenerator>,
    ) -> Result<Value> {
        // Get the prompt template and expand it with request context
        let prompt_template = ai_config
            .prompt
            .as_ref()
            .ok_or_else(|| mockforge_foundation::error::Error::internal("AI prompt is required"))?;

        // Note: expand_prompt_template is now in mockforge-template-expansion crate
        // For now, we'll do a simple string replacement as a fallback
        // In the future, this should be refactored to use the template expansion crate
        let expanded_prompt = prompt_template
            .replace("{{method}}", &context.method)
            .replace("{{path}}", &context.path);

        tracing::info!("AI response generation requested with prompt: {}", expanded_prompt);

        // Use the provided generator if available
        if let Some(gen) = generator {
            tracing::debug!("Using provided AI generator for response");
            return gen.generate(&expanded_prompt, ai_config).await;
        }

        // No generator available — return an error so callers know AI is not configured
        tracing::warn!(
            "No AI generator provided; configure MOCKFORGE_AI_PROVIDER to enable AI responses"
        );
        Err(mockforge_foundation::error::Error::internal(
            "AI response generation is not available: no AI generator configured. \
             Set MOCKFORGE_AI_PROVIDER and MOCKFORGE_AI_API_KEY environment variables to enable AI-assisted responses.",
        ))
    }
}
