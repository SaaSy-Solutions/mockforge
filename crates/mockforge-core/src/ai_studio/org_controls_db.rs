//! Database-backed implementation of OrgControlsAccessor
//!
//! This module provides a PostgreSQL implementation of OrgControlsAccessor
//! for storing and retrieving org-level AI controls from the database.
//!
//! Requires the `database` feature to be enabled.

#[cfg(feature = "database")]
use crate::ai_studio::budget_manager::AiFeature;
#[cfg(feature = "database")]
use crate::ai_studio::org_controls::{
    BudgetCheckResult, OrgAiControlsConfig, OrgBudgetConfig, OrgControlsAccessor, OrgRateLimitConfig,
    RateLimitCheckResult,
};
#[cfg(feature = "database")]
use crate::Result;
#[cfg(feature = "database")]
use async_trait::async_trait;
#[cfg(feature = "database")]
use chrono::{DateTime, Utc};
#[cfg(feature = "database")]
use serde_json::Value;
#[cfg(feature = "database")]
use sqlx::{PgPool, Row};
#[cfg(feature = "database")]
use std::collections::HashMap;
#[cfg(feature = "database")]
use uuid::Uuid;

/// Database-backed org controls accessor
#[cfg(feature = "database")]
pub struct DbOrgControls {
    /// PostgreSQL connection pool
    pool: PgPool,
}

