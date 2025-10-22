//! AI-powered response handler for HTTP requests
//!
//! This module integrates intelligent mock generation and data drift simulation
//! into the HTTP request handling pipeline.

use mockforge_core::Result;
use mockforge_data::{
    DataDriftConfig, DataDriftEngine, IntelligentMockConfig, IntelligentMockGenerator,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// AI response handler that combines intelligent generation and drift simulation
pub struct AiResponseHandler {
    /// Intelligent mock generator (if configured)
    intelligent_generator: Option<IntelligentMockGenerator>,
    /// Data drift engine (if configured)
    drift_engine: Option<Arc<RwLock<DataDriftEngine>>>,
}

impl AiResponseHandler {
    /// Create a new AI response handler
    pub fn new(
        intelligent_config: Option<IntelligentMockConfig>,
        drift_config: Option<DataDriftConfig>,
    ) -> Result<Self> {
        debug!("Creating AI response handler");

        // Initialize intelligent generator if configured
        let intelligent_generator = if let Some(config) = intelligent_config {
            debug!("Initializing intelligent mock generator with mode: {:?}", config.mode);
            Some(IntelligentMockGenerator::new(config).map_err(|e| {
                mockforge_core::Error::Config {
                    message: format!("Failed to initialize intelligent generator: {}", e),
                }
            })?)
        } else {
            None
        };

        // Initialize drift engine if configured
        let drift_engine = if let Some(config) = drift_config {
            debug!("Initializing data drift engine");
            let engine = DataDriftEngine::new(config).map_err(|e| {
                mockforge_core::Error::Config {
                    message: format!("Failed to initialize drift engine: {}", e),
                }
            })?;
            Some(Arc::new(RwLock::new(engine)))
        } else {
            None
        };

        Ok(Self {
            intelligent_generator,
            drift_engine,
        })
    }

    /// Check if this handler has any AI features enabled
    pub fn is_enabled(&self) -> bool {
        self.intelligent_generator.is_some() || self.drift_engine.is_some()
    }

    /// Generate a response using configured AI features
    pub async fn generate_response(&mut self, base_response: Option<Value>) -> Result<Value> {
        debug!("Generating AI-powered response");

        // Step 1: Generate or use base response
        let mut response = if let Some(generator) = &mut self.intelligent_generator {
            match generator.generate().await {
                Ok(resp) => {
                    debug!("Intelligent generation successful");
                    resp
                }
                Err(e) => {
                    warn!("Intelligent generation failed: {}, using fallback", e);
                    base_response.unwrap_or_else(|| serde_json::json!({}))
                }
            }
        } else if let Some(base) = base_response {
            base
        } else {
            serde_json::json!({})
        };

        // Step 2: Apply drift if configured
        if let Some(drift_engine) = &self.drift_engine {
            match drift_engine.read().await.apply_drift(response.clone()).await {
                Ok(drifted) => {
                    debug!("Data drift applied successfully");
                    response = drifted;
                }
                Err(e) => {
                    warn!("Data drift failed: {}, using non-drifted response", e);
                }
            }
        }

        Ok(response)
    }

    /// Reset drift state (useful for testing or specific scenarios)
    pub async fn reset_drift(&self) {
        if let Some(drift_engine) = &self.drift_engine {
            drift_engine.read().await.reset().await;
            debug!("Drift state reset");
        }
    }

    /// Get drift request count
    pub async fn drift_request_count(&self) -> u64 {
        if let Some(drift_engine) = &self.drift_engine {
            drift_engine.read().await.request_count().await
        } else {
            0
        }
    }
}

/// Helper function to create an AI handler from optional configs
pub fn create_ai_handler(
    intelligent_config: Option<IntelligentMockConfig>,
    drift_config: Option<DataDriftConfig>,
) -> Result<Option<AiResponseHandler>> {
    if intelligent_config.is_some() || drift_config.is_some() {
        Ok(Some(AiResponseHandler::new(intelligent_config, drift_config)?))
    } else {
        Ok(None)
    }
}

/// Process a response body with AI features if configured
///
/// This is a helper function that checks if a response has AI configuration
/// and applies intelligent generation or drift if present.
///
/// # Arguments
/// * `response_body` - The base response body (as JSON string or Value)
/// * `intelligent_config` - Optional intelligent mock configuration (from MockResponse.intelligent)
/// * `drift_config` - Optional drift configuration (from MockResponse.drift)
///
/// # Returns
/// The processed response body as a JSON Value
pub async fn process_response_with_ai(
    response_body: Option<Value>,
    intelligent_config: Option<Value>,
    drift_config: Option<Value>,
) -> Result<Value> {
    // Parse configs if present
    let intelligent: Option<IntelligentMockConfig> =
        intelligent_config.and_then(|v| serde_json::from_value(v).ok());

    let drift: Option<DataDriftConfig> = drift_config.and_then(|v| serde_json::from_value(v).ok());

    // If no AI config, return original response
    if intelligent.is_none() && drift.is_none() {
        return Ok(response_body.unwrap_or_else(|| serde_json::json!({})));
    }

    // Create AI handler and generate response
    let mut handler = AiResponseHandler::new(intelligent, drift)?;
    handler.generate_response(response_body).await
}

/// Configuration for AI-powered responses (to be added to MockResponse)
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AiResponseConfig {
    /// Intelligent mock configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligent: Option<IntelligentMockConfig>,

    /// Data drift configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift: Option<DataDriftConfig>,
}

impl AiResponseConfig {
    /// Check if any AI features are configured
    pub fn is_enabled(&self) -> bool {
        self.intelligent.is_some() || self.drift.is_some()
    }

    /// Create an AI handler from this configuration
    pub fn create_handler(&self) -> Result<Option<AiResponseHandler>> {
        create_ai_handler(self.intelligent.clone(), self.drift.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_data::drift::{DriftRule, DriftStrategy};
    use mockforge_data::ResponseMode;

    #[test]
    fn test_ai_handler_creation_intelligent_only() {
        let config = IntelligentMockConfig::new(ResponseMode::Intelligent)
            .with_prompt("Test prompt".to_string());

        let result = AiResponseHandler::new(Some(config), None);
        assert!(result.is_ok());

        let handler = result.unwrap();
        assert!(handler.is_enabled());
        assert!(handler.intelligent_generator.is_some());
        assert!(handler.drift_engine.is_none());
    }

    #[test]
    fn test_ai_handler_creation_drift_only() {
        let rule = DriftRule::new("field".to_string(), DriftStrategy::Linear).with_rate(1.0);
        let drift_config = DataDriftConfig::new().with_rule(rule);

        let result = AiResponseHandler::new(None, Some(drift_config));
        assert!(result.is_ok());

        let handler = result.unwrap();
        assert!(handler.is_enabled());
        assert!(handler.intelligent_generator.is_none());
        assert!(handler.drift_engine.is_some());
    }

    #[test]
    fn test_ai_handler_creation_both() {
        let intelligent_config =
            IntelligentMockConfig::new(ResponseMode::Intelligent).with_prompt("Test".to_string());
        let rule = DriftRule::new("field".to_string(), DriftStrategy::Linear);
        let drift_config = DataDriftConfig::new().with_rule(rule);

        let result = AiResponseHandler::new(Some(intelligent_config), Some(drift_config));
        assert!(result.is_ok());

        let handler = result.unwrap();
        assert!(handler.is_enabled());
        assert!(handler.intelligent_generator.is_some());
        assert!(handler.drift_engine.is_some());
    }

    #[test]
    fn test_ai_handler_creation_neither() {
        let result = AiResponseHandler::new(None, None);
        assert!(result.is_ok());

        let handler = result.unwrap();
        assert!(!handler.is_enabled());
    }

    #[test]
    fn test_ai_response_config_is_enabled() {
        let config = AiResponseConfig {
            intelligent: Some(IntelligentMockConfig::new(ResponseMode::Intelligent)),
            drift: None,
        };
        assert!(config.is_enabled());

        let config = AiResponseConfig {
            intelligent: None,
            drift: None,
        };
        assert!(!config.is_enabled());
    }

    #[tokio::test]
    async fn test_generate_response_with_base() {
        let mut handler = AiResponseHandler::new(None, None).unwrap();
        let base = serde_json::json!({"test": "value"});

        let result = handler.generate_response(Some(base.clone())).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), base);
    }
}
