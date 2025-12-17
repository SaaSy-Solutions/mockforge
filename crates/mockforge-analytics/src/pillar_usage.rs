//! Pillar usage tracking
//!
//! Tracks usage of `MockForge` pillars (Reality, Contracts, `DevX`, Cloud, AI)
//! to help users understand platform adoption and identify under-utilized features.

use crate::database::AnalyticsDatabase;
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Pillar name
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Pillar {
    /// Reality pillar - everything that makes mocks feel like a real, evolving backend
    Reality,
    /// Contracts pillar - schema, drift, validation, and safety nets
    Contracts,
    /// `DevX` pillar - SDKs, generators, playgrounds, ergonomics
    DevX,
    /// Cloud pillar - registry, orgs, governance, monetization, marketplace
    Cloud,
    /// AI pillar - LLM/voice flows, AI diff/assist, generative behaviors
    Ai,
}

impl Pillar {
    /// Convert to string
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Reality => "reality",
            Self::Contracts => "contracts",
            Self::DevX => "devx",
            Self::Cloud => "cloud",
            Self::Ai => "ai",
        }
    }

    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "reality" => Some(Self::Reality),
            "contracts" => Some(Self::Contracts),
            "devx" => Some(Self::DevX),
            "cloud" => Some(Self::Cloud),
            "ai" => Some(Self::Ai),
            _ => None,
        }
    }
}

impl std::fmt::Display for Pillar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Pillar usage event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PillarUsageEvent {
    /// Workspace ID (optional)
    pub workspace_id: Option<String>,
    /// Organization ID (optional)
    pub org_id: Option<String>,
    /// Pillar name
    pub pillar: Pillar,
    /// Metric name (e.g., "`blended_reality_ratio`", "`smart_personas_usage`", "`validation_mode`")
    pub metric_name: String,
    /// Metric value (JSON)
    pub metric_value: Value,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Pillar usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PillarUsageMetrics {
    /// Workspace ID
    pub workspace_id: Option<String>,
    /// Organization ID
    pub org_id: Option<String>,
    /// Time range
    pub time_range: String,
    /// Reality pillar metrics
    pub reality: Option<RealityPillarMetrics>,
    /// Contracts pillar metrics
    pub contracts: Option<ContractsPillarMetrics>,
    /// `DevX` pillar metrics
    pub devx: Option<DevXPillarMetrics>,
    /// Cloud pillar metrics
    pub cloud: Option<CloudPillarMetrics>,
    /// AI pillar metrics
    pub ai: Option<AiPillarMetrics>,
}

/// Reality pillar metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityPillarMetrics {
    /// Percentage of requests using blended reality (reality continuum)
    pub blended_reality_percent: f64,
    /// Percentage of scenarios using Smart Personas
    pub smart_personas_percent: f64,
    /// Percentage of scenarios using static fixtures
    pub static_fixtures_percent: f64,
    /// Average reality level (1-5)
    pub avg_reality_level: f64,
    /// Number of scenarios with chaos enabled
    pub chaos_enabled_count: u64,
    /// Total number of scenarios
    pub total_scenarios: u64,
}

/// Contracts pillar metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractsPillarMetrics {
    /// Percentage of requests with validation disabled
    pub validation_disabled_percent: f64,
    /// Percentage of requests with validation in warn mode
    pub validation_warn_percent: f64,
    /// Percentage of requests with validation in enforce mode
    pub validation_enforce_percent: f64,
    /// Number of endpoints with drift budgets configured
    pub drift_budget_configured_count: u64,
    /// Number of drift incidents
    pub drift_incidents_count: u64,
    /// Number of contract sync cycles
    pub contract_sync_cycles: u64,
}

/// `DevX` pillar metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevXPillarMetrics {
    /// Number of SDK installations
    pub sdk_installations: u64,
    /// Number of client code generations
    pub client_generations: u64,
    /// Number of playground sessions
    pub playground_sessions: u64,
    /// Number of CLI commands executed
    pub cli_commands: u64,
}

/// Cloud pillar metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudPillarMetrics {
    /// Number of shared scenarios
    pub shared_scenarios_count: u64,
    /// Number of marketplace downloads
    pub marketplace_downloads: u64,
    /// Number of org templates used
    pub org_templates_used: u64,
    /// Number of collaborative workspaces
    pub collaborative_workspaces: u64,
}

