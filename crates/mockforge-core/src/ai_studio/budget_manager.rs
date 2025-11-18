//! Budget manager for AI usage tracking and controls
//!
//! This module provides functionality to track token usage, calculate costs,
//! and enforce budget limits. It uses in-memory tracking for local usage,
//! and can integrate with cloud usage tracking when available.

use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Budget manager for AI usage
pub struct BudgetManager {
    /// Budget configuration
    config: BudgetConfig,
    /// In-memory usage tracking (workspace_id -> usage stats)
    usage_tracker: Arc<RwLock<HashMap<String, WorkspaceUsage>>>,
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
}

impl BudgetManager {
    /// Create a new budget manager
    pub fn new(config: BudgetConfig) -> Self {
        Self {
            config,
            usage_tracker: Arc::new(RwLock::new(HashMap::new())),
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
        });

        let usage_percentage = if self.config.max_tokens_per_workspace > 0 {
            (usage.tokens_used as f64 / self.config.max_tokens_per_workspace as f64).min(1.0)
        } else {
            0.0
        };

        Ok(UsageStats {
            tokens_used: usage.tokens_used,
            cost_usd: usage.cost_usd,
            calls_made: usage.calls_made,
            budget_limit: self.config.max_tokens_per_workspace,
            usage_percentage,
        })
    }

    /// Check if request is within budget
    pub async fn check_budget(&self, workspace_id: &str, estimated_tokens: u64) -> Result<bool> {
        let tracker = self.usage_tracker.read().await;
        let usage = tracker.get(workspace_id);

        // Check token budget
        if let Some(usage) = usage {
            if self.config.max_tokens_per_workspace > 0 {
                if usage.tokens_used + estimated_tokens > self.config.max_tokens_per_workspace {
                    return Ok(false);
                }
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

    /// Record token usage and cost
    pub async fn record_usage(&self, workspace_id: &str, tokens: u64, cost_usd: f64) -> Result<()> {
        let mut tracker = self.usage_tracker.write().await;
        let usage = tracker.entry(workspace_id.to_string()).or_insert_with(|| WorkspaceUsage {
            tokens_used: 0,
            cost_usd: 0.0,
            calls_made: 0,
            last_reset: Utc::now(),
            daily_calls: HashMap::new(),
        });

        usage.tokens_used += tokens;
        usage.cost_usd += cost_usd;
        usage.calls_made += 1;

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
}
