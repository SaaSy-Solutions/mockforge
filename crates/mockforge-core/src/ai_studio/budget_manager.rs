//! Budget manager for AI usage tracking and controls
//!
//! This module provides functionality to track token usage, calculate costs,
//! and enforce budget limits. It uses in-memory tracking for local usage,
//! and can integrate with cloud usage tracking when available.

use crate::ai_studio::org_controls::OrgControls;
use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Budget manager for AI usage
pub struct BudgetManager {
    /// Budget configuration (workspace-level defaults)
    config: BudgetConfig,
    /// In-memory usage tracking (workspace_id -> usage stats)
    usage_tracker: Arc<RwLock<HashMap<String, WorkspaceUsage>>>,
    /// Optional org controls for org-level enforcement
    org_controls: Option<Arc<OrgControls>>,
}

/// AI feature types for tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiFeature {
    /// MockAI - Natural language mock generation
    MockAi,
    /// AI Contract Diff - Contract analysis and recommendations
    ContractDiff,
    /// Persona Generation - AI-generated personas
    PersonaGeneration,
    /// Debug Analysis - AI-guided debugging
    DebugAnalysis,
    /// Generative Schema - Schema generation from examples
    GenerativeSchema,
    /// Voice/LLM Interface - Voice commands and chat
    VoiceInterface,
    /// General chat/assistant
    GeneralChat,
}

impl AiFeature {
    /// Get display name for the feature
    pub fn display_name(&self) -> &'static str {
        match self {
            AiFeature::MockAi => "MockAI",
            AiFeature::ContractDiff => "Contract Diff",
            AiFeature::PersonaGeneration => "Persona Generation",
            AiFeature::DebugAnalysis => "Debug Analysis",
            AiFeature::GenerativeSchema => "Generative Schema",
            AiFeature::VoiceInterface => "Voice Interface",
            AiFeature::GeneralChat => "General Chat",
        }
    }
}

/// Per-feature usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureUsage {
    /// Tokens used by this feature
    pub tokens_used: u64,
    /// Cost in USD for this feature
    pub cost_usd: f64,
    /// Number of calls made for this feature
    pub calls_made: u64,
}

/// Per-workspace usage tracking
#[derive(Debug, Clone)]
struct WorkspaceUsage {
    /// Total tokens used
    tokens_used: u64,
    /// Total cost in USD
    cost_usd: f64,
    /// Number of AI calls made
    calls_made: u64,
    /// Last reset time
    last_reset: DateTime<Utc>,
    /// Per-day call tracking (for rate limiting)
    daily_calls: HashMap<chrono::NaiveDate, u64>,
    /// Per-feature usage tracking
    feature_usage: HashMap<AiFeature, FeatureUsage>,
}

impl BudgetManager {
    /// Create a new budget manager
    pub fn new(config: BudgetConfig) -> Self {
        Self {
            config,
            usage_tracker: Arc::new(RwLock::new(HashMap::new())),
            org_controls: None,
        }
    }

    /// Create a new budget manager with org controls
    pub fn with_org_controls(config: BudgetConfig, org_controls: Arc<OrgControls>) -> Self {
        Self {
            config,
            usage_tracker: Arc::new(RwLock::new(HashMap::new())),
            org_controls: Some(org_controls),
        }
    }

    /// Get usage statistics for a workspace
    pub async fn get_usage(&self, workspace_id: &str) -> Result<UsageStats> {
        let tracker = self.usage_tracker.read().await;
        let usage = tracker.get(workspace_id).cloned().unwrap_or_else(|| WorkspaceUsage {
            tokens_used: 0,
            cost_usd: 0.0,
            calls_made: 0,
            last_reset: Utc::now(),
            daily_calls: HashMap::new(),
            feature_usage: HashMap::new(),
        });

        let usage_percentage = if self.config.max_tokens_per_workspace > 0 {
            (usage.tokens_used as f64 / self.config.max_tokens_per_workspace as f64).min(1.0)
        } else {
            0.0
        };

        // Convert feature usage to serializable format
        let feature_breakdown: HashMap<String, FeatureUsage> = usage
            .feature_usage
            .iter()
            .map(|(feature, usage)| (format!("{:?}", feature), usage.clone()))
            .collect();

        Ok(UsageStats {
            tokens_used: usage.tokens_used,
            cost_usd: usage.cost_usd,
            calls_made: usage.calls_made,
            budget_limit: self.config.max_tokens_per_workspace,
            usage_percentage,
            feature_breakdown: Some(feature_breakdown),
        })
    }

