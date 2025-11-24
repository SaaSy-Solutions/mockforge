//! Chat orchestrator for routing natural language commands
//!
//! This module provides the main entry point for processing natural language
//! commands and routing them to appropriate handlers based on intent detection.

use crate::ai_studio::budget_manager::{BudgetConfig, BudgetManager};
use crate::ai_studio::debug_analyzer::DebugRequest;
use crate::intelligent_behavior::{
    config::IntelligentBehaviorConfig, llm_client::LlmClient, LlmUsage,
};
use crate::Result;
use serde::{Deserialize, Serialize};

/// Chat request from user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// User's message/command
    pub message: String,

    /// Optional conversation context
    pub context: Option<ChatContext>,

    /// Optional workspace ID for context
    pub workspace_id: Option<String>,

    /// Optional organization ID for org-level controls
    pub org_id: Option<String>,

    /// Optional user ID for audit logging
    pub user_id: Option<String>,
}

/// Chat context for multi-turn conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContext {
    /// Conversation history
    pub history: Vec<ChatMessage>,

    /// Optional workspace ID
    #[serde(default)]
    pub workspace_id: Option<String>,
}

/// Chat message in conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role (user or assistant)
    pub role: String,

    /// Message content
    pub content: String,
}

/// Chat response from orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Detected intent
    pub intent: ChatIntent,

    /// Response message
    pub message: String,

    /// Optional structured data (e.g., generated spec, persona, etc.)
    pub data: Option<serde_json::Value>,

    /// Optional error message
    pub error: Option<String>,

    /// Token usage for this request
    pub tokens_used: Option<u64>,

    /// Estimated cost in USD
    pub cost_usd: Option<f64>,
}

/// Detected intent from user message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatIntent {
    /// Generate a mock API
    GenerateMock,

    /// Debug a test failure
    DebugTest,

    /// Generate or modify a persona
    GeneratePersona,

    /// Run contract diff analysis
    ContractDiff,

    /// General question/chat
    General,

    /// Unknown intent
    Unknown,
}

/// Chat orchestrator that routes commands to appropriate handlers
pub struct ChatOrchestrator {
    /// LLM client for intent detection and processing
    llm_client: LlmClient,

    /// Configuration
    config: IntelligentBehaviorConfig,

    /// Budget manager for tracking usage
    budget_manager: BudgetManager,
}

impl ChatOrchestrator {
    /// Create a new chat orchestrator
    pub fn new(config: IntelligentBehaviorConfig) -> Self {
        let llm_client = LlmClient::new(config.behavior_model.clone());
        let budget_config = BudgetConfig::default();
        let budget_manager = BudgetManager::new(budget_config);
        Self {
            llm_client,
            config,
            budget_manager,
        }
    }

    /// Helper to calculate cost from usage
    fn calculate_cost(&self, usage: &LlmUsage) -> f64 {
        let provider = &self.config.behavior_model.llm_provider;
        let model = &self.config.behavior_model.model;
        BudgetManager::calculate_cost(provider, model, usage.total_tokens)
    }

    /// Helper to track usage and return token/cost info
    async fn track_usage(
        &self,
        org_id: Option<&str>,
        workspace_id: &str,
        user_id: Option<&str>,
        usage: &LlmUsage,
    ) -> Result<(Option<u64>, Option<f64>)> {
        self.track_usage_with_feature(org_id, workspace_id, user_id, usage, None).await
    }

    /// Helper to track usage with feature information
    async fn track_usage_with_feature(
        &self,
        org_id: Option<&str>,
        workspace_id: &str,
        user_id: Option<&str>,
        usage: &LlmUsage,
        feature: Option<crate::ai_studio::budget_manager::AiFeature>,
    ) -> Result<(Option<u64>, Option<f64>)> {
        let cost = self.calculate_cost(usage);
        self.budget_manager
            .record_usage_with_feature(
                org_id,
                workspace_id,
                user_id,
                usage.total_tokens,
                cost,
                feature,
            )
            .await?;
        Ok((Some(usage.total_tokens), Some(cost)))
    }

