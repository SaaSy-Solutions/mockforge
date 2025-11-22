//! Organization-level AI controls service
//!
//! This module provides functionality to manage org-level AI controls including
//! budgets, rate limits, and feature toggles. Supports YAML defaults with DB
//! authoritative overrides (DB overrides YAML).

use crate::ai_studio::budget_manager::AiFeature;
use crate::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Organization AI controls configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OrgAiControlsConfig {
    /// Budget configuration
    pub budgets: OrgBudgetConfig,
    /// Rate limit configuration
    pub rate_limits: OrgRateLimitConfig,
    /// Feature toggles
    pub feature_toggles: HashMap<String, bool>,
}

impl Default for OrgAiControlsConfig {
    fn default() -> Self {
        Self {
            budgets: OrgBudgetConfig::default(),
            rate_limits: OrgRateLimitConfig::default(),
            feature_toggles: HashMap::from([
                ("mock_generation".to_string(), true),
                ("contract_diff".to_string(), true),
                ("persona_generation".to_string(), true),
                ("free_form_generation".to_string(), true),
                ("debug_analysis".to_string(), true),
            ]),
        }
    }
}

/// Organization budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OrgBudgetConfig {
    /// Maximum tokens per period
    pub max_tokens_per_period: u64,
    /// Period type (day, week, month, year)
    pub period_type: String,
    /// Maximum AI calls per period
    pub max_calls_per_period: u64,
}

impl Default for OrgBudgetConfig {
    fn default() -> Self {
        Self {
            max_tokens_per_period: 1_000_000,
            period_type: "month".to_string(),
            max_calls_per_period: 10_000,
        }
    }
}

/// Organization rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OrgRateLimitConfig {
    /// Rate limit per minute
    pub rate_limit_per_minute: u64,
    /// Optional rate limit per hour
    pub rate_limit_per_hour: Option<u64>,
    /// Optional rate limit per day
    pub rate_limit_per_day: Option<u64>,
}

impl Default for OrgRateLimitConfig {
    fn default() -> Self {
        Self {
            rate_limit_per_minute: 100,
            rate_limit_per_hour: None,
            rate_limit_per_day: None,
        }
    }
}

/// Trait for accessing org controls from database
#[async_trait]
pub trait OrgControlsAccessor: Send + Sync {
    /// Load org controls configuration
    async fn load_org_config(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<Option<OrgAiControlsConfig>>;

    /// Check if budget allows the request
    async fn check_budget(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
        estimated_tokens: u64,
    ) -> Result<BudgetCheckResult>;

    /// Check if rate limit allows the request
    async fn check_rate_limit(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<RateLimitCheckResult>;

    /// Check if a feature is enabled
    async fn is_feature_enabled(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
        feature: &str,
    ) -> Result<bool>;

    /// Record usage for audit
    async fn record_usage(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
        user_id: Option<&str>,
        feature: AiFeature,
        tokens: u64,
        cost_usd: f64,
        metadata: Option<serde_json::Value>,
    ) -> Result<()>;
}

/// Result of budget check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCheckResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Current tokens used in period
    pub current_tokens: u64,
    /// Maximum tokens allowed in period
    pub max_tokens: u64,
    /// Current calls used in period
    pub current_calls: u64,
    /// Maximum calls allowed in period
    pub max_calls: u64,
    /// Period start timestamp
    pub period_start: Option<DateTime<Utc>>,
    /// Reason if not allowed
    pub reason: Option<String>,
}

/// Result of rate limit check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitCheckResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Current requests in the current window
    pub current_requests: u64,
    /// Maximum requests allowed in the window
    pub max_requests: u64,
    /// Window type (minute, hour, day)
    pub window_type: String,
    /// Retry after timestamp (if rate limited)
    pub retry_after: Option<DateTime<Utc>>,
    /// Reason if not allowed
    pub reason: Option<String>,
}

/// Organization controls service
///
/// Manages org-level AI controls with YAML defaults and DB authoritative overrides.
/// DB values override YAML defaults when both are present.
pub struct OrgControls {
    /// YAML-based default configuration
    yaml_config: OrgAiControlsConfig,
    /// Optional database accessor (if available)
    db_accessor: Option<Box<dyn OrgControlsAccessor>>,
}