/// AI pillar metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPillarMetrics {
    /// Number of AI-generated mocks
    pub ai_generated_mocks: u64,
    /// Number of AI contract diffs
    pub ai_contract_diffs: u64,
    /// Number of voice commands
    pub voice_commands: u64,
    /// Number of LLM-assisted operations
    pub llm_assisted_operations: u64,
}

impl AnalyticsDatabase {
    /// Record a pillar usage event
    pub async fn record_pillar_usage(&self, event: &PillarUsageEvent) -> Result<()> {
        let timestamp = event.timestamp.timestamp();
        let metric_value_json = serde_json::to_string(&event.metric_value)?;

        sqlx::query(
            r"
            INSERT INTO pillar_usage_events (
                workspace_id, org_id, pillar, metric_name, metric_value, timestamp
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ",
        )
        .bind(event.workspace_id.as_deref())
        .bind(event.org_id.as_deref())
        .bind(event.pillar.as_str())
        .bind(&event.metric_name)
        .bind(&metric_value_json)
        .bind(timestamp)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    /// Get pillar usage metrics for a workspace
    pub async fn get_workspace_pillar_metrics(
        &self,
        workspace_id: &str,
        duration_seconds: i64,
    ) -> Result<PillarUsageMetrics> {
        let end_time = Utc::now().timestamp();
        let start_time = end_time - duration_seconds;

        // Get reality pillar metrics
        let reality = self
            .get_reality_pillar_metrics(Some(workspace_id), None, start_time, end_time)
            .await?;

        // Get contracts pillar metrics
        let contracts = self
            .get_contracts_pillar_metrics(Some(workspace_id), None, start_time, end_time)
            .await?;

        // Get DevX pillar metrics
        let devx = self
            .get_devx_pillar_metrics(Some(workspace_id), None, start_time, end_time)
            .await?;

        // Get Cloud pillar metrics
        let cloud = self
            .get_cloud_pillar_metrics(Some(workspace_id), None, start_time, end_time)
            .await?;

        // Get AI pillar metrics
        let ai = self
            .get_ai_pillar_metrics(Some(workspace_id), None, start_time, end_time)
            .await?;

        Ok(PillarUsageMetrics {
            workspace_id: Some(workspace_id.to_string()),
            org_id: None,
            time_range: format!("{duration_seconds}s"),
            reality: Some(reality),
            contracts: Some(contracts),
            devx: Some(devx),
            cloud: Some(cloud),
            ai: Some(ai),
        })
    }

    /// Get pillar usage metrics for an organization
    pub async fn get_org_pillar_metrics(
        &self,
        org_id: &str,
        duration_seconds: i64,
    ) -> Result<PillarUsageMetrics> {
        let end_time = Utc::now().timestamp();
        let start_time = end_time - duration_seconds;

        // Get reality pillar metrics
        let reality = self
            .get_reality_pillar_metrics(None, Some(org_id), start_time, end_time)
            .await?;

        // Get contracts pillar metrics
        let contracts = self
            .get_contracts_pillar_metrics(None, Some(org_id), start_time, end_time)
            .await?;

        // Get DevX pillar metrics
        let devx = self.get_devx_pillar_metrics(None, Some(org_id), start_time, end_time).await?;

        // Get Cloud pillar metrics
        let cloud = self.get_cloud_pillar_metrics(None, Some(org_id), start_time, end_time).await?;

        // Get AI pillar metrics
        let ai = self.get_ai_pillar_metrics(None, Some(org_id), start_time, end_time).await?;

        Ok(PillarUsageMetrics {
            workspace_id: None,
            org_id: Some(org_id.to_string()),
            time_range: format!("{duration_seconds}s"),
            reality: Some(reality),
            contracts: Some(contracts),
            devx: Some(devx),
            cloud: Some(cloud),
            ai: Some(ai),
        })
    }

    /// Get reality pillar metrics
    async fn get_reality_pillar_metrics(
        &self,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
        start_time: i64,
        end_time: i64,
    ) -> Result<RealityPillarMetrics> {
        // Query blended reality usage
        let blended_reality_query = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, f64>(
                r"
                SELECT AVG(CAST(json_extract(metric_value, '$.ratio') AS REAL)) * 100.0
                FROM pillar_usage_events
                WHERE pillar = 'reality'
                AND metric_name = 'blended_reality_ratio'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, f64>(
                r"
                SELECT AVG(CAST(json_extract(metric_value, '$.ratio') AS REAL)) * 100.0
                FROM pillar_usage_events
                WHERE pillar = 'reality'
                AND metric_name = 'blended_reality_ratio'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
        } else {
            return Err(crate::error::AnalyticsError::InvalidInput(
                "Either workspace_id or org_id must be provided".to_string(),
            ));
        };

        let blended_reality_percent =
            blended_reality_query.fetch_one(self.pool()).await.unwrap_or(0.0);

        // Query Smart Personas vs static fixtures
        let smart_personas_query = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*) FROM pillar_usage_events
                WHERE pillar = 'reality'
                AND metric_name = 'persona_usage'
                AND json_extract(metric_value, '$.type') = 'smart'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*) FROM pillar_usage_events
                WHERE pillar = 'reality'
                AND metric_name = 'persona_usage'
                AND json_extract(metric_value, '$.type') = 'smart'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
        } else {
            return Err(crate::error::AnalyticsError::InvalidInput(
                "Either workspace_id or org_id must be provided".to_string(),
            ));
        };

        let smart_personas_count = smart_personas_query.fetch_one(self.pool()).await.unwrap_or(0);

        let static_fixtures_query = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*) FROM pillar_usage_events
                WHERE pillar = 'reality'
                AND metric_name = 'persona_usage'
                AND json_extract(metric_value, '$.type') = 'static'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*) FROM pillar_usage_events
                WHERE pillar = 'reality'
                AND metric_name = 'persona_usage'
                AND json_extract(metric_value, '$.type') = 'static'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
        } else {
            return Err(crate::error::AnalyticsError::InvalidInput(
                "Either workspace_id or org_id must be provided".to_string(),
            ));
        };

        let static_fixtures_count = static_fixtures_query.fetch_one(self.pool()).await.unwrap_or(0);

        let total = smart_personas_count + static_fixtures_count;
        let smart_personas_percent = if total > 0 {
            (smart_personas_count as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let static_fixtures_percent = if total > 0 {
            (static_fixtures_count as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        // Query average reality level
        let avg_reality_level = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, f64>(
                r"
                SELECT AVG(CAST(json_extract(metric_value, '$.level') AS REAL))
                FROM pillar_usage_events
                WHERE pillar = 'reality'
                AND metric_name = 'reality_level'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0.0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, f64>(
                r"
                SELECT AVG(CAST(json_extract(metric_value, '$.level') AS REAL))
                FROM pillar_usage_events
                WHERE pillar = 'reality'
                AND metric_name = 'reality_level'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0.0)
        } else {
            0.0
        };

        // Query chaos enabled count
        let chaos_enabled_count = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.scenario_id'))
                FROM pillar_usage_events
                WHERE pillar = 'reality'
                AND metric_name = 'chaos_injection'
                AND json_extract(metric_value, '$.enabled') = 1
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.scenario_id'))
                FROM pillar_usage_events
                WHERE pillar = 'reality'
                AND metric_name = 'chaos_injection'
                AND json_extract(metric_value, '$.enabled') = 1
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        Ok(RealityPillarMetrics {
            blended_reality_percent,
            smart_personas_percent,
            static_fixtures_percent,
            avg_reality_level,
            chaos_enabled_count: chaos_enabled_count as u64,
            total_scenarios: total as u64,
        })
    }

    /// Get contracts pillar metrics
    async fn get_contracts_pillar_metrics(
        &self,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
        start_time: i64,
        end_time: i64,
    ) -> Result<ContractsPillarMetrics> {
        // Query validation mode usage
        let validation_disabled_query = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*) FROM pillar_usage_events
                WHERE pillar = 'contracts'
                AND metric_name = 'validation_mode'
                AND json_extract(metric_value, '$.mode') = 'disabled'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*) FROM pillar_usage_events
                WHERE pillar = 'contracts'
                AND metric_name = 'validation_mode'
                AND json_extract(metric_value, '$.mode') = 'disabled'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
        } else {
            return Err(crate::error::AnalyticsError::InvalidInput(
                "Either workspace_id or org_id must be provided".to_string(),
            ));
        };

        let validation_disabled_count =
            validation_disabled_query.fetch_one(self.pool()).await.unwrap_or(0);

        let validation_warn_query = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*) FROM pillar_usage_events
                WHERE pillar = 'contracts'
                AND metric_name = 'validation_mode'
                AND json_extract(metric_value, '$.mode') = 'warn'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*) FROM pillar_usage_events
                WHERE pillar = 'contracts'
                AND metric_name = 'validation_mode'
                AND json_extract(metric_value, '$.mode') = 'warn'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
        } else {
            return Err(crate::error::AnalyticsError::InvalidInput(
                "Either workspace_id or org_id must be provided".to_string(),
            ));
        };

        let validation_warn_count = validation_warn_query.fetch_one(self.pool()).await.unwrap_or(0);

        let validation_enforce_query = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*) FROM pillar_usage_events
                WHERE pillar = 'contracts'
                AND metric_name = 'validation_mode'
                AND json_extract(metric_value, '$.mode') = 'enforce'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*) FROM pillar_usage_events
                WHERE pillar = 'contracts'
                AND metric_name = 'validation_mode'
                AND json_extract(metric_value, '$.mode') = 'enforce'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
        } else {
            return Err(crate::error::AnalyticsError::InvalidInput(
                "Either workspace_id or org_id must be provided".to_string(),
            ));
        };

        let validation_enforce_count =
            validation_enforce_query.fetch_one(self.pool()).await.unwrap_or(0);

        let total_validation_events =
            validation_disabled_count + validation_warn_count + validation_enforce_count;
        let validation_disabled_percent = if total_validation_events > 0 {
            (validation_disabled_count as f64 / total_validation_events as f64) * 100.0
        } else {
            0.0
        };
        let validation_warn_percent = if total_validation_events > 0 {
            (validation_warn_count as f64 / total_validation_events as f64) * 100.0
        } else {
            0.0
        };
        let validation_enforce_percent = if total_validation_events > 0 {
            (validation_enforce_count as f64 / total_validation_events as f64) * 100.0
        } else {
            0.0
        };

        // Query drift budget configured count
        let drift_budget_configured_count = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.endpoint'))
                FROM pillar_usage_events
                WHERE pillar = 'contracts'
                AND metric_name = 'drift_budget_configured'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.endpoint'))
                FROM pillar_usage_events
                WHERE pillar = 'contracts'
                AND metric_name = 'drift_budget_configured'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        // Query drift incidents count
        let drift_incidents_count = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'contracts'
                AND metric_name = 'drift_detection'
                AND json_extract(metric_value, '$.incident') = 1
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'contracts'
                AND metric_name = 'drift_detection'
                AND json_extract(metric_value, '$.incident') = 1
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        // Query contract sync cycles
        let contract_sync_cycles = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.sync_id'))
                FROM pillar_usage_events
                WHERE pillar = 'contracts'
                AND metric_name = 'contract_sync'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.sync_id'))
                FROM pillar_usage_events
                WHERE pillar = 'contracts'
                AND metric_name = 'contract_sync'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        Ok(ContractsPillarMetrics {
            validation_disabled_percent,
            validation_warn_percent,
            validation_enforce_percent,
            drift_budget_configured_count: drift_budget_configured_count as u64,
            drift_incidents_count: drift_incidents_count as u64,
            contract_sync_cycles: contract_sync_cycles as u64,
        })
    }

    /// Get `DevX` pillar metrics
    async fn get_devx_pillar_metrics(
        &self,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
        start_time: i64,
        end_time: i64,
    ) -> Result<DevXPillarMetrics> {
        // Query SDK installations
        let sdk_installations = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.sdk_type'))
                FROM pillar_usage_events
                WHERE pillar = 'devx'
                AND metric_name = 'sdk_installation'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.sdk_type'))
                FROM pillar_usage_events
                WHERE pillar = 'devx'
                AND metric_name = 'sdk_installation'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        // Query client generations
        let client_generations = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'devx'
                AND metric_name = 'client_generation'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'devx'
                AND metric_name = 'client_generation'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        // Query playground sessions
        let playground_sessions = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.session_id'))
                FROM pillar_usage_events
                WHERE pillar = 'devx'
                AND metric_name = 'playground_session'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.session_id'))
                FROM pillar_usage_events
                WHERE pillar = 'devx'
                AND metric_name = 'playground_session'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        // Query CLI commands
        let cli_commands = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'devx'
                AND metric_name = 'cli_command'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'devx'
                AND metric_name = 'cli_command'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        Ok(DevXPillarMetrics {
            sdk_installations: sdk_installations as u64,
            client_generations: client_generations as u64,
            playground_sessions: playground_sessions as u64,
            cli_commands: cli_commands as u64,
        })
    }

    /// Get Cloud pillar metrics
    async fn get_cloud_pillar_metrics(
        &self,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
        start_time: i64,
        end_time: i64,
    ) -> Result<CloudPillarMetrics> {
        // Query shared scenarios count
        let shared_scenarios_count = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.scenario_id'))
                FROM pillar_usage_events
                WHERE pillar = 'cloud'
                AND metric_name = 'scenario_shared'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.scenario_id'))
                FROM pillar_usage_events
                WHERE pillar = 'cloud'
                AND metric_name = 'scenario_shared'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        // Query marketplace downloads
        let marketplace_downloads = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'cloud'
                AND metric_name = 'marketplace_download'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'cloud'
                AND metric_name = 'marketplace_download'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        // Query org templates used
        let org_templates_used = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.template_id'))
                FROM pillar_usage_events
                WHERE pillar = 'cloud'
                AND metric_name = 'template_use'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.template_id'))
                FROM pillar_usage_events
                WHERE pillar = 'cloud'
                AND metric_name = 'template_use'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        // Query collaborative workspaces
        let collaborative_workspaces = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.workspace_id'))
                FROM pillar_usage_events
                WHERE pillar = 'cloud'
                AND metric_name = 'workspace_creation'
                AND json_extract(metric_value, '$.collaborative') = 1
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(DISTINCT json_extract(metadata, '$.workspace_id'))
                FROM pillar_usage_events
                WHERE pillar = 'cloud'
                AND metric_name = 'workspace_creation'
                AND json_extract(metric_value, '$.collaborative') = 1
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        Ok(CloudPillarMetrics {
            shared_scenarios_count: shared_scenarios_count as u64,
            marketplace_downloads: marketplace_downloads as u64,
            org_templates_used: org_templates_used as u64,
            collaborative_workspaces: collaborative_workspaces as u64,
        })
    }

    /// Get AI pillar metrics
    async fn get_ai_pillar_metrics(
        &self,
        workspace_id: Option<&str>,
        org_id: Option<&str>,
        start_time: i64,
        end_time: i64,
    ) -> Result<AiPillarMetrics> {
        // Query AI-generated mocks
        let ai_generated_mocks = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'ai'
                AND metric_name = 'ai_generation'
                AND json_extract(metric_value, '$.type') = 'mock'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'ai'
                AND metric_name = 'ai_generation'
                AND json_extract(metric_value, '$.type') = 'mock'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        // Query AI contract diffs
        let ai_contract_diffs = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'ai'
                AND metric_name = 'ai_generation'
                AND json_extract(metric_value, '$.type') = 'contract_diff'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'ai'
                AND metric_name = 'ai_generation'
                AND json_extract(metric_value, '$.type') = 'contract_diff'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        // Query AI refinements
        let ai_refinements = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'ai'
                AND metric_name = 'ai_refinement'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'ai'
                AND metric_name = 'ai_refinement'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        // Query voice commands
        let voice_commands = if let Some(ws_id) = workspace_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'ai'
                AND metric_name = 'voice_command'
                AND workspace_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(ws_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else if let Some(org) = org_id {
            sqlx::query_scalar::<_, i64>(
                r"
                SELECT COUNT(*)
                FROM pillar_usage_events
                WHERE pillar = 'ai'
                AND metric_name = 'voice_command'
                AND org_id = $1
                AND timestamp >= $2 AND timestamp <= $3
                ",
            )
            .bind(org)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(self.pool())
            .await
            .unwrap_or(0)
        } else {
            0
        };

        // LLM-assisted operations includes all AI generations, diffs, and refinements
        let llm_assisted_operations = ai_generated_mocks + ai_contract_diffs + ai_refinements;

        Ok(AiPillarMetrics {
            ai_generated_mocks: ai_generated_mocks as u64,
            ai_contract_diffs: ai_contract_diffs as u64,
            voice_commands: voice_commands as u64,
            llm_assisted_operations: llm_assisted_operations as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pillar_as_str() {
        assert_eq!(Pillar::Reality.as_str(), "reality");
        assert_eq!(Pillar::Contracts.as_str(), "contracts");
        assert_eq!(Pillar::DevX.as_str(), "devx");
        assert_eq!(Pillar::Cloud.as_str(), "cloud");
        assert_eq!(Pillar::Ai.as_str(), "ai");
    }

    #[test]
    fn test_pillar_from_str() {
        assert_eq!(Pillar::from_str("reality"), Some(Pillar::Reality));
        assert_eq!(Pillar::from_str("contracts"), Some(Pillar::Contracts));
        assert_eq!(Pillar::from_str("devx"), Some(Pillar::DevX));
        assert_eq!(Pillar::from_str("cloud"), Some(Pillar::Cloud));
        assert_eq!(Pillar::from_str("ai"), Some(Pillar::Ai));
        assert_eq!(Pillar::from_str("unknown"), None);
    }

    #[test]
    fn test_pillar_from_str_case_insensitive() {
        assert_eq!(Pillar::from_str("REALITY"), Some(Pillar::Reality));
        assert_eq!(Pillar::from_str("Reality"), Some(Pillar::Reality));
        assert_eq!(Pillar::from_str("DEVX"), Some(Pillar::DevX));
        assert_eq!(Pillar::from_str("AI"), Some(Pillar::Ai));
    }

    #[test]
    fn test_pillar_display() {
        assert_eq!(format!("{}", Pillar::Reality), "reality");
        assert_eq!(format!("{}", Pillar::Contracts), "contracts");
        assert_eq!(format!("{}", Pillar::DevX), "devx");
        assert_eq!(format!("{}", Pillar::Cloud), "cloud");
        assert_eq!(format!("{}", Pillar::Ai), "ai");
    }

    #[test]
    fn test_pillar_serialize() {
        let pillar = Pillar::Reality;
        let json = serde_json::to_string(&pillar).unwrap();
        assert_eq!(json, "\"reality\"");
    }

    #[test]
    fn test_pillar_deserialize() {
        let pillar: Pillar = serde_json::from_str("\"contracts\"").unwrap();
        assert_eq!(pillar, Pillar::Contracts);
    }

    #[test]
    fn test_pillar_clone() {
        let pillar = Pillar::DevX;
        let cloned = pillar;
        assert_eq!(pillar, cloned);
    }

    #[test]
    fn test_pillar_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Pillar::Reality);
        set.insert(Pillar::Contracts);
        set.insert(Pillar::Reality); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_pillar_usage_event_serialize() {
        let event = PillarUsageEvent {
            workspace_id: Some("ws-123".to_string()),
            org_id: None,
            pillar: Pillar::Reality,
            metric_name: "blended_reality_ratio".to_string(),
            metric_value: serde_json::json!({"ratio": 0.75}),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ws-123"));
        assert!(json.contains("reality"));
        assert!(json.contains("blended_reality_ratio"));
    }

    #[test]
    fn test_pillar_usage_event_deserialize() {
        let json = r#"{
            "workspace_id": "ws-456",
            "org_id": null,
            "pillar": "contracts",
            "metric_name": "validation_mode",
            "metric_value": {"mode": "enforce"},
            "timestamp": "2024-01-15T12:00:00Z"
        }"#;
        let event: PillarUsageEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.workspace_id, Some("ws-456".to_string()));
        assert_eq!(event.pillar, Pillar::Contracts);
        assert_eq!(event.metric_name, "validation_mode");
    }

    #[test]
    fn test_reality_pillar_metrics_serialize() {
        let metrics = RealityPillarMetrics {
            blended_reality_percent: 75.0,
            smart_personas_percent: 60.0,
            static_fixtures_percent: 40.0,
            avg_reality_level: 3.5,
            chaos_enabled_count: 5,
            total_scenarios: 100,
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("blended_reality_percent"));
        assert!(json.contains("75.0"));
    }

    #[test]
    fn test_contracts_pillar_metrics_serialize() {
        let metrics = ContractsPillarMetrics {
            validation_disabled_percent: 10.0,
            validation_warn_percent: 30.0,
            validation_enforce_percent: 60.0,
            drift_budget_configured_count: 15,
            drift_incidents_count: 3,
            contract_sync_cycles: 42,
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("validation_enforce_percent"));
        assert!(json.contains("60.0"));
    }

    #[test]
    fn test_devx_pillar_metrics_serialize() {
        let metrics = DevXPillarMetrics {
            sdk_installations: 100,
            client_generations: 50,
            playground_sessions: 200,
            cli_commands: 1000,
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("sdk_installations"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_cloud_pillar_metrics_serialize() {
        let metrics = CloudPillarMetrics {
            shared_scenarios_count: 25,
            marketplace_downloads: 500,
            org_templates_used: 10,
            collaborative_workspaces: 5,
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("marketplace_downloads"));
        assert!(json.contains("500"));
    }

    #[test]
    fn test_ai_pillar_metrics_serialize() {
        let metrics = AiPillarMetrics {
            ai_generated_mocks: 100,
            ai_contract_diffs: 50,
            voice_commands: 25,
            llm_assisted_operations: 175,
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("ai_generated_mocks"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_pillar_usage_metrics_serialize() {
        let metrics = PillarUsageMetrics {
            workspace_id: Some("ws-123".to_string()),
            org_id: None,
            time_range: "3600s".to_string(),
            reality: Some(RealityPillarMetrics {
                blended_reality_percent: 50.0,
                smart_personas_percent: 75.0,
                static_fixtures_percent: 25.0,
                avg_reality_level: 4.0,
                chaos_enabled_count: 2,
                total_scenarios: 50,
            }),
            contracts: None,
            devx: None,
            cloud: None,
            ai: None,
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("ws-123"));
        assert!(json.contains("3600s"));
        assert!(json.contains("blended_reality_percent"));
    }

    #[test]
    fn test_pillar_usage_metrics_all_pillars() {
        let metrics = PillarUsageMetrics {
            workspace_id: Some("ws-all".to_string()),
            org_id: Some("org-1".to_string()),
            time_range: "86400s".to_string(),
            reality: Some(RealityPillarMetrics {
                blended_reality_percent: 80.0,
                smart_personas_percent: 90.0,
                static_fixtures_percent: 10.0,
                avg_reality_level: 4.5,
                chaos_enabled_count: 10,
                total_scenarios: 200,
            }),
            contracts: Some(ContractsPillarMetrics {
                validation_disabled_percent: 5.0,
                validation_warn_percent: 15.0,
                validation_enforce_percent: 80.0,
                drift_budget_configured_count: 30,
                drift_incidents_count: 2,
                contract_sync_cycles: 100,
            }),
            devx: Some(DevXPillarMetrics {
                sdk_installations: 500,
                client_generations: 200,
                playground_sessions: 1000,
                cli_commands: 5000,
            }),
            cloud: Some(CloudPillarMetrics {
                shared_scenarios_count: 50,
                marketplace_downloads: 1000,
                org_templates_used: 20,
                collaborative_workspaces: 15,
            }),
            ai: Some(AiPillarMetrics {
                ai_generated_mocks: 300,
                ai_contract_diffs: 100,
                voice_commands: 50,
                llm_assisted_operations: 450,
            }),
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("org-1"));
        // Check all pillar sections are present
        assert!(json.contains("reality"));
        assert!(json.contains("contracts"));
        assert!(json.contains("devx"));
        assert!(json.contains("cloud"));
        assert!(json.contains("ai"));
    }

    #[test]
    fn test_pillar_debug() {
        let pillar = Pillar::Reality;
        let debug = format!("{:?}", pillar);
        assert!(debug.contains("Reality"));
    }

    #[test]
    fn test_reality_pillar_metrics_clone() {
        let metrics = RealityPillarMetrics {
            blended_reality_percent: 50.0,
            smart_personas_percent: 60.0,
            static_fixtures_percent: 40.0,
            avg_reality_level: 3.0,
            chaos_enabled_count: 1,
            total_scenarios: 10,
        };
        let cloned = metrics.clone();
        assert_eq!(metrics.blended_reality_percent, cloned.blended_reality_percent);
        assert_eq!(metrics.total_scenarios, cloned.total_scenarios);
    }

    #[test]
    fn test_pillar_usage_event_clone() {
        let event = PillarUsageEvent {
            workspace_id: Some("ws-test".to_string()),
            org_id: None,
            pillar: Pillar::Ai,
            metric_name: "ai_generation".to_string(),
            metric_value: serde_json::json!({"type": "mock"}),
            timestamp: Utc::now(),
        };
        let cloned = event.clone();
        assert_eq!(event.workspace_id, cloned.workspace_id);
        assert_eq!(event.pillar, cloned.pillar);
        assert_eq!(event.metric_name, cloned.metric_name);
    }
}