    /// Process a chat request and return response
    pub async fn process(&self, request: &ChatRequest) -> Result<ChatResponse> {
        // Build message with context if available
        let message_with_context = if let Some(context) = &request.context {
            self.build_contextual_message(&request.message, context)
        } else {
            request.message.clone()
        };

        // Detect intent from message
        let intent = self.detect_intent(&message_with_context).await?;

        // Route to appropriate handler based on intent
        match intent {
            ChatIntent::GenerateMock => {
                // Use MockGenerator to generate mock from message
                use crate::ai_studio::nl_mock_generator::MockGenerator;
                let generator = MockGenerator::new();
                // For now, pass None for deterministic_config - it would need to be loaded from workspace config
                match generator
                    .generate(
                        &request.message,
                        request.workspace_id.as_deref(),
                        None, // ai_mode - could be extracted from request if available
                        None, // deterministic_config - would be loaded from workspace config
                    )
                    .await
                {
                    Ok(result) => {
                        // Estimate tokens (MockGenerator uses LLM internally, but doesn't expose usage)
                        // For now, estimate based on message length and response size
                        let estimated_tokens =
                            (request.message.len() + result.message.len()) as u64 / 4;
                        let usage = LlmUsage::new(estimated_tokens / 2, estimated_tokens / 2);
                        let (tokens, cost) = self
                            .track_usage_with_feature(
                                request.org_id.as_deref(),
                                &request.workspace_id.clone().unwrap_or_default(),
                                request.user_id.as_deref(),
                                &usage,
                                Some(crate::ai_studio::budget_manager::AiFeature::MockAi),
                            )
                            .await
                            .unwrap_or((None, None));
                        Ok(ChatResponse {
                            intent: ChatIntent::GenerateMock,
                            message: result.message,
                            data: result.spec.map(|s| {
                                serde_json::json!({
                                    "spec": s,
                                    "type": "openapi_spec"
                                })
                            }),
                            error: None,
                            tokens_used: tokens,
                            cost_usd: cost,
                        })
                    }
                    Err(e) => Ok(ChatResponse {
                        intent: ChatIntent::GenerateMock,
                        message: format!("Failed to generate mock: {}", e),
                        data: None,
                        error: Some(e.to_string()),
                        tokens_used: None,
                        cost_usd: None,
                    }),
                }
            }
            ChatIntent::DebugTest => {
                // Use DebugAnalyzer to analyze test failure
                use crate::ai_studio::debug_analyzer::DebugAnalyzer;
                let analyzer = DebugAnalyzer::new();
                let debug_request = DebugRequest {
                    test_logs: request.message.clone(),
                    test_name: None,
                    workspace_id: request.workspace_id.clone(),
                };
                match analyzer.analyze(&debug_request).await {
                    Ok(result) => {
                        // Estimate tokens (DebugAnalyzer uses LLM internally)
                        let estimated_tokens =
                            (request.message.len() + result.root_cause.len()) as u64 / 4;
                        let usage = LlmUsage::new(estimated_tokens / 2, estimated_tokens / 2);
                        let (tokens, cost) = self
                            .track_usage_with_feature(
                                request.org_id.as_deref(),
                                &request.workspace_id.clone().unwrap_or_default(),
                                request.user_id.as_deref(),
                                &usage,
                                Some(crate::ai_studio::budget_manager::AiFeature::DebugAnalysis),
                            )
                            .await
                            .unwrap_or((None, None));
                        Ok(ChatResponse {
                            intent: ChatIntent::DebugTest,
                            message: format!("Root cause: {}\n\nFound {} suggestions and {} related configurations.",
                                result.root_cause, result.suggestions.len(), result.related_configs.len()),
                            data: Some(serde_json::json!({
                                "root_cause": result.root_cause,
                                "suggestions": result.suggestions,
                                "related_configs": result.related_configs,
                                "type": "debug_analysis"
                            })),
                            error: None,
                            tokens_used: tokens,
                            cost_usd: cost,
                        })
                    }
                    Err(e) => Ok(ChatResponse {
                        intent: ChatIntent::DebugTest,
                        message: format!("Failed to analyze test failure: {}", e),
                        data: None,
                        error: Some(e.to_string()),
                        tokens_used: None,
                        cost_usd: None,
                    }),
                }
            }
            ChatIntent::GeneratePersona => {
                // Use PersonaGenerator to generate persona from message
                use crate::ai_studio::persona_generator::{
                    PersonaGenerationRequest, PersonaGenerator,
                };
                let generator = PersonaGenerator::new();
                let persona_request = PersonaGenerationRequest {
                    description: request.message.clone(),
                    base_persona_id: None,
                    workspace_id: request.workspace_id.clone(),
                };
                match generator.generate(&persona_request, None, None).await {
                    Ok(result) => {
                        // Estimate tokens (PersonaGenerator uses LLM internally)
                        let estimated_tokens =
                            (request.message.len() + result.message.len()) as u64 / 4;
                        let usage = LlmUsage::new(estimated_tokens / 2, estimated_tokens / 2);
                        let (tokens, cost) = self
                            .track_usage_with_feature(
                                request.org_id.as_deref(),
                                &request.workspace_id.clone().unwrap_or_default(),
                                request.user_id.as_deref(),
                                &usage,
                                Some(
                                    crate::ai_studio::budget_manager::AiFeature::PersonaGeneration,
                                ),
                            )
                            .await
                            .unwrap_or((None, None));
                        Ok(ChatResponse {
                            intent: ChatIntent::GeneratePersona,
                            message: result.message,
                            data: result.persona.map(|p| {
                                serde_json::json!({
                                    "persona": p,
                                    "type": "persona"
                                })
                            }),
                            error: None,
                            tokens_used: tokens,
                            cost_usd: cost,
                        })
                    }
                    Err(e) => Ok(ChatResponse {
                        intent: ChatIntent::GeneratePersona,
                        message: format!("Failed to generate persona: {}", e),
                        data: None,
                        error: Some(e.to_string()),
                        tokens_used: None,
                        cost_usd: None,
                    }),
                }
            }
            ChatIntent::ContractDiff => {
                // Use ContractDiffHandler to process the query
                use crate::ai_studio::contract_diff_handler::ContractDiffHandler;
                let handler = ContractDiffHandler::new().map_err(|e| {
                    crate::Error::generic(format!("Failed to create ContractDiffHandler: {}", e))
                })?;

                // For now, we don't have direct access to specs/requests in the orchestrator
                // The handler will provide guidance on how to use contract diff
                match handler.analyze_from_query(&request.message, None, None).await {
                    Ok(query_result) => {
                        let mut message = query_result.summary.clone();
                        if let Some(link) = &query_result.link_to_viewer {
                            message.push_str(&format!("\n\nView details: {}", link));
                        }

                        Ok(ChatResponse {
                            intent: ChatIntent::ContractDiff,
                            message,
                            data: Some(serde_json::json!({
                                "type": "contract_diff_query",
                                "intent": query_result.intent,
                                "result": query_result.result,
                                "breaking_changes": query_result.breaking_changes,
                                "link_to_viewer": query_result.link_to_viewer,
                            })),
                            error: None,
                            tokens_used: None,
                            cost_usd: None,
                        })
                    }
                    Err(e) => Ok(ChatResponse {
                        intent: ChatIntent::ContractDiff,
                        message: format!("I can help with contract diff analysis! Try asking:\n- \"Analyze the last captured request\"\n- \"Show me breaking changes\"\n- \"Compare contract versions\"\n\nError: {}", e),
                        data: Some(serde_json::json!({
                            "type": "contract_diff_info",
                            "endpoints": {
                                "analyze": "/api/v1/contract-diff/analyze",
                                "capture": "/api/v1/contract-diff/capture",
                                "compare": "/api/v1/contract-diff/compare"
                            }
                        })),
                        error: Some(e.to_string()),
                        tokens_used: None,
                        cost_usd: None,
                    }),
                }
            }
            ChatIntent::General | ChatIntent::Unknown => {
                // General chat response
                Ok(ChatResponse {
                    intent: ChatIntent::General,
                    message: "I'm here to help! You can ask me to generate mocks, debug tests, create personas, or analyze contracts.".to_string(),
                    data: None,
                    error: None,
                    tokens_used: None,
                    cost_usd: None,
                })
            }
        }
    }