    /// Check if request is within budget
    ///
    /// Checks org-level limits first (if available), then workspace-level limits.
    /// Org-level limits take precedence.
    pub async fn check_budget(
        &self,
        org_id: Option<&str>,
        workspace_id: &str,
        estimated_tokens: u64,
    ) -> Result<bool> {
        // Check org-level budget first (if available)
        if let (Some(org_id), Some(ref org_controls)) = (org_id, &self.org_controls) {
            let budget_result =
                org_controls.check_budget(org_id, Some(workspace_id), estimated_tokens).await?;
            if !budget_result.allowed {
                return Ok(false);
            }
        }

        // Check workspace-level budget
        let tracker = self.usage_tracker.read().await;
        let usage = tracker.get(workspace_id);

        // Check token budget
        if let Some(usage) = usage {
            if self.config.max_tokens_per_workspace > 0
                && usage.tokens_used + estimated_tokens > self.config.max_tokens_per_workspace
            {
                return Ok(false);
            }
        }

        // Check daily call limit
        let today = Utc::now().date_naive();
        if let Some(usage) = usage {
            let today_calls = usage.daily_calls.get(&today).copied().unwrap_or(0);
            if today_calls >= self.config.max_ai_calls_per_day {
                return Ok(false);
            }
        }

        // Check rate limit (per minute)
        // Note: This is a simplified check - in production, you'd want more sophisticated rate limiting
        Ok(true)
    }

    /// Check rate limit (org-level first, then workspace-level)
    pub async fn check_rate_limit(&self, org_id: Option<&str>, workspace_id: &str) -> Result<bool> {
        // Check org-level rate limit first (if available)
        if let (Some(org_id), Some(ref org_controls)) = (org_id, &self.org_controls) {
            let rate_limit_result =
                org_controls.check_rate_limit(org_id, Some(workspace_id)).await?;
            if !rate_limit_result.allowed {
                return Ok(false);
            }
        }

        // Workspace-level rate limiting would be handled here if needed
        // For now, we rely on org-level rate limiting
        Ok(true)
    }

    /// Check if a feature is enabled (org-level first, then defaults to true)
    pub async fn is_feature_enabled(
        &self,
        org_id: Option<&str>,
        workspace_id: &str,
        feature: &str,
    ) -> Result<bool> {
        // Check org-level feature toggle first (if available)
        if let (Some(org_id), Some(ref org_controls)) = (org_id, &self.org_controls) {
            return org_controls.is_feature_enabled(org_id, Some(workspace_id), feature).await;
        }

        // Default to enabled if no org controls
        Ok(true)
    }

    /// Record token usage and cost
    pub async fn record_usage(
        &self,
        org_id: Option<&str>,
        workspace_id: &str,
        user_id: Option<&str>,
        tokens: u64,
        cost_usd: f64,
    ) -> Result<()> {
        self.record_usage_with_feature(org_id, workspace_id, user_id, tokens, cost_usd, None)
            .await
    }

