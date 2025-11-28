//! Settings models for user and organization settings
//!
//! Handles storage and retrieval of settings like BYOK configuration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User setting record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserSetting {
    pub id: Uuid,
    pub user_id: Uuid,
    pub setting_key: String,
    pub setting_value: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Organization setting record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OrgSetting {
    pub id: Uuid,
    pub org_id: Uuid,
    pub setting_key: String,
    pub setting_value: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// BYOK configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BYOKConfig {
    pub provider: String, // 'openai', 'anthropic', 'together', 'fireworks', 'custom'
    pub api_key: String,
    pub base_url: Option<String>,
    pub enabled: bool,
}

/// Organization-level AI settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrgAiSettings {
    /// Maximum AI calls per workspace per day
    #[serde(default = "default_max_calls_per_day")]
    pub max_ai_calls_per_workspace_per_day: u64,

    /// Maximum AI calls per workspace per month
    #[serde(default = "default_max_calls_per_month")]
    pub max_ai_calls_per_workspace_per_month: u64,

    /// Feature flags for AI features
    #[serde(default)]
    pub feature_flags: AiFeatureFlags,
}

fn default_max_calls_per_day() -> u64 {
    1_000
}

fn default_max_calls_per_month() -> u64 {
    30_000
}

/// Feature flags for AI features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiFeatureFlags {
    /// Enable AI Studio
    #[serde(default = "default_true")]
    pub ai_studio_enabled: bool,

    /// Enable AI Contract Diff
    #[serde(default = "default_true")]
    pub ai_contract_diff_enabled: bool,

    /// Enable MockAI
    #[serde(default = "default_true")]
    pub mockai_enabled: bool,

    /// Enable persona generation
    #[serde(default = "default_true")]
    pub persona_generation_enabled: bool,

    /// Enable generative schema
    #[serde(default = "default_true")]
    pub generative_schema_enabled: bool,

    /// Enable voice interface
    #[serde(default = "default_true")]
    pub voice_interface_enabled: bool,
}

impl Default for AiFeatureFlags {
    fn default() -> Self {
        Self {
            ai_studio_enabled: true,
            ai_contract_diff_enabled: true,
            mockai_enabled: true,
            persona_generation_enabled: true,
            generative_schema_enabled: true,
            voice_interface_enabled: true,
        }
    }
}

fn default_true() -> bool {
    true
}

impl UserSetting {
    /// Get a user setting by key
    pub async fn get(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        setting_key: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM user_settings WHERE user_id = $1 AND setting_key = $2",
        )
        .bind(user_id)
        .bind(setting_key)
        .fetch_optional(pool)
        .await
    }

    /// Set or update a user setting
    pub async fn set(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        setting_key: &str,
        setting_value: serde_json::Value,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO user_settings (user_id, setting_key, setting_value)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, setting_key) DO UPDATE SET
                setting_value = EXCLUDED.setting_value,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(setting_key)
        .bind(setting_value)
        .fetch_one(pool)
        .await
    }

    /// Delete a user setting
    pub async fn delete(pool: &sqlx::PgPool, user_id: Uuid, setting_key: &str) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM user_settings WHERE user_id = $1 AND setting_key = $2")
            .bind(user_id)
            .bind(setting_key)
            .execute(pool)
            .await?;
        Ok(())
    }
}

impl OrgSetting {
    /// Get an organization setting by key
    pub async fn get(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        setting_key: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM org_settings WHERE org_id = $1 AND setting_key = $2",
        )
        .bind(org_id)
        .bind(setting_key)
        .fetch_optional(pool)
        .await
    }

    /// Set or update an organization setting
    pub async fn set(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        setting_key: &str,
        setting_value: serde_json::Value,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO org_settings (org_id, setting_key, setting_value)
            VALUES ($1, $2, $3)
            ON CONFLICT (org_id, setting_key) DO UPDATE SET
                setting_value = EXCLUDED.setting_value,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(setting_key)
        .bind(setting_value)
        .fetch_one(pool)
        .await
    }

    /// Delete an organization setting
    pub async fn delete(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        setting_key: &str,
    ) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM org_settings WHERE org_id = $1 AND setting_key = $2")
            .bind(org_id)
            .bind(setting_key)
            .execute(pool)
            .await?;
        Ok(())
    }
}