    /// Build contextual message from conversation history
    fn build_contextual_message(&self, current_message: &str, context: &ChatContext) -> String {
        if context.history.is_empty() {
            return current_message.to_string();
        }

        let mut contextual = String::from("Previous conversation:\n");
        for msg in &context.history {
            contextual.push_str(&format!("{}: {}\n", msg.role, msg.content));
        }
        contextual.push_str(&format!("\nCurrent message: {}", current_message));
        contextual
    }

    /// Detect intent from user message using LLM
    async fn detect_intent(&self, message: &str) -> Result<ChatIntent> {
        // Use simple keyword matching for now (can be enhanced with LLM)
        let message_lower = message.to_lowercase();

        if message_lower.contains("create")
            && (message_lower.contains("api") || message_lower.contains("mock"))
        {
            return Ok(ChatIntent::GenerateMock);
        }

        if message_lower.contains("debug")
            || message_lower.contains("test") && message_lower.contains("fail")
        {
            return Ok(ChatIntent::DebugTest);
        }

        if message_lower.contains("persona") {
            return Ok(ChatIntent::GeneratePersona);
        }

        if message_lower.contains("contract") || message_lower.contains("diff") {
            return Ok(ChatIntent::ContractDiff);
        }

        // Default to general for now
        Ok(ChatIntent::General)
    }
}