    /// Record token usage and cost with feature tracking
    ///
    /// Records usage both in-memory (workspace-level) and in org controls (if available).
    pub async fn record_usage_with_feature(
        &self,
        org_id: Option<&str>,
        workspace_id: &str,
        user_id: Option<&str>,
        tokens: u64,
        cost_usd: f64,
        feature: Option<AiFeature>,
    ) -> Result<()> {
        // Record in org controls (if available) for audit log
        if let (Some(org_id), Some(ref org_controls)) = (org_id, &self.org_controls) {
            if let Some(feature) = feature {
                let _feature_name = match feature {
                    AiFeature::MockAi => "mock_generation",
                    AiFeature::ContractDiff => "contract_diff",
                    AiFeature::PersonaGeneration => "persona_generation",
                    AiFeature::DebugAnalysis => "debug_analysis",
                    AiFeature::GenerativeSchema => "generative_schema",
                    AiFeature::VoiceInterface => "voice_interface",
                    AiFeature::GeneralChat => "free_form_generation",
                };
                org_controls
                    .record_usage(
                        org_id,
                        Some(workspace_id),
                        user_id,
                        feature,
                        tokens,
                        cost_usd,
                        None,
                    )
                    .await?;
            }
        }

        // Record in-memory (workspace-level tracking)
        let mut tracker = self.usage_tracker.write().await;
        let usage = tracker.entry(workspace_id.to_string()).or_insert_with(|| WorkspaceUsage {
            tokens_used: 0,
            cost_usd: 0.0,
            calls_made: 0,
            last_reset: Utc::now(),
            daily_calls: HashMap::new(),
            feature_usage: HashMap::new(),
        });

        usage.tokens_used += tokens;
        usage.cost_usd += cost_usd;
        usage.calls_made += 1;

        // Track per-feature usage
        if let Some(feature) = feature {
            let feature_usage =
                usage.feature_usage.entry(feature).or_insert_with(FeatureUsage::default);
            feature_usage.tokens_used += tokens;
            feature_usage.cost_usd += cost_usd;
            feature_usage.calls_made += 1;
        }

        // Track daily calls
        let today = Utc::now().date_naive();
        *usage.daily_calls.entry(today).or_insert(0) += 1;

        Ok(())
    }

    /// Reset usage for a workspace (useful for testing or monthly resets)
    pub async fn reset_usage(&self, workspace_id: &str) -> Result<()> {
        let mut tracker = self.usage_tracker.write().await;
        tracker.remove(workspace_id);
        Ok(())
    }

    /// Calculate cost based on provider and tokens
    ///
    /// Uses approximate pricing for common providers:
    /// - OpenAI GPT-3.5: ~$0.002 per 1K tokens
    /// - OpenAI GPT-4: ~$0.03 per 1K tokens
    /// - Anthropic Claude: ~$0.008 per 1K tokens
    /// - Ollama: $0 (local)
    pub fn calculate_cost(provider: &str, model: &str, tokens: u64) -> f64 {
        let tokens_k = tokens as f64 / 1000.0;

        // Approximate pricing per 1K tokens
        let price_per_1k = if provider.to_lowercase() == "ollama" {
            0.0 // Free local models
        } else if model.contains("gpt-4") {
            0.03 // GPT-4 pricing
        } else if model.contains("gpt-3.5") || model.contains("gpt-3") {
            0.002 // GPT-3.5 pricing
        } else if provider.to_lowercase() == "anthropic" {
            0.008 // Claude pricing
        } else {
            0.002 // Default to GPT-3.5 pricing
        };

        tokens_k * price_per_1k
    }
}

/// Budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Maximum tokens per workspace
    pub max_tokens_per_workspace: u64,

    /// Maximum AI calls per day
    pub max_ai_calls_per_day: u64,

    /// Rate limit per minute
    pub rate_limit_per_minute: u64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_tokens_per_workspace: 100_000,
            max_ai_calls_per_day: 1_000,
            rate_limit_per_minute: 10,
        }
    }
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// Total tokens used
    pub tokens_used: u64,

    /// Total cost in USD
    pub cost_usd: f64,

    /// Number of AI calls made
    pub calls_made: u64,

    /// Budget limit
    pub budget_limit: u64,

    /// Usage percentage (0.0 to 1.0)
    pub usage_percentage: f64,

    /// Per-feature usage breakdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_breakdown: Option<HashMap<String, FeatureUsage>>,
}
