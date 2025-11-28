//! SSO (Single Sign-On) configuration model
//!
//! Supports SAML 2.0 SSO for Team plan organizations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// SSO provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SSOProvider {
    Saml,
    Oidc,
}

impl SSOProvider {
    pub fn to_string(&self) -> &'static str {
        match self {
            SSOProvider::Saml => "saml",
            SSOProvider::Oidc => "oidc",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "saml" => Some(SSOProvider::Saml),
            "oidc" => Some(SSOProvider::Oidc),
            _ => None,
        }
    }
}

/// SSO configuration for an organization
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SSOConfiguration {
    pub id: Uuid,
    pub org_id: Uuid,
    pub provider: String, // "saml" or "oidc"
    pub enabled: bool,

    // SAML 2.0 fields
    pub saml_entity_id: Option<String>,
    pub saml_sso_url: Option<String>,
    pub saml_slo_url: Option<String>,
    pub saml_x509_cert: Option<String>,
    pub saml_name_id_format: Option<String>,

    // OIDC fields (for future use)
    pub oidc_issuer_url: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,

    // Attribute mapping
    pub attribute_mapping: serde_json::Value,

    // Security settings
    pub require_signed_assertions: bool,
    pub require_signed_responses: bool,
    pub allow_unsolicited_responses: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// SSO session
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SSOSession {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub session_index: Option<String>,
    pub name_id: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl SSOConfiguration {
    /// Get provider as enum
    pub fn provider(&self) -> SSOProvider {
        SSOProvider::from_str(&self.provider).unwrap_or(SSOProvider::Saml)
    }

    /// Find SSO configuration by organization ID
    pub async fn find_by_org(
        pool: &sqlx::PgPool,
        org_id: Uuid,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM sso_configurations WHERE org_id = $1",
        )
        .bind(org_id)
        .fetch_optional(pool)
        .await
    }

    /// Create or update SSO configuration
    pub async fn upsert(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        provider: SSOProvider,
        saml_entity_id: Option<&str>,
        saml_sso_url: Option<&str>,
        saml_slo_url: Option<&str>,
        saml_x509_cert: Option<&str>,
        saml_name_id_format: Option<&str>,
        attribute_mapping: Option<serde_json::Value>,
        require_signed_assertions: bool,
        require_signed_responses: bool,
        allow_unsolicited_responses: bool,
    ) -> sqlx::Result<Self> {
        let attribute_mapping = attribute_mapping.unwrap_or_else(|| serde_json::json!({}));

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO sso_configurations (
                org_id, provider, saml_entity_id, saml_sso_url, saml_slo_url,
                saml_x509_cert, saml_name_id_format, attribute_mapping,
                require_signed_assertions, require_signed_responses, allow_unsolicited_responses
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (org_id) DO UPDATE SET
                provider = EXCLUDED.provider,
                saml_entity_id = EXCLUDED.saml_entity_id,
                saml_sso_url = EXCLUDED.saml_sso_url,
                saml_slo_url = EXCLUDED.saml_slo_url,
                saml_x509_cert = EXCLUDED.saml_x509_cert,
                saml_name_id_format = EXCLUDED.saml_name_id_format,
                attribute_mapping = EXCLUDED.attribute_mapping,
                require_signed_assertions = EXCLUDED.require_signed_assertions,
                require_signed_responses = EXCLUDED.require_signed_responses,
                allow_unsolicited_responses = EXCLUDED.allow_unsolicited_responses,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(provider.to_string())
        .bind(saml_entity_id)
        .bind(saml_sso_url)
        .bind(saml_slo_url)
        .bind(saml_x509_cert)
        .bind(saml_name_id_format)
        .bind(&attribute_mapping)
        .bind(require_signed_assertions)
        .bind(require_signed_responses)
        .bind(allow_unsolicited_responses)
        .fetch_one(pool)
        .await
    }

    /// Enable SSO for an organization
    pub async fn enable(
        pool: &sqlx::PgPool,
        org_id: Uuid,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE sso_configurations SET enabled = TRUE, updated_at = NOW() WHERE org_id = $1",
        )
        .bind(org_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Disable SSO for an organization
    pub async fn disable(
        pool: &sqlx::PgPool,
        org_id: Uuid,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE sso_configurations SET enabled = FALSE, updated_at = NOW() WHERE org_id = $1",
        )
        .bind(org_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete SSO configuration
    pub async fn delete(
        pool: &sqlx::PgPool,
        org_id: Uuid,
    ) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM sso_configurations WHERE org_id = $1")
            .bind(org_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

impl SSOSession {
    /// Create a new SSO session
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        user_id: Uuid,
        session_index: Option<&str>,
        name_id: Option<&str>,
        expires_at: DateTime<Utc>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO sso_sessions (org_id, user_id, session_index, name_id, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(session_index)
        .bind(name_id)
        .bind(expires_at)
        .fetch_one(pool)
        .await
    }

    /// Find active session by org and user
    pub async fn find_active(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        user_id: Uuid,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM sso_sessions
            WHERE org_id = $1 AND user_id = $2 AND expires_at > NOW()
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
    }

    /// Delete expired sessions
    pub async fn cleanup_expired(
        pool: &sqlx::PgPool,
    ) -> sqlx::Result<u64> {
        let result = sqlx::query("DELETE FROM sso_sessions WHERE expires_at < NOW()")
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Delete session by ID
    pub async fn delete(
        pool: &sqlx::PgPool,
        session_id: Uuid,
    ) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM sso_sessions WHERE id = $1")
            .bind(session_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