#[cfg(feature = "database")]
impl DbOrgControls {
    /// Create a new database-backed org controls accessor
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "database")]
#[async_trait]
impl OrgControlsAccessor for DbOrgControls {
    /// Load org controls configuration from database
    async fn load_org_config(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<Option<OrgAiControlsConfig>> {
        let org_uuid = Uuid::parse_str(org_id)
            .map_err(|e| crate::Error::generic(format!("Invalid org_id: {}", e)))?;
        let workspace_uuid = workspace_id
            .and_then(|w| Uuid::parse_str(w).ok());

        // Load budget config
        let budget = if let Some(ws_uuid) = workspace_uuid {
            sqlx::query_as::<_, BudgetRow>(
                "SELECT * FROM org_ai_budgets WHERE org_id = $1 AND workspace_id = $2"
            )
            .bind(org_uuid)
            .bind(ws_uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| crate::Error::generic(format!("Failed to load budget: {}", e)))?
        } else {
            sqlx::query_as::<_, BudgetRow>(
                "SELECT * FROM org_ai_budgets WHERE org_id = $1 AND workspace_id IS NULL"
            )
            .bind(org_uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| crate::Error::generic(format!("Failed to load budget: {}", e)))?
        };

        // Load rate limit config
        let rate_limit = if let Some(ws_uuid) = workspace_uuid {
            sqlx::query_as::<_, RateLimitRow>(
                "SELECT * FROM org_ai_rate_limits WHERE org_id = $1 AND workspace_id = $2"
            )
            .bind(org_uuid)
            .bind(ws_uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| crate::Error::generic(format!("Failed to load rate limit: {}", e)))?
        } else {
            sqlx::query_as::<_, RateLimitRow>(
                "SELECT * FROM org_ai_rate_limits WHERE org_id = $1 AND workspace_id IS NULL"
            )
            .bind(org_uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| crate::Error::generic(format!("Failed to load rate limit: {}", e)))?
        };

        // Load feature toggles
        let toggles_query = if let Some(ws_uuid) = workspace_uuid {
            sqlx::query_as::<_, FeatureToggleRow>(
                "SELECT * FROM org_ai_feature_toggles WHERE org_id = $1 AND workspace_id = $2"
            )
            .bind(org_uuid)
            .bind(ws_uuid)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, FeatureToggleRow>(
                "SELECT * FROM org_ai_feature_toggles WHERE org_id = $1 AND workspace_id IS NULL"
            )
            .bind(org_uuid)
            .fetch_all(&self.pool)
            .await
        };

        let toggles = toggles_query
            .map_err(|e| crate::Error::generic(format!("Failed to load feature toggles: {}", e)))?;

        // If no config found, return None
        if budget.is_none() && rate_limit.is_none() && toggles.is_empty() {
            return Ok(None);
        }

        // Build config from database rows
        let budget_config = budget.map(|b| OrgBudgetConfig {
            max_tokens_per_period: b.max_tokens_per_period as u64,
            period_type: b.period_type,
            max_calls_per_period: b.max_calls_per_period as u64,
        }).unwrap_or_default();

        let rate_limit_config = rate_limit.map(|r| OrgRateLimitConfig {
            rate_limit_per_minute: r.rate_limit_per_minute as u64,
            rate_limit_per_hour: r.rate_limit_per_hour.map(|v| v as u64),
            rate_limit_per_day: r.rate_limit_per_day.map(|v| v as u64),
        }).unwrap_or_default();

        let mut feature_toggles = HashMap::new();
        for toggle in toggles {
            feature_toggles.insert(toggle.feature_name, toggle.enabled);
        }

        Ok(Some(OrgAiControlsConfig {
            budgets: budget_config,
            rate_limits: rate_limit_config,
            feature_toggles,
        }))
    }

    /// Check if budget allows the request
    async fn check_budget(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
        estimated_tokens: u64,
    ) -> Result<BudgetCheckResult> {
        let org_uuid = Uuid::parse_str(org_id)
            .map_err(|e| crate::Error::generic(format!("Invalid org_id: {}", e)))?;
        let workspace_uuid = workspace_id
            .and_then(|w| Uuid::parse_str(w).ok());

        // Get current budget
        let budget = if let Some(ws_uuid) = workspace_uuid {
            sqlx::query_as::<_, BudgetRow>(
                "SELECT * FROM org_ai_budgets WHERE org_id = $1 AND workspace_id = $2"
            )
            .bind(org_uuid)
            .bind(ws_uuid)
            .fetch_optional(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, BudgetRow>(
                "SELECT * FROM org_ai_budgets WHERE org_id = $1 AND workspace_id IS NULL"
            )
            .bind(org_uuid)
            .fetch_optional(&self.pool)
            .await
        };

        let budget = budget
            .map_err(|e| crate::Error::generic(format!("Failed to check budget: {}", e)))?;

        if let Some(b) = budget {
            // Check if period has expired and reset if needed
            let now = Utc::now();
            let period_start = b.period_start;
            let period_expired = match b.period_type.as_str() {
                "day" => (now - period_start).num_days() >= 1,
                "week" => (now - period_start).num_weeks() >= 1,
                "month" => (now - period_start).num_days() >= 30,
                "year" => (now - period_start).num_days() >= 365,
                _ => false,
            };

            let (current_tokens, current_calls, period_start_actual) = if period_expired {
                // Reset period
                let new_period_start = now;
                sqlx::query(
                    "UPDATE org_ai_budgets SET current_tokens_used = 0, current_calls_used = 0, period_start = $1 WHERE id = $2"
                )
                .bind(new_period_start)
                .bind(b.id)
                .execute(&self.pool)
                .await
                .map_err(|e| crate::Error::generic(format!("Failed to reset budget period: {}", e)))?;
                (0u64, 0u64, Some(new_period_start))
            } else {
                (b.current_tokens_used as u64, b.current_calls_used as u64, Some(period_start))
            };

            // Check limits
            let tokens_allowed = current_tokens + estimated_tokens <= b.max_tokens_per_period as u64;
            let calls_allowed = current_calls < b.max_calls_per_period as u64;
            let allowed = tokens_allowed && calls_allowed;

            Ok(BudgetCheckResult {
                allowed,
                current_tokens,
                max_tokens: b.max_tokens_per_period as u64,
                current_calls,
                max_calls: b.max_calls_per_period as u64,
                period_start: period_start_actual,
                reason: if !allowed {
                    Some(if !tokens_allowed {
                        "Token budget exceeded".to_string()
                    } else {
                        "Call limit exceeded".to_string()
                    })
                } else {
                    None
                },
            })
        } else {
            // No budget configured - allow
            Ok(BudgetCheckResult {
                allowed: true,
                current_tokens: 0,
                max_tokens: 0,
                current_calls: 0,
                max_calls: 0,
                period_start: None,
                reason: None,
            })
        }
    }

    /// Check if rate limit allows the request
    async fn check_rate_limit(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<RateLimitCheckResult> {
        let org_uuid = Uuid::parse_str(org_id)
            .map_err(|e| crate::Error::generic(format!("Invalid org_id: {}", e)))?;
        let workspace_uuid = workspace_id
            .and_then(|w| Uuid::parse_str(w).ok());

        // Load rate limit configuration
        let rate_limit_config = if let Some(ws_uuid) = workspace_uuid {
            sqlx::query_as::<_, RateLimitRow>(
                "SELECT * FROM org_ai_rate_limits WHERE org_id = $1 AND workspace_id = $2"
            )
            .bind(org_uuid)
            .bind(ws_uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| crate::Error::generic(format!("Failed to load rate limit: {}", e)))?
        } else {
            sqlx::query_as::<_, RateLimitRow>(
                "SELECT * FROM org_ai_rate_limits WHERE org_id = $1 AND workspace_id IS NULL"
            )
            .bind(org_uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| crate::Error::generic(format!("Failed to load rate limit: {}", e)))?
        };

        // If no rate limit config, allow the request
        let config = match rate_limit_config {
            Some(row) => row,
            None => {
                return Ok(RateLimitCheckResult {
                    allowed: true,
                    current_requests: 0,
                    max_requests: 1000, // Default high limit
                    window_type: "minute".to_string(),
                    retry_after: None,
                    reason: None,
                });
            }
        };

        // Determine which rate limit to use (prefer minute, then hour, then day)
        let (max_requests, window_type, window_seconds) = if config.rate_limit_per_minute > 0 {
            (config.rate_limit_per_minute as u64, "minute".to_string(), 60)
        } else if let Some(per_hour) = config.rate_limit_per_hour {
            if per_hour > 0 {
                (per_hour as u64, "hour".to_string(), 3600)
            } else if let Some(per_day) = config.rate_limit_per_day {
                if per_day > 0 {
                    (per_day as u64, "day".to_string(), 86400)
                } else {
                    (1000, "minute".to_string(), 60) // Default fallback
                }
            } else {
                (1000, "minute".to_string(), 60) // Default fallback
            }
        } else if let Some(per_day) = config.rate_limit_per_day {
            if per_day > 0 {
                (per_day as u64, "day".to_string(), 86400)
            } else {
                (1000, "minute".to_string(), 60) // Default fallback
            }
        } else {
            (1000, "minute".to_string(), 60) // Default fallback
        };

        // Calculate current window start time
        let now = Utc::now();
        let window_start = now.timestamp() / window_seconds * window_seconds;
        let window_start_dt = DateTime::<Utc>::from_timestamp(window_start, 0)
            .ok_or_else(|| crate::Error::generic("Invalid timestamp".to_string()))?;

        // Count requests in current window
        // We'll use a usage tracking table or create one if it doesn't exist
        // For now, we'll track in org_ai_usage table with a window_start column
        let current_requests: i64 = if let Some(ws_uuid) = workspace_uuid {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM org_ai_usage
                WHERE org_id = $1
                  AND workspace_id = $2
                  AND created_at >= $3
                "#
            )
            .bind(org_uuid)
            .bind(ws_uuid)
            .bind(window_start_dt)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0)
        } else {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM org_ai_usage
                WHERE org_id = $1
                  AND workspace_id IS NULL
                  AND created_at >= $3
                "#
            )
            .bind(org_uuid)
            .bind(window_start_dt)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0)
        };