impl OrgControls {
    /// Create a new org controls service with YAML defaults only
    pub fn new(yaml_config: OrgAiControlsConfig) -> Self {
        Self {
            yaml_config,
            db_accessor: None,
        }
    }

    /// Create a new org controls service with YAML defaults and DB accessor
    pub fn with_db_accessor(
        yaml_config: OrgAiControlsConfig,
        db_accessor: Box<dyn OrgControlsAccessor>,
    ) -> Self {
        Self {
            yaml_config,
            db_accessor: Some(db_accessor),
        }
    }

    /// Load org configuration (DB overrides YAML)
    pub async fn load_org_config(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<OrgAiControlsConfig> {
        // Try to load from DB first
        if let Some(ref accessor) = self.db_accessor {
            if let Some(db_config) = accessor.load_org_config(org_id, workspace_id).await? {
                // Merge DB config with YAML defaults (DB values take precedence)
                return Ok(self.merge_configs(self.yaml_config.clone(), db_config));
            }
        }

        // Fall back to YAML config
        Ok(self.yaml_config.clone())
    }

    /// Check if budget allows the request
    pub async fn check_budget(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
        estimated_tokens: u64,
    ) -> Result<BudgetCheckResult> {
        // Check DB first if available
        if let Some(ref accessor) = self.db_accessor {
            return accessor.check_budget(org_id, workspace_id, estimated_tokens).await;
        }

        // Fall back to YAML config check (simplified - no period tracking)
        let config = self.load_org_config(org_id, workspace_id).await?;
        Ok(BudgetCheckResult {
            allowed: true, // YAML-only mode: always allow (no enforcement)
            current_tokens: 0,
            max_tokens: config.budgets.max_tokens_per_period,
            current_calls: 0,
            max_calls: config.budgets.max_calls_per_period,
            period_start: None,
            reason: None,
        })
    }

    /// Check if rate limit allows the request
    pub async fn check_rate_limit(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<RateLimitCheckResult> {
        // Check DB first if available
        if let Some(ref accessor) = self.db_accessor {
            return accessor.check_rate_limit(org_id, workspace_id).await;
        }

        // Fall back to YAML config check (simplified - no rate limiting)
        let config = self.load_org_config(org_id, workspace_id).await?;
        Ok(RateLimitCheckResult {
            allowed: true, // YAML-only mode: always allow (no enforcement)
            current_requests: 0,
            max_requests: config.rate_limits.rate_limit_per_minute,
            window_type: "minute".to_string(),
            retry_after: None,
            reason: None,
        })
    }

    /// Check if a feature is enabled
    pub async fn is_feature_enabled(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
        feature: &str,
    ) -> Result<bool> {
        // Check DB first if available
        if let Some(ref accessor) = self.db_accessor {
            return accessor.is_feature_enabled(org_id, workspace_id, feature).await;
        }

        // Fall back to YAML config
        let config = self.load_org_config(org_id, workspace_id).await?;
        Ok(config.feature_toggles.get(feature).copied().unwrap_or(true))
    }

    /// Record usage for audit
    pub async fn record_usage(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
        user_id: Option<&str>,
        feature: AiFeature,
        tokens: u64,
        cost_usd: f64,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        // Record in DB if available
        if let Some(ref accessor) = self.db_accessor {
            return accessor
                .record_usage(org_id, workspace_id, user_id, feature, tokens, cost_usd, metadata)
                .await;
        }

        // YAML-only mode: no recording (in-memory tracking would be handled by BudgetManager)
        Ok(())
    }

    /// Merge DB config with YAML defaults (DB values take precedence)
    fn merge_configs(&self, yaml: OrgAiControlsConfig, db: OrgAiControlsConfig) -> OrgAiControlsConfig {
        // Merge feature toggles (DB overrides YAML)
        let mut merged_toggles = yaml.feature_toggles.clone();
        for (key, value) in db.feature_toggles {
            merged_toggles.insert(key, value);
        }

        OrgAiControlsConfig {
            budgets: db.budgets, // DB budget config takes precedence
            rate_limits: db.rate_limits, // DB rate limit config takes precedence
            feature_toggles: merged_toggles,
        }
    }
}

impl Default for OrgControls {
    fn default() -> Self {
        Self::new(OrgAiControlsConfig::default())
    }
}