        let current_requests = current_requests as u64;

        // Check if rate limit is exceeded
        if current_requests >= max_requests {
            // Calculate retry after (next window start)
            let next_window_start = window_start + window_seconds;
            let retry_after = DateTime::<Utc>::from_timestamp(next_window_start, 0)
                .ok_or_else(|| crate::Error::generic("Invalid timestamp".to_string()))?;

            Ok(RateLimitCheckResult {
                allowed: false,
                current_requests,
                max_requests,
                window_type: window_type.clone(),
                retry_after: Some(retry_after),
                reason: Some(format!(
                    "Rate limit exceeded: {}/{} requests in current {} window",
                    current_requests, max_requests, window_type
                )),
            })
        } else {
            Ok(RateLimitCheckResult {
                allowed: true,
                current_requests,
                max_requests,
                window_type: window_type.clone(),
                retry_after: None,
                reason: None,
            })
        }
    }

    /// Check if a feature is enabled
    async fn is_feature_enabled(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
        feature: &str,
    ) -> Result<bool> {
        let org_uuid = Uuid::parse_str(org_id)
            .map_err(|e| crate::Error::generic(format!("Invalid org_id: {}", e)))?;
        let workspace_uuid = workspace_id
            .and_then(|w| Uuid::parse_str(w).ok());

        let result = if let Some(ws_uuid) = workspace_uuid {
            sqlx::query(
                "SELECT enabled FROM org_ai_feature_toggles WHERE org_id = $1 AND workspace_id = $2 AND feature_name = $3"
            )
            .bind(org_uuid)
            .bind(ws_uuid)
            .bind(feature)
            .fetch_optional(&self.pool)
            .await
        } else {
            sqlx::query(
                "SELECT enabled FROM org_ai_feature_toggles WHERE org_id = $1 AND workspace_id IS NULL AND feature_name = $2"
            )
            .bind(org_uuid)
            .bind(feature)
            .fetch_optional(&self.pool)
            .await
        };

        match result {
            Ok(Some(row)) => Ok(row.get::<bool, _>("enabled")),
            Ok(None) => Ok(true), // Default to enabled if not configured
            Err(e) => Err(crate::Error::generic(format!("Failed to check feature: {}", e))),
        }
    }

    /// Record usage for audit
    async fn record_usage(
        &self,
        org_id: &str,
        workspace_id: Option<&str>,
        user_id: Option<&str>,
        feature: AiFeature,
        tokens: u64,
        cost_usd: f64,
        metadata: Option<Value>,
    ) -> Result<()> {
        let org_uuid = Uuid::parse_str(org_id)
            .map_err(|e| crate::Error::generic(format!("Invalid org_id: {}", e)))?;
        let workspace_uuid = workspace_id
            .and_then(|w| Uuid::parse_str(w).ok());
        let user_uuid = user_id
            .and_then(|u| Uuid::parse_str(u).ok());

        let feature_name = match feature {
            AiFeature::MockAi => "mock_generation",
            AiFeature::ContractDiff => "contract_diff",
            AiFeature::PersonaGeneration => "persona_generation",
            AiFeature::DebugAnalysis => "debug_analysis",
            AiFeature::GenerativeSchema => "generative_schema",
            AiFeature::VoiceInterface => "voice_interface",
            AiFeature::GeneralChat => "free_form_generation",
        };

        // Insert usage log
        sqlx::query(
            "INSERT INTO org_ai_usage_logs (org_id, workspace_id, user_id, feature_name, tokens_used, cost_usd, metadata)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(org_uuid)
        .bind(workspace_uuid)
        .bind(user_uuid)
        .bind(feature_name)
        .bind(tokens as i64)
        .bind(cost_usd as f64)
        .bind(metadata.unwrap_or_else(|| serde_json::json!({})))
        .execute(&self.pool)
        .await
        .map_err(|e| crate::Error::generic(format!("Failed to record usage: {}", e)))?;

        // Update budget counters
        if let Some(ws_uuid) = workspace_uuid {
            sqlx::query(
                "UPDATE org_ai_budgets
                 SET current_tokens_used = current_tokens_used + $1,
                     current_calls_used = current_calls_used + 1,
                     updated_at = NOW()
                 WHERE org_id = $2 AND workspace_id = $3"
            )
            .bind(tokens as i64)
            .bind(org_uuid)
            .bind(ws_uuid)
            .execute(&self.pool)
            .await
            .map_err(|e| crate::Error::generic(format!("Failed to update budget: {}", e)))?;
        } else {
            sqlx::query(
                "UPDATE org_ai_budgets
                 SET current_tokens_used = current_tokens_used + $1,
                     current_calls_used = current_calls_used + 1,
                     updated_at = NOW()
                 WHERE org_id = $2 AND workspace_id IS NULL"
            )
            .bind(tokens as i64)
            .bind(org_uuid)
            .execute(&self.pool)
            .await
            .map_err(|e| crate::Error::generic(format!("Failed to update budget: {}", e)))?;
        }

        Ok(())
    }
}

/// Database row for org_ai_budgets table
#[cfg(feature = "database")]
#[derive(sqlx::FromRow)]
struct BudgetRow {
    id: Uuid,
    org_id: Uuid,
    workspace_id: Option<Uuid>,
    max_tokens_per_period: i64,
    period_type: String,
    max_calls_per_period: i64,
    current_tokens_used: i64,
    current_calls_used: i64,
    period_start: DateTime<Utc>,
}

/// Database row for org_ai_rate_limits table
#[cfg(feature = "database")]
#[derive(sqlx::FromRow)]
struct RateLimitRow {
    id: Uuid,
    org_id: Uuid,
    workspace_id: Option<Uuid>,
    rate_limit_per_minute: i32,
    rate_limit_per_hour: Option<i32>,
    rate_limit_per_day: Option<i32>,
}

/// Database row for org_ai_feature_toggles table
#[cfg(feature = "database")]
#[derive(sqlx::FromRow)]
struct FeatureToggleRow {
    id: Uuid,
    org_id: Uuid,
    workspace_id: Option<Uuid>,
    feature_name: String,
    enabled: bool,
}
